// This file is part of StorageHub.

// Copyright (C) Moonsong Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Migration from storage version 1 to 2.
//!
//! This migration inlines the `StorageRequestBsps` double map into `StorageRequestMetadata`
//! as a `bsps: BoundedBTreeMap<ProviderIdFor<T>, bool, MaxReplicationTarget<T>>` field.
//! After migration, the `StorageRequestBsps` storage is drained per request and no longer used.

use crate::{
    pallet::Pallet,
    types::{
        BalanceOf, BucketIdFor, FileLocation, Fingerprint, MaxReplicationTarget, MerkleHash,
        MspStorageRequestStatus, PeerIds, ProviderIdFor, ReplicationTargetType, StorageDataUnit,
        StorageRequestMetadata, TickNumber,
    },
    Config,
};
use codec::{Decode, Encode};
use frame_support::{
    pallet_prelude::*,
    storage_alias,
    traits::{Get, UncheckedOnRuntimeUpgrade},
    weights::Weight,
};
use scale_info::TypeInfo;
use sp_std::collections::btree_map::BTreeMap;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

/// Module containing the old (v1) storage format before inlining BSPs.
pub mod v1 {
    use super::*;

    /// V1 representation of `StorageRequestMetadata` (no inline `bsps` field).
    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
    #[scale_info(skip_type_params(T))]
    pub struct StorageRequestMetadataV1<T: Config> {
        pub requested_at: TickNumber<T>,
        pub expires_at: TickNumber<T>,
        pub owner: T::AccountId,
        pub bucket_id: BucketIdFor<T>,
        pub location: FileLocation<T>,
        pub fingerprint: Fingerprint<T>,
        pub size: StorageDataUnit<T>,
        pub msp_status: MspStorageRequestStatus<T>,
        pub user_peer_ids: PeerIds<T>,
        pub bsps_required: ReplicationTargetType<T>,
        pub bsps_confirmed: ReplicationTargetType<T>,
        pub bsps_volunteered: ReplicationTargetType<T>,
        pub deposit_paid: BalanceOf<T>,
    }

    /// Old BSP entry value (confirmed flag only).
    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
    #[scale_info(skip_type_params(T))]
    pub struct StorageRequestBspsMetadataV1<T: Config> {
        pub confirmed: bool,
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[storage_alias]
    pub type StorageRequests<T: Config> =
        StorageMap<Pallet<T>, Blake2_128Concat, MerkleHash<T>, StorageRequestMetadataV1<T>>;

    #[storage_alias]
    pub type StorageRequestBsps<T: Config> = StorageDoubleMap<
        Pallet<T>,
        Blake2_128Concat,
        MerkleHash<T>,
        Blake2_128Concat,
        ProviderIdFor<T>,
        StorageRequestBspsMetadataV1<T>,
        OptionQuery,
    >;
}

/// Implements [`UncheckedOnRuntimeUpgrade`], migrating the state from V1 to V2 by inlining
/// `StorageRequestBsps` into each `StorageRequestMetadata`.
pub struct InnerMigrateV1ToV2<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV1ToV2<T> {
    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
        let request_count = v1::StorageRequests::<T>::iter().count() as u32;
        let bsp_entries: u32 = v1::StorageRequestBsps::<T>::iter().count() as u32;
        log::info!(
            target: "runtime::file-system",
            "Pre-upgrade: {} storage requests, {} BSP entries to migrate",
            request_count,
            bsp_entries
        );
        Ok((request_count, bsp_entries).encode())
    }

    fn on_runtime_upgrade() -> Weight {
        let mut reads: u64 = 0;
        let mut writes: u64 = 0;

        let keys: sp_std::vec::Vec<_> = v1::StorageRequests::<T>::iter_keys().collect();

        for file_key in keys {
            reads += 1;

            let Some(old_metadata) = v1::StorageRequests::<T>::take(&file_key) else {
                continue;
            };

            // Drain old BSP entries for this file key and build inline map (with bounded overflow handling).
            let mut bsps_map: BTreeMap<ProviderIdFor<T>, bool> = BTreeMap::new();
            for (bsp_id, bsp_meta) in v1::StorageRequestBsps::<T>::drain_prefix(&file_key) {
                reads += 1;
                writes += 1; // drain counts as a write per deleted entry
                let confirmed = bsp_meta.confirmed;
                // Respect MaxReplicationTarget: if we already have max entries, skip extra (defensive).
                if bsps_map.len() as u32 >= T::MaxReplicationTarget::get() {
                    log::warn!(
                        target: "runtime::file-system",
                        "Migration: skipping BSP {:?} for file_key (max entries reached)",
                        bsp_id
                    );
                    continue;
                }
                bsps_map.insert(bsp_id, confirmed);
            }

            let mut bounded_bsps =
                frame_support::BoundedBTreeMap::<ProviderIdFor<T>, bool, MaxReplicationTarget<T>>::new();
            for (bsp_id, confirmed) in bsps_map {
                if bounded_bsps.try_insert(bsp_id, confirmed).is_err() {
                    log::warn!(
                        target: "runtime::file-system",
                        "Migration: skipping BSP entry (inline map full)"
                    );
                }
            }

            let new_metadata = StorageRequestMetadata::<T> {
                requested_at: old_metadata.requested_at,
                expires_at: old_metadata.expires_at,
                owner: old_metadata.owner,
                bucket_id: old_metadata.bucket_id,
                location: old_metadata.location,
                fingerprint: old_metadata.fingerprint,
                size: old_metadata.size,
                msp_status: old_metadata.msp_status,
                user_peer_ids: old_metadata.user_peer_ids,
                bsps_required: old_metadata.bsps_required,
                bsps_confirmed: old_metadata.bsps_confirmed,
                bsps_volunteered: old_metadata.bsps_volunteered,
                bsps: bounded_bsps,
                deposit_paid: old_metadata.deposit_paid,
            };

            crate::StorageRequests::<T>::insert(&file_key, new_metadata);
            writes += 1;
        }

        log::info!(
            target: "runtime::file-system",
            "Migration v1->v2 complete: migrated {} storage requests",
            writes
        );

        T::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        use frame_support::ensure;

        let (old_count, _old_bsp_count) = <(u32, u32)>::decode(&mut &state[..])
            .map_err(|_| TryRuntimeError::Other("Failed to decode pre-upgrade state"))?;

        let new_count = crate::StorageRequests::<T>::iter().count() as u32;
        ensure!(
            old_count == new_count,
            TryRuntimeError::Other("Migration failed: storage request count mismatch")
        );

        // Verify all migrated entries have decodable metadata with bsps field
        for (_key, metadata) in crate::StorageRequests::<T>::iter() {
            let _ = &metadata.bsps;
        }

        log::info!(
            target: "runtime::file-system",
            "Post-upgrade: verified {} storage requests",
            new_count
        );

        Ok(())
    }
}

/// Versioned migration from storage version 1 to 2.
pub type MigrateV1ToV2<T> = frame_support::migrations::VersionedMigration<
    1,
    2,
    InnerMigrateV1ToV2<T>,
    Pallet<T>,
    <T as frame_system::Config>::DbWeight,
>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::BoundedVec;
    use sp_core::H256;
    use sp_keyring::sr25519::Keyring;

    fn create_v1_metadata() -> v1::StorageRequestMetadataV1<Test> {
        v1::StorageRequestMetadataV1 {
            requested_at: 1,
            expires_at: 100,
            owner: Keyring::Alice.to_account_id(),
            bucket_id: H256::zero(),
            location: BoundedVec::try_from(b"test/file.txt".to_vec()).unwrap(),
            fingerprint: H256::random(),
            size: 1024,
            msp_status: MspStorageRequestStatus::None,
            user_peer_ids: BoundedVec::default(),
            bsps_required: 3,
            bsps_confirmed: 0,
            bsps_volunteered: 0,
            deposit_paid: 100,
        }
    }

    #[test]
    fn migration_works_with_no_storage_requests() {
        new_test_ext().execute_with(|| {
            assert_eq!(v1::StorageRequests::<Test>::iter().count(), 0);
            let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(0, 0)
            );
            assert_eq!(crate::StorageRequests::<Test>::iter().count(), 0);
        });
    }

    #[test]
    fn migration_works_with_storage_requests_no_bsps() {
        new_test_ext().execute_with(|| {
            let file_key = H256::random();
            let v1_meta = create_v1_metadata();
            v1::StorageRequests::<Test>::insert(file_key, v1_meta.clone());

            let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
            );

            let new_meta = crate::StorageRequests::<Test>::get(file_key).unwrap();
            assert!(new_meta.bsps.is_empty());
            assert_eq!(new_meta.owner, v1_meta.owner);
            assert_eq!(new_meta.size, v1_meta.size);
        });
    }

    #[test]
    fn migration_inlines_bsps_confirmed_and_unconfirmed() {
        new_test_ext().execute_with(|| {
            let file_key = H256::random();
            let bsp1 = H256::random();
            let bsp2 = H256::random();
            v1::StorageRequests::<Test>::insert(file_key, create_v1_metadata());
            v1::StorageRequestBsps::<Test>::insert(
                file_key,
                bsp1,
                v1::StorageRequestBspsMetadataV1 { confirmed: false, _phantom: Default::default() },
            );
            v1::StorageRequestBsps::<Test>::insert(
                file_key,
                bsp2,
                v1::StorageRequestBspsMetadataV1 { confirmed: true, _phantom: Default::default() },
            );

            let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();
            // 1 read request + 2 read+write for drain_prefix (2 BSP entries)
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3)
            );

            let new_meta = crate::StorageRequests::<Test>::get(file_key).unwrap();
            assert_eq!(new_meta.bsps.len(), 2);
            assert_eq!(new_meta.bsps.get(&bsp1), Some(&false));
            assert_eq!(new_meta.bsps.get(&bsp2), Some(&true));
        });
    }

    #[test]
    fn migration_drains_old_storage_request_bsps() {
        new_test_ext().execute_with(|| {
            let file_key = H256::random();
            let bsp_id = H256::random();
            v1::StorageRequests::<Test>::insert(file_key, create_v1_metadata());
            v1::StorageRequestBsps::<Test>::insert(
                file_key,
                bsp_id,
                v1::StorageRequestBspsMetadataV1 { confirmed: true, _phantom: Default::default() },
            );

            InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

            assert!(v1::StorageRequestBsps::<Test>::iter_prefix(&file_key).next().is_none());
        });
    }

    #[test]
    fn migration_preserves_all_fields() {
        new_test_ext().execute_with(|| {
            let file_key = H256::random();
            let v1_meta = v1::StorageRequestMetadataV1 {
                requested_at: 42,
                expires_at: 1000,
                owner: Keyring::Bob.to_account_id(),
                bucket_id: H256::from_low_u64_be(123),
                location: BoundedVec::try_from(b"path/file.dat".to_vec()).unwrap(),
                fingerprint: H256::from_low_u64_be(456),
                size: 999999,
                msp_status: MspStorageRequestStatus::None,
                user_peer_ids: BoundedVec::default(),
                bsps_required: 5,
                bsps_confirmed: 2,
                bsps_volunteered: 3,
                deposit_paid: 50000,
            };
            v1::StorageRequests::<Test>::insert(file_key, v1_meta.clone());

            InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

            let new_meta = crate::StorageRequests::<Test>::get(file_key).unwrap();
            assert_eq!(new_meta.requested_at, v1_meta.requested_at);
            assert_eq!(new_meta.expires_at, v1_meta.expires_at);
            assert_eq!(new_meta.owner, v1_meta.owner);
            assert_eq!(new_meta.bucket_id, v1_meta.bucket_id);
            assert_eq!(new_meta.location, v1_meta.location);
            assert_eq!(new_meta.fingerprint, v1_meta.fingerprint);
            assert_eq!(new_meta.size, v1_meta.size);
            assert_eq!(new_meta.msp_status, v1_meta.msp_status);
            assert_eq!(new_meta.user_peer_ids, v1_meta.user_peer_ids);
            assert_eq!(new_meta.bsps_required, v1_meta.bsps_required);
            assert_eq!(new_meta.bsps_confirmed, v1_meta.bsps_confirmed);
            assert_eq!(new_meta.bsps_volunteered, v1_meta.bsps_volunteered);
            assert_eq!(new_meta.deposit_paid, v1_meta.deposit_paid);
            assert!(new_meta.bsps.is_empty());
        });
    }
}

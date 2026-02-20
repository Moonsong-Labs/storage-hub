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
//! This migration moves the `StorageRequestBsps` double map into a single-entry
//! `StorageMap<MerkleHash, BoundedBTreeMap<ProviderIdFor, bool, MaxBspVolunteers>>`.

use crate::{
    pallet::Pallet,
    types::{
        BalanceOf, BucketIdFor, FileLocation, Fingerprint, MaxBspVolunteers, MerkleHash,
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

/// Module containing the old (v1) storage format before the BSP map migration.
pub mod v1 {
    use super::*;

    /// V1 representation of `StorageRequestMetadata` (no inline `bsps` field).
    /// This matches the current `StorageRequestMetadata` exactly.
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

/// Implements [`UncheckedOnRuntimeUpgrade`], migrating the state from V1 to V2 by moving
/// `StorageRequestBsps` from a DoubleMap to a single-entry `StorageMap`.
pub struct InnerMigrateV1ToV2<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV1ToV2<T> {
    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
        use frame_support::ensure;
        use sp_std::collections::btree_map::BTreeMap as StdBTreeMap;

        let request_count = v1::StorageRequests::<T>::iter().count() as u32;

        // Single-pass: count BSPs per file_key using iter() which yields (K1, K2, V).
        let mut per_file_counts: StdBTreeMap<MerkleHash<T>, u32> = StdBTreeMap::new();
        for (file_key, _bsp_id, _) in v1::StorageRequestBsps::<T>::iter() {
            *per_file_counts.entry(file_key).or_default() += 1;
        }
        let bsp_entries: u32 = per_file_counts.values().copied().sum();

        // Fail-fast: if any file_key has more BSP entries than MaxBspVolunteers, the
        // migration would silently drop the excess. Abort try-runtime before any data is touched.
        let max_bsps = T::MaxBspVolunteers::get();
        for (_, count) in &per_file_counts {
            ensure!(
                *count <= max_bsps,
                TryRuntimeError::Other(
                    "pre_upgrade: BSP count for a file_key exceeds MaxBspVolunteers"
                )
            );
        }

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

        // Re-encode StorageRequests entries from v1 format to current format.
        // The field layout is identical, but re-encoding ensures any future codec changes are applied.
        let keys: sp_std::vec::Vec<_> = v1::StorageRequests::<T>::iter_keys().collect();

        for file_key in keys {
            reads += 1;

            let Some(old_metadata) = v1::StorageRequests::<T>::take(&file_key) else {
                continue;
            };

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
                deposit_paid: old_metadata.deposit_paid,
            };

            crate::StorageRequests::<T>::insert(&file_key, new_metadata);
            writes += 1;
        }

        // Drain old DoubleMap and group by file_key into new StorageMap.
        let mut grouped: BTreeMap<MerkleHash<T>, BTreeMap<ProviderIdFor<T>, bool>> =
            BTreeMap::new();

        for (file_key, bsp_id, bsp_meta) in v1::StorageRequestBsps::<T>::drain() {
            reads += 1;
            writes += 1; // drain counts as a write per deleted entry
            grouped
                .entry(file_key)
                .or_default()
                .insert(bsp_id, bsp_meta.confirmed);
        }

        // Write grouped BSP maps into the new StorageRequestBsps StorageMap.
        for (file_key, bsp_map) in grouped {
            let mut bounded_bsps =
                frame_support::BoundedBTreeMap::<ProviderIdFor<T>, bool, MaxBspVolunteers<T>>::new(
                );
            for (bsp_id, confirmed) in bsp_map {
                if bounded_bsps.try_insert(bsp_id, confirmed).is_err() {
                    log::warn!(
                        target: "runtime::file-system",
                        "Migration: skipping BSP entry (map full for file_key)"
                    );
                }
            }
            if !bounded_bsps.is_empty() {
                crate::StorageRequestBsps::<T>::insert(&file_key, bounded_bsps);
                writes += 1;
            }
        }

        log::info!(
            target: "runtime::file-system",
            "Migration v1->v2 complete: {} reads, {} writes",
            reads, writes
        );

        T::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        use frame_support::ensure;

        ensure!(
            Pallet::<T>::on_chain_storage_version() == StorageVersion::new(2),
            TryRuntimeError::Other("Migration failed: on-chain storage version not updated to 2")
        );

        let (old_count, old_bsp_count) = <(u32, u32)>::decode(&mut &state[..])
            .map_err(|_| TryRuntimeError::Other("Failed to decode pre-upgrade state"))?;

        let new_count = crate::StorageRequests::<T>::iter().count() as u32;
        ensure!(
            old_count == new_count,
            TryRuntimeError::Other("Migration failed: storage request count mismatch")
        );

        // Verify all BSP entries are readable from the new StorageMap
        let mut new_bsp_total: u32 = 0;
        for (file_key, bsps) in crate::StorageRequestBsps::<T>::iter() {
            ensure!(
                !bsps.is_empty(),
                TryRuntimeError::Other("Migration failed: empty BSP map should not be stored")
            );
            // Verify the storage request exists for this BSP entry
            ensure!(
                crate::StorageRequests::<T>::contains_key(&file_key),
                TryRuntimeError::Other("Migration failed: orphaned BSP map entry")
            );
            new_bsp_total = new_bsp_total.saturating_add(bsps.len() as u32);
        }

        // Verify no BSP entries were lost during migration.
        ensure!(
            old_bsp_count == new_bsp_total,
            TryRuntimeError::Other(
                "Migration failed: BSP entry count mismatch (entries were lost)"
            )
        );

        log::info!(
            target: "runtime::file-system",
            "Post-upgrade: verified {} storage requests, {} BSP entries",
            new_count,
            new_bsp_total
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
            assert_eq!(crate::StorageRequestBsps::<Test>::iter().count(), 0);
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
            assert_eq!(new_meta.owner, v1_meta.owner);
            assert_eq!(new_meta.size, v1_meta.size);
            // No BSP map entry should exist
            assert!(crate::StorageRequestBsps::<Test>::get(file_key).is_none());
        });
    }

    #[test]
    fn migration_moves_bsps_to_separate_map() {
        new_test_ext().execute_with(|| {
            let file_key = H256::random();
            let bsp1 = H256::random();
            let bsp2 = H256::random();
            v1::StorageRequests::<Test>::insert(file_key, create_v1_metadata());
            v1::StorageRequestBsps::<Test>::insert(
                file_key,
                bsp1,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: false,
                    _phantom: Default::default(),
                },
            );
            v1::StorageRequestBsps::<Test>::insert(
                file_key,
                bsp2,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );

            let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();
            // 1 read+write for storage request + 2 read+write for drain + 1 write for new BSP map
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 4)
            );

            // BSP data should be in the new StorageRequestBsps map
            let bsps = crate::StorageRequestBsps::<Test>::get(file_key).unwrap();
            assert_eq!(bsps.len(), 2);
            assert_eq!(bsps.get(&bsp1), Some(&false));
            assert_eq!(bsps.get(&bsp2), Some(&true));
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
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );

            InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

            assert!(v1::StorageRequestBsps::<Test>::iter_prefix(&file_key)
                .next()
                .is_none());
        });
    }

    #[test]
    fn migration_truncates_bsps_at_max_bsp_volunteers_when_limit_exceeded() {
        new_test_ext().execute_with(|| {
            // Insert MaxBspVolunteers + 1 BSPs so the migration must truncate.
            let max = <MaxBspVolunteers<Test> as frame_support::traits::Get<u32>>::get() as u64;
            let file_key = H256::random();
            v1::StorageRequests::<Test>::insert(file_key, create_v1_metadata());

            let bsp_ids: Vec<H256> = (0u64..max + 1).map(H256::from_low_u64_be).collect();
            for bsp_id in &bsp_ids {
                v1::StorageRequestBsps::<Test>::insert(
                    file_key,
                    bsp_id,
                    v1::StorageRequestBspsMetadataV1 {
                        confirmed: false,
                        _phantom: Default::default(),
                    },
                );
            }

            InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

            // Old double-map must be fully drained.
            assert!(v1::StorageRequestBsps::<Test>::iter_prefix(&file_key)
                .next()
                .is_none());

            // New map is capped at MaxBspVolunteers.
            let bsps = crate::StorageRequestBsps::<Test>::get(file_key).unwrap();
            assert_eq!(bsps.len(), max as usize);
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
        });
    }
}

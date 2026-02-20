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
    types::{MaxBspVolunteers, MerkleHash, ProviderIdFor},
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

    /// Old BSP entry value (confirmed flag only).
    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
    #[scale_info(skip_type_params(T))]
    pub struct StorageRequestBspsMetadataV1<T: Config> {
        pub confirmed: bool,
        pub _phantom: core::marker::PhantomData<T>,
    }

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

        let request_count = crate::StorageRequests::<T>::iter().count() as u32;

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
    use crate::{
        mock::{new_test_ext, Test},
        types::{MerkleHash, MspStorageRequestStatus, ProviderIdFor, StorageRequestMetadata},
    };
    use frame_support::BoundedVec;
    use sp_runtime::traits::Zero;

    fn create_test_metadata() -> StorageRequestMetadata<Test> {
        StorageRequestMetadata {
            requested_at: 1,
            expires_at: 100,
            owner: Default::default(),
            bucket_id: Default::default(),
            location: BoundedVec::try_from(b"test/file.txt".to_vec()).unwrap(),
            fingerprint: Default::default(),
            size: 1024,
            msp_status: MspStorageRequestStatus::None,
            user_peer_ids: BoundedVec::default(),
            bsps_required: 3,
            bsps_confirmed: Zero::zero(),
            bsps_volunteered: Zero::zero(),
            deposit_paid: 100,
        }
    }

    fn file_key(seed: u8) -> MerkleHash<Test> {
        MerkleHash::<Test>::from([seed; 32])
    }

    fn bsp_id(seed: u8) -> ProviderIdFor<Test> {
        ProviderIdFor::<Test>::from([seed; 32])
    }

    #[test]
    fn migration_works_with_no_storage_requests() {
        new_test_ext().execute_with(|| {
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
            let key = file_key(1);
            let meta = create_test_metadata();
            crate::StorageRequests::<Test>::insert(key, meta.clone());

            let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(0, 0)
            );

            let new_meta = crate::StorageRequests::<Test>::get(key).unwrap();
            assert_eq!(new_meta.owner, meta.owner);
            assert_eq!(new_meta.size, meta.size);
            // No BSP map entry should exist
            assert!(crate::StorageRequestBsps::<Test>::get(key).is_none());
        });
    }

    #[test]
    fn migration_moves_bsps_to_separate_map() {
        new_test_ext().execute_with(|| {
            let key = file_key(1);
            let bsp1 = bsp_id(2);
            let bsp2 = bsp_id(3);
            crate::StorageRequests::<Test>::insert(key, create_test_metadata());
            v1::StorageRequestBsps::<Test>::insert(
                key,
                bsp1,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: false,
                    _phantom: Default::default(),
                },
            );
            v1::StorageRequestBsps::<Test>::insert(
                key,
                bsp2,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );

            let weight = InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();
            // 2 read+write for drain + 1 write for new BSP map
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(2, 3)
            );

            // BSP data should be in the new StorageRequestBsps map
            let bsps = crate::StorageRequestBsps::<Test>::get(key).unwrap();
            assert_eq!(bsps.len(), 2);
            assert_eq!(bsps.get(&bsp1), Some(&false));
            assert_eq!(bsps.get(&bsp2), Some(&true));
        });
    }

    #[test]
    fn migration_drains_old_storage_request_bsps() {
        new_test_ext().execute_with(|| {
            let key = file_key(1);
            let bsp = bsp_id(2);
            crate::StorageRequests::<Test>::insert(key, create_test_metadata());
            v1::StorageRequestBsps::<Test>::insert(
                key,
                bsp,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );

            InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

            assert!(v1::StorageRequestBsps::<Test>::iter_prefix(&key)
                .next()
                .is_none());
        });
    }

    #[test]
    fn migration_truncates_bsps_at_max_bsp_volunteers_when_limit_exceeded() {
        new_test_ext().execute_with(|| {
            // Insert MaxBspVolunteers + 1 BSPs so the migration must truncate.
            let max = <MaxBspVolunteers<Test> as frame_support::traits::Get<u32>>::get() as usize;
            let key = file_key(1);
            crate::StorageRequests::<Test>::insert(key, create_test_metadata());

            for i in 0..=max {
                v1::StorageRequestBsps::<Test>::insert(
                    key,
                    bsp_id(i as u8),
                    v1::StorageRequestBspsMetadataV1 {
                        confirmed: false,
                        _phantom: Default::default(),
                    },
                );
            }

            InnerMigrateV1ToV2::<Test>::on_runtime_upgrade();

            // Old double-map must be fully drained.
            assert!(v1::StorageRequestBsps::<Test>::iter_prefix(&key)
                .next()
                .is_none());

            // New map is capped at MaxBspVolunteers.
            let bsps = crate::StorageRequestBsps::<Test>::get(key).unwrap();
            assert_eq!(bsps.len(), max);
        });
    }
}

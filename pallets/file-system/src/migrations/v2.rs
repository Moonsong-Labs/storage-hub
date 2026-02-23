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
    types::{MerkleHash, ProviderIdFor},
    Config,
};
use codec::{Decode, Encode};
use frame_support::{
    migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
    pallet_prelude::*,
    storage_alias,
    traits::Get,
    weights::WeightMeter,
};
use scale_info::TypeInfo;

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

/// Pallet name bytes used as the MigrationId pallet_id (must be exactly 18 bytes = "pallet-file-system").
const PALLET_MIGRATIONS_ID: &[u8; 18] = b"pallet-file-system";

/// Multi-block migration (stepped) from storage version 1 to 2.
///
/// Each step processes one `file_key` at a time: it drains all BSP entries for that key from the
/// old `StorageDoubleMap` and writes them into the new `StorageMap<MerkleHash, BoundedBTreeMap>`.
/// Because entries are drained as they are processed, subsequent steps automatically skip
/// already-migrated keys — no cursor ordering across keys is required.
///
/// BSP operations (`bsp_volunteer`, `bsp_confirm_storing`) are blocked via
/// `UserOperationPauseFlags` while the migration is in progress. The pause flags and the
/// proofs-dealer challenge ticker are managed by the runtime's `MigrationStatusHandler`
/// (see `V2MigrationStatusHandler` in the runtime).
pub struct MigrateV1ToV2Stepped<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> SteppedMigration for MigrateV1ToV2Stepped<T> {
    /// `()` cursor — since we remove entries as we process them, the migration is complete when
    /// the old DoubleMap is empty. No positional cursor is needed.
    type Cursor = ();

    /// Unique identifier for this migration.
    type Identifier = MigrationId<18>;

    fn id() -> Self::Identifier {
        MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 1, version_to: 2 }
    }

    fn step(
        _cursor: Option<Self::Cursor>,
        meter: &mut WeightMeter,
    ) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
        // One step processes a single v1 BSP entry:
        //   reads: iter().next() (key scan + value read) + read v2 map
        //   writes: remove v1 entry + write updated v2 map
        // Using reads_writes(3, 2) as a conservative estimate to stay well within
        // MaxServiceWeight (10% of max_block ≈ 524 KB proof_size).
        let required = T::DbWeight::get().reads_writes(3, 2);
        if meter.remaining().any_lt(required) {
            return Err(SteppedMigrationError::InsufficientWeight { required });
        }

        // Find any remaining entry in the old DoubleMap.
        let Some((file_key, bsp_id, bsp_meta)) = v1::StorageRequestBsps::<T>::iter().next()
        else {
            // Old map is empty — migration is complete.
            return Ok(None);
        };

        // Remove this v1 entry.
        v1::StorageRequestBsps::<T>::remove(&file_key, &bsp_id);

        // Add BSP to the v2 map (creating it if it doesn't exist yet).
        let mut bsps = crate::StorageRequestBsps::<T>::get(&file_key).unwrap_or_default();
        if bsps.try_insert(bsp_id, bsp_meta.confirmed).is_err() {
            log::warn!(
                target: "runtime::file-system",
                "MigrateV1ToV2Stepped: skipping BSP entry (map full for file_key)"
            );
        } else {
            crate::StorageRequestBsps::<T>::insert(&file_key, bsps);
        }

        meter.consume(required);

        Ok(Some(()))
    }

    /// Captures the pre-migration state: file_key count and total BSP entry count from the
    /// old DoubleMap. Also fails fast if any file_key has more BSP entries than
    /// `MaxBspVolunteers`, which would cause silent truncation during migration.
    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
        use frame_support::ensure;
        use sp_std::collections::btree_map::BTreeMap as StdBTreeMap;

        // Single-pass: count BSP entries per file_key in the old DoubleMap.
        let mut per_file_counts: StdBTreeMap<MerkleHash<T>, u32> = StdBTreeMap::new();
        for (file_key, _bsp_id, _) in v1::StorageRequestBsps::<T>::iter() {
            *per_file_counts.entry(file_key).or_default() += 1;
        }
        let bsp_entries: u32 = per_file_counts.values().copied().sum();
        let file_key_count = per_file_counts.len() as u32;

        // Fail-fast: abort if any file_key exceeds MaxBspVolunteers (would cause silent drop).
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
            "MigrateV1ToV2Stepped pre_upgrade: {} file_keys, {} BSP entries to migrate",
            file_key_count,
            bsp_entries
        );
        Ok((file_key_count, bsp_entries).encode())
    }

    /// Verifies the post-migration state: old DoubleMap is fully drained, new StorageMap has
    /// the same number of file_keys and total BSP entries as before, with no orphaned entries.
    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        use frame_support::ensure;

        let (old_file_key_count, old_bsp_count) = <(u32, u32)>::decode(&mut &state[..])
            .map_err(|_| {
                TryRuntimeError::Other(
                    "MigrateV1ToV2Stepped post_upgrade: failed to decode pre-upgrade state",
                )
            })?;

        // Old DoubleMap must be fully drained — typed check.
        let remaining_old = v1::StorageRequestBsps::<T>::iter().count() as u32;
        ensure!(
            remaining_old == 0,
            TryRuntimeError::Other(
                "MigrateV1ToV2Stepped post_upgrade: old StorageRequestBsps not fully drained"
            )
        );

        // Count new map entries and verify invariants.
        let mut new_bsp_total: u32 = 0;
        let mut new_file_key_count: u32 = 0;
        for (_file_key, bsps) in crate::StorageRequestBsps::<T>::iter() {
            ensure!(
                !bsps.is_empty(),
                TryRuntimeError::Other(
                    "MigrateV1ToV2Stepped post_upgrade: empty BSP map should not be stored"
                )
            );
            new_bsp_total = new_bsp_total.saturating_add(bsps.len() as u32);
            new_file_key_count += 1;
        }

        ensure!(
            new_file_key_count == old_file_key_count,
            TryRuntimeError::Other(
                "MigrateV1ToV2Stepped post_upgrade: file_key count mismatch"
            )
        );
        ensure!(
            new_bsp_total == old_bsp_count,
            TryRuntimeError::Other(
                "MigrateV1ToV2Stepped post_upgrade: BSP entry count mismatch (entries were lost)"
            )
        );

        log::info!(
            target: "runtime::file-system",
            "MigrateV1ToV2Stepped post_upgrade: verified {} file_keys, {} BSP entries",
            new_file_key_count,
            new_bsp_total
        );
        Ok(())
    }
}

/// For `try-runtime` only: run all `MigrateV1ToV2Stepped` steps synchronously as a single
/// `OnRuntimeUpgrade`, bypassing the MBM (`pallet_migrations`) multi-block executor.
///
/// ## Why this wrapper exists
///
/// On Cumulus parachains, try-runtime Phase 2 (empty-block production to advance the MBM cursor)
/// panics because `cumulus_pallet_parachain_system::create_inherent` requires relay-chain
/// validation data that is never available in try-runtime's mock block environment. By running
/// the migration here in Phase 1, we avoid Phase 2 entirely (combined with:
///   - `pallet_migrations::Config::Migrations = ()` for the `try-runtime` feature, and
///   - `--disable-mbm-checks` in the try-runtime CLI invocation).
///
/// ## Verification strategy
///
/// The try-runtime test uses `--checks none` to prevent `try_decode_entire_state` from running
/// against unrelated storage items (e.g. `StorageRequests`) whose codec may differ from the
/// testnet's older runtime. Verification of the BSP migration is therefore embedded directly in
/// `on_runtime_upgrade`: it calls the stepped migration's `pre_upgrade`/`post_upgrade` internally
/// and panics on failure, causing try-runtime to exit non-zero if the migration is incorrect.
#[cfg(feature = "try-runtime")]
pub struct TryRuntimeMigrate<T: Config>(core::marker::PhantomData<T>);

#[cfg(feature = "try-runtime")]
impl<T: Config> frame_support::traits::OnRuntimeUpgrade for TryRuntimeMigrate<T> {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        // Capture pre-migration state for post-migration verification.
        // Called at the start of on_runtime_upgrade before any storage is modified,
        // equivalent to what the pre_upgrade framework hook would capture.
        let pre_state = MigrateV1ToV2Stepped::<T>::pre_upgrade()
            .expect("TryRuntimeMigrate: pre_upgrade check failed");

        // Run all migration steps to completion.
        let mut cursor = None;
        loop {
            let mut meter = frame_support::weights::WeightMeter::with_limit(
                frame_support::weights::Weight::MAX,
            );
            match MigrateV1ToV2Stepped::<T>::step(cursor, &mut meter) {
                Ok(Some(c)) => cursor = Some(c),
                Ok(None) => break,
                Err(e) => {
                    panic!("TryRuntimeMigrate: step failed: {:?}", e);
                }
            }
        }

        // Bump the pallet storage version: pallet_migrations normally does this when it
        // completes a SteppedMigration, but in try-runtime pallet_migrations::Migrations = ()
        // so it never fires. Bump explicitly here.
        StorageVersion::new(2).put::<Pallet<T>>();

        // Verify post-migration state only when there were v1 entries to migrate.
        // On the idempotency run (second call to on_runtime_upgrade), pre_upgrade finds 0 v1
        // entries (they were all migrated in the first run). Calling post_upgrade in that case
        // would fail: new_count (35 v2 entries) ≠ old_count (0). This is correct — the
        // migration is already done — so we skip the check when old_file_key_count == 0.
        let old_file_key_count = <(u32, u32)>::decode(&mut &pre_state[..])
            .map(|(count, _)| count)
            .unwrap_or(0);
        if old_file_key_count > 0 {
            MigrateV1ToV2Stepped::<T>::post_upgrade(pre_state)
                .expect("TryRuntimeMigrate: post_upgrade check failed");
        }

        T::DbWeight::get().reads_writes(1, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mock::{new_test_ext, Test},
        types::{MaxBspVolunteers, MerkleHash, MspStorageRequestStatus, ProviderIdFor, StorageRequestMetadata},
    };
    use frame_support::BoundedVec;
    use sp_runtime::traits::Zero;

    fn create_test_metadata() -> StorageRequestMetadata<Test> {
        use sp_keyring::sr25519::Keyring;
        StorageRequestMetadata {
            requested_at: 1,
            expires_at: 100,
            owner: Keyring::Alice.to_account_id(),
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

    /// Produces a unique `ProviderIdFor<Test>` for any `usize` index by encoding
    /// the index as little-endian bytes in the first 8 bytes of the 32-byte array.
    fn bsp_id_n(index: usize) -> ProviderIdFor<Test> {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&(index as u64).to_le_bytes());
        ProviderIdFor::<Test>::from(bytes)
    }

    /// Run `MigrateV1ToV2Stepped` to completion, panicking on any step error.
    fn run_migration() {
        let mut cursor = None;
        loop {
            let mut meter = WeightMeter::with_limit(Weight::MAX);
            match MigrateV1ToV2Stepped::<Test>::step(cursor, &mut meter) {
                Ok(Some(c)) => cursor = Some(c),
                Ok(None) => break,
                Err(e) => panic!("Migration step failed: {:?}", e),
            }
        }
    }

    #[test]
    fn migration_works_with_no_storage_requests() {
        new_test_ext().execute_with(|| {
            run_migration();
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

            run_migration();

            let new_meta = crate::StorageRequests::<Test>::get(key).unwrap();
            assert_eq!(new_meta.owner, meta.owner);
            assert_eq!(new_meta.size, meta.size);
            // No BSP map entry should exist (no BSPs were in old map)
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

            run_migration();

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

            run_migration();

            assert!(v1::StorageRequestBsps::<Test>::iter_prefix(&key)
                .next()
                .is_none());
        });
    }

    #[test]
    fn migration_groups_bsps_by_file_key() {
        new_test_ext().execute_with(|| {
            let key1 = file_key(1);
            let key2 = file_key(2);
            let bsp_a = bsp_id(10);
            let bsp_b = bsp_id(11);
            let bsp_c = bsp_id(20);
            let bsp_d = bsp_id(21);

            crate::StorageRequests::<Test>::insert(key1, create_test_metadata());
            crate::StorageRequests::<Test>::insert(key2, create_test_metadata());

            // key1 → bsp_a (unconfirmed), bsp_b (confirmed)
            v1::StorageRequestBsps::<Test>::insert(
                key1,
                bsp_a,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: false,
                    _phantom: Default::default(),
                },
            );
            v1::StorageRequestBsps::<Test>::insert(
                key1,
                bsp_b,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );
            // key2 → bsp_c (confirmed), bsp_d (unconfirmed)
            v1::StorageRequestBsps::<Test>::insert(
                key2,
                bsp_c,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: true,
                    _phantom: Default::default(),
                },
            );
            v1::StorageRequestBsps::<Test>::insert(
                key2,
                bsp_d,
                v1::StorageRequestBspsMetadataV1 {
                    confirmed: false,
                    _phantom: Default::default(),
                },
            );

            run_migration();

            let bsps1 = crate::StorageRequestBsps::<Test>::get(key1).unwrap();
            assert_eq!(bsps1.len(), 2);
            assert_eq!(bsps1.get(&bsp_a), Some(&false));
            assert_eq!(bsps1.get(&bsp_b), Some(&true));
            // key2's BSPs must not appear under key1
            assert_eq!(bsps1.get(&bsp_c), None);
            assert_eq!(bsps1.get(&bsp_d), None);

            let bsps2 = crate::StorageRequestBsps::<Test>::get(key2).unwrap();
            assert_eq!(bsps2.len(), 2);
            assert_eq!(bsps2.get(&bsp_c), Some(&true));
            assert_eq!(bsps2.get(&bsp_d), Some(&false));

            // Old double map must be fully drained.
            assert_eq!(v1::StorageRequestBsps::<Test>::iter().count(), 0);
        });
    }

    #[test]
    fn migration_keeps_all_bsps_when_exactly_at_max() {
        new_test_ext().execute_with(|| {
            let max =
                <MaxBspVolunteers<Test> as frame_support::traits::Get<u32>>::get() as usize;
            let key = file_key(1);
            crate::StorageRequests::<Test>::insert(key, create_test_metadata());

            // Insert exactly MaxBspVolunteers entries (0..max, NOT 0..=max).
            for i in 0..max {
                v1::StorageRequestBsps::<Test>::insert(
                    key,
                    bsp_id_n(i),
                    v1::StorageRequestBspsMetadataV1 {
                        confirmed: false,
                        _phantom: Default::default(),
                    },
                );
            }

            run_migration();

            // All entries preserved — none truncated.
            let bsps = crate::StorageRequestBsps::<Test>::get(key).unwrap();
            assert_eq!(bsps.len(), max);
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
                    bsp_id_n(i),
                    v1::StorageRequestBspsMetadataV1 {
                        confirmed: false,
                        _phantom: Default::default(),
                    },
                );
            }

            run_migration();

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

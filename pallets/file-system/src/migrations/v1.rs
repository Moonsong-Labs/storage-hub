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

//! Migration from storage version 0 to 1.
//!
//! This migration transforms the `msp` field in `StorageRequestMetadata` from
//! `Option<(ProviderId, bool)>` to `MspStorageRequestStatus<T>`.
//!
//! ## Migration Logic
//!
//! - `None` -> `MspStorageRequestStatus::None`
//! - `Some((msp_id, false))` -> `MspStorageRequestStatus::Pending(msp_id)`
//! - `Some((msp_id, true))` -> `MspStorageRequestStatus::AcceptedNewFile(msp_id)`

use crate::{
    pallet::Pallet,
    types::{
        BalanceOf, BucketIdFor, FileLocation, Fingerprint, MerkleHash, MspStorageRequestStatus,
        PeerIds, ProviderIdFor, ReplicationTargetType, StorageDataUnit, StorageRequestMetadata,
        TickNumber,
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

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

/// Module containing the old (v0) storage format.
///
/// Before running this migration, the storage alias defined here represents the
/// on-chain storage format.
pub mod v0 {
    use super::*;

    /// V0 representation of `StorageRequestMetadata`.
    ///
    /// The key difference is the `msp` field which was `Option<(ProviderIdFor<T>, bool)>`
    /// where the bool represented whether the MSP had accepted the storage request.
    #[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq, Clone)]
    #[scale_info(skip_type_params(T))]
    pub struct StorageRequestMetadataV0<T: Config> {
        /// Tick number at which the storage request was made.
        pub requested_at: TickNumber<T>,

        /// Tick number at which the storage request will expire.
        pub expires_at: TickNumber<T>,

        /// AccountId of the user who owns the data being stored.
        pub owner: T::AccountId,

        /// Bucket id where this file is stored.
        pub bucket_id: BucketIdFor<T>,

        /// User defined name of the file being stored.
        pub location: FileLocation<T>,

        /// Identifier of the data being stored.
        pub fingerprint: Fingerprint<T>,

        /// Size of the data being stored.
        pub size: StorageDataUnit<T>,

        /// MSP assigned to this storage request (old format).
        ///
        /// - `None`: No MSP assigned (e.g., BSP redundancy recovery)
        /// - `Some((msp_id, false))`: MSP assigned but hasn't accepted yet
        /// - `Some((msp_id, true))`: MSP has accepted the storage request
        pub msp: Option<(ProviderIdFor<T>, bool)>,

        /// Peer Ids of the user who requested the storage.
        pub user_peer_ids: PeerIds<T>,

        /// Number of BSPs requested to store the data.
        pub bsps_required: ReplicationTargetType<T>,

        /// Number of BSPs that have successfully volunteered AND confirmed.
        pub bsps_confirmed: ReplicationTargetType<T>,

        /// Number of BSPs that have volunteered to store the data.
        pub bsps_volunteered: ReplicationTargetType<T>,

        /// Deposit paid by the user to open this storage request.
        pub deposit_paid: BalanceOf<T>,
    }

    /// Storage alias for the old StorageRequests map with v0 format.
    #[storage_alias]
    pub type StorageRequests<T: Config> =
        StorageMap<Pallet<T>, Blake2_128Concat, MerkleHash<T>, StorageRequestMetadataV0<T>>;
}

/// Implements [`UncheckedOnRuntimeUpgrade`], migrating the state of this pallet from V0 to V1.
///
/// This migration transforms the `msp` field from `Option<(ProviderId, bool)>` to
/// `MspStorageRequestStatus<T>`.
pub struct InnerMigrateV0ToV1<T: Config>(core::marker::PhantomData<T>);

impl<T: Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T> {
    /// Return the count of existing storage requests so we can verify the migration.
    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
        use codec::Encode;

        let count = v0::StorageRequests::<T>::iter().count() as u32;
        log::info!(
            target: "runtime::file-system",
            "Pre-upgrade: Found {} storage requests to migrate",
            count
        );

        Ok(count.encode())
    }

    /// Migrate the storage from V0 to V1.
    ///
    /// For each storage request:
    /// - Read the old format with `msp: Option<(ProviderId, bool)>`
    /// - Transform to new format with `msp_status: MspStorageRequestStatus`
    /// - Write back to storage
    fn on_runtime_upgrade() -> Weight {
        let mut reads: u64 = 0;
        let mut writes: u64 = 0;

        // Collect all keys first to avoid iterator invalidation
        let keys: sp_std::vec::Vec<_> = v0::StorageRequests::<T>::iter_keys().collect();

        for key in keys {
            reads += 1;

            if let Some(old_metadata) = v0::StorageRequests::<T>::take(&key) {
                // Transform the msp field to msp_status
                let msp_status = match old_metadata.msp {
                    None => MspStorageRequestStatus::None,
                    Some((msp_id, false)) => MspStorageRequestStatus::Pending(msp_id),
                    Some((msp_id, true)) => MspStorageRequestStatus::AcceptedNewFile(msp_id),
                };

                // Create the new metadata with the transformed field
                let new_metadata = StorageRequestMetadata::<T> {
                    requested_at: old_metadata.requested_at,
                    expires_at: old_metadata.expires_at,
                    owner: old_metadata.owner,
                    bucket_id: old_metadata.bucket_id,
                    location: old_metadata.location,
                    fingerprint: old_metadata.fingerprint,
                    size: old_metadata.size,
                    msp_status,
                    user_peer_ids: old_metadata.user_peer_ids,
                    bsps_required: old_metadata.bsps_required,
                    bsps_confirmed: old_metadata.bsps_confirmed,
                    bsps_volunteered: old_metadata.bsps_volunteered,
                    deposit_paid: old_metadata.deposit_paid,
                };

                // Write the new format
                crate::StorageRequests::<T>::insert(&key, new_metadata);
                writes += 1;
            }
        }

        log::info!(
            target: "runtime::file-system",
            "Migration complete: Migrated {} storage requests",
            writes
        );

        // Return the weight consumed: reads + writes
        T::DbWeight::get().reads_writes(reads, writes)
    }

    /// Verify the migration was successful.
    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        use codec::Decode;
        use frame_support::ensure;

        let old_count = u32::decode(&mut &state[..])
            .map_err(|_| TryRuntimeError::Other("Failed to decode old count"))?;

        let new_count = crate::StorageRequests::<T>::iter().count() as u32;

        ensure!(
            old_count == new_count,
            TryRuntimeError::Other("Migration failed: count mismatch")
        );

        log::info!(
            target: "runtime::file-system",
            "Post-upgrade: Successfully migrated {} storage requests",
            new_count
        );

        // Verify that all storage requests can be decoded with the new format
        for (key, metadata) in crate::StorageRequests::<T>::iter() {
            // Just accessing the msp_status field verifies it was correctly migrated
            let _status = &metadata.msp_status;
            log::debug!(
                target: "runtime::file-system",
                "Verified storage request {:?} with status {:?}",
                key,
                metadata.msp_status
            );
        }

        Ok(())
    }
}

/// [`UncheckedOnRuntimeUpgrade`] implementation [`InnerMigrateV0ToV1`] wrapped in a
/// [`VersionedMigration`](frame_support::migrations::VersionedMigration), which ensures that:
/// - The migration only runs once when the on-chain storage version is 0
/// - The on-chain storage version is updated to `1` after the migration is complete
/// - Reads/Writes from checking/setting the on-chain storage version are accounted for
pub type MigrateV0ToV1<T> = frame_support::migrations::VersionedMigration<
    0, // The migration will only execute when the on-chain storage version is 0
    1, // The on-chain storage version will be set to 1 after the migration is complete
    InnerMigrateV0ToV1<T>,
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

    /// Helper to create a v0 storage request metadata for testing.
    fn create_v0_metadata(
        msp: Option<(H256, bool)>,
    ) -> v0::StorageRequestMetadataV0<Test> {
        v0::StorageRequestMetadataV0 {
            requested_at: 1,
            expires_at: 100,
            owner: Keyring::Alice.to_account_id(),
            bucket_id: H256::zero(),
            location: BoundedVec::try_from(b"test/file.txt".to_vec()).unwrap(),
            fingerprint: H256::random(),
            size: 1024,
            msp,
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
            // No storage requests exist
            assert_eq!(v0::StorageRequests::<Test>::iter().count(), 0);

            // Run migration
            let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

            // Should have consumed minimal weight (no reads/writes for data)
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(0, 0)
            );

            // Still no storage requests
            assert_eq!(crate::StorageRequests::<Test>::iter().count(), 0);
        });
    }

    #[test]
    fn migration_transforms_none_msp_to_none_status() {
        new_test_ext().execute_with(|| {
            // Insert a v0 storage request with msp: None
            let file_key = H256::random();
            let v0_metadata = create_v0_metadata(None);
            v0::StorageRequests::<Test>::insert(file_key, v0_metadata.clone());

            // Run migration
            let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

            // Should have consumed weight for 1 read and 1 write
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
            );

            // Verify the new format
            let new_metadata = crate::StorageRequests::<Test>::get(file_key).unwrap();
            assert_eq!(new_metadata.msp_status, MspStorageRequestStatus::None);
            assert_eq!(new_metadata.owner, v0_metadata.owner);
            assert_eq!(new_metadata.size, v0_metadata.size);
            assert_eq!(new_metadata.fingerprint, v0_metadata.fingerprint);
        });
    }

    #[test]
    fn migration_transforms_pending_msp_to_pending_status() {
        new_test_ext().execute_with(|| {
            // Insert a v0 storage request with msp: Some((id, false)) - pending
            let file_key = H256::random();
            let msp_id = H256::random();
            let v0_metadata = create_v0_metadata(Some((msp_id, false)));
            v0::StorageRequests::<Test>::insert(file_key, v0_metadata.clone());

            // Run migration
            let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

            // Should have consumed weight for 1 read and 1 write
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
            );

            // Verify the new format
            let new_metadata = crate::StorageRequests::<Test>::get(file_key).unwrap();
            assert_eq!(
                new_metadata.msp_status,
                MspStorageRequestStatus::Pending(msp_id)
            );
            assert_eq!(new_metadata.owner, v0_metadata.owner);
        });
    }

    #[test]
    fn migration_transforms_accepted_msp_to_accepted_new_file_status() {
        new_test_ext().execute_with(|| {
            // Insert a v0 storage request with msp: Some((id, true)) - accepted
            let file_key = H256::random();
            let msp_id = H256::random();
            let v0_metadata = create_v0_metadata(Some((msp_id, true)));
            v0::StorageRequests::<Test>::insert(file_key, v0_metadata.clone());

            // Run migration
            let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

            // Should have consumed weight for 1 read and 1 write
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
            );

            // Verify the new format
            let new_metadata = crate::StorageRequests::<Test>::get(file_key).unwrap();
            assert_eq!(
                new_metadata.msp_status,
                MspStorageRequestStatus::AcceptedNewFile(msp_id)
            );
            assert_eq!(new_metadata.owner, v0_metadata.owner);
        });
    }

    #[test]
    fn migration_handles_multiple_storage_requests() {
        new_test_ext().execute_with(|| {
            // Insert multiple v0 storage requests with different msp values
            let file_key_1 = H256::random();
            let file_key_2 = H256::random();
            let file_key_3 = H256::random();
            let msp_id_1 = H256::random();
            let msp_id_2 = H256::random();

            v0::StorageRequests::<Test>::insert(file_key_1, create_v0_metadata(None));
            v0::StorageRequests::<Test>::insert(
                file_key_2,
                create_v0_metadata(Some((msp_id_1, false))),
            );
            v0::StorageRequests::<Test>::insert(
                file_key_3,
                create_v0_metadata(Some((msp_id_2, true))),
            );

            // Verify we have 3 v0 storage requests
            assert_eq!(v0::StorageRequests::<Test>::iter().count(), 3);

            // Run migration
            let weight = InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

            // Should have consumed weight for 3 reads and 3 writes
            assert_eq!(
                weight,
                <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 3)
            );

            // Verify we have 3 new storage requests
            assert_eq!(crate::StorageRequests::<Test>::iter().count(), 3);

            // Verify each was migrated correctly
            let new_1 = crate::StorageRequests::<Test>::get(file_key_1).unwrap();
            assert_eq!(new_1.msp_status, MspStorageRequestStatus::None);

            let new_2 = crate::StorageRequests::<Test>::get(file_key_2).unwrap();
            assert_eq!(
                new_2.msp_status,
                MspStorageRequestStatus::Pending(msp_id_1)
            );

            let new_3 = crate::StorageRequests::<Test>::get(file_key_3).unwrap();
            assert_eq!(
                new_3.msp_status,
                MspStorageRequestStatus::AcceptedNewFile(msp_id_2)
            );
        });
    }

    #[test]
    fn migration_preserves_all_other_fields() {
        new_test_ext().execute_with(|| {
            let file_key = H256::random();
            let msp_id = H256::random();

            let v0_metadata = v0::StorageRequestMetadataV0 {
                requested_at: 42,
                expires_at: 1000,
                owner: Keyring::Bob.to_account_id(),
                bucket_id: H256::from_low_u64_be(123),
                location: BoundedVec::try_from(b"path/to/important/file.dat".to_vec()).unwrap(),
                fingerprint: H256::from_low_u64_be(456),
                size: 999999,
                msp: Some((msp_id, true)),
                user_peer_ids: BoundedVec::default(),
                bsps_required: 5,
                bsps_confirmed: 2,
                bsps_volunteered: 3,
                deposit_paid: 50000,
            };
            v0::StorageRequests::<Test>::insert(file_key, v0_metadata.clone());

            // Run migration
            InnerMigrateV0ToV1::<Test>::on_runtime_upgrade();

            // Verify all fields are preserved
            let new_metadata = crate::StorageRequests::<Test>::get(file_key).unwrap();

            assert_eq!(new_metadata.requested_at, v0_metadata.requested_at);
            assert_eq!(new_metadata.expires_at, v0_metadata.expires_at);
            assert_eq!(new_metadata.owner, v0_metadata.owner);
            assert_eq!(new_metadata.bucket_id, v0_metadata.bucket_id);
            assert_eq!(new_metadata.location, v0_metadata.location);
            assert_eq!(new_metadata.fingerprint, v0_metadata.fingerprint);
            assert_eq!(new_metadata.size, v0_metadata.size);
            assert_eq!(
                new_metadata.msp_status,
                MspStorageRequestStatus::AcceptedNewFile(msp_id)
            );
            assert_eq!(new_metadata.user_peer_ids, v0_metadata.user_peer_ids);
            assert_eq!(new_metadata.bsps_required, v0_metadata.bsps_required);
            assert_eq!(new_metadata.bsps_confirmed, v0_metadata.bsps_confirmed);
            assert_eq!(new_metadata.bsps_volunteered, v0_metadata.bsps_volunteered);
            assert_eq!(new_metadata.deposit_paid, v0_metadata.deposit_paid);
        });
    }
}


use crate::types::{BucketIdFor, BucketNameFor};
use crate::{
    mock::*,
    types::{
        FileLocation, PeerIds, ProviderIdFor, StorageData, StorageRequestBspsMetadata,
        StorageRequestMetadata, TargetBspsRequired,
    },
    Config, Error, Event, StorageRequestExpirations,
};
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchResultWithPostInfo,
    traits::{nonfungibles_v2::Destroy, Hooks, OriginTrait},
    weights::Weight,
};
use pallet_proofs_dealer::PriorityChallengesQueue;
use shp_traits::{ReadProvidersInterface, SubscribeProvidersInterface, TrieRemoveMutation};
use sp_core::{ByteArray, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{BlakeTwo256, Get, One, Zero},
    BoundedVec, DispatchError, FixedU128,
};

mod create_bucket_tests {
    use super::*;

    #[test]
    fn create_private_bucket_success() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let private = true;

            let msp_id = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::create_bucket(
                origin,
                msp_id,
                name.clone(),
                private
            ));

            // Check if collection was created
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::NewBucket {
                    who: owner,
                    msp_id,
                    bucket_id,
                    name,
                    collection_id: Some(0),
                    private,
                }
                .into(),
            );
        });
    }

    #[test]
    fn create_public_bucket_success() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let private = false;

            let msp_id = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::create_bucket(
                origin,
                msp_id,
                name.clone(),
                private
            ));

            // Check that the bucket does not have a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_none()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::NewBucket {
                    who: owner,
                    msp_id,
                    bucket_id,
                    name,
                    collection_id: None,
                    private,
                }
                .into(),
            );
        });
    }

    #[test]
    fn create_bucket_msp_not_provider_fail() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            assert_noop!(
                FileSystem::create_bucket(origin, H256::from_slice(&msp.as_slice()), name, true),
                Error::<Test>::NotAMsp
            );
        });
    }
}

mod update_bucket_privacy_tests {
    use super::*;

    #[test]
    fn update_bucket_privacy_success() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let private = true;

            let msp_id = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::create_bucket(
                origin.clone(),
                msp_id,
                name.clone(),
                private
            ));

            // Check if collection was created
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::NewBucket {
                    who: owner.clone(),
                    msp_id,
                    bucket_id,
                    name,
                    collection_id: Some(0),
                    private,
                }
                .into(),
            );

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::update_bucket_privacy(origin, bucket_id, false));

            // Check that the bucket still has a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::BucketPrivacyUpdated {
                    who: owner,
                    bucket_id,
                    collection_id: Some(0),
                    private: false,
                }
                .into(),
            );
        });
    }

    #[test]
    fn update_bucket_privacy_bucket_not_found_fail() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            assert_noop!(
                FileSystem::update_bucket_privacy(origin, bucket_id, false),
                pallet_storage_providers::Error::<Test>::BucketNotFound
            );
        });
    }

    #[test]
    fn update_bucket_privacy_collection_remains_after_many_privacy_updates_success() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let private = true;

            let msp_id = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::create_bucket(
                origin.clone(),
                msp_id,
                name.clone(),
                private
            ));

            // Check if collection was created
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::NewBucket {
                    who: owner.clone(),
                    msp_id,
                    bucket_id,
                    name,
                    collection_id: Some(0),
                    private,
                }
                .into(),
            );

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::update_bucket_privacy(
                origin.clone(),
                bucket_id,
                false
            ));

            // Check that the bucket still has a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::BucketPrivacyUpdated {
                    who: owner.clone(),
                    bucket_id,
                    collection_id: Some(0),
                    private: false,
                }
                .into(),
            );

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::update_bucket_privacy(origin, bucket_id, true));

            // Check that the bucket still has a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::BucketPrivacyUpdated {
                    who: owner,
                    bucket_id,
                    collection_id: Some(0),
                    private: true,
                }
                .into(),
            );
        });
    }

    #[test]
    fn update_bucket_privacy_delete_collection_before_going_from_public_to_private_success() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let private = true;

            let msp_id = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::create_bucket(
                origin.clone(),
                msp_id,
                name.clone(),
                private
            ));

            // Check that the bucket does not have a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::NewBucket {
                    who: owner.clone(),
                    msp_id,
                    bucket_id,
                    name,
                    collection_id: Some(0),
                    private,
                }
                .into(),
            );

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::update_bucket_privacy(
                origin.clone(),
                bucket_id,
                false
            ));

            // Check that the bucket still has a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            let collection_id =
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .expect("Collection ID should exist");

            let w = Nfts::get_destroy_witness(&collection_id).unwrap();

            // Delete collection before going from public to private bucket
            assert_ok!(Nfts::destroy(origin.clone(), collection_id, w));

            // Update bucket privacy from public to private
            assert_ok!(FileSystem::update_bucket_privacy(origin, bucket_id, true));

            // Check that the bucket still has a corresponding collection
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            // Assert that the correct event was deposited and that a new collection with index 1 has been created
            System::assert_last_event(
                Event::BucketPrivacyUpdated {
                    who: owner,
                    bucket_id,
                    collection_id: Some(1),
                    private: true,
                }
                .into(),
            );
        });
    }
}

mod create_and_associate_collection_with_bucket_tests {
    use super::*;

    #[test]
    fn create_and_associate_collection_with_bucket_success() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let private = true;

            let msp_id = add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            // Dispatch a signed extrinsic.
            assert_ok!(FileSystem::create_bucket(
                origin.clone(),
                msp_id,
                name.clone(),
                private
            ));

            // Check if collection was created
            assert!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .is_some()
            );

            let collection_id =
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .expect("Collection ID should exist");

            assert_ok!(FileSystem::create_and_associate_collection_with_bucket(
                origin, bucket_id
            ));

            // Check if collection was associated with the bucket
            assert_ne!(
                <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(&bucket_id)
                    .unwrap()
                    .expect("Collection ID should exist"),
                collection_id
            );

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::NewCollectionAndAssociation {
                    who: owner,
                    bucket_id,
                    collection_id: 1, // Collection ID should be incremented from 0
                }
                .into(),
            );
        });
    }

    #[test]
    fn create_and_associate_collection_with_bucket_bucket_not_found_fail() {
        new_test_ext().execute_with(|| {
            let owner = Keyring::Alice.to_account_id();
            let origin = RuntimeOrigin::signed(owner.clone());
            let msp = Keyring::Charlie.to_account_id();
            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

            add_msp_to_provider_storage(&msp);

            let bucket_id =
                <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

            assert_noop!(
                FileSystem::create_and_associate_collection_with_bucket(origin, bucket_id),
                pallet_storage_providers::Error::<Test>::BucketNotFound
            );
        });
    }
}

#[test]
fn request_storage_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let user = RuntimeOrigin::signed(owner_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            user.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: Some(msp_id),
                user_peer_ids: peer_ids.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
            })
        );

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::NewStorageRequest {
                who: owner_account_id,
                file_key,
                bucket_id,
                location: location.clone(),
                fingerprint,
                size: 4,
                peer_ids,
            }
            .into(),
        );
    });
}

#[test]
fn request_storage_expiration_clear_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let size = 4;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: Some(msp_id),
                user_peer_ids: peer_ids.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
            })
        );

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        let expected_expiration_inserted_at_block_number: BlockNumber =
            FileSystem::next_expiration_insertion_block_number().into();

        // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_inserted_at_block_number),
            vec![file_key]
        );

        roll_to(expected_expiration_inserted_at_block_number + 1);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_inserted_at_block_number),
            vec![]
        );
    });
}

#[test]
fn request_storage_expiration_current_block_increment_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

        let mut expected_expiration_block_number: BlockNumber =
            FileSystem::next_expiration_insertion_block_number().into();

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        // Append storage request expiration to the list at `StorageRequestTtl`
        let max_storage_request_expiry: u32 = <Test as Config>::MaxExpiredStorageRequests::get();
        for _ in 0..(max_storage_request_expiry - 1) {
            assert_ok!(StorageRequestExpirations::<Test>::try_append(
                expected_expiration_block_number,
                file_key
            ));
        }

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            peer_ids,
        ));

        // Assert that the storage request expirations storage is at max capacity
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number).len(),
            max_storage_request_expiry as usize
        );

        expected_expiration_block_number =
            FileSystem::next_expiration_insertion_block_number().into();

        // Assert that the `CurrentExpirationBlock` storage is incremented by 1
        assert_eq!(
            FileSystem::next_available_expiration_insertion_block(),
            expected_expiration_block_number
        );

        // Go to block number after which the storage request expirations should be removed
        roll_to(expected_expiration_block_number);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number),
            vec![]
        );
    });
}

#[test]
fn request_storage_clear_old_expirations_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

        let expected_expiration_block_number: BlockNumber =
            FileSystem::next_expiration_insertion_block_number().into();

        // Append storage request expiration to the list at `StorageRequestTtl`
        let max_storage_request_expiry: u32 = <Test as Config>::MaxExpiredStorageRequests::get();

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        for _ in 0..(max_storage_request_expiry - 1) {
            assert_ok!(StorageRequestExpirations::<Test>::try_append(
                expected_expiration_block_number,
                file_key
            ));
        }

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            peer_ids,
        ));

        System::set_block_number(expected_expiration_block_number);

        // Assert that the `NextExpirationInsertionBlockNumber` storage is set to 0 initially
        assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

        // Assert that the storage request expirations storage is at max capacity
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number).len(),
            max_storage_request_expiry as usize
        );

        let used_weight = FileSystem::on_idle(System::block_number(), Weight::zero());

        // Assert that the weight used is zero
        assert_eq!(used_weight, Weight::zero());

        // Assert that the storage request expirations storage is at max capacity
        // TODO: Fix this test...
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number).len(),
            max_storage_request_expiry as usize
        );

        // Assert that the `NextExpirationInsertionBlockNumber` storage did not update
        assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

        // Go to block number after which the storage request expirations should be removed
        roll_to(expected_expiration_block_number + 1);

        // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(expected_expiration_block_number),
            vec![]
        );

        // Assert that the `NextExpirationInsertionBlockNumber` storage is set to the next block number
        assert_eq!(
            FileSystem::next_starting_block_to_clean_up(),
            System::block_number() + 1
        );
    });
}

#[test]
fn request_storage_bucket_does_not_exist_fail() {
    new_test_ext().execute_with(|| {
        let owner = Keyring::Alice.to_account_id();
        let origin = RuntimeOrigin::signed(owner.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name: BucketNameFor<Test> = BoundedVec::try_from([0u8; 32].to_vec()).unwrap();
        let bucket_id = H256::from_slice(&name);

        assert_noop!(
            FileSystem::issue_storage_request(
                origin,
                bucket_id,
                location.clone(),
                fingerprint,
                4,
                msp_id,
                peer_ids.clone(),
            ),
            pallet_storage_providers::Error::<Test>::BucketNotFound
        );
    });
}

#[test]
fn request_storage_not_bucket_owner_fail() {
    new_test_ext().execute_with(|| {
        let owner = Keyring::Alice.to_account_id();
        let not_owner = Keyring::Bob.to_account_id();
        let origin = RuntimeOrigin::signed(not_owner.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner, name.clone(), msp_id);

        assert_noop!(
            FileSystem::issue_storage_request(
                origin,
                bucket_id,
                location.clone(),
                fingerprint,
                4,
                msp_id,
                peer_ids.clone(),
            ),
            Error::<Test>::NotBucketOwner
        );
    });
}

#[test]
fn revoke_request_storage_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            Default::default()
        ));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
        assert_eq!(
            FileSystem::storage_request_expirations(
                FileSystem::next_expiration_insertion_block_number()
            ),
            vec![file_key]
        );

        assert_ok!(FileSystem::revoke_storage_request(owner.clone(), file_key));

        System::assert_last_event(Event::StorageRequestRevoked { file_key }.into());
    });
}

#[test]
fn revoke_non_existing_storage_request_fail() {
    new_test_ext().execute_with(|| {
        let owner = RuntimeOrigin::signed(Keyring::Alice.to_account_id());
        let file_key = H256::zero();

        assert_noop!(
            FileSystem::revoke_storage_request(owner.clone(), file_key),
            Error::<Test>::StorageRequestNotFound
        );
    });
}

#[test]
fn revoke_storage_request_not_owner_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let not_owner = RuntimeOrigin::signed(Keyring::Bob.to_account_id());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            Default::default()
        ));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        assert_noop!(
            FileSystem::revoke_storage_request(not_owner.clone(), file_key),
            Error::<Test>::StorageRequestNotAuthorized
        );
    });
}

#[test]
fn revoke_storage_request_with_volunteered_bsps_success() {
    new_test_ext().execute_with(|| {
        let owner_account = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account.clone(), name.clone(), msp_id);

        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            peer_ids.clone(),
        ));

        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

        let storage_amount: StorageData<Test> = 100;

        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
                .unwrap();

        let file_key = FileSystem::compute_file_key(
            owner_account.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

        // Check StorageRequestBsps storage for confirmed BSPs
        assert_eq!(
            FileSystem::storage_request_bsps(file_key, bsp_id)
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: false,
                _phantom: Default::default()
            }
        );

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::revoke_storage_request(owner.clone(), file_key));

        // Assert that the correct event was deposited
        System::assert_last_event(Event::StorageRequestRevoked { file_key }.into());
    });
}

#[test]
fn revoke_storage_request_with_confirmed_bsps_success() {
    new_test_ext().execute_with(|| {
        let owner_account = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account.clone(), name.clone(), msp_id);

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            peer_ids.clone(),
        ));

        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

        let storage_amount: StorageData<Test> = 100;

        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let file_key = FileSystem::compute_file_key(
            owner_account.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            file_key,
            H256::zero(),
            ForestProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            },
            KeyProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            }
        ));

        // Dispatch a signed extrinsic.
        assert_ok!(FileSystem::revoke_storage_request(owner.clone(), file_key));

        // Check ProofsDealer pallet storage for queued custom challenges for remove trie mutation of file key
        let priority_challenges_queue = PriorityChallengesQueue::<Test>::get();

        assert!(priority_challenges_queue.contains(&(file_key, Some(TrieRemoveMutation))));

        // Assert that the correct event was deposited
        System::assert_last_event(Event::StorageRequestRevoked { file_key }.into());
    });
}

#[test]
fn bsp_volunteer_success() {
    new_test_ext().execute_with(|| {
        let owner = Keyring::Alice.to_account_id();
        let origin = RuntimeOrigin::signed(owner.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = StorageData::<Test>::try_from(4).unwrap();
        // TODO: right now we are bypassing the volunteer assignment threshold
        let fingerprint = H256::zero();
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner.clone(), name.clone(), msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            origin,
            bucket_id,
            location.clone(),
            fingerprint,
            4,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let file_key = FileSystem::compute_file_key(
            owner.clone(),
            bucket_id,
            location.clone(),
            4,
            fingerprint,
        );

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
            .unwrap();

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

        // Assert that the RequestStorageBsps has the correct value
        assert_eq!(
            FileSystem::storage_request_bsps(file_key, bsp_id)
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: false,
                _phantom: Default::default()
            }
        );

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::AcceptedBspVolunteer {
                bsp_id,
                bucket_id,
                location,
                fingerprint,
                multiaddresses: create_sp_multiaddresses(),
                owner,
                size,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_volunteer_storage_request_not_found_fail() {
    new_test_ext().execute_with(|| {
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let fingerprint = H256::zero();

        assert_ok!(bsp_sign_up(bsp_signed.clone(), 100,));

        let file_key = FileSystem::compute_file_key(
            bsp_account_id.clone(),
            H256::zero(),
            location.clone(),
            4,
            fingerprint,
        );

        assert_noop!(
            FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
            Error::<Test>::StorageRequestNotFound
        );
    });
}

#[test]
fn bsp_already_volunteered_failed() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

        assert_noop!(
            FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
            Error::<Test>::BspAlreadyVolunteered
        );
    });
}

#[test]
fn bsp_volunteer_above_threshold_high_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        crate::BspsAssignmentThreshold::<Test>::put(FixedU128::zero());

        // Dispatch BSP volunteer.
        assert_noop!(
            FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
            Error::<Test>::AboveThreshold
        );
    });
}

#[test]
fn bsp_confirm_storing_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let fingerprint = H256::zero();
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
            .unwrap();

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            file_key,
            H256::zero(), // TODO construct a real proof
            ForestProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            },
            KeyProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            }
        ));

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: Some(msp_id),
                user_peer_ids: peer_ids.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 1,
                bsps_volunteered: 1,
            })
        );

        // Assert that the RequestStorageBsps was updated
        assert_eq!(
            FileSystem::storage_request_bsps(file_key, bsp_id)
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: true,
                _phantom: Default::default()
            }
        );

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        let new_root =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_root(
                bsp_id,
            )
            .unwrap();

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspConfirmedStoring {
                bsp_id,
                file_key,
                new_root,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_confirm_storing_storage_request_not_found_fail() {
    new_test_ext().execute_with(|| {
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), 100,));

        let file_key = FileSystem::compute_file_key(
            bsp_account_id.clone(),
            H256::zero(),
            location.clone(),
            4,
            H256::zero(),
        );

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                file_key,
                H256::zero(),
                ForestProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                },
                KeyProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                }
            ),
            Error::<Test>::StorageRequestNotFound
        );
    });
}

#[test]
fn bsp_confirm_storing_not_volunteered_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(vec![1]).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                file_key,
                H256::zero(), // TODO construct a real proof
                ForestProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                },
                KeyProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                }
            ),
            Error::<Test>::BspNotVolunteered
        );
    });
}

#[test]
fn bsp_already_confirmed_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(vec![1]).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner_signed.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            file_key,
            H256::zero(), // TODO construct a real proof
            ForestProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            },
            KeyProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            }
        ));

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                file_key,
                H256::zero(), // TODO construct a real proof
                ForestProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                },
                KeyProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                }
            ),
            Error::<Test>::BspAlreadyConfirmed
        );
    });
}

#[test]
fn bsp_actions_not_a_bsp_fail() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let file_content = b"test".to_vec();
        let fingerprint = BlakeTwo256::hash(&file_content);
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(vec![1]).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            RuntimeOrigin::signed(owner_account_id.clone()),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        assert_noop!(
            FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
            Error::<Test>::NotABsp
        );

        assert_noop!(
            FileSystem::bsp_confirm_storing(
                bsp_signed.clone(),
                file_key,
                H256::zero(), // TODO construct a real proof
                ForestProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                },
                KeyProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                }
            ),
            Error::<Test>::NotABsp
        );
    });
}

#[test]
fn bsp_stop_storing_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        // TODO: right now we are bypassing the volunteer assignment threshold
        let fingerprint = H256::zero();
        let peer_id = BoundedVec::try_from(vec![1]).unwrap();
        let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(vec![1]).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            peer_ids.clone(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
            .unwrap();

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            file_key,
            H256::zero(), // TODO construct a real proof
            ForestProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            },
            KeyProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            }
        ));

        // Assert that the RequestStorageBsps now contains the BSP under the location
        assert_eq!(
            FileSystem::storage_request_bsps(file_key, bsp_id)
                .expect("BSP should exist in storage"),
            StorageRequestBspsMetadata::<Test> {
                confirmed: true,
                _phantom: Default::default()
            }
        );

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: Some(msp_id),
                user_peer_ids: peer_ids.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 1,
                bsps_volunteered: 1,
            })
        );

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            bucket_id,
            location.clone(),
            owner_account_id.clone(),
            fingerprint,
            size,
            false,
            ForestProof {
                encoded_nodes: vec![file_key.as_ref().to_vec()],
            },
        ));

        // Assert that the RequestStorageBsps has the correct value
        assert!(FileSystem::storage_request_bsps(file_key, bsp_id).is_none());

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: Some(msp_id),
                user_peer_ids: peer_ids.clone(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
            })
        );

        let new_root =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_root(
                bsp_id,
            )
            .unwrap();

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspStoppedStoring {
                bsp_id,
                file_key,
                new_root,
                owner: owner_account_id,
                location,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_stop_storing_while_storage_request_open_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let fingerprint = H256::zero();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(vec![1]).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            Default::default(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
            .unwrap();

        // Dispatch BSP volunteer.
        assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

        // Dispatch BSP confirm storing.
        assert_ok!(FileSystem::bsp_confirm_storing(
            bsp_signed.clone(),
            file_key,
            H256::zero(),
            ForestProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            },
            KeyProof {
                encoded_nodes: vec![H256::default().as_ref().to_vec()],
            }
        ));

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            bucket_id,
            location.clone(),
            owner_account_id.clone(),
            H256::zero(),
            size,
            false,
            ForestProof {
                encoded_nodes: vec![file_key.as_ref().to_vec()],
            },
        ));

        // Assert that the RequestStorageBsps has the correct value
        assert!(FileSystem::storage_request_bsps(file_key, bsp_id).is_none());

        // Assert that the storage was updated
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint: H256::zero(),
                size,
                msp: Some(msp_id),
                user_peer_ids: Default::default(),
                data_server_sps: BoundedVec::default(),
                bsps_required: TargetBspsRequired::<Test>::get(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
            })
        );

        let new_root =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_root(
                bsp_id,
            )
            .unwrap();

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspStoppedStoring {
                bsp_id,
                file_key,
                new_root,
                owner: owner_account_id,
                location,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_stop_storing_not_volunteered_success() {
    new_test_ext().execute_with(|| {
        let owner_account_id = Keyring::Alice.to_account_id();
        let owner = RuntimeOrigin::signed(owner_account_id.clone());
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let msp = Keyring::Charlie.to_account_id();
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let size = 4;
        let fingerprint = H256::zero();
        let storage_amount: StorageData<Test> = 100;

        let msp_id = add_msp_to_provider_storage(&msp);

        let name = BoundedVec::try_from(vec![1]).unwrap();
        let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

        // Dispatch storage request.
        assert_ok!(FileSystem::issue_storage_request(
            owner.clone(),
            bucket_id,
            location.clone(),
            fingerprint,
            size,
            msp_id,
            Default::default(),
        ));

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
            .unwrap();

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            bucket_id,
            location.clone(),
            owner_account_id.clone(),
            fingerprint,
            size,
            false,
            ForestProof {
                encoded_nodes: vec![file_key.as_ref().to_vec()],
            },
        ));

        let current_bsps_required: <Test as Config>::StorageRequestBspsRequiredType =
            TargetBspsRequired::<Test>::get();

        // Assert that the storage request bsps_required was incremented
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 1,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: Some(msp_id),
                user_peer_ids: Default::default(),
                data_server_sps: BoundedVec::default(),
                bsps_required: current_bsps_required.checked_add(1).unwrap(),
                bsps_confirmed: 0,
                bsps_volunteered: 0,
            })
        );

        let new_root =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_root(
                bsp_id,
            )
            .unwrap();

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspStoppedStoring {
                bsp_id,
                file_key,
                new_root,
                owner: owner_account_id,
                location,
            }
            .into(),
        );
    });
}

#[test]
fn bsp_stop_storing_no_storage_request_success() {
    new_test_ext().execute_with(|| {
        let bsp_account_id = Keyring::Bob.to_account_id();
        let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
        let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
        let owner_account_id = Keyring::Alice.to_account_id();
        let size = 4;
        let fingerprint = H256::zero();

        let msp_id = add_msp_to_provider_storage(&Keyring::Charlie.to_account_id());

        let bucket_id = create_bucket(
            &owner_account_id,
            BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
            msp_id,
        );

        // Sign up account as a Backup Storage Provider
        assert_ok!(bsp_sign_up(bsp_signed.clone(), 100));

        let bsp_id =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_provider_id(
                bsp_account_id,
            )
            .unwrap();

        let file_key = FileSystem::compute_file_key(
            owner_account_id.clone(),
            bucket_id,
            location.clone(),
            size,
            fingerprint,
        );

        // Dispatch BSP stop storing.
        assert_ok!(FileSystem::bsp_stop_storing(
            bsp_signed.clone(),
            file_key,
            bucket_id,
            location.clone(),
            owner_account_id.clone(),
            fingerprint,
            size,
            false,
            ForestProof {
                encoded_nodes: vec![file_key.as_ref().to_vec()],
            },
        ));

        // Assert that the storage request was created with one bsps_required
        assert_eq!(
            FileSystem::storage_requests(file_key),
            Some(StorageRequestMetadata {
                requested_at: 5,
                owner: owner_account_id.clone(),
                bucket_id,
                location: location.clone(),
                fingerprint,
                size,
                msp: None,
                user_peer_ids: Default::default(),
                data_server_sps: BoundedVec::default(),
                bsps_required: 1,
                bsps_confirmed: 0,
                bsps_volunteered: 0,
            })
        );

        let new_root =
            <<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_root(
                bsp_id,
            )
            .unwrap();

        // Assert that the correct event was deposited
        System::assert_last_event(
            Event::BspStoppedStoring {
                bsp_id,
                file_key,
                new_root,
                owner: owner_account_id,
                location,
            }
            .into(),
        );
    });
}

#[test]
fn compute_asymptotic_threshold_point_success() {
    new_test_ext().execute_with(|| {
        // Test the computation of the asymptotic threshold
        let threshold = FileSystem::compute_asymptotic_threshold_point(1)
            .expect("Threshold should be computable");

        // Assert that the computed threshold is as expected
        assert!(
            threshold > FixedU128::zero()
                && threshold >= <Test as Config>::AssignmentThresholdAsymptote::get(),
            "Threshold should be positive"
        );
    });
}

#[test]
fn subscribe_bsp_sign_up_decreases_threshold_success() {
    new_test_ext().execute_with(|| {
        let initial_threshold = compute_set_get_initial_threshold();

        // Simulate the threshold decrease due to a new BSP sign up
        FileSystem::subscribe_bsp_sign_up(&H256::from_slice(&[1; 32]))
            .expect("BSP sign up should be successful");

        let updated_threshold = FileSystem::bsps_assignment_threshold();
        // Verify that the threshold decreased
        assert!(
            updated_threshold < initial_threshold
                && updated_threshold >= <Test as Config>::AssignmentThresholdAsymptote::get(),
            "Threshold should decrease after BSP sign up"
        );
    });
}

#[test]
fn subscribe_bsp_sign_off_increases_threshold_success() {
    new_test_ext().execute_with(|| {
        let initial_threshold = compute_set_get_initial_threshold();

        // Simulate the threshold increase due to a new BSP sign off
        FileSystem::subscribe_bsp_sign_off(&H256::from_slice(&[1; 32]))
            .expect("BSP sign off should be successful");

        let updated_threshold = FileSystem::bsps_assignment_threshold();
        // Verify that the threshold increased
        assert!(
            updated_threshold > initial_threshold
                && updated_threshold >= <Test as Config>::AssignmentThresholdAsymptote::get(),
            "Threshold should increase after BSP sign off"
        );
    });
}

mod force_bsps_assignment_threshold_tests {
    use super::*;

    #[test]
    fn force_bsps_assignment_threshold_non_root_signer_fail() {
        new_test_ext().execute_with(|| {
            let non_root = Keyring::Bob.to_account_id();
            let non_root_signed = RuntimeOrigin::signed(non_root.clone());

            // Assert BadOrigin error when non-root account tries to set the threshold
            assert_noop!(
                FileSystem::force_update_bsps_assignment_threshold(
                    non_root_signed,
                    FixedU128::zero()
                ),
                DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn force_bsps_assignment_threshold_below_asymptote_fail() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                FileSystem::force_update_bsps_assignment_threshold(
                    RuntimeOrigin::root(),
                    <Test as Config>::AssignmentThresholdAsymptote::get() - FixedU128::one()
                ),
                Error::<Test>::ThresholdBelowAsymptote
            );
        });
    }

    #[test]
    fn force_bsps_assignment_threshold_above_asymptote_success() {
        new_test_ext().execute_with(|| {
            let new_threshold = <Test as crate::Config>::AssignmentThresholdAsymptote::get();

            FileSystem::force_update_bsps_assignment_threshold(
                RuntimeOrigin::root(),
                new_threshold,
            )
            .expect("Threshold should be set successfully");

            // Verify that the threshold increased
            assert!(
                FileSystem::bsps_assignment_threshold() == new_threshold,
                "Threshold should be set to one"
            );
        });
    }
}

#[test]
fn threshold_does_not_exceed_asymptote_success() {
    new_test_ext().execute_with(|| {
        crate::BspsAssignmentThreshold::<Test>::put(
            <Test as Config>::AssignmentThresholdAsymptote::get(),
        );

        // Simulate the threshold decrease due to a new BSP sign up
        FileSystem::subscribe_bsp_sign_up(&H256::from_slice(&[1; 32]))
            .expect("BSP sign up should be successful");

        // Verify that the threshold does is equal to the asymptote
        assert!(
            FileSystem::bsps_assignment_threshold()
                == <Test as Config>::AssignmentThresholdAsymptote::get(),
            "Threshold should not go below the asymptote"
        );
    });
}

/// Helper function that registers an account as a Backup Storage Provider
fn bsp_sign_up(
    bsp_signed: RuntimeOrigin,
    storage_amount: StorageData<Test>,
) -> DispatchResultWithPostInfo {
    let multiaddresses = create_sp_multiaddresses();

    // Request to sign up the account as a Backup Storage Provider
    assert_ok!(Providers::request_bsp_sign_up(
        bsp_signed.clone(),
        storage_amount,
        multiaddresses,
        bsp_signed.clone().into_signer().unwrap()
    ));

    // Advance enough blocks for randomness to be valid
    roll_to(frame_system::Pallet::<Test>::block_number() + 4);

    // Confirm the sign up of the account as a Backup Storage Provider
    assert_ok!(Providers::confirm_sign_up(bsp_signed.clone(), None));

    Ok(().into())
}

fn create_sp_multiaddresses(
) -> BoundedVec<BoundedVec<u8, MaxMultiAddressSize>, MaxMultiAddressAmount> {
    let mut multiaddresses: BoundedVec<BoundedVec<u8, MaxMultiAddressSize>, MaxMultiAddressAmount> =
        BoundedVec::new();
    multiaddresses.force_push(
        "/ip4/127.0.0.1/udp/1234"
            .as_bytes()
            .to_vec()
            .try_into()
            .unwrap(),
    );
    multiaddresses
}

fn add_msp_to_provider_storage(msp: &sp_runtime::AccountId32) -> ProviderIdFor<Test> {
    let msp_hash = <<Test as frame_system::Config>::Hashing as Hasher>::hash(msp.as_slice());

    let msp_info = pallet_storage_providers::types::MainStorageProvider {
        buckets: BoundedVec::default(),
        capacity: 100,
        data_used: 0,
        multiaddresses: BoundedVec::default(),
        value_prop: pallet_storage_providers::types::ValueProposition {
            identifier: pallet_storage_providers::types::ValuePropId::<Test>::default(),
            data_limit: 100,
            protocols: BoundedVec::default(),
        },
        last_capacity_change: frame_system::Pallet::<Test>::block_number(),
        payment_account: msp.clone(),
    };

    pallet_storage_providers::MainStorageProviders::<Test>::insert(msp_hash, msp_info);
    pallet_storage_providers::AccountIdToMainStorageProviderId::<Test>::insert(
        msp.clone(),
        msp_hash,
    );

    msp_hash
}

fn create_bucket(
    owner: &sp_runtime::AccountId32,
    name: BucketNameFor<Test>,
    msp_id: ProviderIdFor<Test>,
) -> BucketIdFor<Test> {
    let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(&owner, name.clone());

    let origin = RuntimeOrigin::signed(owner.clone());

    // Dispatch a signed extrinsic.
    assert_ok!(FileSystem::create_bucket(
        origin,
        msp_id,
        name.clone(),
        true
    ));

    bucket_id
}

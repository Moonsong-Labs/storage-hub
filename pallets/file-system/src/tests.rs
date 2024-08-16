use crate::{
    mock::*,
    types::{
        BucketIdFor, BucketNameFor, ExpiredItems, FileLocation, PeerIds,
        PendingFileDeletionRequestTtl, ProviderIdFor, StorageData, StorageRequestBspsMetadata,
        StorageRequestMetadata, StorageRequestTtl, TargetBspsRequired,
    },
    Config, Error, Event, ItemExpirations, PendingStopStoringRequests,
};
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchResultWithPostInfo,
    traits::{nonfungibles_v2::Destroy, Hooks, OriginTrait},
    weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_proofs_dealer::{LastTickProviderSubmittedAProofFor, PriorityChallengesQueue};
use pallet_storage_providers::types::Bucket;
use shp_traits::{ReadProvidersInterface, SubscribeProvidersInterface, TrieRemoveMutation};
use sp_core::{ByteArray, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{BlakeTwo256, Get, One, Zero},
    BoundedVec, DispatchError, FixedU128,
};
use sp_trie::CompactProof;

mod create_bucket_tests {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn create_bucket_msp_not_provider_fail() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

                assert_noop!(
                    FileSystem::create_bucket(
                        origin,
                        H256::from_slice(&msp.as_slice()),
                        name,
                        true
                    ),
                    Error::<Test>::NotAMsp
                );
            });
        }
    }

    mod success {
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
    }
}

mod update_bucket_privacy_tests {
    use super::*;

    mod failure {
        use super::*;

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
    }

    mod success {
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
                    .unwrap()
                    .is_some()
                );

                let collection_id =
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id,
                    )
                    .unwrap()
                    .expect("Collection ID should exist");

                let w = Nfts::get_destroy_witness(&collection_id).unwrap();

                // Delete collection before going from public to private bucket
                assert_ok!(Nfts::destroy(origin.clone(), collection_id, w));

                // Update bucket privacy from public to private
                assert_ok!(FileSystem::update_bucket_privacy(origin, bucket_id, true));

                // Check that the bucket still has a corresponding collection
                assert!(
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
}

mod create_and_associate_collection_with_bucket_tests {
    use super::*;

    mod failure {
        use super::*;

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

    mod success {
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
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
                    .unwrap()
                    .is_some()
                );

                let collection_id =
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id,
                    )
                    .unwrap()
                    .expect("Collection ID should exist");

                assert_ok!(FileSystem::create_and_associate_collection_with_bucket(
                    origin, bucket_id
                ));

                // Check if collection was associated with the bucket
                assert_ne!(
                    <Test as crate::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
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
    }
}

mod request_storage {
    use super::*;
    mod failure {
        use super::*;

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
    }

    mod success {
        use super::*;

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
        fn two_request_storage_in_same_block() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let user = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let file_1_location = FileLocation::<Test>::try_from(b"test1".to_vec()).unwrap();
                let file_2_location = FileLocation::<Test>::try_from(b"test2".to_vec()).unwrap();
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
                    file_1_location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    file_1_location.clone(),
                    size,
                    fingerprint,
                );

                // Assert that the storage was updated
                assert_eq!(
                    FileSystem::storage_requests(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: 1,
                        owner: owner_account_id.clone(),
                        bucket_id: bucket_id.clone(),
                        location: file_1_location.clone(),
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

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    file_2_location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    file_2_location.clone(),
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
                        location: file_2_location.clone(),
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
                        location: file_2_location.clone(),
                        fingerprint,
                        size,
                        peer_ids,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn request_storage_failure_if_size_is_zero() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let user = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 0;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

                // Dispatch a signed extrinsic.
                assert_noop!(
                    FileSystem::issue_storage_request(
                        user.clone(),
                        bucket_id,
                        location.clone(),
                        fingerprint,
                        size,
                        msp_id,
                        peer_ids.clone(),
                    ),
                    Error::<Test>::FileSizeCannotBeZero
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

                let storage_request_ttl: u32 = StorageRequestTtl::<Test>::get();
                let storage_request_ttl: BlockNumberFor<Test> = storage_request_ttl.into();

                // Assert that the next starting block to clean up is set to 0 initially
                assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

                // Assert that the next expiration block number is the storage request ttl since a single storage request was made
                assert_eq!(
                    FileSystem::next_available_expiration_insertion_block(),
                    storage_request_ttl
                );

                // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::item_expirations(storage_request_ttl),
                    vec![ExpiredItems::StorageRequest(file_key)]
                );

                roll_to(storage_request_ttl + 1);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(FileSystem::item_expirations(storage_request_ttl), vec![]);
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

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                let expected_expiration_block_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_block_number: BlockNumberFor<Test> =
                    expected_expiration_block_number.into();

                // Append storage request expiration to the list at `StorageRequestTtl`
                let max_expired_items_in_block: u32 =
                    <Test as Config>::MaxExpiredItemsInBlock::get();
                for _ in 0..max_expired_items_in_block {
                    assert_ok!(ItemExpirations::<Test>::try_append(
                        expected_expiration_block_number,
                        ExpiredItems::StorageRequest(file_key)
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
                    FileSystem::item_expirations(expected_expiration_block_number).len(),
                    max_expired_items_in_block as usize
                );

                // Go to block number after which the storage request expirations should be removed
                roll_to(expected_expiration_block_number);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::item_expirations(expected_expiration_block_number),
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

                // Append storage request expiration to the list at `StorageRequestTtl`
                let max_storage_request_expiry: u32 =
                    <Test as Config>::MaxExpiredItemsInBlock::get();

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                let expected_expiration_block_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_block_number: BlockNumberFor<Test> =
                    expected_expiration_block_number.into();

                for _ in 0..max_storage_request_expiry {
                    assert_ok!(ItemExpirations::<Test>::try_append(
                        expected_expiration_block_number,
                        ExpiredItems::StorageRequest(file_key)
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

                let expected_expiration_block_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_block_number: BlockNumberFor<Test> =
                    expected_expiration_block_number.into();

                // Assert that the `NextExpirationInsertionBlockNumber` storage is set to 0 initially
                assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

                // Assert that the storage request expirations storage is at max capacity
                assert_eq!(
                    FileSystem::item_expirations(expected_expiration_block_number).len(),
                    max_storage_request_expiry as usize
                );

                let used_weight = FileSystem::on_idle(System::block_number(), Weight::zero());

                // Assert that the weight used is zero
                assert_eq!(used_weight, Weight::zero());

                // Assert that the storage request expirations storage is at max capacity
                // TODO: Fix this test...
                assert_eq!(
                    FileSystem::item_expirations(expected_expiration_block_number).len(),
                    max_storage_request_expiry as usize
                );

                // Assert that the `NextExpirationInsertionBlockNumber` storage did not update
                assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

                // Go to block number after which the storage request expirations should be removed
                roll_to(expected_expiration_block_number + 1);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::item_expirations(expected_expiration_block_number),
                    vec![]
                );

                // Assert that the `NextExpirationInsertionBlockNumber` storage is set to the next block number
                assert_eq!(
                    FileSystem::next_starting_block_to_clean_up(),
                    System::block_number() + 1
                );
            });
        }
    }
}

mod revoke_storage_request {
    use super::*;
    mod failure {
        use super::*;

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
    }

    mod success {
        use super::*;

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

                let storage_request_ttl: u32 = StorageRequestTtl::<Test>::get();
                let storage_request_ttl: BlockNumberFor<Test> = storage_request_ttl.into();

                // Assert that the NextExpirationInsertionBlockNumber storage is set to 0 initially
                assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

                // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::item_expirations(storage_request_ttl),
                    vec![ExpiredItems::StorageRequest(file_key)]
                );

                assert_ok!(FileSystem::revoke_storage_request(owner.clone(), file_key));

                System::assert_last_event(Event::StorageRequestRevoked { file_key }.into());
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![(
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    )])
                    .unwrap(),
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
    }
}

mod bsp_volunteer {
    use super::*;
    mod failure {
        use super::*;

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
    }

    mod success {
        use super::*;

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
    }
}

mod bsp_confirm {
    use super::*;
    mod failure {
        use super::*;

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
                    FileSystem::bsp_confirm_storing(
                        bsp_signed.clone(),
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![(
                            file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        )])
                        .unwrap(),
                    ),
                    Error::<Test>::NotABsp
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
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![(
                            file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        )])
                        .unwrap(),
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
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![(
                            file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        )])
                        .unwrap(),
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![(
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    )])
                    .unwrap(),
                ));

                assert_noop!(
                    FileSystem::bsp_confirm_storing(
                        bsp_signed.clone(),
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![(
                            file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        )])
                        .unwrap(),
                    ),
                    Error::<Test>::BspAlreadyConfirmed
                );
            });
        }
    }

    mod success {
        use super::*;

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
                        bsp_account_id.clone(),
                    )
                        .unwrap();

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // In this case, the tick number is going to be equal to the current block number
                // minus one (on_poll hook not executed in first block)
                let tick_when_confirming = System::block_number() - 1;

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![(file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            })]).unwrap()
                    ,
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
                        who: bsp_account_id.clone(),
                        bsp_id,
                        file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        new_root,
                    }
                        .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let last_tick_provider_submitted_proof =
                    LastTickProviderSubmittedAProofFor::<Test>::get(&bsp_id).unwrap();
                assert_eq!(last_tick_provider_submitted_proof, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                        .into(),
                );
            });
        }
    }
}

mod bsp_stop_storing {
    use super::*;
    mod failure {
        use super::*;
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

                // Dispatch BSP stop storing.
                assert_noop!(
                    FileSystem::bsp_request_stop_storing(
                        bsp_signed.clone(),
                        file_key,
                        bucket_id,
                        location.clone(),
                        owner_account_id.clone(),
                        fingerprint,
                        size,
                        false,
                        CompactProof {
                            encoded_nodes: vec![file_key.as_ref().to_vec()],
                        },
                    ),
                    Error::<Test>::NotABsp
                );
            });
        }

        #[test]
        fn bsp_request_stop_storing_fails_if_file_key_does_not_match_metadata() {
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },BoundedVec::try_from(vec![(file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            })]).unwrap()
                    ,
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
                    size - 1, // We change the size so the file key doesn't match the file's metadata
                    fingerprint,
                );

                // Dispatch BSP stop storing.
                assert_noop!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
						encoded_nodes: vec![file_key.as_ref().to_vec()],
					},
				), Error::<Test>::InvalidFileKeyMetadata);

            });
        }

        #[test]
        fn bsp_request_stop_storing_fails_if_pending_stop_storing_request_exists() {
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },BoundedVec::try_from(vec![(file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            })]).unwrap()
                    ,
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

                // Dispatch BSP request stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
						encoded_nodes: vec![file_key.as_ref().to_vec()],
					},
				));

				// Check that the request now exists.
				assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_some());

				// Try sending the stop storing request again.
                assert_noop!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
						encoded_nodes: vec![file_key.as_ref().to_vec()],
					},
				), Error::<Test>::PendingStopStoringRequestAlreadyExists);

            });
        }

        #[test]
        fn bsp_confirm_stop_storing_fails_if_not_enough_time_has_passed_since_request() {
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },BoundedVec::try_from(vec![(file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            })]).unwrap()
                    ,
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

                // Dispatch BSP request stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
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

				// Assert that the request was added to the pending stop storing requests.
				assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_some());

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id,
                        location,
                    }
                        .into(),
                );

				// Dispatch BSP confirm stop storing.
				assert_noop!(FileSystem::bsp_confirm_stop_storing(
					bsp_signed.clone(),
					file_key,
					CompactProof {
						encoded_nodes: vec![file_key.as_ref().to_vec()],
					},
				), Error::<Test>::MinWaitForStopStoringNotReached);

				// Assert that the pending stop storing request is still there.
				assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_some());
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn bsp_request_stop_storing_success() {
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },BoundedVec::try_from(vec![(file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            })]).unwrap()
                    ,
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

                // Dispatch BSP request stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
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

				// Assert that the request was added to the pending stop storing requests.
				assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_some());

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id,
                        location,
                    }
                        .into(),
                );
            });
        }

        #[test]
        fn bsp_confirm_stop_storing_success() {
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },BoundedVec::try_from(vec![(file_key,
                            CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            })]).unwrap()
                    ,
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

                // Dispatch BSP request stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
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

				// Assert that the request was added to the pending stop storing requests.
				assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_some());

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id,
                        location,
                    }
                        .into(),
                );

				// Advance enough blocks to allow the BSP to confirm the stop storing request.
				roll_to(frame_system::Pallet::<Test>::block_number() + MinWaitForStopStoring::get());

				// Dispatch BSP confirm stop storing.
				assert_ok!(FileSystem::bsp_confirm_stop_storing(
					bsp_signed.clone(),
					file_key,
					CompactProof {
						encoded_nodes: vec![file_key.as_ref().to_vec()],
					},
				));

				// Assert that the pending stop storing request was removed.
				assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_none());

				// Assert that the correct event was deposited.
				let new_root =
					<<Test as crate::Config>::Providers as shp_traits::ProvidersInterface>::get_root(
						bsp_id,
					)
						.unwrap();

				System::assert_last_event(
					Event::BspConfirmStoppedStoring {
						bsp_id,
						file_key,
						new_root,
					}
						.into(),
				);
            });
        }

        #[test]
        fn bsp_request_stop_storing_while_storage_request_open_success() {
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
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![(file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    })]).unwrap()
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Dispatch BSP stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					H256::zero(),
					size,
					false,
					CompactProof {
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

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id,
                        location,
                    }
                        .into(),
                );
            });
        }

        #[test]
        fn bsp_request_stop_storing_not_volunteered_success() {
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

                // Increase the data used by the registered bsp, to simulate that it is indeed storing the file
                assert_ok!(<<Test as crate::Config>::Providers as shp_traits::MutateProvidersInterface>::increase_data_used(
					&bsp_id, size,
				));

                // Dispatch BSP stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
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

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id,
                        location,
                    }
                        .into(),
                );
            });
        }

        #[test]
        fn bsp_request_stop_storing_no_storage_request_success() {
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

                // Increase the data used by the registered bsp, to simulate that it is indeed storing the file
                assert_ok!(<<Test as crate::Config>::Providers as shp_traits::MutateProvidersInterface>::increase_data_used(
					&bsp_id, size,
				));

                // Dispatch BSP stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
					bsp_signed.clone(),
					file_key,
					bucket_id,
					location.clone(),
					owner_account_id.clone(),
					fingerprint,
					size,
					false,
					CompactProof {
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

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id,
                        location,
                    }
                        .into(),
                );
            });
        }
    }
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

mod bsp_subscribe_asymptotic_threshold_computation {
    use super::*;
    mod success {
        use super::*;

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
                        && updated_threshold
                            >= <Test as Config>::AssignmentThresholdAsymptote::get(),
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
                        && updated_threshold
                            >= <Test as Config>::AssignmentThresholdAsymptote::get(),
                    "Threshold should increase after BSP sign off"
                );
            });
        }
    }
}

mod force_bsps_assignment_threshold_tests {
    use super::*;

    mod failure {
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
    }

    mod success {
        use super::*;

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
}

mod delete_file_and_pending_deletions_tests {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn delete_file_bucket_not_owned_by_user_fail() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let _ = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

                let other_user = Keyring::Bob.to_account_id();
                let bucket_id = create_bucket(&other_user.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Assert that the user does not own the bucket
                assert_noop!(
                    FileSystem::delete_file(
                        owner_signed,
                        bucket_id,
                        file_key,
                        location,
                        size,
                        fingerprint,
                        Some(forest_proof),
                    ),
                    Error::<Test>::NotBucketOwner
                );
            });
        }

        #[test]
        fn delete_file_beyond_maximum_limit_allowed_fail() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = u32::MAX;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name.clone(), msp_id);

                // For loop to create 1 over maximum of MaxUserPendingDeletionRequests
                for i in 0..<Test as crate::Config>::MaxUserPendingDeletionRequests::get() {
                    let file_key = FileSystem::compute_file_key(
                        owner_account_id.clone(),
                        bucket_id,
                        location.clone(),
                        i,
                        fingerprint,
                    );

                    assert_ok!(FileSystem::delete_file(
                        owner_signed.clone(),
                        bucket_id,
                        file_key,
                        location.clone(),
                        i,
                        fingerprint,
                        None,
                    ));
                }

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                assert_noop!(
                    FileSystem::delete_file(
                        owner_signed,
                        bucket_id,
                        file_key,
                        location,
                        size,
                        fingerprint,
                        None
                    ),
                    Error::<Test>::MaxUserPendingDeletionRequestsReached
                );
            });
        }

        #[test]
        fn delete_file_pending_file_deletion_request_submit_proof_not_msp_of_bucket_fail() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Delete file
                assert_ok!(FileSystem::delete_file(
					owner_signed.clone(),
					bucket_id,
					file_key,
					location,
					size,
					fingerprint,
					None,
				));

                // Assert that the pending file deletion request was added to storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id.clone()),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![(file_key, bucket_id)]
                    )
                        .unwrap()
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![vec![0]],
                };

                let msp_dave = Keyring::Dave.to_account_id();
                add_msp_to_provider_storage(&msp_dave);
                let msp_origin = RuntimeOrigin::signed(msp_dave.clone());

                assert_noop!(
					FileSystem::pending_file_deletion_request_submit_proof(
						msp_origin,
						owner_account_id.clone(),
						file_key,
						bucket_id,
						forest_proof
					),
					Error::<Test>::MspNotStoringBucket
				);

                // Assert that the pending file deletion request was not removed from storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![(file_key, bucket_id)]
                    )
                        .unwrap()
                );
            });
        }

        #[test]
        fn submit_proof_pending_file_deletion_not_found_fail() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();

                let msp = Keyring::Charlie.to_account_id();
                let msp_origin = RuntimeOrigin::signed(msp.clone());

                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![vec![0]],
                };

                assert_noop!(
                    FileSystem::pending_file_deletion_request_submit_proof(
                        msp_origin,
                        owner_account_id.clone(),
                        file_key,
                        bucket_id,
                        forest_proof
                    ),
                    Error::<Test>::FileKeyNotPendingDeletion
                );
            });
        }
    }

    mod success {
        use super::*;
        #[test]
        fn delete_file_with_proof_of_inclusion_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Delete file
                assert_ok!(FileSystem::delete_file(
                    owner_signed.clone(),
                    bucket_id,
                    file_key,
                    location,
                    size,
                    fingerprint,
                    Some(forest_proof),
                ));

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(
                    // Find file key in vec of queued priority challenges
                    pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
                        .iter()
                        .any(|x| *x == (file_key, Some(TrieRemoveMutation))),
                );

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::FileDeletionRequest {
                        user: owner_account_id.clone(),
                        file_key,
                        bucket_id,
                        msp_id,
                        proof_of_inclusion: true,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn delete_file_expired_pending_file_deletion_request_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Delete file
                assert_ok!(FileSystem::delete_file(
					owner_signed.clone(),
					bucket_id,
					file_key,
					location,
					size,
					fingerprint,
					None,
				));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::FileDeletionRequest {
                        user: owner_account_id.clone(),
                        file_key,
                        bucket_id,
                        msp_id,
                        proof_of_inclusion: false,
                    }
                        .into(),
                );

                // Assert that the pending file deletion request was added to storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id.clone()),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![(file_key, bucket_id)]
                    )
                        .unwrap()
                );

                let pending_file_deletion_request_ttl: u32 =
                    PendingFileDeletionRequestTtl::<Test>::get();
                let pending_file_deletion_request_ttl: BlockNumberFor<Test> =
                    pending_file_deletion_request_ttl.into();

                // Assert that the pending file deletion request was added to storage
                assert_eq!(
                    FileSystem::item_expirations(pending_file_deletion_request_ttl),
                    vec![ExpiredItems::PendingFileDeletionRequests((
                        owner_account_id.clone(),
                        file_key
                    ))]
                );

                // Roll past the expiration block
                roll_to(pending_file_deletion_request_ttl);

                // Item expiration should be removed
                assert_eq!(
                    FileSystem::item_expirations(pending_file_deletion_request_ttl),
                    vec![]
                );

                // Asser that the pending file deletion request was removed from storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id.clone()),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::default()
                );

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
                .iter()
                .any(|x| *x == (file_key, Some(TrieRemoveMutation))),);
            });
        }

        #[test]
        fn delete_file_pending_file_deletion_request_submit_proof_of_inclusion_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Delete file
                assert_ok!(FileSystem::delete_file(
					owner_signed.clone(),
					bucket_id,
					file_key,
					location,
					size,
					fingerprint,
					None,
				));

                // Assert that the pending file deletion request was added to storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id.clone()),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![(file_key, bucket_id)]
                    )
                        .unwrap()
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                let msp_origin = RuntimeOrigin::signed(msp.clone());

                assert_ok!(FileSystem::pending_file_deletion_request_submit_proof(
					msp_origin,
					owner_account_id.clone(),
					file_key,
					bucket_id,
					forest_proof
				));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::ProofSubmittedForPendingFileDeletionRequest {
                        msp_id,
                        user: owner_account_id.clone(),
                        file_key,
                        bucket_id,
                        proof_of_inclusion: true,
                    }
                        .into(),
                );

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
                .iter()
                .any(|x| *x == (file_key, Some(TrieRemoveMutation))),);

                // Assert that the pending file deletion request was removed from storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::default()
                );
            });
        }

        #[test]
        fn delete_file_pending_file_deletion_request_submit_proof_of_non_inclusion_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Delete file
                assert_ok!(FileSystem::delete_file(
					owner_signed.clone(),
					bucket_id,
					file_key,
					location,
					size,
					fingerprint,
					None,
				));

                // Assert that the pending file deletion request was added to storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id.clone()),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![(file_key, bucket_id)]
                    )
                        .unwrap()
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::zero().as_bytes().to_vec()],
                };

                let msp_origin = RuntimeOrigin::signed(msp.clone());

                assert_ok!(FileSystem::pending_file_deletion_request_submit_proof(
					msp_origin,
					owner_account_id.clone(),
					file_key,
					bucket_id,
					forest_proof
				));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::ProofSubmittedForPendingFileDeletionRequest {
                        msp_id,
                        user: owner_account_id.clone(),
                        file_key,
                        bucket_id,
                        proof_of_inclusion: false,
                    }
                        .into(),
                );

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(
					!pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
						.iter()
						.any(|x| *x == (file_key, Some(TrieRemoveMutation))),
				);

                // Assert that the pending file deletion request was removed from storage
                assert_eq!(
                    FileSystem::pending_file_deletion_requests(owner_account_id),
                    BoundedVec::<_, <Test as crate::Config>::MaxUserPendingDeletionRequests>::default()
                );
            });
        }
    }
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
        owner_account: msp.clone(),
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
        false
    ));

    // Assert bucket was created
    assert_eq!(
        pallet_storage_providers::Buckets::<Test>::get(bucket_id),
        Some(Bucket {
            root: <Test as pallet_storage_providers::pallet::Config>::DefaultMerkleRoot::get(),
            user_id: owner.clone(),
            msp_id,
            private: false,
            read_access_group_id: None,
        })
    );

    bucket_id
}

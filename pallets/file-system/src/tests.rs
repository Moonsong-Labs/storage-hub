use crate::{
    mock::*,
    types::{
        BucketIdFor, BucketMoveRequestResponse, BucketNameFor, FileLocation,
        MoveBucketRequestMetadata, PeerIds, PendingFileDeletionRequestTtl, ProviderIdFor,
        StorageData, StorageRequestBspsMetadata, StorageRequestMetadata, StorageRequestTtl,
        ThresholdType,
    },
    BlockRangeToMaximumThreshold, Config, DataServersForMoveBucket, Error, Event,
    PendingBucketsToMove, PendingMoveBucketRequests, PendingStopStoringRequests, ReplicationTarget,
    StorageRequestExpirations, StorageRequests,
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
use shp_traits::{ReadBucketsInterface, ReadStorageProvidersInterface, TrieRemoveMutation};
use sp_core::{ByteArray, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{BlakeTwo256, Get},
    BoundedVec, DispatchError,
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

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

mod request_move_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn move_bucket_while_storage_request_opened() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    origin.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_charlie_id,
                    peer_ids.clone(),
                ));

                assert_noop!(
                    FileSystem::request_move_bucket(origin, bucket_id, msp_dave_id),
                    Error::<Test>::StorageRequestExists
                );
            });
        }

        #[test]
        fn move_bucket_when_already_requested() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                assert_noop!(
                    FileSystem::request_move_bucket(origin, bucket_id, msp_dave_id),
                    Error::<Test>::BucketIsBeingMoved
                );
            });
        }

        #[test]
        fn move_bucket_request_to_msp_already_storing_it() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                assert_noop!(
                    FileSystem::request_move_bucket(origin, bucket_id, msp_charlie_id),
                    Error::<Test>::MspAlreadyStoringBucket
                );
            });
        }

        #[test]
        fn move_bucket_to_non_existent_msp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id =
                    <<Test as frame_system::Config>::Hashing as Hasher>::hash(&msp_dave.as_slice());

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                assert_noop!(
                    FileSystem::request_move_bucket(origin, bucket_id, msp_dave_id),
                    Error::<Test>::NotAMsp
                );
            });
        }

        #[test]
        fn move_bucket_not_bucket_owner() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let not_owner = Keyring::Bob.to_account_id();
                let origin = RuntimeOrigin::signed(not_owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                assert_noop!(
                    FileSystem::request_move_bucket(origin, bucket_id, msp_dave_id),
                    Error::<Test>::NotBucketOwner
                );
            });
        }

        #[test]
        fn move_bucket_request_accepted_msp_not_enough_capacity() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Issue storage request with a big file size
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let file_content = vec![0; 1000];
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let size = 1000;

                // Set replication target to 1 to automatically fulfill the storage request after a single bsp confirms.
                crate::ReplicationTarget::<Test>::put(1);

                assert_ok!(FileSystem::issue_storage_request(
                    origin.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_charlie_id,
                    peer_ids.clone(),
                ));

				// Compute the file key.
                let file_key = FileSystem::compute_file_key(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Manually set enough capacity for Charlie
                pallet_storage_providers::MainStorageProviders::<Test>::mutate(
                    msp_charlie_id,
                    |msp| {
                        if let Some(msp) = msp {
                            msp.capacity = 1000;
                        }
                    },
                );

                // Dispatch the MSP accept request.
                // This opereration increases the bucket size.
                assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp_charlie),
					file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

                // Check bucket size
                let bucket_size = <<Test as crate::Config>::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id).unwrap();
                assert_eq!(bucket_size, size);

                // BSP confirm storage request
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));

                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

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
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert!(PendingBucketsToMove::<Test>::contains_key(&bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                    }
                    .into(),
                );

                // Manually set low capacity for Dave
                pallet_storage_providers::MainStorageProviders::<Test>::mutate(
                    msp_dave_id,
                    |msp| {
                        if let Some(msp) = msp {
                            msp.capacity = 0;
                        }
                    },
                );

                // Dispatch a signed extrinsic.
                assert_noop!(
                    FileSystem::msp_respond_move_bucket_request(
                        RuntimeOrigin::signed(msp_dave),
                        bucket_id,
                        BucketMoveRequestResponse::Accepted
                    ),
                    Error::<Test>::InsufficientAvailableCapacity
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn move_bucket_request_and_accepted_by_new_msp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert!(PendingBucketsToMove::<Test>::contains_key(&bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                    }
                    .into(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::msp_respond_move_bucket_request(
                    RuntimeOrigin::signed(msp_dave),
                    bucket_id,
                    BucketMoveRequestResponse::Accepted
                ));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketAccepted {
                        msp_id: msp_dave_id,
                        bucket_id,
                    }
                    .into(),
                );

                // Check bucket is stored by Dave
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_dave_id, &bucket_id)
                );

                // Check pending bucket storages are cleared
                assert!(!PendingBucketsToMove::<Test>::contains_key(&bucket_id));
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(&msp_dave_id, bucket_id));
            });
        }

        #[test]
        fn move_bucket_request_and_rejected_by_new_msp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert!(PendingBucketsToMove::<Test>::contains_key(&bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                    }
                    .into(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::msp_respond_move_bucket_request(
                    RuntimeOrigin::signed(msp_dave),
                    bucket_id,
                    BucketMoveRequestResponse::Rejected
                ));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRejected {
                        msp_id: msp_dave_id,
                        bucket_id,
                    }
                    .into(),
                );

                // Check bucket is still stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Check pending bucket storages are cleared
                assert!(!PendingBucketsToMove::<Test>::contains_key(&bucket_id));
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(&msp_dave_id, bucket_id));
            });
        }

        #[test]
        fn move_bucket_request_and_expires() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert!(PendingBucketsToMove::<Test>::contains_key(&bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                    }
                    .into(),
                );

                // Check move bucket request expires after MoveBucketRequestTtl
                let move_bucket_request_ttl: u32 = <Test as Config>::MoveBucketRequestTtl::get() ;
                let move_bucket_request_ttl: BlockNumber = move_bucket_request_ttl.into();
                let expiration = move_bucket_request_ttl + System::block_number();

                // Move block number to expiration
                roll_to(expiration);

                assert!(!PendingBucketsToMove::<Test>::contains_key(&bucket_id));
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(&msp_dave_id, bucket_id));
            });
        }
    }
}

mod bsp_add_data_server_for_move_bucket_request {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn not_a_bsp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let bucket_id = H256::zero();

                assert_noop!(
                    FileSystem::bsp_add_data_server_for_move_bucket_request(origin, bucket_id),
                    Error::<Test>::NotABsp
                );
            });
        }

        #[test]
        fn no_move_bucket_request_found() {
            new_test_ext().execute_with(|| {
                let bsp_account_id = Keyring::Bob.to_account_id();
                let origin = RuntimeOrigin::signed(bsp_account_id.clone());
                let bucket_id = H256::zero();

                assert_ok!(bsp_sign_up(origin.clone(), 1000));

                assert_noop!(
                    FileSystem::bsp_add_data_server_for_move_bucket_request(origin, bucket_id),
                    Error::<Test>::MoveBucketRequestNotFound
                );
            });
        }

        #[test]
        fn bsp_already_data_server() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();
                let bsp_account_id = Keyring::Bob.to_account_id();
                assert_ok!(bsp_sign_up(RuntimeOrigin::signed(bsp_account_id.clone()), 1000));

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert!(PendingBucketsToMove::<Test>::contains_key(&bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner.clone(),
                        bucket_id,
                        new_msp_id: msp_dave_id,
                    }
                    .into(),
                );

                assert_ok!(FileSystem::bsp_add_data_server_for_move_bucket_request(
                    RuntimeOrigin::signed(bsp_account_id.clone()),
                    bucket_id,
                ));

                let bsp_id =
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                        bsp_account_id.clone(),
                    )
                        .unwrap();

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner }));
                assert_eq!(DataServersForMoveBucket::<Test>::iter_key_prefix(&bucket_id).next(), Some(bsp_id));

                assert_noop!(
                    FileSystem::bsp_add_data_server_for_move_bucket_request(
                        RuntimeOrigin::signed(bsp_account_id.clone()),
                        bucket_id,
                    ),
                    Error::<Test>::BspAlreadyDataServer
                );
            });
        }
    }

    mod success {
        use crate::DataServersForMoveBucket;

        use super::*;

        #[test]
        fn add_bsp_as_data_server() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();
                let bsp_account_id = Keyring::Bob.to_account_id();
                assert_ok!(bsp_sign_up(RuntimeOrigin::signed(bsp_account_id.clone()), 1000));

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert!(PendingBucketsToMove::<Test>::contains_key(&bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner.clone(),
                        bucket_id,
                        new_msp_id: msp_dave_id,
                    }
                    .into(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::bsp_add_data_server_for_move_bucket_request(
                    RuntimeOrigin::signed(bsp_account_id.clone()),
                    bucket_id,
                ));

                let bsp_id =
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                        bsp_account_id,
                    )
                        .unwrap();

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner }));
                assert_eq!(DataServersForMoveBucket::<Test>::iter_key_prefix(&bucket_id).next(), Some(bsp_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::DataServerRegisteredForMoveBucket {
                        bsp_id,
                        bucket_id,
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

                let msp_id = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

                let msp_id = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

                let bucket_id = <Test as crate::Config>::Providers::derive_bucket_id(
                    &msp_id,
                    &owner,
                    name.clone(),
                );

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

        #[test]
        fn request_storage_while_pending_move_bucket() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let msp_charlie_id = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id);

                // Check bucket is stored by Charlie
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::is_bucket_stored_by_msp(&msp_charlie_id, &bucket_id)
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(pending_move_bucket, Some(MoveBucketRequestMetadata { requester: owner.clone() }));

                assert_noop!(
                    FileSystem::issue_storage_request(
                        origin,
                        bucket_id,
                        location.clone(),
                        fingerprint,
                        4,
                        msp_charlie_id,
                        peer_ids.clone(),
                    ),
                    Error::<Test>::BucketIsBeingMoved
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                let expiration_block = System::block_number() + storage_request_ttl;

                // Assert that the next starting block to clean up is set to 0 initially
                assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

                // Assert that the next expiration block number is the storage request ttl since a single storage request was made
                assert_eq!(
                    FileSystem::next_available_storage_request_expiration_block(),
                    expiration_block
                );

                // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::storage_request_expirations(expiration_block),
                    vec![file_key]
                );

                roll_to(expiration_block + 1);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::storage_request_expirations(expiration_block),
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
                    max_expired_items_in_block as usize
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

                let expected_expiration_block_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_block_number: BlockNumberFor<Test> =
                    expected_expiration_block_number.into();

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
                let expiration_block = System::block_number() + storage_request_ttl;

                // Assert that the NextExpirationInsertionBlockNumber storage is set to 0 initially
                assert_eq!(FileSystem::next_starting_block_to_clean_up(), 0);

                // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
                assert_eq!(
                    FileSystem::storage_request_expirations(expiration_block),
                    vec![file_key]
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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

mod msp_accept_storage_request {

    use super::*;

    mod success {

        use super::*;

        #[test]
        fn msp_accept_storage_request_works() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

				// Register the MSP.
                let msp_id = add_msp_to_provider_storage(&msp);

				// Create the bucket that will hold the file.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
					owner_signed.clone(),
					bucket_id,
					location.clone(),
					fingerprint,
					size,
					msp_id,
					peer_ids.clone(),
				));

				// Compute the file key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Dispatch the MSP accept request.
                assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

                // Assert that the storage was updated
                assert_eq!(
                    FileSystem::storage_requests(file_key).unwrap().msp,
                    Some((msp_id, true))
                );

				// Get the new root of the bucket.
                let new_bucket_root =
                    <<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id,)
                    .unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspAcceptedStoring {
						file_key,
						msp_id,
						bucket_id,
						owner: owner_account_id,
						new_bucket_root
					}
                    .into(),
                );
            });
        }

        #[test]
        fn msp_accept_storage_request_works_multiple_times_for_same_user_same_bucket() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
				let second_location = FileLocation::<Test>::try_from(b"never/go/to/a/second/location".to_vec()).unwrap();
                let first_size = 4;
				let second_size = 8;
                let first_fingerprint = H256::zero();
				let second_fingerprint =  H256::random();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

				// Register the MSP.
                let msp_id = add_msp_to_provider_storage(&msp);

				// Create the bucket that will hold both files.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

				// Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    first_location.clone(),
                    first_size,
                    first_fingerprint,
                );

				// Compute the file key for the second file.
				let second_file_key = FileSystem::compute_file_key(
					owner_account_id.clone(),
					bucket_id,
					second_location.clone(),
					second_size,
					second_fingerprint,
				);

                // Dispatch a storage request for the first file.
                assert_ok!(FileSystem::issue_storage_request(
					owner_signed.clone(),
					bucket_id,
					first_location.clone(),
					first_fingerprint,
					first_size,
					msp_id,
					peer_ids.clone(),
				));

				// Dispatch a storage request for the second file.
				assert_ok!(FileSystem::issue_storage_request(
					owner_signed.clone(),
					bucket_id,
					second_location.clone(),
					second_fingerprint,
					second_size,
					msp_id,
					peer_ids.clone(),
				));

                // Dispatch the MSP accept request for the first file.
                assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					first_file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

                // Assert that the storage was updated
                assert_eq!(
                    FileSystem::storage_requests(first_file_key).unwrap().msp,
                    Some((msp_id, true))
                );

				// Get the new root of the bucket.
                let new_bucket_root =
                    <<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id,)
                    .unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspAcceptedStoring {
						file_key: first_file_key,
						msp_id,
						bucket_id,
						owner: owner_account_id.clone(),
						new_bucket_root
					}
                    .into(),
                );

				// Assert that the MSP used capacity has been updated.
				assert_eq!(
					<Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
					first_size
				);

				// Dispatch the MSP accept request for the second file.
				assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					second_file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

				// Assert that the storage was updated
				assert_eq!(
					FileSystem::storage_requests(second_file_key).unwrap().msp,
					Some((msp_id, true))
				);

				// Get the new root of the bucket.
				let new_bucket_root =
					<<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id,)
					.unwrap();

				// Assert that the correct event was deposited
				System::assert_last_event(
					Event::MspAcceptedStoring {
						file_key: second_file_key,
						msp_id,
						bucket_id,
						owner: owner_account_id,
						new_bucket_root
					}
					.into(),
				);

				// Assert that the MSP used capacity has been updated.
				assert_eq!(
					<Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
					first_size + second_size
				);
            });
        }

        #[test]
        fn msp_accept_storage_request_works_multiple_times_for_same_user_different_bucket() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
				let second_location = FileLocation::<Test>::try_from(b"never/go/to/a/second/location".to_vec()).unwrap();
                let first_size = 4;
				let second_size = 8;
                let first_fingerprint = H256::zero();
				let second_fingerprint =  H256::random();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

				// Register the MSP.
                let msp_id = add_msp_to_provider_storage(&msp);

				// Create the bucket that will hold the first file.
                let first_name = BoundedVec::try_from(b"first bucket".to_vec()).unwrap();
                let first_bucket_id = create_bucket(&owner_account_id.clone(), first_name, msp_id);

				// Create the bucket that will hold the second file.
				let second_name = BoundedVec::try_from(b"second bucket".to_vec()).unwrap();
				let second_bucket_id = create_bucket(&owner_account_id.clone(), second_name, msp_id);

				// Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    first_bucket_id,
                    first_location.clone(),
                    first_size,
                    first_fingerprint,
                );

				// Compute the file key for the second file.
				let second_file_key = FileSystem::compute_file_key(
					owner_account_id.clone(),
					second_bucket_id,
					second_location.clone(),
					second_size,
					second_fingerprint,
				);

                // Dispatch a storage request for the first file.
                assert_ok!(FileSystem::issue_storage_request(
					owner_signed.clone(),
					first_bucket_id,
					first_location.clone(),
					first_fingerprint,
					first_size,
					msp_id,
					peer_ids.clone(),
				));

				// Dispatch a storage request for the second file.
				assert_ok!(FileSystem::issue_storage_request(
					owner_signed.clone(),
					second_bucket_id,
					second_location.clone(),
					second_fingerprint,
					second_size,
					msp_id,
					peer_ids.clone(),
				));

                // Dispatch the MSP accept request for the first file.
                assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					first_file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

                // Assert that the storage was updated
                assert_eq!(
                    FileSystem::storage_requests(first_file_key).unwrap().msp,
                    Some((msp_id, true))
                );

				// Get the new root of the bucket.
                let new_bucket_root =
                    <<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&first_bucket_id,)
                    .unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspAcceptedStoring {
						file_key: first_file_key,
						msp_id,
						bucket_id: first_bucket_id,
						owner: owner_account_id.clone(),
						new_bucket_root
					}
                    .into(),
                );

				// Assert that the MSP used capacity has been updated.
				assert_eq!(
					<Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
					first_size
				);

				// Dispatch the MSP accept request for the second file.
				assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					second_file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

				// Assert that the storage was updated
				assert_eq!(
					FileSystem::storage_requests(second_file_key).unwrap().msp,
					Some((msp_id, true))
				);

				// Get the new root of the bucket.
				let new_bucket_root =
					<<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&second_bucket_id,)
					.unwrap();

				// Assert that the correct event was deposited
				System::assert_last_event(
					Event::MspAcceptedStoring {
						file_key: second_file_key,
						msp_id,
						bucket_id: second_bucket_id,
						owner: owner_account_id,
						new_bucket_root
					}
					.into(),
				);

				// Assert that the MSP used capacity has been updated.
				assert_eq!(
					<Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
					first_size + second_size
				);
            });
        }

        #[test]
        fn msp_accept_storage_request_works_multiple_times_for_different_users() {
            new_test_ext().execute_with(|| {
                let first_owner_account_id = Keyring::Alice.to_account_id();
                let first_owner_signed = RuntimeOrigin::signed(first_owner_account_id.clone());
				let second_owner_account_id = Keyring::Bob.to_account_id();
				let second_owner_signed = RuntimeOrigin::signed(second_owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
				let second_location = FileLocation::<Test>::try_from(b"never/go/to/a/second/location".to_vec()).unwrap();
                let first_size = 4;
				let second_size = 8;
                let first_fingerprint = H256::zero();
				let second_fingerprint =  H256::random();
                let first_peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let first_peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![first_peer_id]).unwrap();
				let second_peer_id = BoundedVec::try_from(vec![2]).unwrap();
				let second_peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![second_peer_id]).unwrap();

				// Register the MSP.
                let msp_id = add_msp_to_provider_storage(&msp);

				// Create the bucket that will hold the first file.
                let first_name = BoundedVec::try_from(b"first bucket".to_vec()).unwrap();
                let first_bucket_id = create_bucket(&first_owner_account_id.clone(), first_name, msp_id);

				// Create the bucket that will hold the second file.
				let second_name = BoundedVec::try_from(b"second bucket".to_vec()).unwrap();
				let second_bucket_id = create_bucket(&second_owner_account_id.clone(), second_name, msp_id);

				// Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    first_owner_account_id.clone(),
                    first_bucket_id,
                    first_location.clone(),
                    first_size,
                    first_fingerprint,
                );

				// Compute the file key for the second file.
				let second_file_key = FileSystem::compute_file_key(
					second_owner_account_id.clone(),
					second_bucket_id,
					second_location.clone(),
					second_size,
					second_fingerprint,
				);

                // Dispatch a storage request for the first file.
                assert_ok!(FileSystem::issue_storage_request(
					first_owner_signed.clone(),
					first_bucket_id,
					first_location.clone(),
					first_fingerprint,
					first_size,
					msp_id,
					first_peer_ids.clone(),
				));

				// Dispatch a storage request for the second file.
				assert_ok!(FileSystem::issue_storage_request(
					second_owner_signed.clone(),
					second_bucket_id,
					second_location.clone(),
					second_fingerprint,
					second_size,
					msp_id,
					second_peer_ids.clone(),
				));

                // Dispatch the MSP accept request for the first file.
                assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					first_file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

                // Assert that the storage was updated
                assert_eq!(
                    FileSystem::storage_requests(first_file_key).unwrap().msp,
                    Some((msp_id, true))
                );

				// Get the new root of the bucket.
                let new_bucket_root =
                    <<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&first_bucket_id,)
                    .unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspAcceptedStoring {
						file_key: first_file_key,
						msp_id,
						bucket_id: first_bucket_id,
						owner: first_owner_account_id.clone(),
						new_bucket_root
					}
                    .into(),
                );

				// Assert that the MSP used capacity has been updated.
				assert_eq!(
					<Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
					first_size
				);

				// Dispatch the MSP accept request for the second file.
				assert_ok!(FileSystem::msp_accept_storage_request(
					RuntimeOrigin::signed(msp.clone()),
					second_file_key,
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					},
					CompactProof {
						encoded_nodes: vec![H256::default().as_ref().to_vec()],
					}
				));

				// Assert that the storage was updated
				assert_eq!(
					FileSystem::storage_requests(second_file_key).unwrap().msp,
					Some((msp_id, true))
				);

				// Get the new root of the bucket.
				let new_bucket_root =
					<<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&second_bucket_id,)
					.unwrap();

				// Assert that the correct event was deposited
				System::assert_last_event(
					Event::MspAcceptedStoring {
						file_key: second_file_key,
						msp_id,
						bucket_id: second_bucket_id,
						owner: second_owner_account_id,
						new_bucket_root
					}
					.into(),
				);

				// Assert that the MSP used capacity has been updated.
				assert_eq!(
					<Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
					first_size + second_size
				);
            });
        }

        #[test]
        fn msp_accept_storage_request_fullfilled() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                // Register the MSP.
                let msp_id = add_msp_to_provider_storage(&msp);

                // Create the bucket that will hold the file.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                // Set replication target to 1
                crate::ReplicationTarget::<Test>::put(1);

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                ));

                // Compute the file key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Dispatch the BSP volunteer
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch the BSP confirm storing
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
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

                // Dispatch the MSP accept request.
                assert_ok!(FileSystem::msp_accept_storage_request(
                    RuntimeOrigin::signed(msp.clone()),
                    file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    }
                ));

                // Storage request should be removed
                assert!(FileSystem::storage_requests(file_key).is_none());
                assert!(FileSystem::storage_request_buckets(bucket_id, file_key).is_none());
            });
        }
    }

    mod failure {

        use super::*;

        #[test]
        fn fails_if_storage_request_not_found() {
            new_test_ext().execute_with(|| {
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let file_key = H256::zero();

                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::StorageRequestNotFound
                );
            });
        }

        #[test]
        fn fails_if_caller_not_a_provider() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                // Dispatch a storage request.
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

                let not_msp = Keyring::Bob.to_account_id();
                let not_msp_signed = RuntimeOrigin::signed(not_msp.clone());

                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        not_msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::NotASp
                );
            });
        }

        #[test]
        fn fails_if_caller_not_a_msp() {
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
                let storage_amount = 100;

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                // Dispatch a storage request.
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

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        bsp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::NotAMsp
                );
            });
        }

        #[test]
        fn fails_if_request_is_not_expecting_a_msp() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

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

                // Insert a storage request that is not expecting a MSP.
                StorageRequests::<Test>::insert(
                    file_key,
                    StorageRequestMetadata {
                        requested_at: 1,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: None,
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                    },
                );

                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::RequestWithoutMsp
                );
            });
        }

        #[test]
        fn fails_if_caller_is_msp_but_not_assigned_one() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let expected_msp = Keyring::Charlie.to_account_id();
                let caller_msp = Keyring::Dave.to_account_id();
                let caller_msp_signed = RuntimeOrigin::signed(caller_msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let expected_msp_id = add_msp_to_provider_storage(&expected_msp);
                let _caller_msp_id = add_msp_to_provider_storage(&caller_msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, expected_msp_id);

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    expected_msp_id,
                    peer_ids.clone(),
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Try to accept storing a file with a MSP that is not the one assigned to the file.
                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        caller_msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::NotSelectedMsp
                );
            });
        }

        #[test]
        fn fails_if_msp_already_accepted_storing() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                // Dispatch a storage request.
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

                // Accept storing the file.
                assert_ok!(FileSystem::msp_accept_storage_request(
                    msp_signed.clone(),
                    file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()]
                    },
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()]
                    }
                ));

                // Try to accept storing the file again.
                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()]
                        },
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()]
                        }
                    ),
                    Error::<Test>::MspAlreadyConfirmed
                );
            });
        }

        #[test]
        fn fails_if_msp_is_not_the_one_storing_the_bucket() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let expected_msp = Keyring::Charlie.to_account_id();
                let expected_msp_signed = RuntimeOrigin::signed(expected_msp.clone());
                let other_msp = Keyring::Dave.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let expected_msp_id = add_msp_to_provider_storage(&expected_msp);
                let other_msp_id = add_msp_to_provider_storage(&other_msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, other_msp_id);

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Insert a storage request with the expected MSP but a bucket ID from another MSP.
                // Note: this should never happen since `issue_storage_request` checks that the bucket ID
                // belongs to the MSP, but we are testing it just in case.
                StorageRequests::<Test>::insert(
                    file_key,
                    StorageRequestMetadata {
                        requested_at: 1,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((expected_msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                    },
                );

                // Try to accept storing a file with a MSP that is not the owner of the bucket ID
                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        expected_msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::MspNotStoringBucket
                );
            });
        }

        #[test]
        fn fails_if_msp_does_not_have_enough_available_capacity() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 200;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

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

                // Insert a storage request for a MSP with not enough available capacity.
                // Note: `issue_storage_request` checks that the MSP has enough available capacity, but it could happen
                // that when the storage request was initially created the MSP had enough available capacity but it
                // accepted other storage requests in the meantime and now it does not have enough available capacity.
                StorageRequests::<Test>::insert(
                    file_key,
                    StorageRequestMetadata {
                        requested_at: 1,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                    },
                );

                // Try to accept storing a file with a MSP that does not have enough available capacity
                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![]
                        },
                        CompactProof {
                            encoded_nodes: vec![]
                        }
                    ),
                    Error::<Test>::InsufficientAvailableCapacity
                );
            });
        }

        #[test]
        fn fails_if_the_non_inclusion_proof_includes_the_file_key() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let msp_id = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                // Dispatch a storage request.
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

                // Try to accept storing a file with a non-inclusion proof that includes the file key
                assert_noop!(
                    FileSystem::msp_accept_storage_request(
                        msp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()]
                        },
                        CompactProof {
                            encoded_nodes: vec![file_key.as_ref().to_vec()]
                        }
                    ),
                    Error::<Test>::ExpectedNonInclusionProof
                );
            });
        }
    }
}

mod bsp_volunteer {
    use super::*;
    mod failure {
        use core::u32;

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

                // Set very high block range to maximum threshold.
                assert_ok!(FileSystem::set_global_parameters(
                    RuntimeOrigin::root(),
                    None,
                    Some(u32::MAX.into())
                ));

                // Dispatch BSP volunteer.
                assert_noop!(
                    FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
                    Error::<Test>::AboveThreshold
                );
            });
        }

        #[test]
        fn bsp_volunteer_with_insufficient_capacity() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
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

                let bsp_id =
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                        bsp_account_id.clone(),
                    )
                        .unwrap();

                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
                    bsp_id,
                    |bsp| {
                        assert!(bsp.is_some());
                        if let Some(bsp) = bsp {
                            bsp.capacity = 0;
                        }
                    },
                );

                let file_key = FileSystem::compute_file_key(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                // Dispatch BSP volunteer.
                assert_noop!(
                    FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
                    Error::<Test>::InsufficientAvailableCapacity
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
        use pallet_storage_providers::types::ReputationWeightType;

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
        fn bsp_confirming_for_non_existent_storage_request() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_bob_account_id = Keyring::Bob.to_account_id();
                let bsp_bob_signed = RuntimeOrigin::signed(bsp_bob_account_id.clone());
                let bsp_charlie_account_id = Keyring::Dave.to_account_id();
                let bsp_charlie_signed = RuntimeOrigin::signed(bsp_charlie_account_id.clone());
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

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let msp_signed = RuntimeOrigin::signed(msp.clone());
                assert_ok!(FileSystem::msp_accept_storage_request(
                    msp_signed.clone(),
                    file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                ));

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_bob_signed.clone(), storage_amount,));
                assert_ok!(bsp_sign_up(bsp_charlie_signed.clone(), storage_amount,));

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_bob_signed.clone(), file_key,));

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_bob_signed.clone(),
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

                assert_ok!(FileSystem::bsp_volunteer(
                    bsp_charlie_signed.clone(),
                    file_key,
                ));

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_charlie_signed.clone(),
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
                        bsp_bob_signed.clone(),
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
        fn bsp_failing_to_confirm_all_proofs_submitted_insufficient_capacity() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let storage_amount: StorageData<Test> = 100;
                let size = 4;

                let msp_id = add_msp_to_provider_storage(&msp);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id = <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                    bsp_account_id.clone(),
                ).unwrap();

                // Force BSP to pass all threshold checks when volunteering.
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(&bsp_id, |bsp| {
                    if let Some(bsp) = bsp {
                        bsp.reputation_weight = ReputationWeightType::<Test>::max_value();
                    }
                });

                let mut file_keys = Vec::new();

                // Issue 5 storage requests and volunteer for each
                for i in 0..5 {
                    let location = FileLocation::<Test>::try_from(format!("test{}", i).into_bytes()).unwrap();
                    let fingerprint = H256::repeat_byte(i as u8);

                    let name = BoundedVec::try_from(format!("bucket{}", i).into_bytes()).unwrap();
                    let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

                    // Issue storage request
                    assert_ok!(FileSystem::issue_storage_request(
                        owner.clone(),
                        bucket_id,
                        location.clone(),
                        fingerprint,
                        size,
                        msp_id,
                        Default::default(),
                    ));

                    let file_key = FileSystem::compute_file_key(
                        owner_account_id.clone(),
                        bucket_id,
                        location.clone(),
                        size,
                        fingerprint,
                    );

                    file_keys.push(file_key);

                    // Volunteer for storage
                    assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                }

                // Set BSP storage capacity to 0
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(&bsp_id, |bsp| {
                    if let Some(bsp) = bsp {
                        bsp.capacity = size * 2;
                    }
                });

                // Prepare proofs for all files
                let non_inclusion_forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                let file_keys_and_proofs: BoundedVec<_, <Test as crate::Config>::MaxBatchConfirmStorageRequests> = file_keys
                    .into_iter()
                    .map(|file_key| {
                        (file_key, CompactProof {
                            encoded_nodes: vec![file_key.as_ref().to_vec()],
                        })
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();

                // Attempt to confirm storing all files at once
                assert_noop!(
                    FileSystem::bsp_confirm_storing(
                        bsp_signed.clone(),
                        non_inclusion_forest_proof,
                        file_keys_and_proofs,
                    ),
                    Error::<Test>::InsufficientAvailableCapacity
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_root(
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
					<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_root(
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                        msp: Some((msp_id, false)),
                        user_peer_ids: Default::default(),
                        data_server_sps: BoundedVec::default(),
                        bsps_required: ReplicationTarget::<Test>::get(),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                assert_ok!(<<Test as crate::Config>::Providers as shp_traits::MutateStorageProvidersInterface>::increase_capacity_used(
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

                let current_bsps_required: <Test as Config>::ReplicationTargetType =
                    ReplicationTarget::<Test>::get();

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
                        msp: Some((msp_id, false)),
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
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                assert_ok!(<<Test as crate::Config>::Providers as shp_traits::MutateStorageProvidersInterface>::increase_capacity_used(
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

mod set_global_parameters_tests {
    use super::*;

    mod failure {
        use super::*;
        #[test]
        fn set_global_parameters_non_root_signer_fail() {
            new_test_ext().execute_with(|| {
                let non_root = Keyring::Bob.to_account_id();
                let non_root_signed = RuntimeOrigin::signed(non_root.clone());

                // Assert BadOrigin error when non-root account tries to set the threshold
                assert_noop!(
                    FileSystem::set_global_parameters(non_root_signed, None, None),
                    DispatchError::BadOrigin
                );
            });
        }

        #[test]
        fn set_global_parameters_0_value() {
            new_test_ext().execute_with(|| {
                assert_noop!(
                    FileSystem::set_global_parameters(RuntimeOrigin::root(), Some(0), None),
                    Error::<Test>::ReplicationTargetCannotBeZero
                );

                assert_noop!(
                    FileSystem::set_global_parameters(RuntimeOrigin::root(), None, Some(0)),
                    Error::<Test>::BlockRangeToMaximumThresholdCannotBeZero
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn set_global_parameters() {
            new_test_ext().execute_with(|| {
                let root = RuntimeOrigin::root();

                // Set the global parameters
                assert_ok!(FileSystem::set_global_parameters(
                    root.clone(),
                    Some(3),
                    Some(10)
                ));

                // Assert that the global parameters were set correctly
                assert_eq!(ReplicationTarget::<Test>::get(), 3);
                assert_eq!(BlockRangeToMaximumThreshold::<Test>::get(), 10);
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
                let size = u64::MAX;
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
                        i as u64,
                        fingerprint,
                    );

                    assert_ok!(FileSystem::delete_file(
                        owner_signed.clone(),
                        bucket_id,
                        file_key,
                        location.clone(),
                        i as u64,
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
                let expiration_block = pending_file_deletion_request_ttl + System::block_number();

                // Assert that the pending file deletion request was added to storage
                assert_eq!(
                    FileSystem::file_deletion_request_expirations(expiration_block),
                    vec![(
                        owner_account_id.clone(),
                        file_key
                    )]
                );

                // Roll past the expiration block
                roll_to(pending_file_deletion_request_ttl + 1);

                // Item expiration should be removed
                assert_eq!(
                    FileSystem::file_deletion_request_expirations(expiration_block),
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

mod compute_threshold {
    use super::*;
    mod success {
        use super::*;
        #[test]
        fn query_earliest_file_volunteer_block() {
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

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id =
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                        bsp_account_id,
                    )
                        .unwrap();

                let block_number = FileSystem::query_earliest_file_volunteer_block(bsp_id, file_key).unwrap();

                assert!(frame_system::Pallet::<Test>::block_number() <= block_number);
            });
        }

        #[test]
        fn compute_threshold_to_succeed() {
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

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id =
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                        bsp_account_id,
                    )
                        .unwrap();

                let storage_request = FileSystem::storage_requests(file_key).unwrap();

                FileSystem::set_global_parameters(RuntimeOrigin::root(), None, Some(1)).unwrap();

                assert_eq!(BlockRangeToMaximumThreshold::<Test>::get(), 1);

                let (threshold_to_succeed, slope) = FileSystem::compute_threshold_to_succeed(&bsp_id, storage_request.requested_at).unwrap();

                assert!(threshold_to_succeed > 0 && threshold_to_succeed <= ThresholdType::<Test>::max_value());
                assert!(slope > 0);

                let block_number = FileSystem::query_earliest_file_volunteer_block(bsp_id, file_key).unwrap();

                // BSP should be able to volunteer immediately for the storage request since the BlockRangeToMaximumThreshold is 1
                assert_eq!(block_number, frame_system::Pallet::<Test>::block_number());

                let starting_bsp_weight: pallet_storage_providers::types::ReputationWeightType<Test> = <Test as pallet_storage_providers::Config>::StartingReputationWeight::get();

                // Simulate there being many BSPs in the network with high reputation weight
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::set(1000u32.saturating_mul(starting_bsp_weight.into()));

                FileSystem::set_global_parameters(RuntimeOrigin::root(), None, Some(1000000000)).unwrap();

                assert_eq!(BlockRangeToMaximumThreshold::<Test>::get(), 1000000000);

                let (threshold_to_succeed, slope) = FileSystem::compute_threshold_to_succeed(&bsp_id, storage_request.requested_at).unwrap();

                assert!(threshold_to_succeed > 0 && threshold_to_succeed <= ThresholdType::<Test>::max_value());
                assert!(slope > 0);

                let block_number = FileSystem::query_earliest_file_volunteer_block(bsp_id, file_key).unwrap();

                // BSP can only volunteer after some number of blocks have passed.
                assert!(block_number > frame_system::Pallet::<Test>::block_number());

                // Set reputation weight of BSP to max
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(&bsp_id, |bsp| {
                    match bsp {
                        Some(bsp) => {
                            bsp.reputation_weight = u32::MAX;
                        }
                        None => {
                            panic!("BSP should exits");
                        }
                    }
                });

                let (threshold_to_succeed, slope) = FileSystem::compute_threshold_to_succeed(&bsp_id, storage_request.requested_at).unwrap();

                assert!(threshold_to_succeed > 0 && threshold_to_succeed <= ThresholdType::<Test>::max_value());
                assert!(slope > 0);

                let block_number = FileSystem::query_earliest_file_volunteer_block(bsp_id, file_key).unwrap();

                // BSP should be able to volunteer immediately for the storage request since the reputation weight is so high.
                assert_eq!(block_number, frame_system::Pallet::<Test>::block_number());
            });
        }

        #[test]
        fn compute_threshold_to_succeed_returns_max_when_bsp_weight_max() {
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

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id =
                    <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
                        bsp_account_id,
                    )
                        .unwrap();

                // Set reputation weight of BSP to max
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(&bsp_id, |bsp| {
                    match bsp {
                        Some(bsp) => {
                            bsp.reputation_weight = u32::MAX;
                        }
                        None => {
                            panic!("BSP should exits");
                        }
                    }
                });

                FileSystem::set_global_parameters(RuntimeOrigin::root(), None, Some(1)).unwrap();

                assert_eq!(BlockRangeToMaximumThreshold::<Test>::get(), 1);

                let (threshold_to_succeed, slope) = FileSystem::compute_threshold_to_succeed(&bsp_id, 0).unwrap();

                assert_eq!(threshold_to_succeed, ThresholdType::<Test>::max_value());
                assert!(slope > 0);

                let block_number = FileSystem::query_earliest_file_volunteer_block(bsp_id, file_key).unwrap();

                // BSP should be able to volunteer immediately for the storage request since the reputation weight is so high.
                assert_eq!(block_number, frame_system::Pallet::<Test>::block_number());
            });
        }
    }
}

mod stop_storing_for_insolvent_user {

    use super::*;

    #[test]
    fn stop_storing_for_insolvent_user_works_for_bsps() {
        new_test_ext().execute_with(|| {
            let owner_account_id = Keyring::Eve.to_account_id();
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
				<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                BoundedVec::try_from(vec![(
                    file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    }
                )])
                .unwrap(),
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
                    msp: Some((msp_id, false)),
                    user_peer_ids: peer_ids.clone(),
                    data_server_sps: BoundedVec::default(),
                    bsps_required: ReplicationTarget::<Test>::get(),
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
				<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_root(
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

            // Assert that the capacity used by the BSP was updated
            assert_eq!(
                pallet_storage_providers::BackupStorageProviders::<Test>::get(bsp_id)
                    .expect("BSP should exist in storage")
                    .capacity_used,
                size
            );

            // Now that the BSP has confirmed storing, we can simulate the user being insolvent
            // and the BSP stopping storing for the user.
            // Try to stop storing for the insolvent user.
            assert_ok!(FileSystem::stop_storing_for_insolvent_user(
                bsp_signed.clone(),
                file_key,
                bucket_id,
                location.clone(),
                owner_account_id.clone(),
                fingerprint,
                size,
                CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                },
            ));

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::SpStopStoringInsolventUser {
                    sp_id: bsp_id,
                    file_key,
                    owner: owner_account_id,
                    location,
                    new_root,
                }
                .into(),
            );

            // Assert that the capacity used by the BSP was updated
            assert_eq!(
                pallet_storage_providers::BackupStorageProviders::<Test>::get(bsp_id)
                    .expect("BSP should exist in storage")
                    .capacity_used,
                0
            );
        });
    }

    #[test]
    fn stop_storing_for_insolvent_user_works_for_msps() {
        new_test_ext().execute_with(|| {
            let owner_account_id = Keyring::Eve.to_account_id();
            let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
            let bsp_account_id = Keyring::Bob.to_account_id();
            let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
            let msp = Keyring::Charlie.to_account_id();
            let msp_signed = RuntimeOrigin::signed(msp.clone());
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let size = 4;
            let fingerprint = H256::zero();
            let peer_id = BoundedVec::try_from(vec![1]).unwrap();
            let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
            let storage_amount: StorageData<Test> = 50;

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

            // Dispatch MSP confirm storing.
            assert_ok!(FileSystem::msp_accept_storage_request(
                RuntimeOrigin::signed(msp.clone()),
                file_key,
                CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                },
                CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                },
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
                    msp: Some((msp_id, true)),
                    user_peer_ids: peer_ids.clone(),
                    data_server_sps: BoundedVec::default(),
                    bsps_required: ReplicationTarget::<Test>::get(),
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

            let new_bucket_root =
				<<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(
					&bucket_id,
				)
					.unwrap();

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::MspAcceptedStoring {
                    file_key,
                    msp_id,
                    bucket_id,
                    owner: owner_account_id.clone(),
                    new_bucket_root,
                }
                .into(),
            );

            // Assert that the capacity used by the MSP was updated
            assert_eq!(
                pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
                    .expect("MSP should exist in storage")
                    .capacity_used,
                size
            );

            // Now that the MSP has accepted storing, we can simulate the user being insolvent
            // and the MSP stopping storing for the user.
            // Try to stop storing for the insolvent user.
            assert_ok!(FileSystem::stop_storing_for_insolvent_user(
                msp_signed.clone(),
                file_key,
                bucket_id,
                location.clone(),
                owner_account_id.clone(),
                fingerprint,
                size,
                CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                },
            ));

            // Get the new bucket root after deletion
            let new_bucket_root_after_deletion =
				<<Test as crate::Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(
					&bucket_id,
				)
					.unwrap();

            // Assert that the correct event was deposited
            System::assert_last_event(
                Event::SpStopStoringInsolventUser {
                    sp_id: msp_id,
                    file_key,
                    owner: owner_account_id,
                    location,
                    new_root: new_bucket_root_after_deletion,
                }
                .into(),
            );

            // Assert that the capacity used by the MSP was updated
            assert_eq!(
                pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
                    .expect("MSP should exist in storage")
                    .capacity_used,
                0
            );
        });
    }

    #[test]
    fn stop_storing_for_insolvent_user_fails_if_caller_not_a_sp() {
        new_test_ext().execute_with(|| {
            let owner_account_id = Keyring::Eve.to_account_id();
            let bsp_account_id = Keyring::Bob.to_account_id();
            let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
            let msp = Keyring::Charlie.to_account_id();
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let size = 4;
            let fingerprint = H256::zero();

            let msp_id = add_msp_to_provider_storage(&msp);

            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

            // Try to stop storing for the insolvent user.
            assert_noop!(
                FileSystem::stop_storing_for_insolvent_user(
                    bsp_signed.clone(),
                    H256::zero(),
                    bucket_id,
                    location.clone(),
                    owner_account_id.clone(),
                    fingerprint,
                    size,
                    CompactProof {
                        encoded_nodes: vec![H256::zero().as_ref().to_vec()],
                    },
                ),
                Error::<Test>::NotASp
            );
        });
    }

    #[test]
    fn stop_storing_for_insolvent_user_fails_if_user_not_insolvent() {
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
				<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                BoundedVec::try_from(vec![(
                    file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    }
                )])
                .unwrap(),
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
                    msp: Some((msp_id, false)),
                    user_peer_ids: peer_ids.clone(),
                    data_server_sps: BoundedVec::default(),
                    bsps_required: ReplicationTarget::<Test>::get(),
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
				<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_root(
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

            // Now that the BSP has confirmed storing, we can simulate the user being not insolvent
            // and the BSP trying to stop storing a file for the user using the stop_storing_for_insolvent_user function.
            assert_noop!(
                FileSystem::stop_storing_for_insolvent_user(
                    bsp_signed.clone(),
                    file_key,
                    bucket_id,
                    location.clone(),
                    owner_account_id.clone(),
                    fingerprint,
                    size,
                    CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                ),
                Error::<Test>::UserNotInsolvent
            );
        });
    }

    #[test]
    fn stop_storing_for_insolvent_user_fails_if_caller_not_owner_of_bucket() {
        new_test_ext().execute_with(|| {
            let owner_account_id = Keyring::Eve.to_account_id();
            let msp = Keyring::Charlie.to_account_id();
            let another_msp = Keyring::Dave.to_account_id();
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let size = 4;
            let fingerprint = H256::zero();

            let msp_id = add_msp_to_provider_storage(&msp);
            add_msp_to_provider_storage(&another_msp);

            let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
            let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id);

            let file_key = FileSystem::compute_file_key(
                owner_account_id.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint,
            );

            // Try to stop storing for the insolvent user using another MSP account.
            assert_noop!(
                FileSystem::stop_storing_for_insolvent_user(
                    RuntimeOrigin::signed(another_msp.clone()),
                    file_key,
                    bucket_id,
                    location.clone(),
                    owner_account_id.clone(),
                    fingerprint,
                    size,
                    CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                ),
                Error::<Test>::MspNotStoringBucket
            );
        });
    }

    #[test]
    fn stop_storing_for_insolvent_user_fails_if_proof_does_not_contain_file_key() {
        new_test_ext().execute_with(|| {
            let owner_account_id = Keyring::Eve.to_account_id();
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
				<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_provider_id(
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
                BoundedVec::try_from(vec![(
                    file_key,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    }
                )])
                .unwrap(),
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
                    msp: Some((msp_id, false)),
                    user_peer_ids: peer_ids.clone(),
                    data_server_sps: BoundedVec::default(),
                    bsps_required: ReplicationTarget::<Test>::get(),
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
				<<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_root(
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

            // Now that the BSP has confirmed storing, we can simulate the user being insolvent
            // and the BSP trying to stop storing for that user with an incorrect inclusion proof.
            assert_noop!(
                FileSystem::stop_storing_for_insolvent_user(
                    bsp_signed.clone(),
                    file_key,
                    bucket_id,
                    location.clone(),
                    owner_account_id.clone(),
                    fingerprint,
                    size,
                    CompactProof {
                        encoded_nodes: vec![H256::zero().as_ref().to_vec()],
                    },
                ),
                Error::<Test>::ExpectedInclusionProof
            );
        });
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
        capacity_used: 0,
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
    let bucket_id =
        <Test as crate::Config>::Providers::derive_bucket_id(&msp_id, &owner, name.clone());

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
            size: 0,
        })
    );

    bucket_id
}

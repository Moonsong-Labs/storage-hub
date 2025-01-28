use crate::{
    self as file_system,
    mock::*,
    types::{
        BucketIdFor, BucketMoveRequestResponse, BucketNameFor, FileKeyWithProof, FileLocation,
        MoveBucketRequestMetadata, PeerIds, PendingFileDeletionRequest, ProviderIdFor,
        ReplicationTarget, StorageData, StorageRequestBspsMetadata, StorageRequestMetadata,
        StorageRequestMspAcceptedFileKeys, StorageRequestMspBucketResponse, StorageRequestTtl,
        ThresholdType, TickNumber, ValuePropId,
    },
    Config, Error, Event, NextAvailableStorageRequestExpirationTick, PendingBucketsToMove,
    PendingMoveBucketRequests, PendingStopStoringRequests, StorageRequestExpirations,
    StorageRequests,
};
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchResultWithPostInfo,
    traits::{
        fungible::{Inspect, InspectHold, Mutate},
        nonfungibles_v2::Destroy,
        tokens::{Fortitude, Preservation},
        Hooks, OriginTrait,
    },
    weights::Weight,
};
use pallet_proofs_dealer::types::CustomChallenge;
use pallet_proofs_dealer::{PriorityChallengesQueue, ProviderToProofSubmissionRecord};
use pallet_storage_providers::types::{Bucket, StorageProviderId, ValueProposition};
use shp_traits::{
    MutateBucketsInterface, MutateStorageProvidersInterface, PaymentStreamsInterface,
    ReadBucketsInterface, ReadProvidersInterface, ReadStorageProvidersInterface,
};
use sp_core::{ByteArray, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    bounded_vec,
    traits::{BlakeTwo256, Get},
    BoundedVec,
};
use sp_std::cmp::max;
use sp_trie::CompactProof;

mod create_bucket_tests {
    use super::*;

    mod failure {
        use crate::types::ValuePropId;

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
                        true,
                        Some(ValuePropId::<Test>::default())
                    ),
                    Error::<Test>::NotAMsp
                );
            });
        }

        #[test]
        fn create_bucket_user_without_enough_funds_for_deposit_fail() {
            new_test_ext().execute_with(|| {
                let owner_without_balance = Keyring::Ferdie.to_account_id();
                let origin = RuntimeOrigin::signed(owner_without_balance.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = false;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                assert_noop!(
                    FileSystem::create_bucket(
                        origin,
                        msp_id,
                        name.clone(),
                        private,
                        Some(value_prop_id)
                    ),
                    pallet_storage_providers::Error::<Test>::NotEnoughBalance
                );
            });
        }

        #[test]
        fn create_public_bucket_fails_with_insolvent_provider() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = false;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(msp_id),
                    (),
                );

                // Dispatch a signed extrinsic.
                assert_noop!(
                    FileSystem::create_bucket(
                        origin,
                        msp_id,
                        name.clone(),
                        private,
                        Some(value_prop_id)
                    ),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
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
                let owner_initial_balance = <Test as Config>::Currency::free_balance(&owner);
                let bucket_creation_deposit =
                    <Test as pallet_storage_providers::Config>::BucketDeposit::get();
                let nft_collection_deposit: crate::types::BalanceOf<Test> =
                    <Test as pallet_nfts::Config>::CollectionDeposit::get();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = true;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    origin,
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Check if collection was created
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
                    .unwrap()
                    .is_some()
                );

                // Check that the deposit was held from the owner's balance
                assert_eq!(
                    <Test as Config>::Currency::balance_on_hold(
                        &RuntimeHoldReason::Providers(
                            pallet_storage_providers::HoldReason::BucketDeposit
                        ),
                        &owner
                    ),
                    bucket_creation_deposit
                );

                let new_stream_deposit: u64 = <Test as pallet_payment_streams::Config>::NewStreamDeposit::get();
				let base_deposit: u128 = <Test as pallet_payment_streams::Config>::BaseDeposit::get();
                assert_eq!(
                    <Test as Config>::Currency::free_balance(&owner),
                    owner_initial_balance - bucket_creation_deposit - nft_collection_deposit - new_stream_deposit as u128 - base_deposit
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
                        value_prop_id: Some(value_prop_id),
                        root: <<Test as Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash::default(),
                    }
                    .into(),
                );

                // Check fixed rate payment stream is created
                assert!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(&msp_id, &owner));
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    origin,
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Check that the bucket does not have a corresponding collection
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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
                        value_prop_id: Some(value_prop_id),
                        root: <<Test as Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash::default(),
                    }
                    .into(),
                );
            });
        }
    }
}

mod delete_bucket_tests {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn remove_bucket_bucket_not_found_fail() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();

                let _ = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                assert_noop!(
                    FileSystem::delete_bucket(origin, bucket_id),
                    Error::<Test>::BucketNotFound
                );
            });
        }

        #[test]
        fn remove_bucket_not_bucket_owner_fail() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let not_owner = Keyring::Bob.to_account_id();
                let not_owner_origin = RuntimeOrigin::signed(not_owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = false;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    RuntimeOrigin::signed(owner.clone()),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                assert_noop!(
                    FileSystem::delete_bucket(not_owner_origin, bucket_id),
                    Error::<Test>::NotBucketOwner
                );
            });
        }

        #[test]
        fn remove_bucket_bucket_not_empty_fail() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = false;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Create a new bucket.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Dispatch a signed extrinsic of a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    origin.clone(),
                    bucket_id,
                    FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
                    BlakeTwo256::hash(&b"test".to_vec()),
                    4,
                    msp_id,
                    BoundedVec::try_from(vec![BoundedVec::try_from(vec![1]).unwrap()]).unwrap(),
                    ReplicationTarget::Standard
                ));

                // Accept the storage request to store the file, so the bucket is not empty.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key: FileSystem::compute_file_key(
                                    owner.clone(),
                                    bucket_id,
                                    FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
                                    4,
                                    BlakeTwo256::hash(&b"test".to_vec())
                                ),
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Make sure the bucket is now not empty.
                assert!(
                    <<Test as crate::Config>::Providers as ReadBucketsInterface>::get_bucket_size(
                        &bucket_id
                    )
                    .unwrap()
                        != 0
                );

                // Ensure that the bucket cannot be deleted.
                assert_noop!(
                    FileSystem::delete_bucket(origin, bucket_id),
                    Error::<Test>::BucketNotEmpty
                );
            });
        }

        #[test]
        fn remove_bucket_bucket_provider_insolvent() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = false;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Create a new bucket.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Dispatch a signed extrinsic of a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    origin.clone(),
                    bucket_id,
                    FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
                    BlakeTwo256::hash(&b"test".to_vec()),
                    4,
                    msp_id,
                    BoundedVec::try_from(vec![BoundedVec::try_from(vec![1]).unwrap()]).unwrap(),
                    ReplicationTarget::Standard
                ));

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(msp_id),
                    (),
                );

                // Accept the storage request to store the file, so the bucket is not empty.
                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        RuntimeOrigin::signed(msp),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key: FileSystem::compute_file_key(
                                        owner.clone(),
                                        bucket_id,
                                        FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
                                        4,
                                        BlakeTwo256::hash(&b"test".to_vec())
                                    ),
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
                    ),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn remove_bucket_success() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = false;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Create a new bucket.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::delete_bucket(origin, bucket_id));

                // Check that the bucket was removed.
                assert!(
                    !<<Test as crate::Config>::Providers as ReadBucketsInterface>::bucket_exists(
                        &bucket_id
                    )
                );

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BucketDeleted {
                        who: owner,
                        bucket_id,
                        maybe_collection_id: None,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn remove_bucket_with_collection_success() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp = Keyring::Charlie.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let private = true;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Create a new bucket.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Get the bucket's collection ID.
                let collection_id =
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id,
                    )
                    .unwrap()
                    .unwrap();

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::delete_bucket(origin, bucket_id));

                // Check that the bucket was removed.
                assert!(
                    !<<Test as crate::Config>::Providers as ReadBucketsInterface>::bucket_exists(
                        &bucket_id
                    )
                );

                // Check that the associated collection was destroyed.
                assert!(pallet_nfts::Collection::<Test>::get(&collection_id).is_none());

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BucketDeleted {
                        who: owner,
                        bucket_id,
                        maybe_collection_id: Some(collection_id),
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn remove_bucket_after_being_used_success() {
            new_test_ext().execute_with(|| {
				let owner = Keyring::Alice.to_account_id();
				let origin = RuntimeOrigin::signed(owner.clone());
				let msp = Keyring::Charlie.to_account_id();
				let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
				let private = false;

				let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

				let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
					&owner,
					name.clone(),
				);

				// Create a new bucket.
				assert_ok!(FileSystem::create_bucket(
					origin.clone(),
					msp_id,
					name.clone(),
					private,
					Some(value_prop_id)
				));

				// Dispatch a signed extrinsic of a storage request.
				assert_ok!(FileSystem::issue_storage_request(
					origin.clone(),
					bucket_id,
					FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
					BlakeTwo256::hash(&b"test".to_vec()),
					4,
					msp_id,
					BoundedVec::try_from(vec![BoundedVec::try_from(vec![1]).unwrap()]).unwrap(),
                    ReplicationTarget::Standard
				));

				// Accept the storage request to store the file, so the bucket is not empty.
				assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
					RuntimeOrigin::signed(msp),
					vec![StorageRequestMspBucketResponse {
						bucket_id,
						accept: Some(StorageRequestMspAcceptedFileKeys {
							file_keys_and_proofs: vec![FileKeyWithProof {
								file_key: FileSystem::compute_file_key(
									owner.clone(),
									bucket_id,
									FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
									4,
									BlakeTwo256::hash(&b"test".to_vec())
								),
								proof: CompactProof {
									encoded_nodes: vec![H256::default().as_ref().to_vec()],
								}
							}],
							forest_proof: CompactProof {
								encoded_nodes: vec![H256::default().as_ref().to_vec()],
							},
						}),
						reject: vec![],
					}],
				));

				// Make sure the bucket is now not empty.
				assert!(
					<<Test as crate::Config>::Providers as ReadBucketsInterface>::get_bucket_size(
						&bucket_id
					)
					.unwrap()
						!= 0
				);

				// Issue a revoke storage request to remove the file from the bucket.
				assert_ok!(FileSystem::revoke_storage_request(
					origin.clone(),
					FileSystem::compute_file_key(
						owner.clone(),
						bucket_id,
						FileLocation::<Test>::try_from(b"test".to_vec()).unwrap(),
						4,
						BlakeTwo256::hash(&b"test".to_vec())
					)
				));

				// Remove the file from the bucket.
				assert_ok!(<<Test as crate::Config>::Providers as MutateBucketsInterface>::decrease_bucket_size(&bucket_id, 4));
				assert_ok!(<<Test as crate::Config>::Providers as MutateBucketsInterface>::change_root_bucket(bucket_id, <<Test as crate::Config>::Providers as ReadProvidersInterface>::get_default_root()));

				// Delete the bucket.
				assert_ok!(FileSystem::delete_bucket(origin, bucket_id));

				// Check that the bucket was removed.
				assert!(
					!<<Test as crate::Config>::Providers as ReadBucketsInterface>::bucket_exists(
						&bucket_id
					)
				);

				// Assert that the correct event was deposited
				System::assert_last_event(
					Event::BucketDeleted {
						who: owner,
						bucket_id,
						maybe_collection_id: None,
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

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    origin.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_charlie_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

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

                let (msp_charlie_id, value_prop_id) = add_msp_to_provider_storage(&msp_charlie);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

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

                let (msp_charlie_id, value_prop_id) = add_msp_to_provider_storage(&msp_charlie);
                let msp_dave_id =
                    <<Test as frame_system::Config>::Hashing as Hasher>::hash(&msp_dave.as_slice());

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

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

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

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

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Issue storage request with a big file size
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let file_content = vec![0; 1000];
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let size = 1000;

                assert_ok!(FileSystem::issue_storage_request(
                    origin.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_charlie_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(1)
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
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

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
                // This operation increases the bucket size.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp_charlie),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Check bucket size
                let bucket_size = Providers::get_bucket_size(&bucket_id).unwrap();
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
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
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
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone()
                    })
                );

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

        #[test]
        fn move_bucket_request_fails_with_insolvent_provider() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(msp_dave_id),
                    (),
                );

                // Dispatch a signed extrinsic.
                assert_noop!(
                    FileSystem::request_move_bucket(origin.clone(), bucket_id, msp_dave_id),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
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

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone()
                    })
                );

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
                assert!(Providers::is_bucket_stored_by_msp(&msp_dave_id, &bucket_id));

                // Check pending bucket storages are cleared
                assert!(!PendingBucketsToMove::<Test>::contains_key(&bucket_id));
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(
                    &msp_dave_id,
                    bucket_id
                ));
            });
        }

        #[test]
        fn move_bucket_request_and_rejected_by_new_msp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone()
                    })
                );

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
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Check pending bucket storages are cleared
                assert!(!PendingBucketsToMove::<Test>::contains_key(&bucket_id));
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(
                    &msp_dave_id,
                    bucket_id
                ));
            });
        }

        #[test]
        fn move_bucket_request_and_expires() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone()
                    })
                );

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
                let move_bucket_request_ttl: u32 = <Test as Config>::MoveBucketRequestTtl::get();
                let move_bucket_request_ttl: TickNumber<Test> = move_bucket_request_ttl.into();
                let expiration = move_bucket_request_ttl + <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

                // Move tick number to expiration
                roll_to(expiration);

                assert!(!PendingBucketsToMove::<Test>::contains_key(&bucket_id));
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(
                    &msp_dave_id,
                    bucket_id
                ));
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

                let _ = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Check if collection was created
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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
                        value_prop_id: Some(value_prop_id),
                        root: <<Test as Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash::default(),
                    }
                    .into(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::update_bucket_privacy(origin, bucket_id, false));

                // Check that the bucket still has a corresponding collection
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Check if collection was created
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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
                        value_prop_id: Some(value_prop_id),
                        root: <<Test as Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash::default(),
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
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Check that the bucket does not have a corresponding collection
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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
                        value_prop_id: Some(value_prop_id),
                        root: <<Test as Config>::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash::default(),
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
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
                    .unwrap()
                    .is_some()
                );

                let collection_id =
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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

                let _ = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner,
                    name.clone(),
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::create_bucket(
                    origin.clone(),
                    msp_id,
                    name.clone(),
                    private,
                    Some(value_prop_id)
                ));

                // Check if collection was created
                assert!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id
                    )
                    .unwrap()
                    .is_some()
                );

                let collection_id =
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
                        &bucket_id,
                    )
                    .unwrap()
                    .expect("Collection ID should exist");

                assert_ok!(FileSystem::create_and_associate_collection_with_bucket(
                    origin, bucket_id
                ));

                // Check if collection was associated with the bucket
                assert_ne!(
                    <Test as file_system::Config>::Providers::get_read_access_group_id_of_bucket(
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

                let (msp_id, _) = add_msp_to_provider_storage(&msp);

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
                        ReplicationTarget::Standard
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_id, value_prop_id);

                assert_noop!(
                    FileSystem::issue_storage_request(
                        origin,
                        bucket_id,
                        location.clone(),
                        fingerprint,
                        4,
                        msp_id,
                        peer_ids.clone(),
                        ReplicationTarget::Standard
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

                let (msp_charlie_id, _) = add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id
                ));

                let pending_move_bucket =
                    PendingMoveBucketRequests::<Test>::get(&msp_dave_id, bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone()
                    })
                );

                assert_noop!(
                    FileSystem::issue_storage_request(
                        origin,
                        bucket_id,
                        location.clone(),
                        fingerprint,
                        4,
                        msp_charlie_id,
                        peer_ids.clone(),
                        ReplicationTarget::Standard
                    ),
                    Error::<Test>::BucketIsBeingMoved
                );
            });
        }

        #[test]
        fn request_storage_not_enough_balance_for_deposit_fails() {
            new_test_ext().execute_with(|| {
                let owner_without_funds = Keyring::Ferdie.to_account_id();
                let user = RuntimeOrigin::signed(owner_without_funds.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Mint enough funds for the bucket deposit and existential deposit but not enough for the storage request deposit
                let new_stream_deposit: u64 =
                    <Test as pallet_payment_streams::Config>::NewStreamDeposit::get();
                let base_stream_deposit: u128 =
                    <Test as pallet_payment_streams::Config>::BaseDeposit::get();
                let balance_to_mint: crate::types::BalanceOf<Test> =
                    <<Test as pallet_storage_providers::Config>::BucketDeposit as Get<
                        crate::types::BalanceOf<Test>,
                    >>::get()
                    .saturating_add(new_stream_deposit as u128)
                    .saturating_add(base_stream_deposit)
                    .saturating_add(<Test as pallet_balances::Config>::ExistentialDeposit::get());
                <Test as file_system::Config>::Currency::mint_into(
                    &owner_without_funds,
                    balance_to_mint.into(),
                )
                .unwrap();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_without_funds.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

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
                        ReplicationTarget::Standard
                    ),
                    Error::<Test>::CannotHoldDeposit
                );

                let file_key = FileSystem::compute_file_key(
                    owner_without_funds.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Assert that the storage was not updated
                assert_eq!(file_system::StorageRequests::<Test>::get(file_key), None);
            });
        }

        #[test]
        fn request_storage_replication_target_cannot_be_zero() {
            new_test_ext().execute_with(|| {
                let owner_without_funds = Keyring::Alice.to_account_id();
                let user = RuntimeOrigin::signed(owner_without_funds.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_without_funds.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

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
                        ReplicationTarget::Custom(0)
                    ),
                    Error::<Test>::ReplicationTargetCannotBeZero
                );
            });
        }

        #[test]
        fn request_storage_replication_target_cannot_exceed_maximum() {
            new_test_ext().execute_with(|| {
                let owner_without_funds = Keyring::Alice.to_account_id();
                let user = RuntimeOrigin::signed(owner_without_funds.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_without_funds.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

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
                        ReplicationTarget::Custom(
                            <<Test as crate::Config>::MaxReplicationTarget as Get<u32>>::get() + 1
                        )
                    ),
                    Error::<Test>::ReplicationTargetExceedsMaximum
                );
            });
        }

        #[test]
        fn request_storage_fails_with_insolvent_provider() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    &StorageProviderId::<Test>::MainStorageProvider(msp_id),
                    (),
                );

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
                        ReplicationTarget::Standard
                    ),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
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
                let storage_request_deposit =
                    <Test as file_system::Config>::StorageRequestCreationDeposit::get();
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                let owner_initial_balance =
                    <Test as file_system::Config>::Currency::free_balance(&owner_account_id);

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request
                    })
                );

                // Check that the deposit was held from the owner's balance
                assert_eq!(
                    <Test as Config>::Currency::balance_on_hold(
                        &RuntimeHoldReason::FileSystem(
                            file_system::HoldReason::StorageRequestCreationHold
                        ),
                        &owner_account_id
                    ),
                    storage_request_deposit
                );
                assert_eq!(
                    <Test as Config>::Currency::free_balance(&owner_account_id),
                    owner_initial_balance - storage_request_deposit
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
                        expires_at: next_expiration_tick_storage_request,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn request_storage_with_maximum_replication_target() {
            new_test_ext().execute_with(|| {
                let owner_without_funds = Keyring::Alice.to_account_id();
                let user = RuntimeOrigin::signed(owner_without_funds.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_without_funds.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(<Test as crate::Config>::MaxReplicationTarget::get())
                ),);
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    file_1_location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id: bucket_id.clone(),
                        location: file_1_location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request
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
                    ReplicationTarget::Standard
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
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: file_2_location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request
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
                        expires_at: next_expiration_tick_storage_request,
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

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
                        ReplicationTarget::Standard
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request
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
                let storage_request_ttl: TickNumber<Test> = storage_request_ttl.into();
                let expiration_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick() + storage_request_ttl;

                // Assert that the next expiration tick number is the storage request ttl since a single storage request was made
                assert_eq!(
                    file_system::NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    expiration_tick
                );

                // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(expiration_tick),
                    vec![file_key]
                );

                roll_to(expiration_tick + 1);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(expiration_tick),
                    vec![]
                );
            });
        }

        #[test]
        fn request_storage_expiration_current_tick_increment_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                let expected_expiration_tick_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_tick_number: TickNumber<Test> =
                    expected_expiration_tick_number.into();

                // Append storage request expiration to the list at `StorageRequestTtl`
                let max_expired_items_in_tick: u32 = <Test as Config>::MaxExpiredItemsInTick::get();
                for _ in 0..max_expired_items_in_tick {
                    assert_ok!(StorageRequestExpirations::<Test>::try_append(
                        expected_expiration_tick_number,
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
                    ReplicationTarget::Standard
                ));

                // Assert that the storage request expirations storage is at max capacity
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(
                        expected_expiration_tick_number
                    )
                    .len(),
                    max_expired_items_in_tick as usize
                );

                // Go to tick number after which the storage request expirations should be removed
                roll_to(expected_expiration_tick_number);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(
                        expected_expiration_tick_number
                    ),
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Append storage request expiration to the list at `StorageRequestTtl`
                let max_storage_request_expiry: u32 =
                    <Test as Config>::MaxExpiredItemsInTick::get();

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                let expected_expiration_tick_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_tick_number: TickNumber<Test> =
                    expected_expiration_tick_number.into();

                for _ in 0..max_storage_request_expiry {
                    assert_ok!(StorageRequestExpirations::<Test>::try_append(
                        expected_expiration_tick_number,
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
                    ReplicationTarget::Standard
                ));

                let expected_expiration_tick_number: u32 = StorageRequestTtl::<Test>::get();
                let expected_expiration_tick_number: TickNumber<Test> =
                    expected_expiration_tick_number.into();

                // Assert that the storage request expirations storage is at max capacity
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(
                        expected_expiration_tick_number
                    )
                    .len(),
                    max_storage_request_expiry as usize
                );

                let used_weight = FileSystem::on_idle(<<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(), Weight::zero());

                // Assert that the weight used is zero
                assert_eq!(used_weight, Weight::zero());

                // Assert that the storage request expirations storage is at max capacity
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(
                        expected_expiration_tick_number
                    )
                    .len(),
                    max_storage_request_expiry as usize
                );

                // Go to tick number after which the storage request expirations should be removed
                roll_to(expected_expiration_tick_number + 1);

                // Assert that the storage request expiration was removed from the list at `StorageRequestTtl`
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(
                        expected_expiration_tick_number
                    ),
                    vec![]
                );

                // Assert that the `NextExpirationInsertionTickNumber` storage is set to the next tick number
                assert_eq!(
                    file_system::NextStartingTickToCleanUp::<Test>::get(),
                    <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick() + 1
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_id,
                    Default::default(),
                    ReplicationTarget::Standard
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_id,
                    Default::default(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                let storage_request_ttl: u32 = StorageRequestTtl::<Test>::get();
                let storage_request_ttl: TickNumber<Test> = storage_request_ttl.into();
                let expiration_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick() + storage_request_ttl;

                // Assert that the storage request expiration was appended to the list at `StorageRequestTtl`
                assert_eq!(
                    file_system::StorageRequestExpirations::<Test>::get(expiration_tick),
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account.clone(), name.clone(), msp_id, value_prop_id);

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                let file_key = FileSystem::compute_file_key(
                    owner_account.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Check StorageRequestBsps storage for confirmed BSPs
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account.clone(), name.clone(), msp_id, value_prop_id);

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::revoke_storage_request(owner.clone(), file_key));

                // Check ProofsDealer pallet storage for queued custom challenges for remove trie mutation of file key
                let priority_challenges_queue = PriorityChallengesQueue::<Test>::get();

                assert!(priority_challenges_queue.contains(&CustomChallenge {
                    key: file_key,
                    should_remove_key: true
                }));

                // Assert that the correct event was deposited
                System::assert_last_event(Event::StorageRequestRevoked { file_key }.into());
            });
        }
    }
}

mod msp_respond_storage_request {
    use super::*;

    mod success {
        use super::*;

        #[test]
        fn msp_respond_storage_request_works() {
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
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Create the bucket that will hold the file.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(1)
                ));

                // Compute the file key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Simulate a BSP already confirming the storage request.
                StorageRequests::<Test>::mutate(file_key, |storage_request| {
                    storage_request.as_mut().unwrap().bsps_confirmed = 1;
                });

                // Dispatch the MSP accept request.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Check that the storage request has been deleted since it is fulfilled.
                assert_eq!(StorageRequests::<Test>::get(file_key), None);

                // Get bucket root
                let bucket_root = <<Test as Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id).unwrap();

                // Check that the bucket root is not default
                assert_ne!(bucket_root, <<Test as Config>::Providers as shp_traits::ReadProvidersInterface>::get_default_root());

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(1)
                ));

                // Compute the file key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Simulate a BSP already confirming the storage request.
                StorageRequests::<Test>::mutate(file_key, |storage_request| {
                    storage_request.as_mut().unwrap().bsps_confirmed = 1;
                });

                // Dispatch the MSP accept request with the file key in the forest proof.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec(), file_key.as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                let post_state_bucket_root = <<Test as Config>::Providers as shp_traits::ReadBucketsInterface>::get_root_bucket(&bucket_id).unwrap();

                // Bucket root should not have changed
                assert_eq!(bucket_root, post_state_bucket_root);
            });
        }

        #[test]
        fn msp_respond_storage_request_works_multiple_times_for_same_user_same_bucket() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let second_location =
                    FileLocation::<Test>::try_from(b"never/go/to/a/second/location".to_vec())
                        .unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                // Register the MSP.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Create the bucket that will hold both files.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    first_location.clone(),
                    size,
                    fingerprint,
                );

                // Compute the file key for the second file.
                let second_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    second_location.clone(),
                    size,
                    fingerprint,
                );

                // Dispatch a storage request for the first file.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    first_location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                // Dispatch a storage request for the second file.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    second_location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                // Dispatch the MSP accept request for the first file.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: bounded_vec![
                                FileKeyWithProof {
                                    file_key: first_file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                },
                                FileKeyWithProof {
                                    file_key: second_file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }
                            ],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(first_file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                assert_eq!(
                    file_system::StorageRequests::<Test>::get(second_file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                // Assert that the MSP used capacity has been updated.
                assert_eq!(
                    <Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
                    size * 2
                );
            });
        }

        #[test]
        fn msp_respond_storage_request_works_multiple_times_for_same_user_different_bucket() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let second_location =
                    FileLocation::<Test>::try_from(b"never/go/to/a/second/location".to_vec())
                        .unwrap();
                let first_size = 4;
                let second_size = 8;
                let first_fingerprint = H256::zero();
                let second_fingerprint = H256::random();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                // Register the MSP.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Create the bucket that will hold the first file.
                let first_name = BoundedVec::try_from(b"first bucket".to_vec()).unwrap();
                let first_bucket_id =
                    create_bucket(&owner_account_id.clone(), first_name, msp_id, value_prop_id);

                // Create the bucket that will hold the second file.
                let second_name = BoundedVec::try_from(b"second bucket".to_vec()).unwrap();
                let second_bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    second_name,
                    msp_id,
                    value_prop_id,
                );

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
                    ReplicationTarget::Standard
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
                    ReplicationTarget::Standard
                ));

                // Dispatch the MSP accept request for the second file.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    bounded_vec![
                        StorageRequestMspBucketResponse {
                            bucket_id: first_bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key: first_file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        },
                        StorageRequestMspBucketResponse {
                            bucket_id: second_bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key: second_file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }
                    ],
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(first_file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(second_file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                // Assert that the MSP used capacity has been updated.
                assert_eq!(
                    <Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
                    first_size + second_size
                );
            });
        }

        #[test]
        fn msp_respond_storage_request_works_multiple_times_for_different_users() {
            new_test_ext().execute_with(|| {
                let first_owner_account_id = Keyring::Alice.to_account_id();
                let first_owner_signed = RuntimeOrigin::signed(first_owner_account_id.clone());
                let second_owner_account_id = Keyring::Bob.to_account_id();
                let second_owner_signed = RuntimeOrigin::signed(second_owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let second_location =
                    FileLocation::<Test>::try_from(b"never/go/to/a/second/location".to_vec())
                        .unwrap();
                let first_size = 4;
                let second_size = 8;
                let first_fingerprint = H256::zero();
                let second_fingerprint = H256::random();
                let first_peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let first_peer_ids: PeerIds<Test> =
                    BoundedVec::try_from(vec![first_peer_id]).unwrap();
                let second_peer_id = BoundedVec::try_from(vec![2]).unwrap();
                let second_peer_ids: PeerIds<Test> =
                    BoundedVec::try_from(vec![second_peer_id]).unwrap();

                // Register the MSP.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Create the bucket that will hold the first file.
                let first_name = BoundedVec::try_from(b"first bucket".to_vec()).unwrap();
                let first_bucket_id = create_bucket(
                    &first_owner_account_id.clone(),
                    first_name,
                    msp_id,
                    value_prop_id,
                );

                // Create the bucket that will hold the second file.
                let second_name = BoundedVec::try_from(b"second bucket".to_vec()).unwrap();
                let second_bucket_id = create_bucket(
                    &second_owner_account_id.clone(),
                    second_name,
                    msp_id,
                    value_prop_id,
                );

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
                    ReplicationTarget::Standard
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
                    ReplicationTarget::Standard
                ));

                // Dispatch the MSP accept request for the second file.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    bounded_vec![
                        StorageRequestMspBucketResponse {
                            bucket_id: first_bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key: first_file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        },
                        StorageRequestMspBucketResponse {
                            bucket_id: second_bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key: second_file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }
                    ],
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(first_file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(second_file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                // Assert that the MSP used capacity has been updated.
                assert_eq!(
                    <Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id),
                    first_size + second_size
                );
            });
        }

        #[test]
        fn msp_respond_storage_request_fulfilled() {
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
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Create the bucket that will hold the file.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageData<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(1)
                ));

                // Compute the file key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Ensure the storage request expiration item was added to the expiration queue
                assert!(file_system::StorageRequestExpirations::<Test>::get(
                    next_expiration_tick_storage_request
                )
                .contains(&file_key));

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch the BSP volunteer
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch the BSP confirm storing
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Dispatch the MSP accept request.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                System::assert_has_event(Event::StorageRequestFulfilled { file_key }.into());

                // Storage request should be removed
                assert!(file_system::StorageRequests::<Test>::get(file_key).is_none());
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(bucket_id, file_key)
                        .is_none()
                );

                // And the storage request expiration item should have been removed from the queue
                assert!(!file_system::StorageRequestExpirations::<Test>::get(
                    next_expiration_tick_storage_request
                )
                .contains(&file_key));
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
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // create bucket
                let owner_account_id = Keyring::Alice.to_account_id();
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                let file_key = H256::zero();

                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        not_msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        bsp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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
                        requested_at: <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: None,
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: 100,
                    },
                );

                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
                    ),
                    Error::<Test>::RequestWithoutMsp
                );
            });
        }

        #[test]
        fn fails_if_request_with_insolvent_provider() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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
                        requested_at: <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: None,
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: 100,
                    },
                );

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(msp_id),
                    (),
                );

                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
                    ),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
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

                let (expected_msp_id, value_prop_id) = add_msp_to_provider_storage(&expected_msp);
                let _caller_msp_id = add_msp_to_provider_storage(&caller_msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name,
                    expected_msp_id,
                    value_prop_id,
                );

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    expected_msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        caller_msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
                    ),
                    Error::<Test>::MspNotStoringBucket
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch a storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Accept storing the file.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    msp_signed.clone(),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Try to accept storing the file again.
                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
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

                let (expected_msp_id, _) = add_msp_to_provider_storage(&expected_msp);
                let (other_msp_id, value_prop_id) = add_msp_to_provider_storage(&other_msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, other_msp_id, value_prop_id);

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
                        requested_at: <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((expected_msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: 100,
                    },
                );

                // Try to accept storing a file with a MSP that is not the owner of the bucket ID
                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        expected_msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
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
                let size = 100 * 1024 * 1024 * 1024;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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
                        requested_at: <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick(),
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: 100,
                    },
                );

                // Try to accept storing a file with a MSP that does not have enough available capacity
                assert_noop!(
                    FileSystem::msp_respond_storage_requests_multiple_buckets(
                        msp_signed.clone(),
                        vec![StorageRequestMspBucketResponse {
                            bucket_id,
                            accept: Some(StorageRequestMspAcceptedFileKeys {
                                file_keys_and_proofs: vec![FileKeyWithProof {
                                    file_key,
                                    proof: CompactProof {
                                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                    }
                                }],
                                forest_proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                },
                            }),
                            reject: vec![],
                        }],
                    ),
                    Error::<Test>::InsufficientAvailableCapacity
                );
            });
        }
    }
}

mod bsp_volunteer {
    use super::*;
    mod failure {
        use super::*;
        use core::u32;

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner_account_id.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

                assert_noop!(
                    FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
                    Error::<Test>::BspAlreadyVolunteered
                );
            });
        }

        #[test]
        fn volunteer_fails_when_bsp_is_insolvent() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(bsp_id),
                    (),
                );

                assert_noop!(
                    FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Set very high global weight so BSP does not have priority
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::put(u32::MAX);

                // Dispatch BSP volunteer.
                assert_noop!(
                    FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
                    Error::<Test>::BspNotEligibleToVolunteer
                );
            });
        }

        #[test]
        fn bsp_volunteer_above_threshold_high_fail_even_with_spamming() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Get BSP ID.
                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                // Compute the file key to volunteer for.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Set a high enough global weight so BSP does not have priority
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::put(u32::MAX);

                // Calculate how many ticks until this BSP can volunteer for the file.
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();
                let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                let current_block = System::block_number();

                // Advance by the number of ticks until this BSP can volunteer for the file.
                // In the process, this BSP will spam the chain to prevent others from volunteering and confirming.
                roll_to_spammed(current_block + ticks_to_advance);

                // Dispatch BSP volunteer.
                assert_noop!(
                    FileSystem::bsp_volunteer(bsp_signed.clone(), file_key),
                    Error::<Test>::BspNotEligibleToVolunteer
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner.clone(), name.clone(), msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    origin,
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(bsp_id, |bsp| {
                    assert!(bsp.is_some());
                    if let Some(bsp) = bsp {
                        bsp.capacity = 0;
                    }
                });

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
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner.clone(), name.clone(), msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    origin,
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    4,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

                // Assert that the RequestStorageBsps has the correct value
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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
        fn bsp_volunteer_succeeds_after_waiting_enough_ticks_without_spam() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Get BSP ID.
                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                // Compute the file key to volunteer for.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Set a high enough global weight so BSP does not have priority
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::put(u32::MAX / 2);

                // Calculate how many ticks until this BSP can volunteer for the file.
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();
                let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                let current_block = System::block_number();

                // Advance by the number of ticks until this BSP can volunteer for the file.
                roll_to(current_block + ticks_to_advance);

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

                // Assert that the RequestStorageBsps has the correct value
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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
                        owner: owner_account_id,
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
        use crate::types::FileKeyWithProof;

        use super::*;
        use pallet_storage_providers::types::ReputationWeightType;

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner_account_id.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                        BoundedVec::try_from(vec![FileKeyWithProof {
                            file_key,
                            proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        }])
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
                        BoundedVec::try_from(vec![FileKeyWithProof {
                            file_key,
                            proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        }])
                        .unwrap(),
                    ),
                    Error::<Test>::NoFileKeysToConfirm
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                        BoundedVec::try_from(vec![FileKeyWithProof {
                            file_key,
                            proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        }])
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_bob_signed.clone(), storage_amount,));
                assert_ok!(bsp_sign_up(bsp_charlie_signed.clone(), storage_amount,));

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(2)
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let msp_signed = RuntimeOrigin::signed(msp.clone());
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    msp_signed.clone(),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key)
                        .unwrap()
                        .msp,
                    Some((msp_id, true))
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_bob_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_bob_signed.clone(), file_key,));

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_bob_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
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
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                assert_noop!(
                    FileSystem::bsp_confirm_storing(
                        bsp_bob_signed.clone(),
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![FileKeyWithProof {
                            file_key,
                            proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        }])
                        .unwrap(),
                    ),
                    Error::<Test>::NoFileKeysToConfirm
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Force BSP to pass all threshold checks when volunteering.
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(&bsp_id, |bsp| {
                    if let Some(bsp) = bsp {
                        bsp.reputation_weight = ReputationWeightType::<Test>::max_value();
                    }
                });

                let mut file_keys = Vec::new();

                // Issue 5 storage requests and volunteer for each
                for i in 0..5 {
                    let location =
                        FileLocation::<Test>::try_from(format!("test{}", i).into_bytes()).unwrap();
                    let fingerprint = H256::repeat_byte(i as u8);
                    let name = BoundedVec::try_from(format!("bucket{}", i).into_bytes()).unwrap();
                    let bucket_id =
                        create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                    // Issue storage request
                    assert_ok!(FileSystem::issue_storage_request(
                        owner.clone(),
                        bucket_id,
                        location.clone(),
                        fingerprint,
                        size,
                        msp_id,
                        Default::default(),
                        ReplicationTarget::Standard
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

                let file_keys_and_proofs: BoundedVec<
                    _,
                    <Test as file_system::Config>::MaxBatchConfirmStorageRequests,
                > = file_keys
                    .into_iter()
                    .map(|file_key| FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![file_key.as_ref().to_vec()],
                        },
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

        #[test]
        fn bsp_confirm_storing_fails_with_no_file_keys_to_confirm() {
            new_test_ext().execute_with(|| {
                // Setup accounts
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();

                // Setup common test parameters
                let size = 4;
                let fingerprint = H256::zero();
                let peer_ids =
                    BoundedVec::try_from(vec![BoundedVec::try_from(vec![1]).unwrap()]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                // Setup MSP and bucket
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                let bucket_id = create_bucket(
                    &owner_account_id,
                    BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
                    msp_id,
                    value_prop_id,
                );

                // Setup BSP
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Create file keys with different file locations
                let location =
                    FileLocation::<Test>::try_from(format!("test-location").into_bytes()).unwrap();

                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(1)
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

                // Pre-confirm one file key to simulate a previous BSP already confirming it
                file_system::StorageRequests::<Test>::mutate(file_key, |maybe_metadata| {
                    if let Some(metadata) = maybe_metadata {
                        metadata.bsps_confirmed = 1;
                    }
                });

                assert_noop!(
                    FileSystem::bsp_confirm_storing(
                        bsp_signed,
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![FileKeyWithProof {
                            file_key,
                            proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        }])
                        .unwrap(),
                    ),
                    Error::<Test>::NoFileKeysToConfirm
                );
            });
        }

        #[test]
        fn bsp_confirm_storing_fails_with_insolvent_provider() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(bsp_id),
                    (),
                );

                // Dispatch BSP confirm storing.
                assert_noop!(
                    FileSystem::bsp_confirm_storing(
                        bsp_signed.clone(),
                        CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                        BoundedVec::try_from(vec![FileKeyWithProof {
                            file_key,
                            proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            }
                        }])
                        .unwrap(),
                    ),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
                );
            });
        }
    }

    mod success {
        use shp_traits::PaymentStreamsInterface;

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
                    }
                    .into(),
                );

                // Assert that the payment stream between the BSP and the user has been created
                assert!(PaymentStreams::has_active_payment_stream_with_user(
                    &bsp_id,
                    &owner_account_id
                ));
            });
        }

        #[test]
        fn bsp_confirm_storing_with_skipped_file_keys_success() {
            new_test_ext().execute_with(|| {
                // Setup accounts
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();

                // Setup common test parameters
                let size = 4;
                let fingerprint = H256::zero();
                let peer_ids =
                    BoundedVec::try_from(vec![BoundedVec::try_from(vec![1]).unwrap()]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                // Setup MSP and bucket
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                let bucket_id = create_bucket(
                    &owner_account_id,
                    BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
                    msp_id,
                    value_prop_id,
                );

                // Setup BSP
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Create file keys with different file locations
                let locations: Vec<FileLocation<Test>> = (0..3)
                    .map(|i| {
                        FileLocation::<Test>::try_from(format!("test{}", i).into_bytes()).unwrap()
                    })
                    .collect();

                // Issue storage requests for each file
                let file_keys: Vec<_> = locations
                    .iter()
                    .map(|location| {
                        // Set replication target to 1 so that the storage request would be skipped if the confirmed bsps are equal to 1.
                        assert_ok!(FileSystem::issue_storage_request(
                            owner_signed.clone(),
                            bucket_id,
                            location.clone(),
                            fingerprint,
                            size,
                            msp_id,
                            peer_ids.clone(),
                            ReplicationTarget::Custom(1)
                        ));

                        let file_key = FileSystem::compute_file_key(
                            owner_account_id.clone(),
                            bucket_id,
                            location.clone(),
                            size,
                            fingerprint,
                        );

                        file_key
                    })
                    .collect();

                // Calculate in how many ticks the BSP can volunteer for the files
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = file_keys
                    .iter()
                    .map(|&file_key| {
                        FileSystem::query_earliest_file_volunteer_tick(
                            Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                            file_key,
                        )
                        .unwrap()
                    })
                    .max()
                    .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Volunteer for all files
                for &file_key in &file_keys {
                    assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                }

                // Pre-confirm one file key to simulate a previous BSP already confirming it
                let pre_confirmed_file_key = file_keys[0];
                file_system::StorageRequests::<Test>::mutate(
                    pre_confirmed_file_key,
                    |maybe_metadata| {
                        if let Some(metadata) = maybe_metadata {
                            metadata.bsps_confirmed = 1;
                        }
                    },
                );

                // Confirm storing for all files
                let file_keys_with_proofs: Vec<_> = file_keys
                    .iter()
                    .map(|&file_key| FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        },
                    })
                    .collect();

                let old_root = Providers::get_root(bsp_id).unwrap();

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(file_keys_with_proofs).unwrap(),
                ));

                let successful_file_keys: Vec<_> = file_keys
                    .iter()
                    .filter(|&&file_key| file_key != pre_confirmed_file_key)
                    .copied()
                    .collect();

                let new_root = Providers::get_root(bsp_id).unwrap();

                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id,
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(successful_file_keys).unwrap(),
                        skipped_file_keys: BoundedVec::try_from(vec![pre_confirmed_file_key])
                            .unwrap(),
                        new_root,
                    }
                    .into(),
                );

                // Verify root was updated while there being a skipped file key
                assert_ne!(old_root, new_root);
            });
        }

        #[test]
        fn bsp_confirm_storing_correctly_updates_already_existing_payment_stream() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: BoundedVec::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id.clone(),
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
                    }
                    .into(),
                );

                // Assert that the payment stream between the BSP and the user has been created and get its amount provided
                let amount_provided_payment_stream =
                    PaymentStreams::get_dynamic_rate_payment_stream_amount_provided(
                        &bsp_id,
                        &owner_account_id,
                    );
                assert!(amount_provided_payment_stream.is_some());
                assert_eq!(amount_provided_payment_stream.unwrap(), size);

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch another storage request.
                let new_size = 8;
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    new_size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    new_size,
                    fingerprint,
                );

                // Advance a few ticks and dispatch BSP volunteer.
                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size: new_size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the payment stream between the BSP and the user has been correctly updated
                let new_amount_provided_payment_stream =
                    PaymentStreams::get_dynamic_rate_payment_stream_amount_provided(
                        &bsp_id,
                        &owner_account_id,
                    )
                    .unwrap();
                assert_eq!(
                    amount_provided_payment_stream.unwrap() + new_size,
                    new_amount_provided_payment_stream
                );
            });
        }

        #[test]
        fn bsp_confirm_storing_final_bsp_success() {
            new_test_ext().execute_with(|| {
                // Setup variables for the test.
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

                // Sign up the MSP that will be used in the test and create a bucket under it for the file.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider and get its ID.
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Compute the file key of the storage request to issue.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Compute the expiration tick for the storage request to issue.
				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch the storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Custom(1)
                ));

                // Ensure the storage request expiration item was added to the expiration queue.
                assert!(StorageRequestExpirations::<Test>::get(
                    next_expiration_tick_storage_request
                )
                .contains(&file_key));

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch the BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Modify the storage request to simulate the MSP having accepted it.
                file_system::StorageRequests::<Test>::mutate(file_key, |maybe_metadata| {
                    if let Some(metadata) = maybe_metadata {
                        metadata.msp = Some((msp_id, true))
                    }
                });

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage request was deleted since it has been fulfilled
                assert!(!file_system::StorageRequests::<Test>::contains_key(
                    &file_key
                ));

                // Assert that the StorageRequestBsps storage for this file key was drained
                assert!(
                    file_system::StorageRequestBsps::<Test>::iter_prefix(file_key)
                        .next()
                        .is_none()
                );

                // Assert that the storage request was removed from the expiration queue
                assert!(!StorageRequestExpirations::<Test>::get(
                    next_expiration_tick_storage_request
                )
                .contains(&file_key));

                // Get the new root of the BSP after confirming to store the file.
                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct events were deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );
                System::assert_has_event(Event::StorageRequestFulfilled { file_key }.into());

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
                    }
                    .into(),
                );

                // Assert that the payment stream between the BSP and the user has been created
                assert!(PaymentStreams::has_active_payment_stream_with_user(
                    &bsp_id,
                    &owner_account_id
                ));
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner_account_id.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
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
                    Error::<Test>::InvalidFileKeyMetadata
                );
            });
        }

        #[test]
        fn bsp_request_stop_storing_fails_if_cannot_pay_for_fee() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Set BSPs free balance to existential deposit
                let existential_deposit = ExistentialDeposit::get();
                <Test as Config>::Currency::set_balance(&bsp_account_id, existential_deposit);

                // Dispatch BSP request stop storing.
                let error = FileSystem::bsp_request_stop_storing(
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
                )
                .unwrap_err();

                match error {
                    sp_runtime::DispatchError::Token(_) => {
                        assert!(true);
                    }
                    _ => {
                        panic!("Unexpected error: {:?}", error);
                    }
                }
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
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
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
                    Error::<Test>::PendingStopStoringRequestAlreadyExists
                );
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
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
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
                assert!(file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id).is_none());

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request,
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
                assert_noop!(
                    FileSystem::bsp_confirm_stop_storing(
                        bsp_signed.clone(),
                        file_key,
                        CompactProof {
                            encoded_nodes: vec![file_key.as_ref().to_vec()],
                        },
                    ),
                    Error::<Test>::MinWaitForStopStoringNotReached
                );

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
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                <Test as Config>::Currency::mint_into(
                    &bsp_account_id,
                    <Test as Config>::BspStopStoringFilePenalty::get(),
                )
                .unwrap();

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
                assert!(file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id).is_none());

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request,
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
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
                    }
                    .into(),
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
                assert!(file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id).is_none());

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request,
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
                roll_to(
                    frame_system::Pallet::<Test>::block_number() + MinWaitForStopStoring::get(),
                );

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

                // Assert that the used capacity of this BSP is now 0
                assert_eq!(Providers::get_used_capacity(&bsp_id), 0);

                // Assert that the randomness cycle for this BSP has been stopped
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_none());

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleStopped {
                        provider_id: bsp_id,
                    }
                    .into(),
                );

                // Assert that the correct event was deposited.
                let new_root = Providers::get_root(bsp_id).unwrap();

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
        fn bsp_confirm_stop_storing_success_and_deletes_payment_stream_for_last_file() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
				let current_tick_plus_storage_request_ttl = current_tick + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(NextAvailableStorageRequestExpirationTick::<Test>::get(), current_tick_plus_storage_request_ttl);

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
					ReplicationTarget::Standard
                ));


                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Check that the dynamic-rate payment stream between the user and the provider doesn't exist
                assert!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
					&bsp_id,
					&owner_account_id
				).is_none());

				// Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Check that a dynamic-rate payment stream between the user and the provider was created and get its amount provided
                assert!(
                    <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
                        &bsp_id,
                        &owner_account_id
                    )
                    .is_some()
                );
                let maybe_amount_provided = <Test as crate::Config>::PaymentStreams::get_dynamic_rate_payment_stream_amount_provided(
					&bsp_id,
					&owner_account_id
				);
				assert!(maybe_amount_provided.is_some());
				let amount_provided = maybe_amount_provided.unwrap();
				assert_eq!(amount_provided, size);

                // Assert that the RequestStorageBsps now contains the BSP under the location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
						expires_at: next_expiration_tick_storage_request,
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
                assert!(file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id).is_none());

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
						expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the request was added to the pending stop storing requests.
                assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).is_some());

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key,
                        owner: owner_account_id.clone(),
                        location,
                    }
                    .into(),
                );

                // Advance enough blocks to allow the BSP to confirm the stop storing request.
                roll_to(
                    frame_system::Pallet::<Test>::block_number() + MinWaitForStopStoring::get(),
                );

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
                let new_root = Providers::get_root(bsp_id).unwrap();

                System::assert_last_event(
                    Event::BspConfirmStoppedStoring {
                        bsp_id,
                        file_key,
                        new_root,
                    }
                    .into(),
                );

				// Check that the dynamic-rate payment stream between the user and the provider doesn't exist
                assert!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
					&bsp_id,
					&owner_account_id
				).is_none());
            });
        }

        #[test]
        fn bsp_confirm_stop_storing_success_and_updates_payment_streams_amount_provided() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let first_file_location = FileLocation::<Test>::try_from(b"first_file_test".to_vec()).unwrap();
				let second_file_location = FileLocation::<Test>::try_from(b"second_file_test".to_vec()).unwrap();
                let size = 4;
                let first_file_fingerprint = H256::zero();
				let second_file_fingerprint = H256::random();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageData<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
				let current_tick_plus_storage_request_ttl = current_tick + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
				let next_expiration_tick_storage_request = max(NextAvailableStorageRequestExpirationTick::<Test>::get(), current_tick_plus_storage_request_ttl);

                // Dispatch first storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    first_file_location.clone(),
                    first_file_fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
					ReplicationTarget::Standard
                ));

				// Dispatch second storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    second_file_location.clone(),
                    second_file_fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
					ReplicationTarget::Standard
                ));

                let first_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    first_file_location.clone(),
                    size,
                    first_file_fingerprint,
                );
				let second_file_key = FileSystem::compute_file_key(
					owner_account_id.clone(),
					bucket_id,
					second_file_location.clone(),
					size,
					second_file_fingerprint,
				);

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Check that the dynamic-rate payment stream between the user and the provider doesn't exist
                assert!(
					<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
						&bsp_id,
						&owner_account_id
					)
					.is_none()
				);

				// Calculate in how many ticks the BSP can volunteer for the files
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = max(FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    first_file_key,
                )
                .unwrap(), FileSystem::query_earliest_file_volunteer_tick(
					Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
					second_file_key,
				)
				.unwrap());
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteers.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), first_file_key,));
				assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), second_file_key,));

                // Dispatch first BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key: first_file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

				// Dispatch second BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key: second_file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Check that a dynamic-rate payment stream between the user and the provider was created and get its amount provided
                assert!(
                    <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
                        &bsp_id,
                        &owner_account_id
                    )
                    .is_some()
                );
                let maybe_amount_provided = <Test as crate::Config>::PaymentStreams::get_dynamic_rate_payment_stream_amount_provided(
					&bsp_id,
					&owner_account_id
				);
				assert!(maybe_amount_provided.is_some());
				let amount_provided = maybe_amount_provided.unwrap();
				assert_eq!(amount_provided, 2 * size);

                // Assert that the RequestStorageBsps now contains the BSP under both location
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(first_file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );
				assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(second_file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(first_file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: first_file_location.clone(),
                        fingerprint: first_file_fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
						expires_at: next_expiration_tick_storage_request,
                    })
                );
				assert_eq!(
                    file_system::StorageRequests::<Test>::get(second_file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: second_file_location.clone(),
                        fingerprint: second_file_fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
						expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Dispatch BSP request stop storing.
                assert_ok!(FileSystem::bsp_request_stop_storing(
                    bsp_signed.clone(),
                    first_file_key,
                    bucket_id,
                    first_file_location.clone(),
                    owner_account_id.clone(),
                    first_file_fingerprint,
                    size,
                    false,
                    CompactProof {
                        encoded_nodes: vec![first_file_key.as_ref().to_vec()],
                    },
                ));

                // Assert that the RequestStorageBsps has the correct value
                assert!(file_system::StorageRequestBsps::<Test>::get(first_file_key, bsp_id).is_none());

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(first_file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: first_file_location.clone(),
                        fingerprint: first_file_fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
						expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the request was added to the pending stop storing requests.
                assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &first_file_key).is_some());

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspRequestedToStopStoring {
                        bsp_id,
                        file_key: first_file_key,
                        owner: owner_account_id.clone(),
                        location: first_file_location,
                    }
                    .into(),
                );

                // Advance enough blocks to allow the BSP to confirm the stop storing request.
                roll_to(
                    frame_system::Pallet::<Test>::block_number() + MinWaitForStopStoring::get(),
                );

                // Dispatch BSP confirm stop storing.
                assert_ok!(FileSystem::bsp_confirm_stop_storing(
                    bsp_signed.clone(),
                    first_file_key,
                    CompactProof {
                        encoded_nodes: vec![first_file_key.as_ref().to_vec()],
                    },
                ));

                // Assert that the pending stop storing request was removed.
                assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &first_file_key).is_none());

                // Assert that the correct event was deposited.
                let new_root = Providers::get_root(bsp_id).unwrap();

                System::assert_last_event(
                    Event::BspConfirmStoppedStoring {
                        bsp_id,
                        file_key: first_file_key,
                        new_root,
                    }
                    .into(),
                );

				// Check that the amount provided of the dynamic-rate payment stream between the user and the provider was updated
                assert!(
					<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
						&bsp_id,
						&owner_account_id
					)
					.is_some()
				);
				let maybe_amount_provided = <Test as crate::Config>::PaymentStreams::get_dynamic_rate_payment_stream_amount_provided(
					&bsp_id,
					&owner_account_id
				);
				assert!(maybe_amount_provided.is_some());
				let amount_provided = maybe_amount_provided.unwrap();
				assert_eq!(amount_provided, size);
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    Default::default(),
                    ReplicationTarget::Standard,
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap()
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
                assert!(file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id).is_none());

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint: H256::zero(),
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: Default::default(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request,
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    Default::default(),
                    ReplicationTarget::Standard,
                ));

                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Increase the data used by the registered bsp, to simulate that it is indeed storing the file
                assert_ok!(Providers::increase_capacity_used(&bsp_id, size,));

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
                    <Test as Config>::StandardReplicationTarget::get();

                // Assert that the storage request bsps_required was incremented
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: Default::default(),
                        bsps_required: current_bsps_required.checked_add(1).unwrap(),
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request,
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

                let (msp_id, value_prop_id) =
                    add_msp_to_provider_storage(&Keyring::Charlie.to_account_id());

                let bucket_id = create_bucket(
                    &owner_account_id,
                    BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
                    msp_id,
                    value_prop_id,
                );

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), 100));

                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Increase the data used by the registered bsp, to simulate that it is indeed storing the file
                assert_ok!(Providers::increase_capacity_used(&bsp_id, size,));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

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
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: 5,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: None,
                        user_peer_ids: Default::default(),
                        bsps_required: 1,
                        bsps_confirmed: 0,
                        bsps_volunteered: 0,
                        expires_at: next_expiration_tick_storage_request,
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let _ = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                let other_user = Keyring::Bob.to_account_id();
                let bucket_id = create_bucket(&other_user.clone(), name, msp_id, value_prop_id);

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // For loop to create 1 over maximum of MaxUserPendingDeletionRequests
                for i in 0..<Test as file_system::Config>::MaxUserPendingDeletionRequests::get() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone()),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![PendingFileDeletionRequest {
							file_key,
							user: owner_account_id.clone(),
							bucket_id,
							file_size: size,
							deposit_paid_for_creation: file_deletion_request_deposit,
						}]
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
						size,
						bucket_id,
						forest_proof
					),
					Error::<Test>::MspNotStoringBucket
				);

                // Assert that the pending file deletion request was added to storage
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone()),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![PendingFileDeletionRequest {
							file_key,
							user: owner_account_id.clone(),
							bucket_id,
							file_size: size,
							deposit_paid_for_creation: file_deletion_request_deposit,
						}]
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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
                        size,
                        bucket_id,
                        forest_proof
                    ),
                    Error::<Test>::FileKeyNotPendingDeletion
                );
            });
        }
    }

    mod success {
        use crate::MspsAmountOfPendingFileDeletionRequests;

        use super::*;
        #[test]
        fn delete_file_with_proof_of_inclusion_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 1024 * 1024 * 1024;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

				// Increase bucket size and payment stream rate to simulate it storing the file
				let initial_bucket_size = 2 * size;
				assert_ok!(<<Test as crate::Config>::Providers as MutateBucketsInterface>::increase_bucket_size(&bucket_id, initial_bucket_size));
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids,
                    ReplicationTarget::Standard
                ));

                // Dispatch MSP confirm storing.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

				let bucket_size_after_confirm = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap().size;
				let payment_stream_rate_after_confirm = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				let msp_used_capacity_after_confirm = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);

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
                        .any(|x| *x == CustomChallenge { key: file_key, should_remove_key: true }),
                );

				// Assert that the deposit for a file deletion request was NOT held from the user's balance.
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
				assert_ne!(
					<<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&crate::HoldReason::FileDeletionRequestHold.into(), &owner_account_id),
					file_deletion_request_deposit
				);

				// Assert that the Bucket root was correctly updated
				let bucket_info = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap();
				let bucket_root = bucket_info.root;
				assert_eq!(bucket_root, file_key); // This is because our mocked apply delta sets the root as the last mutation

				// Assert that the Bucket's size was decreased by the file size
				let new_bucket_size = bucket_info.size;
				assert_eq!(new_bucket_size, bucket_size_after_confirm - size);

				// Assert that the MSP's used capacity was decreased by the file size
				let new_msp_used_capacity = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);
				assert_eq!(new_msp_used_capacity, msp_used_capacity_after_confirm - size);

				// Assert that the payment stream rate decrease
				let new_payment_stream_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				assert!(new_payment_stream_rate < payment_stream_rate_after_confirm);

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::FileDeletionRequest {
                        user: owner_account_id.clone(),
                        file_key,
                        file_size: size,
                        bucket_id,
                        msp_id,
                        proof_of_inclusion: true,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn delete_file_pending_file_deletion_request_submit_proof_of_inclusion_success() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 1024 * 1024 * 1024; // One gigabyte
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
				let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

				let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

				// Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids,
                    ReplicationTarget::Standard
                ));

                // Dispatch MSP confirm storing.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

				// Get the initial bucket size, MSP's used capacity and payment stream rate
				let initial_bucket_size = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap().size;
				let initial_payment_stream_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				let msp_used_capacity = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);

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
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone()),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![PendingFileDeletionRequest {
							file_key,
							user: owner_account_id.clone(),
							bucket_id,
							file_size: size,
							deposit_paid_for_creation: file_deletion_request_deposit,
						}]
                    )
                        .unwrap()
                );

				// Assert that the deposit for a file deletion request was held from the user's balance.
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
				assert_eq!(
					<<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&crate::HoldReason::FileDeletionRequestHold.into(), &owner_account_id),
					file_deletion_request_deposit
				);

				// Assert that the MSP was removed from the privileged providers list and that it has one pending file deletion request.
				assert!(!pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 1);

				// Get the user's free balance and the payment stream deposit before the MSP submits the proof of inclusion.
				let user_free_balance_before_proof = <<Test as crate::Config>::Currency as Inspect<<Test as frame_system::Config>::AccountId>>::reducible_balance(&owner_account_id, Preservation::Preserve, Fortitude::Force);
				let user_payment_stream_deposit_before_proof = <<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&pallet_payment_streams::HoldReason::PaymentStreamDeposit.into(), &owner_account_id);

                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                let msp_origin = RuntimeOrigin::signed(msp.clone());

                assert_ok!(FileSystem::pending_file_deletion_request_submit_proof(
					msp_origin,
					owner_account_id.clone(),
					file_key,
					size,
					bucket_id,
					forest_proof
				));

				// Assert that the Bucket root was correctly updated
				let bucket_info = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap();
				let bucket_root = bucket_info.root;
				assert_eq!(bucket_root, file_key); // This is because our mocked apply delta sets the root as the last mutation

				// Assert that the Bucket's size was decreased by the file size
				let new_bucket_size = bucket_info.size;
				assert_eq!(new_bucket_size, initial_bucket_size - size);

				// Assert that the MSP's used capacity was decreased by the file size
				let new_msp_used_capacity = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);
				assert_eq!(new_msp_used_capacity, msp_used_capacity - size);

				// Assert that the payment stream rate decreased
				let new_payment_stream_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				assert!(new_payment_stream_rate < initial_payment_stream_rate);

				// Get the new deposit of the payment stream
				let user_payment_stream_deposit_after_proof = <<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&pallet_payment_streams::HoldReason::PaymentStreamDeposit.into(), &owner_account_id);

				// Assert that the MSP was added back to the privileged providers list since it no longer has any pending file deletion requests.
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 0);
				assert!(pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));

				// Assert that the deposit for the file deletion request was returned to the user and that the deposit for the payment stream was updated according to the new rate.
				assert_eq!(
					<<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&crate::HoldReason::FileDeletionRequestHold.into(), &owner_account_id),
					0
				);
				assert_eq!(
					<<Test as crate::Config>::Currency as Inspect<<Test as frame_system::Config>::AccountId>>::reducible_balance(&owner_account_id, Preservation::Preserve, Fortitude::Force),
					user_free_balance_before_proof + file_deletion_request_deposit + (user_payment_stream_deposit_before_proof - user_payment_stream_deposit_after_proof)
				);

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::ProofSubmittedForPendingFileDeletionRequest {
                        msp_id,
                        user: owner_account_id.clone(),
                        file_key,
                        file_size: size,
                        bucket_id,
                        proof_of_inclusion: true,
                    }
                        .into(),
                );

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
                .iter()
                .any(|x| *x == CustomChallenge { key: file_key, should_remove_key: true }),);

                // Assert that the pending file deletion request was removed from storage
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::default()
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone()),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![PendingFileDeletionRequest {
							file_key,
							user: owner_account_id.clone(),
							bucket_id,
							file_size: size,
							deposit_paid_for_creation: file_deletion_request_deposit,
						}]
                    )
                        .unwrap()
                );

				// Assert that the deposit for a file deletion request was held from the user's balance.
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
				assert_eq!(
					<<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&crate::HoldReason::FileDeletionRequestHold.into(), &owner_account_id),
					file_deletion_request_deposit
				);

				// Assert that the MSP was removed from the privileged providers list and that it has one pending file deletion request.
				assert!(!pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 1);

				// Get the user's and MSP's free balance before the MSP submits the proof of non-inclusion.
				let user_free_balance_before_proof = <<Test as crate::Config>::Currency as Inspect<<Test as frame_system::Config>::AccountId>>::reducible_balance(&owner_account_id, Preservation::Preserve, Fortitude::Force);
				let msp_free_balance_before_proof = <<Test as crate::Config>::Currency as Inspect<<Test as frame_system::Config>::AccountId>>::reducible_balance(&msp, Preservation::Preserve, Fortitude::Force);

                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::zero().as_bytes().to_vec()],
                };

                let msp_origin = RuntimeOrigin::signed(msp.clone());

                assert_ok!(FileSystem::pending_file_deletion_request_submit_proof(
					msp_origin,
					owner_account_id.clone(),
					file_key,
					size,
					bucket_id,
					forest_proof
				));

				// Assert that the MSP was added back to the privileged providers list since it no longer has any pending file deletion requests.
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 0);
				assert!(pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));

				// Assert that the deposit for the file deletion request was given to the MSP for their troubles.
				assert_eq!(
					<<Test as crate::Config>::Currency as InspectHold<<Test as frame_system::Config>::AccountId>>::balance_on_hold(&crate::HoldReason::FileDeletionRequestHold.into(), &owner_account_id),
					0
				);
				assert_eq!(
					<<Test as crate::Config>::Currency as Inspect<<Test as frame_system::Config>::AccountId>>::reducible_balance(&owner_account_id, Preservation::Preserve, Fortitude::Force),
					user_free_balance_before_proof
				);
				assert_eq!(
					<<Test as crate::Config>::Currency as Inspect<<Test as frame_system::Config>::AccountId>>::reducible_balance(&msp, Preservation::Preserve, Fortitude::Force),
					msp_free_balance_before_proof + file_deletion_request_deposit
				);

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::ProofSubmittedForPendingFileDeletionRequest {
                        msp_id,
                        user: owner_account_id.clone(),
                        file_key,
                        file_size: size,
                        bucket_id,
                        proof_of_inclusion: false,
                    }
                        .into(),
                );

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(
					!pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
						.iter()
						.any(|x| *x == CustomChallenge { key: file_key, should_remove_key: true }),
				);

                // Assert that the pending file deletion request was removed from storage
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::default()
                );
            });
        }

        #[test]
        fn delete_file_pending_file_deletion_request_submit_proof_does_not_add_msp_to_privileged_providers_if_it_has_more_pending_file_deletion_requests(
        ) {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 1024 * 1024 * 1024; // One gigabyte
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
				let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

				let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

				// Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    None
                ));

				// Issue another storage request
				let other_fingerprint = BlakeTwo256::hash(&b"other".to_vec());
				let other_file_key = FileSystem::compute_file_key(
					owner_account_id.clone(),
					bucket_id,
					location.clone(),
					size,
					other_fingerprint,
				);
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    other_fingerprint,
                    size,
                    msp_id,
                    peer_ids,
                    ReplicationTarget::Custom(1)
                ));

                // Dispatch the MSP accept storing for both files
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: bounded_vec![
								FileKeyWithProof {
                                	file_key,
                                	proof: CompactProof {
                                    	encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                	}
                            	},
								FileKeyWithProof {
									file_key: other_file_key,
									proof: CompactProof {
										encoded_nodes: vec![H256::default().as_ref().to_vec()],
									}
								},
							],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // Get the initial bucket size, MSP's used capacity and payment stream rate
				let initial_bucket_size = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap().size;
				let initial_payment_stream_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				let msp_used_capacity = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);

                // Request to delete the first file
                assert_ok!(FileSystem::delete_file(
					owner_signed.clone(),
					bucket_id,
					file_key,
					location.clone(),
					size,
					fingerprint,
					None,
				));

                // Assert that the pending file deletion request was added to storage
				let file_deletion_request_deposit = <Test as crate::Config>::FileDeletionRequestDeposit::get();
                assert_eq!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone()),
                    BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::try_from(
                        vec![PendingFileDeletionRequest {
							file_key,
							user: owner_account_id.clone(),
							bucket_id,
							file_size: size,
							deposit_paid_for_creation: file_deletion_request_deposit,
						}]
                    )
                        .unwrap()
                );


				// Assert that the MSP was removed from the privileged providers list and that it has one pending file deletion request.
				assert!(!pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 1);

				// Request to delete the second file
				assert_ok!(FileSystem::delete_file(
					owner_signed.clone(),
					bucket_id,
					other_file_key,
					location,
					size,
					other_fingerprint,
					None,
				));

				// Assert that the pending file deletion request was added to storage
				assert!(
					file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone()).iter().any(|x| x.file_key == other_file_key)
				);

				// Assert that the MSP is still not in the privileged providers list and that it has two pending file deletion requests.
				assert!(!pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 2);


                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                let msp_origin = RuntimeOrigin::signed(msp.clone());

				// Submit an inclusion proof for the first file to be deleted
                assert_ok!(FileSystem::pending_file_deletion_request_submit_proof(
					msp_origin.clone(),
					owner_account_id.clone(),
					file_key,
					size,
					bucket_id,
					forest_proof
				));

				// Assert that the Bucket root was correctly updated
				let bucket_info = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap();
				let bucket_root = bucket_info.root;
				assert_eq!(bucket_root, file_key); // This is because our mocked apply delta sets the root as the last mutation

				// Assert that the Bucket's size was decreased by the file size
				let new_bucket_size = bucket_info.size;
				assert_eq!(new_bucket_size, initial_bucket_size - size);

				// Assert that the MSP's used capacity was decreased by the file size
				let new_msp_used_capacity = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);
				assert_eq!(new_msp_used_capacity, msp_used_capacity - size);

				// Assert that the payment stream rate decreased
				let new_payment_stream_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				assert!(new_payment_stream_rate < initial_payment_stream_rate);

				// Assert that the MSP was NOT added back to the privileged providers list since it still has one pending file deletion request.
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 1);
				assert!(!pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::ProofSubmittedForPendingFileDeletionRequest {
                        msp_id,
                        user: owner_account_id.clone(),
                        file_key,
						file_size: size,
                        bucket_id,
                        proof_of_inclusion: true,
                    }
                        .into(),
                );

                // Assert that there is a queued priority challenge for file key in proofs dealer pallet
                assert!(pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
                .iter()
                .any(|x| *x == CustomChallenge { key: file_key, should_remove_key: true } ));

                // Assert that the pending file deletion request was removed from storage
                assert!(
                    file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id.clone())
						.iter()
						.all(|x| x.file_key != file_key)
                );

				// Submit an inclusion proof for the second file to be deleted
				let other_forest_proof = CompactProof {
					encoded_nodes: vec![other_file_key.as_ref().to_vec()],
				};
				assert_ok!(FileSystem::pending_file_deletion_request_submit_proof(
					msp_origin,
					owner_account_id.clone(),
					other_file_key,
					size,
					bucket_id,
					other_forest_proof
				));

				// Assert that the Bucket root was correctly updated
				let bucket_info = pallet_storage_providers::Buckets::<Test>::get(bucket_id).unwrap();
				let bucket_root = bucket_info.root;
				assert_eq!(bucket_root, other_file_key); // This is because our mocked apply delta sets the root as the last mutation

				// Assert that the Bucket's size was decreased by the file size
				let new_bucket_size_after_second_deletion = bucket_info.size;
				assert_eq!(new_bucket_size_after_second_deletion, new_bucket_size - size);

				// Assert that the MSP's used capacity was decreased by the file size
				let new_msp_used_capacity_after_second_deletion = <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);
				assert_eq!(new_msp_used_capacity_after_second_deletion, new_msp_used_capacity - size);

				// Assert that the payment stream rate decreased
				let new_payment_stream_rate_after_second_deletion = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner_account_id).unwrap();
				assert!(new_payment_stream_rate_after_second_deletion < new_payment_stream_rate);

				// Assert that the MSP was added back to the privileged providers list since it no longer has any pending file deletion requests.
				assert_eq!(MspsAmountOfPendingFileDeletionRequests::<Test>::get(&msp_id), 0);
				assert!(pallet_payment_streams::PrivilegedProviders::<Test>::contains_key(&msp_id));

				// Assert that the correct event was deposited
				System::assert_last_event(
					Event::ProofSubmittedForPendingFileDeletionRequest {
						msp_id,
						user: owner_account_id.clone(),
						file_key: other_file_key,
						file_size: size,
						bucket_id,
						proof_of_inclusion: true,
					}
						.into(),
				);

				// Assert that there is a queued priority challenge for file key in proofs dealer pallet
				assert!(
					pallet_proofs_dealer::PriorityChallengesQueue::<Test>::get()
						.iter()
						.any(|x| *x == CustomChallenge { key: other_file_key, should_remove_key: true }),
				);

				// Assert that the last pending file deletion request was removed from storage
				assert_eq!(
					file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id),
					BoundedVec::<_, <Test as file_system::Config>::MaxUserPendingDeletionRequests>::default()
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
        fn query_earliest_file_volunteer_tick() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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

                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

                let tick_number =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();

                assert!(<<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick() <= tick_number);
            });
        }

        #[test]
        fn compute_request_eligibility_criteria() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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

                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

                let storage_request = file_system::StorageRequests::<Test>::get(file_key).unwrap();

                let (eligibility_value, slope) = FileSystem::compute_request_eligibility_criteria(
                    &bsp_id,
                    storage_request.requested_at,
                    <Test as crate::Config>::StandardReplicationTarget::get(),
                )
                .unwrap();

                assert!(
                    eligibility_value > 0
                        && eligibility_value <= ThresholdType::<Test>::max_value()
                );
                assert!(slope > 0);

                let volunteer_tick_number_only_bsp =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();

                // BSP should be able to volunteer either this block or in the future.
                assert!(
                    volunteer_tick_number_only_bsp >= frame_system::Pallet::<Test>::block_number()
                );

                // Simulate there being many BSPs in the network with high reputation weight
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::set(u32::MAX);

                let (eligibility_value, slope) = FileSystem::compute_request_eligibility_criteria(
                    &bsp_id,
                    storage_request.requested_at,
                    <Test as crate::Config>::StandardReplicationTarget::get(),
                )
                .unwrap();

                assert!(
                    eligibility_value > 0
                        && eligibility_value <= ThresholdType::<Test>::max_value()
                );
                assert!(slope > 0);

                let volunteer_tick_number_many_bsps =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();

                // BSP can only volunteer after some number of blocks have passed.
                assert!(
                    volunteer_tick_number_many_bsps > frame_system::Pallet::<Test>::block_number()
                );

                // BSP can volunteer further in the future compared to when it was the only BSP.
                assert!(volunteer_tick_number_many_bsps > volunteer_tick_number_only_bsp);

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

                let (eligibility_value, slope) = FileSystem::compute_request_eligibility_criteria(
                    &bsp_id,
                    storage_request.requested_at,
                    <Test as crate::Config>::StandardReplicationTarget::get(),
                )
                .unwrap();

                assert!(
                    eligibility_value > 0
                        && eligibility_value <= ThresholdType::<Test>::max_value()
                );
                assert!(slope > 0);

                let volunteer_tick =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();

                // BSP should be able to volunteer immediately for the storage request since its reputation weight is so high.
                assert_eq!(volunteer_tick, <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick());
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
                );

                // Dispatch a signed extrinsic.
				let requested_at = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                assert_ok!(FileSystem::issue_storage_request(
                    user.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
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

                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

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

                let (eligibility_value, slope) = FileSystem::compute_request_eligibility_criteria(
                    &bsp_id,
                	requested_at,
                    <Test as crate::Config>::StandardReplicationTarget::get(),
                )
                .unwrap();

                assert_eq!(eligibility_value, ThresholdType::<Test>::max_value());
                assert!(slope > 0);

                let volunteer_tick =
                    FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key).unwrap();

                // BSP should be able to volunteer immediately for the storage request since the reputation weight is so high.
                assert_eq!(volunteer_tick, <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick());
            });
        }
        #[test]
        fn compute_threshold_to_succeed_fails_when_global_weight_zero() {
            new_test_ext().execute_with(|| {
                // Setup: create a BSP
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageData<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

                // Set global_weight to zero
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::set(0);

                let requested_at = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

                let result =
                    FileSystem::compute_request_eligibility_criteria(&bsp_id, requested_at, 1);

                assert_noop!(result, Error::<Test>::NoGlobalReputationWeightSet);
            });
        }

        #[test]
        fn compute_threshold_to_succeed_with_max_slope() {
            new_test_ext().execute_with(|| {
                // Setup: create a BSP
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageData<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(bsp_account_id).unwrap();

                // Set global_weight to 1
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::set(1);

                // Set max reputation weight
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(&bsp_id, |bsp| {
                    match bsp {
                        Some(bsp) => {
                            bsp.reputation_weight = u32::MAX;
                        }
                        None => {
                            panic!("BSP should exist");
                        }
                    }
                });

                let requested_at = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

                let (_eligibility_value, slope) = FileSystem::compute_request_eligibility_criteria(
                    &bsp_id,
                    requested_at,
                    <Test as crate::Config>::StandardReplicationTarget::get(),
                )
                .unwrap();

                assert_eq!(slope, ThresholdType::<Test>::max_value());
            });
        }

        #[test]
        fn bsp_with_higher_weight_should_have_higher_slope() {
            new_test_ext().execute_with(|| {
                // Setup: create a BSP
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageData<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_bob_id = Providers::get_provider_id(bsp_account_id).unwrap();

                // Create another BSP with higher weight
                let bsp_account_id = Keyring::Charlie.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageData<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_charlie_id = Providers::get_provider_id(bsp_account_id).unwrap();

                // Set global_weight to the sum of the two BSPs reputation weights
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::set(10 + 1);

                let requested_at = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

                let (_eligibility_value, slope_bsp_1) =
                    FileSystem::compute_request_eligibility_criteria(
                        &bsp_bob_id,
                        requested_at,
                        <Test as crate::Config>::StandardReplicationTarget::get(),
                    )
                    .unwrap();

                // Set BSP's reputation weight to 10 (10 times higher than the other BSP)
                pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
                    &bsp_charlie_id,
                    |bsp| match bsp {
                        Some(bsp) => {
                            bsp.reputation_weight = 10;
                        }
                        None => {
                            panic!("BSP should exist");
                        }
                    },
                );

                let (_eligibility_value, slope_bsp_2) =
                    FileSystem::compute_request_eligibility_criteria(
                        &bsp_charlie_id,
                        requested_at,
                        <Test as crate::Config>::StandardReplicationTarget::get(),
                    )
                    .unwrap();

                // BSP with higher weight should have higher slope
                assert!(slope_bsp_2 > slope_bsp_1);
            });
        }

        #[test]
        fn compute_threshold_to_succeed_slope_should_be_equal_for_all_starting_weight() {
            new_test_ext().execute_with(|| {
                // Setup: create a BSP
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageData<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_bob_id = Providers::get_provider_id(bsp_account_id).unwrap();
                // Create another BSP
                let bsp_account_id = Keyring::Charlie.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageData<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_charlie_id = Providers::get_provider_id(bsp_account_id).unwrap();

                // Set global_weight to the sum of the weights of the BSPs
                pallet_storage_providers::GlobalBspsReputationWeight::<Test>::set(1 + 1);

                let requested_at = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

                let (_eligibility_value, slope_bsp_1) =
                    FileSystem::compute_request_eligibility_criteria(
                        &bsp_bob_id,
                        requested_at,
                        <Test as crate::Config>::StandardReplicationTarget::get(),
                    )
                    .unwrap();

                let (_eligibility_value, slope_bsp_2) =
                    FileSystem::compute_request_eligibility_criteria(
                        &bsp_charlie_id,
                        requested_at,
                        <Test as crate::Config>::StandardReplicationTarget::get(),
                    )
                    .unwrap();

                // BSPs with equal weight should have equal slope
                assert_eq!(slope_bsp_2, slope_bsp_1);
            });
        }
    }
}

mod stop_storing_for_insolvent_user {
    use super::*;

    mod success {

        use shp_traits::PaymentStreamsInterface;

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Dispatch MSP confirm storing.
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: Some(StorageRequestMspAcceptedFileKeys {
                            file_keys_and_proofs: vec![FileKeyWithProof {
                                file_key,
                                proof: CompactProof {
                                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                                }
                            }],
                            forest_proof: CompactProof {
                                encoded_nodes: vec![H256::default().as_ref().to_vec()],
                            },
                        }),
                        reject: vec![],
                    }],
                ));

                // TODO: Fix the
                // TODO: Uncommenting this would result in test failing with the error `UserWithoutFunds`
                // pallet_payment_streams::UsersWithoutFunds::<Test>::insert(
                //     owner_account_id.clone(),
                //     System::block_number(),
                // );

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, true)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
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
                    Providers::get_root_bucket(&bucket_id).unwrap();

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
        fn stop_storing_for_insolvent_user_works_if_user_does_not_have_payment_stream_with_sp() {
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
                        .expect("BSP should exist in storage"),
                    StorageRequestBspsMetadata::<Test> {
                        confirmed: true,
                        _phantom: Default::default()
                    }
                );

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
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

                // Now that the BSP has confirmed storing, we can simulate the payment stream being deleted
                // and the BSP stopping storing for the user. Note that we use Alice as a user for this test,
                // which is NOT an insolvent user. This simulates the case where the user has correctly paid all
                // its debt but a lagging SP has not updated its storage state yet.

                // Delete the payment stream between the user and the BSP.
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                        &bsp_id,
                        &owner_account_id,
                    )
                );
                // Try to stop storing the user's file as the BSP.
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
    }

    mod failure {

        use super::*;

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                add_msp_to_provider_storage(&another_msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

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

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

                // Dispatch storage request.
                assert_ok!(FileSystem::issue_storage_request(
                    owner_signed.clone(),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    peer_ids.clone(),
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                );

                let bsp_id = Providers::get_provider_id(bsp_account_id.clone()).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(bsp_account_id.clone()).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

                // Dispatch BSP volunteer.
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key,));

                // Get the current tick number.
                let tick_when_confirming = ProofsDealer::get_current_tick();

                // Dispatch BSP confirm storing.
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    CompactProof {
                        encoded_nodes: vec![H256::default().as_ref().to_vec()],
                    },
                    BoundedVec::try_from(vec![FileKeyWithProof {
                        file_key,
                        proof: CompactProof {
                            encoded_nodes: vec![H256::default().as_ref().to_vec()],
                        }
                    }])
                    .unwrap(),
                ));

                // Assert that the storage was updated
                assert_eq!(
                    file_system::StorageRequests::<Test>::get(file_key),
                    Some(StorageRequestMetadata {
                        requested_at: current_tick,
                        owner: owner_account_id.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        size,
                        msp: Some((msp_id, false)),
                        user_peer_ids: peer_ids.clone(),
                        bsps_required: <Test as Config>::StandardReplicationTarget::get(),
                        bsps_confirmed: 1,
                        bsps_volunteered: 1,
                        expires_at: next_expiration_tick_storage_request,
                    })
                );

                // Assert that the RequestStorageBsps was updated
                assert_eq!(
                    file_system::StorageRequestBsps::<Test>::get(file_key, bsp_id)
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

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![file_key]).unwrap(),
                        skipped_file_keys: Default::default(),
                        new_root,
                    }
                    .into(),
                );

                // Assert that the proving cycle was initialised for this BSP.
                let proof_record = ProviderToProofSubmissionRecord::<Test>::get(&bsp_id).unwrap();
                assert_eq!(proof_record.last_tick_proven, tick_when_confirming);

                // Assert that the correct event was deposited.
                System::assert_has_event(
                    Event::BspChallengeCycleInitialised {
                        who: bsp_account_id,
                        bsp_id,
                    }
                    .into(),
                );

                // Assert that the randomness cycle was initialised for this BSP.
                let maybe_first_randomness_provider_deadline =
                    pallet_cr_randomness::ProvidersWithoutCommitment::<Test>::get(&bsp_id);
                assert!(maybe_first_randomness_provider_deadline.is_some());
                assert!(pallet_cr_randomness::ActiveProviders::<Test>::get(&bsp_id).is_some());

                // Assert that the correct event was deposited.
                let first_randomness_provider_deadline =
                    maybe_first_randomness_provider_deadline.unwrap();
                System::assert_has_event(
                    pallet_cr_randomness::Event::ProviderCycleInitialised {
                        provider_id: bsp_id,
                        first_seed_commitment_deadline_tick: first_randomness_provider_deadline,
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
}

mod msp_stop_storing_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn msp_not_registered() {
            new_test_ext().execute_with(|| {
                let msp = Keyring::Charlie.to_account_id();
                let owner_account_id = Keyring::Alice.to_account_id();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                let none_registered_msp = Keyring::Dave.to_account_id();
                let none_registered_msp_signed = RuntimeOrigin::signed(none_registered_msp.clone());

                // Try to stop storing for the bucket.
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket(none_registered_msp_signed, bucket_id),
                    Error::<Test>::NotAMsp
                );
            });
        }

        #[test]
        fn msp_not_storing_bucket() {
            new_test_ext().execute_with(|| {
                let msp = Keyring::Charlie.to_account_id();
                let owner_account_id = Keyring::Alice.to_account_id();

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                let another_msp = Keyring::Dave.to_account_id();
                add_msp_to_provider_storage(&another_msp);
                let another_msp_signed = RuntimeOrigin::signed(another_msp.clone());

                // Try to stop storing for the bucket.
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket(another_msp_signed, bucket_id),
                    Error::<Test>::MspNotStoringBucket
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn msp_stop_storing_bucket_works_payment_stream_deleted() {
            new_test_ext().execute_with(|| {
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let owner_account_id = Keyring::Alice.to_account_id();

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                // Dispatch MSP stop storing bucket.
                assert_ok!(FileSystem::msp_stop_storing_bucket(msp_signed, bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspStoppedStoringBucket {
                        msp_id,
                        bucket_id,
                        owner: owner_account_id.clone(),
                    }
                    .into(),
                );

                // Check that the payment stream between the user and the MSP was deleted since there are no more buckets stored by the MSP for the user.
                assert!(!<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(&msp_id, &owner_account_id));
            });
        }

        #[test]
        fn msp_stop_storing_bucket_works_payment_stream_updated() {
            new_test_ext().execute_with(|| {
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let owner_account_id = Keyring::Alice.to_account_id();

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id);

                let another_name = BoundedVec::try_from(b"another_bucket".to_vec()).unwrap();
                create_bucket(&owner_account_id.clone(), another_name, msp_id, value_prop_id);

                // Dispatch MSP stop storing bucket.
                assert_ok!(FileSystem::msp_stop_storing_bucket(msp_signed, bucket_id));

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspStoppedStoringBucket {
                        msp_id,
                        bucket_id,
                        owner: owner_account_id.clone(),
                    }
                    .into(),
                );

                // Check that the payment stream between the user and the MSP was updated since there are still buckets stored by the MSP for the user.
                assert!(
                    <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(&msp_id, &owner_account_id)
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

fn add_msp_to_provider_storage(
    msp: &sp_runtime::AccountId32,
) -> (ProviderIdFor<Test>, ValuePropId<Test>) {
    let msp_hash = <<Test as frame_system::Config>::Hashing as Hasher>::hash(msp.as_slice());

    let msp_info = pallet_storage_providers::types::MainStorageProvider {
        capacity: 10 * 1024 * 1024 * 1024,
        capacity_used: 0,
        multiaddresses: BoundedVec::default(),
        last_capacity_change: frame_system::Pallet::<Test>::block_number(),
        owner_account: msp.clone(),
        payment_account: msp.clone(),
        sign_up_block: frame_system::Pallet::<Test>::block_number(),
    };

    pallet_storage_providers::MainStorageProviders::<Test>::insert(msp_hash, msp_info);
    pallet_storage_providers::AccountIdToMainStorageProviderId::<Test>::insert(
        msp.clone(),
        msp_hash,
    );

    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10 * 1024 * 1024 * 1024);
    let value_prop_id = value_prop.derive_id();
    pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<Test>::insert(
        msp_hash,
        value_prop_id,
        value_prop,
    );

    (msp_hash, value_prop_id)
}

fn create_bucket(
    owner: &sp_runtime::AccountId32,
    name: BucketNameFor<Test>,
    msp_id: ProviderIdFor<Test>,
    value_prop_id: ValuePropId<Test>,
) -> BucketIdFor<Test> {
    let bucket_id =
        <Test as file_system::Config>::Providers::derive_bucket_id(&owner, name.clone());

    let origin = RuntimeOrigin::signed(owner.clone());

    // Dispatch a signed extrinsic.
    assert_ok!(FileSystem::create_bucket(
        origin,
        msp_id,
        name.clone(),
        false,
        Some(value_prop_id)
    ));

    // Assert bucket was created
    assert_eq!(
        pallet_storage_providers::Buckets::<Test>::get(bucket_id),
        Some(Bucket {
            root: <Test as pallet_storage_providers::pallet::Config>::DefaultMerkleRoot::get(),
            user_id: owner.clone(),
            msp_id: Some(msp_id),
            private: false,
            read_access_group_id: None,
            size: 0,
            value_prop_id: Some(value_prop_id),
        })
    );

    assert!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(&msp_id, &owner));
    assert!(
        <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner).is_some()
    );

    bucket_id
}

use crate::{
    self as file_system,
    mock::*,
    types::{
        BalanceOf, BucketIdFor, BucketMoveRequestResponse, BucketNameFor, CollectionIdFor,
        FileKeyWithProof, FileLocation, FileMetadata, FileOperation, FileOperationIntention,
        MoveBucketRequestMetadata, PeerIds, ProviderIdFor, ReplicationTarget, StorageDataUnit,
        StorageRequestBspsMetadata, StorageRequestMetadata, StorageRequestMspAcceptedFileKeys,
        StorageRequestMspBucketResponse, StorageRequestTtl, ThresholdType, TickNumber, ValuePropId,
    },
    weights::WeightInfo,
    Config, Error, Event, NextAvailableStorageRequestExpirationTick, PendingMoveBucketRequests,
    PendingStopStoringRequests, StorageRequestExpirations, StorageRequests,
};
use codec::Encode;
use frame_support::{
    assert_noop, assert_ok,
    dispatch::DispatchResultWithPostInfo,
    traits::{
        fungible::{InspectHold, Mutate},
        nonfungibles_v2::Destroy,
        Hooks, OriginTrait,
    },
    weights::Weight,
};
use pallet_proofs_dealer::ProviderToProofSubmissionRecord;
use pallet_storage_providers::types::{Bucket, StorageProviderId, ValueProposition};
use shp_traits::{
    MutateBucketsInterface, MutateStorageProvidersInterface, PaymentStreamsInterface,
    PricePerGigaUnitPerTickInterface, ReadBucketsInterface, ReadProvidersInterface,
    ReadStorageProvidersInterface,
};
use sp_core::{ByteArray, Hasher, Pair, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    bounded_vec,
    traits::{BlakeTwo256, Convert, Get},
    BoundedVec, MultiSignature,
};
use sp_std::cmp::max;
use sp_trie::CompactProof;

/// Helper function to create FileMetadata for tests
fn create_test_file_metadata(
    owner_account_id: &<Test as frame_system::Config>::AccountId,
    bucket_id: &BucketIdFor<Test>,
    location: &FileLocation<Test>,
    size: u64,
    fingerprint: H256,
) -> FileMetadata {
    FileMetadata::new(
        owner_account_id.encode(),
        bucket_id.as_ref().to_vec(),
        location.to_vec(),
        size,
        fingerprint.as_bytes().into(),
    )
    .unwrap()
}

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
                        true,
                        ValuePropId::<Test>::default()
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
                    FileSystem::create_bucket(origin, msp_id, name.clone(), private, value_prop_id),
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
                    FileSystem::create_bucket(origin, msp_id, name.clone(), private, value_prop_id),
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
                let nft_collection_deposit: BalanceOf<Test> =
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
                    value_prop_id
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
                        value_prop_id: value_prop_id,
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
                    value_prop_id
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
                        value_prop_id,
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
        fn delete_bucket_bucket_not_found_fail() {
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
        fn delete_bucket_not_bucket_owner_fail() {
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
                    value_prop_id
                ));

                assert_noop!(
                    FileSystem::delete_bucket(not_owner_origin, bucket_id),
                    Error::<Test>::NotBucketOwner
                );
            });
        }

        #[test]
        fn delete_bucket_bucket_not_empty_fail() {
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
                    value_prop_id
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
                                )
                                .unwrap(),
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
        fn delete_bucket_bucket_provider_insolvent() {
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
                    value_prop_id
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
                                    )
                                    .unwrap(),
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
        fn delete_bucket_success() {
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
                    value_prop_id
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
        fn delete_bucket_with_collection_success() {
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
                    value_prop_id
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
        fn delete_bucket_after_being_used_success() {
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
					value_prop_id
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
								)
								.unwrap(),
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
					.unwrap(),
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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

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
                    FileSystem::request_move_bucket(
                        origin,
                        bucket_id,
                        msp_dave_id,
                        dave_value_prop_id
                    ),
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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id,
                    dave_value_prop_id
                ));

                assert_noop!(
                    FileSystem::request_move_bucket(
                        origin,
                        bucket_id,
                        msp_dave_id,
                        dave_value_prop_id
                    ),
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
                let (bucket_id, _) =
                    create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id, false);

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                assert_noop!(
                    FileSystem::request_move_bucket(
                        origin,
                        bucket_id,
                        msp_charlie_id,
                        value_prop_id
                    ),
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
                let (bucket_id, _) =
                    create_bucket(&owner, name.clone(), msp_charlie_id, value_prop_id, false);

                assert_noop!(
                    FileSystem::request_move_bucket(origin, bucket_id, msp_dave_id, value_prop_id),
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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                assert_noop!(
                    FileSystem::request_move_bucket(
                        origin,
                        bucket_id,
                        msp_dave_id,
                        dave_value_prop_id
                    ),
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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                    msp_dave_id,
                    dave_value_prop_id
                ));

                let pending_move_bucket = PendingMoveBucketRequests::<Test>::get(&bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone(),
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id
                    })
                );

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id,
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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

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
                    FileSystem::request_move_bucket(
                        origin.clone(),
                        bucket_id,
                        msp_dave_id,
                        dave_value_prop_id
                    ),
                    Error::<Test>::OperationNotAllowedForInsolventProvider
                );
            });
        }

        #[test]
        fn move_bucket_with_new_value_proposition_not_belonging_to_new_msp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                // Register Charlie and Dave as MSPs.
                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, _) = add_msp_to_provider_storage(&msp_dave);

                // Create a bucket with Charlie as the MSP.
                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                // Make sure bucket is stored by Charlie after creation.
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Create a value proposition that does not belong to Dave.
                let test_value_prop = ValueProposition::<Test>::new(1000, bounded_vec![], 1000);
                let test_value_prop_id = test_value_prop.derive_id();

                // Request to move the bucket to Dave with a value proposition that does not belong to Dave.
                assert_noop!(
                    FileSystem::request_move_bucket(
                        origin.clone(),
                        bucket_id,
                        msp_dave_id,
                        test_value_prop_id
                    ),
                    Error::<Test>::ValuePropositionNotAvailable
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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id,
                    dave_value_prop_id
                ));

                let pending_move_bucket = PendingMoveBucketRequests::<Test>::get(&bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone(),
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id
                    })
                );

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id,
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
                        old_msp_id: Some(msp_charlie_id),
                        new_msp_id: msp_dave_id,
                        bucket_id,
                        value_prop_id: dave_value_prop_id,
                    }
                    .into(),
                );

                // Check bucket is stored by Dave
                assert!(Providers::is_bucket_stored_by_msp(&msp_dave_id, &bucket_id));

                // Check that the bucket's value proposition was updated
                assert_eq!(
                    Providers::get_bucket_value_prop_id(&bucket_id).unwrap(),
                    dave_value_prop_id
                );

                // Check pending bucket storages are cleared
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(bucket_id));
            });
        }

        #[test]
        fn move_bucket_request_and_rejected_by_new_msp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id,
                    dave_value_prop_id
                ));

                let pending_move_bucket = PendingMoveBucketRequests::<Test>::get(&bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone(),
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id
                    })
                );

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id,
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
                        old_msp_id: Some(msp_charlie_id),
                        new_msp_id: msp_dave_id,
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
                assert!(!PendingMoveBucketRequests::<Test>::contains_key(bucket_id));
            });
        }

        #[test]
        fn move_bucket_request_and_expires() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let msp_charlie = Keyring::Charlie.to_account_id();
                let msp_dave = Keyring::Dave.to_account_id();

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id,
					dave_value_prop_id
                ));

                let pending_move_bucket = PendingMoveBucketRequests::<Test>::get(&bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone(),
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id
                    })
                );

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MoveBucketRequested {
                        who: owner,
                        bucket_id,
                        new_msp_id: msp_dave_id,
						new_value_prop_id: dave_value_prop_id
                    }
                    .into(),
                );

                // Check move bucket request expires after MoveBucketRequestTtl
                let move_bucket_request_ttl: u32 = <Test as Config>::MoveBucketRequestTtl::get();
                let move_bucket_request_ttl: TickNumber<Test> = move_bucket_request_ttl.into();
                let expiration = move_bucket_request_ttl + <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();

                // Move tick number to expiration
                roll_to(expiration);

                assert!(!PendingMoveBucketRequests::<Test>::contains_key(bucket_id));
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
                    value_prop_id
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
                        value_prop_id: value_prop_id,
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
                    value_prop_id
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
                        value_prop_id: value_prop_id,
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
                    value_prop_id
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
                        value_prop_id: value_prop_id,
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
                    value_prop_id
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
                let (bucket_id, _) =
                    create_bucket(&owner, name.clone(), msp_id, value_prop_id, false);

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

                let (msp_charlie_id, charlie_value_prop_id) =
                    add_msp_to_provider_storage(&msp_charlie);
                let (msp_dave_id, dave_value_prop_id) = add_msp_to_provider_storage(&msp_dave);

                let name: BucketNameFor<Test> = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner,
                    name.clone(),
                    msp_charlie_id,
                    charlie_value_prop_id,
                    false,
                );

                // Check bucket is stored by Charlie
                assert!(Providers::is_bucket_stored_by_msp(
                    &msp_charlie_id,
                    &bucket_id
                ));

                // Dispatch a signed extrinsic.
                assert_ok!(FileSystem::request_move_bucket(
                    origin.clone(),
                    bucket_id,
                    msp_dave_id,
                    dave_value_prop_id
                ));

                let pending_move_bucket = PendingMoveBucketRequests::<Test>::get(&bucket_id);
                assert_eq!(
                    pending_move_bucket,
                    Some(MoveBucketRequestMetadata {
                        requester: owner.clone(),
                        new_msp_id: msp_dave_id,
                        new_value_prop_id: dave_value_prop_id
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
                let balance_to_mint: BalanceOf<Test> =
                    <<Test as pallet_storage_providers::Config>::BucketDeposit as Get<
                        BalanceOf<Test>,
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
                let (bucket_id, _) = create_bucket(
                    &owner_without_funds,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_without_funds,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                let (bucket_id, _) = create_bucket(
                    &owner_without_funds,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let file_content = b"test".to_vec();
                let fingerprint = BlakeTwo256::hash(&file_content);
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
				let replication_target = ReplicationTarget::Standard;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
					false
                );

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

				// Calculate the upfront amount that the user is going to have to pay to the treasury to issue this storage request and
				// get the treasury's balance before issuing the request.
				let upfront_amount_to_pay = calculate_upfront_amount_to_pay(replication_target.clone(), size);
				let treasury_balance_before = <Test as file_system::Config>::Currency::free_balance(&<Test as crate::Config>::TreasuryAccount::get());

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
                    replication_target
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: storage_request_deposit
                    })
                );

				// Check that the owner paid upfront to cover the retrieval costs and the corresponding
				// deposit was held from the owner's balance.
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
					owner_initial_balance - storage_request_deposit - upfront_amount_to_pay
				);

				// Check that the treasury received the correct amount of funds.
				assert_eq!(
					<Test as file_system::Config>::Currency::free_balance(&<Test as crate::Config>::TreasuryAccount::get()),
					treasury_balance_before + upfront_amount_to_pay
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
                let (bucket_id, _) = create_bucket(
                    &owner_without_funds,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

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
                        expires_at: next_expiration_tick_storage_request,
						deposit_paid: storage_request_deposit
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
                ).unwrap();

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
                        expires_at: next_expiration_tick_storage_request,
						deposit_paid: storage_request_deposit
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

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
						deposit_paid: storage_request_deposit
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id.clone(),
                    name.clone(),
                    msp_id,
                    value_prop_id,
					false
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
                ).unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                ).unwrap();

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

				// Ensure a file deletion request was not created
                assert!(
                    !file_system::PendingFileDeletionRequests::<Test>::get(owner_account_id).iter().any(|r| r.file_key == file_key)
                )
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
                let (bucket_id, _) =
                    create_bucket(&owner_account, name.clone(), msp_id, value_prop_id, false);

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageDataUnit<Test> = 100;

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

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                let file_key = FileSystem::compute_file_key(
                    owner_account.clone(),
                    bucket_id,
                    location.clone(),
                    4,
                    fingerprint,
                )
                .unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                ).unwrap();

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
                ).unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    first_location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

                // Compute the file key for the second file.
                let second_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    second_location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

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
                let (first_bucket_id, _) = create_bucket(
                    &owner_account_id,
                    first_name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Create the bucket that will hold the second file.
                let second_name = BoundedVec::try_from(b"second bucket".to_vec()).unwrap();
                let (second_bucket_id, _) = create_bucket(
                    &owner_account_id,
                    second_name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    first_bucket_id,
                    first_location.clone(),
                    first_size,
                    first_fingerprint,
                )
                .unwrap();

                // Compute the file key for the second file.
                let second_file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    second_bucket_id,
                    second_location.clone(),
                    second_size,
                    second_fingerprint,
                )
                .unwrap();

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
                let (first_bucket_id, _) = create_bucket(
                    &first_owner_account_id.clone(),
                    first_name,
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Create the bucket that will hold the second file.
                let second_name = BoundedVec::try_from(b"second bucket".to_vec()).unwrap();
                let (second_bucket_id, _) = create_bucket(
                    &second_owner_account_id.clone(),
                    second_name,
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Compute the file key for the first file.
                let first_file_key = FileSystem::compute_file_key(
                    first_owner_account_id.clone(),
                    first_bucket_id,
                    first_location.clone(),
                    first_size,
                    first_fingerprint,
                )
                .unwrap();

                // Compute the file key for the second file.
                let second_file_key = FileSystem::compute_file_key(
                    second_owner_account_id.clone(),
                    second_bucket_id,
                    second_location.clone(),
                    second_size,
                    second_fingerprint,
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageDataUnit<Test> = 100;

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
                ).unwrap();

                // Ensure the storage request expiration item was added to the expiration queue
                assert!(file_system::StorageRequestExpirations::<Test>::get(
                    next_expiration_tick_storage_request
                )
                .contains(&file_key));

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id.clone(),
                    name,
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: 0,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: 0,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    expected_msp_id,
                    value_prop_id,
                    false,
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
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                    ReplicationTarget::Standard
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    other_msp_id,
                    value_prop_id,
                    false,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: 0,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: 0,
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                )
                .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Get BSP ID.
                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                )
                .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) =
                    create_bucket(&owner, name.clone(), msp_id, value_prop_id, false);

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

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                )
                .unwrap();

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
                let size = StorageDataUnit::<Test>::try_from(4).unwrap();
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) =
                    create_bucket(&owner, name.clone(), msp_id, value_prop_id, false);

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
                )
                .unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
        fn bsp_volunteer_is_correctly_paid_from_user_deposit() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let origin = RuntimeOrigin::signed(owner.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = StorageDataUnit::<Test>::try_from(4).unwrap();
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) =
                    create_bucket(&owner, name.clone(), msp_id, value_prop_id, false);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
                    file_key,
                )
                .unwrap();
                if tick_when_bsp_can_volunteer > current_tick {
                    let ticks_to_advance = tick_when_bsp_can_volunteer - current_tick + 1;
                    let current_block = System::block_number();

                    // Advance by the number of ticks until this BSP can volunteer for the file.
                    roll_to(current_block + ticks_to_advance);
                }

				// Get the BSP's free balance before volunteering.
				let bsp_initial_balance = <Test as Config>::Currency::free_balance(&bsp_account_id);

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

				// Calculate how much should the BSP have gotten from the user's deposit.
       			let amount_paid_to_bsp = <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
			&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
        		);

				// Assert that the storage request's deposit paid was updated in storage.
				assert_eq!(
					file_system::StorageRequests::<Test>::get(file_key).unwrap().deposit_paid,
					storage_request_deposit - amount_paid_to_bsp
				);

				// Assert that the user's balance on hold decreased by that amount.
				assert_eq!(
                    <Test as Config>::Currency::balance_on_hold(
                        &RuntimeHoldReason::FileSystem(
                            file_system::HoldReason::StorageRequestCreationHold
                        ),
                        &owner
                    ),
                    storage_request_deposit - amount_paid_to_bsp
                );

				// Assert that the BSP's free balance increased by that amount.
                assert_eq!(
                    <Test as Config>::Currency::free_balance(&bsp_account_id),
                    bsp_initial_balance + amount_paid_to_bsp
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id.clone(),
                    name,
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount,));

                // Get BSP ID.
                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                )
                .unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                )
                .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_bob_signed.clone(), storage_amount,));
                assert_ok!(bsp_sign_up(bsp_charlie_signed.clone(), storage_amount,));

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                    ReplicationTarget::Custom(2)
                ));

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

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
                    Providers::get_provider_id(&bsp_bob_account_id).unwrap(),
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
                let storage_amount: StorageDataUnit<Test> = 100;
                let size = 4;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                    let (bucket_id, _) = create_bucket(
                        &owner_account_id,
                        name.clone(),
                        msp_id,
                        value_prop_id,
                        false,
                    );

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
                    )
                    .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                // Setup MSP and bucket
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
                    msp_id,
                    value_prop_id,
                    false,
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
                )
                .unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
						// The deposit paid should have been updated after paying the BSP that volunteered.
						deposit_paid: storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
							&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
								)
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
                ).unwrap();

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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
                let storage_amount: StorageDataUnit<Test> = 100;

                // Setup MSP and bucket
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Setup BSP
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                        )
                        .unwrap();

                        file_key
                    })
                    .collect();

                // Calculate in how many ticks the BSP can volunteer for the files
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = file_keys
                    .iter()
                    .map(|&file_key| {
                        FileSystem::query_earliest_file_volunteer_tick(
                            Providers::get_provider_id(&bsp_account_id).unwrap(),
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

                // Create the correct file metadata for each successful file key
                let confirmed_file_keys_with_metadata: Vec<_> = successful_file_keys
                    .into_iter()
                    .enumerate()
                    .map(|(i, fk)| {
                        // Skip index 0 since that's the pre-confirmed file
                        let location_index = if i == 0 { 1 } else { i + 1 };
                        let location = FileLocation::<Test>::try_from(
                            format!("test{}", location_index).into_bytes(),
                        )
                        .unwrap();
                        let metadata = create_test_file_metadata(
                            &owner_account_id,
                            &bucket_id,
                            &location,
                            size,
                            fingerprint,
                        );
                        (fk, metadata)
                    })
                    .collect();

                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id,
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(
                            confirmed_file_keys_with_metadata,
                        )
                        .unwrap(),
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
						// The deposit paid should have been updated after paying the BSP that volunteered.
						deposit_paid: storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
							&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
						),
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
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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
                ).unwrap();

                // Advance a few ticks and dispatch BSP volunteer.
                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
						// The deposit paid should have been updated after paying the BSP that volunteered.
						deposit_paid: storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
							&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
						),
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
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, new_size, fingerprint))]).unwrap(),
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
                let storage_amount: StorageDataUnit<Test> = 100;

                // Sign up the MSP that will be used in the test and create a bucket under it for the file.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up account as a Backup Storage Provider and get its ID.
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Compute the file key of the storage request to issue.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                )
                .unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
						// The deposit paid should have been updated after paying the BSP that volunteered.
						deposit_paid: storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
							&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
						),
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size - 1, // We change the size so the file key doesn't match the file's metadata
                    fingerprint,
                ).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id, false);

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
						// The deposit paid should have been updated after paying the BSP that volunteered.
						deposit_paid: storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
							&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
						),
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id, false);

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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
						// The deposit paid should have been updated after paying the BSP that volunteered.
						deposit_paid: storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
							&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
						),
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) =
                    create_bucket(&owner_account_id.clone(), name, msp_id, value_prop_id, false);

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: new_deposit_paid
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: new_deposit_paid
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
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
                ).unwrap();

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
						deposit_paid: new_deposit_paid
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

				// Move the pending stop storing request from the `file_key` to the default trie root.
				// This is so the cycles of the BSP are stopped, given that in the mock used for this tests
				// the new root after applying the change will be the `file_key` of the applied change.
				let pending_stop_storing_request = PendingStopStoringRequests::<Test>::get(&bsp_id, &file_key).unwrap();
				let default_trie_root = <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_default_root();
				PendingStopStoringRequests::<Test>::insert(&bsp_id, &default_trie_root, pending_stop_storing_request);

                // Advance enough blocks to allow the BSP to confirm the stop storing request.
                roll_to(
                    frame_system::Pallet::<Test>::block_number() + MinWaitForStopStoring::get(),
                );

                // Dispatch BSP confirm stop storing.
                assert_ok!(FileSystem::bsp_confirm_stop_storing(
                    bsp_signed.clone(),
                    default_trie_root,
                    CompactProof {
                        encoded_nodes: vec![default_trie_root.as_ref().to_vec()],
                    },
                ));

                // Assert that the pending stop storing request was removed.
                assert!(PendingStopStoringRequests::<Test>::get(&bsp_id, &default_trie_root).is_none());

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
                        file_key: default_trie_root,
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
				let current_tick_plus_storage_request_ttl = current_tick + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(NextAvailableStorageRequestExpirationTick::<Test>::get(), current_tick_plus_storage_request_ttl);

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Check that the dynamic-rate payment stream between the user and the provider doesn't exist
                assert!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_info(
					&bsp_id,
					&owner_account_id
				).is_none());

				// Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: new_deposit_paid
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
				let current_tick_plus_storage_request_ttl = current_tick + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
				let next_expiration_tick_storage_request = max(NextAvailableStorageRequestExpirationTick::<Test>::get(), current_tick_plus_storage_request_ttl);

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();
				let second_file_key = FileSystem::compute_file_key(
					owner_account_id.clone(),
					bucket_id,
					second_file_location.clone(),
					size,
					second_file_fingerprint,
				).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
                    first_file_key,
                )
                .unwrap(), FileSystem::query_earliest_file_volunteer_tick(
					Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
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
						deposit_paid: new_deposit_paid
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
						deposit_paid: new_deposit_paid
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
                ).unwrap();

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
						deposit_paid: new_deposit_paid
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(vec![1]).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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
						deposit_paid: storage_request_deposit
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

				let (bucket_id, _) = create_bucket(
					&owner_account_id,
					BoundedVec::try_from(b"bucket".to_vec()).unwrap(),
					msp_id,
					value_prop_id,
					false,
				);

                // Sign up account as a Backup Storage Provider
                assert_ok!(bsp_sign_up(bsp_signed.clone(), 100));

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

				// Get the free balance of the owner and the treasury before the storage request is issued
				let owner_free_balance_before = <Test as Config>::Currency::free_balance(&owner_account_id);
				let treasury_free_balance_before = <Test as Config>::Currency::free_balance(&<Test as Config>::TreasuryAccount::get());

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

				// Assert that the treasury's free balance has only increased by the BSP stop storing file penalty and the owner's
				// free balance has only diminished by the storage request creation deposit, since a storage request generated by
				// a BSP stop storing request does not make the user pay anything upfront.
				assert_eq!(<Test as Config>::Currency::free_balance(&owner_account_id), owner_free_balance_before - storage_request_deposit);
				assert_eq!(<Test as Config>::Currency::free_balance(&<Test as Config>::TreasuryAccount::get()), treasury_free_balance_before + <<Test as crate::Config>::BspStopStoringFilePenalty as Get<BalanceOf<Test>>>::get());

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
						deposit_paid: storage_request_deposit
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                ).unwrap();

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageDataUnit<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                ).unwrap();

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageDataUnit<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
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
                ).unwrap();

                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());

                let storage_amount: StorageDataUnit<Test> = 100;

                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_bob_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Create another BSP with higher weight
                let bsp_account_id = Keyring::Charlie.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageDataUnit<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_charlie_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_bob_id = Providers::get_provider_id(&bsp_account_id).unwrap();
                // Create another BSP
                let bsp_account_id = Keyring::Charlie.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let storage_amount: StorageDataUnit<Test> = 100;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));
                let bsp_charlie_id = Providers::get_provider_id(&bsp_account_id).unwrap();

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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

				// Initially set Eve as solvent, acquiring the read lock.
				let eve_flag_read_lock = set_eve_insolvent(false);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
       			let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
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
                ).unwrap();

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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
				// To do this, we need to release the read lock on the Eve flag, wait until we can write the new
				// value and acquire a new read lock.
				drop(eve_flag_read_lock);
				let eve_flag_read_lock = set_eve_insolvent(true);
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

				// Drop the read lock so other tests that use it can continue.
				drop(eve_flag_read_lock);
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
                let storage_amount: StorageDataUnit<Test> = 50;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

				// Initially set Eve as solvent, acquiring the read lock.
				let eve_flag_read_lock = set_eve_insolvent(false);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
                    })
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

                // Assert that the capacity used by the MSP was updated
                assert_eq!(
                    pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
                        .expect("MSP should exist in storage")
                        .capacity_used,
                    size
                );

                // Now that the MSP has accepted storing, we can simulate the user being insolvent
                // and the MSP stopping storing for the user.
				// To make the user insolvent, we release the read lock and wait until we can write
				// the new value and acquire a new read lock.
				drop(eve_flag_read_lock);
				let eve_flag_read_lock = set_eve_insolvent(true);
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

				// Drop the read lock so other tests that use it can continue.
				drop(eve_flag_read_lock);
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
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
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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

                // Initially set Eve as solvent and get the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Try to stop storing for the insolvent user. To mark Eve as insolvent, drop the read lock and wait to be
                // able to write the new value and acquire a new read lock.
                drop(eve_flag_read_lock);
                let eve_flag_read_lock = set_eve_insolvent(true);
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

                // Drop the latest acquired read lock.
                drop(eve_flag_read_lock);
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
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
                ).unwrap();

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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

                // Initially set Eve as solvent, acquiring the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

                // Try to stop storing for the insolvent user using another MSP account.
                // To do this, we need to release the read lock on the Eve flag, wait until we can write the new
                // value and acquire a new read lock.
                drop(eve_flag_read_lock);
                let eve_flag_read_lock = set_eve_insolvent(true);
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

                // Drop the read lock so other tests that use it can continue.
                drop(eve_flag_read_lock);
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
                let storage_amount: StorageDataUnit<Test> = 100;

                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

				// Initially unset Eve as insolvent, acquiring the read lock.
				let eve_flag_read_lock = set_eve_insolvent(false);

                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let storage_request_deposit = calculate_storage_request_deposit();

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
                ).unwrap();

                let bsp_id = Providers::get_provider_id(&bsp_account_id).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// The deposit paid for the storage request should have been updated after paying the BSP that volunteered.
				let new_deposit_paid = storage_request_deposit - <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);

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
						deposit_paid: new_deposit_paid
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
                ).unwrap();

                let new_root = Providers::get_root(bsp_id).unwrap();

                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::BspConfirmedStoring {
                        who: bsp_account_id.clone(),
                        bsp_id,
                        confirmed_file_keys: BoundedVec::try_from(vec![(file_key, create_test_file_metadata(&owner_account_id, &bucket_id, &location, size, fingerprint))]).unwrap(),
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
				// To do this, we need to release the read lock on the Eve flag, wait until we can write the new
				// value and acquire a new read lock.
				drop(eve_flag_read_lock);
				let eve_flag_read_lock = set_eve_insolvent(true);
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

				// Drop the read lock so other tests that use it can continue.
				drop(eve_flag_read_lock);
            });
        }
    }
}

mod msp_stop_storing_bucket_for_insolvent_user {
    use super::*;

    mod success {

        use super::*;

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_works() {
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
                let storage_amount: StorageDataUnit<Test> = 50;

				// Sign up an account as a Main Storage Provider.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

				// Initially set Eve as solvent, acquiring the read lock.
				let eve_flag_read_lock = set_eve_insolvent(false);

				// Create the bucket, setting it as private so we can check that the collection is deleted.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, maybe_collection_id) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    true,
                );

				// Check that the collection under the collection ID exists.
				assert!(maybe_collection_id.is_some());
				assert!(<<Test as crate::Config>::CollectionInspector as shp_traits::InspectCollections>::collection_exists(&maybe_collection_id.unwrap()));

                // Sign up an account as a Backup Storage Provider.
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				// Calculate the expiration tick that the storage request is going to have.
				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let mut storage_request_deposit = calculate_storage_request_deposit();

                // Issue a storage request for a file in the created bucket.
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

				// Compute the file's key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file.
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// Update the deposit paid by the user since a BSP has used part of it.
				let amount_to_pay = <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);
				storage_request_deposit -= amount_to_pay;

                // Dispatch the BSP confirm storing.
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

                // Dispatch the MSP accept storing the file.
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

                // Assert that the storage request was updated.
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
						deposit_paid: storage_request_deposit
                    })
                );

                // Assert that the capacity used by the MSP was updated.
				let new_msp_used_capacity = pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
					.expect("MSP should exist in storage")
					.capacity_used;
                assert_eq!(new_msp_used_capacity, size);

                // Now that the MSP has accepted storing, we can simulate the user being insolvent
                // and the MSP stopping to store the bucket of the user.
				// To make the user insolvent, we release the read lock and wait until we can write
				// the new value and acquire a new read lock.
				drop(eve_flag_read_lock);
				let eve_flag_read_lock = set_eve_insolvent(true);
                assert_ok!(FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                    msp_signed.clone(),
                    bucket_id,
                ));

				// Check that the payment stream between the MSP and the user does not exist anymore.
				assert_eq!(PaymentStreams::has_active_payment_stream_with_user(&msp_id, &owner_account_id), false);

				// Check that the collection under the collection ID does not exist anymore.
				assert!(!<<Test as crate::Config>::CollectionInspector as shp_traits::InspectCollections>::collection_exists(&maybe_collection_id.unwrap()));

				// Check that the used capacity of the MSP has decreased by the bucket's size.
				assert_eq!(
					pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
						.expect("MSP should exist in storage")
						.capacity_used,
					new_msp_used_capacity - size
				);

				// Check that the bucket was deleted from the system.
				assert!(pallet_storage_providers::Buckets::<Test>::get(bucket_id).is_none());
				assert!(pallet_storage_providers::MainStorageProviderIdsToBuckets::<Test>::get(msp_id, &bucket_id).is_none());
				assert_eq!(pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id).unwrap().amount_of_buckets, 0);
				assert_eq!(<Test as crate::Config>::Currency::balance_on_hold(&pallet_storage_providers::HoldReason::BucketDeposit.into(), &owner_account_id), 0);


                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspStopStoringBucketInsolventUser {
						msp_id,
						owner: owner_account_id,
						bucket_id,
					}
                    .into(),
                );

				// Drop the read lock so other tests that use it can continue.
				drop(eve_flag_read_lock);
            });
        }

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_works_if_user_is_solvent_but_payment_stream_does_not_exist(
        ) {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Alice.to_account_id();
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
                let storage_amount: StorageDataUnit<Test> = 50;

				// Sign up an account as a Main Storage Provider.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

				// Create the bucket, setting it as private so we can check that the collection is deleted.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, maybe_collection_id) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    true,
                );

				// Check that the collection under the collection ID exists.
				assert!(maybe_collection_id.is_some());
				assert!(<<Test as crate::Config>::CollectionInspector as shp_traits::InspectCollections>::collection_exists(&maybe_collection_id.unwrap()));

                // Sign up an account as a Backup Storage Provider.
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

				// Calculate the expiration tick that the storage request is going to have.
				let current_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick();
                let current_tick_plus_storage_request_ttl =
                    current_tick
                        + <<Test as crate::Config>::StorageRequestTtl as Get<u32>>::get() as u64;
                let next_expiration_tick_storage_request = max(
                    NextAvailableStorageRequestExpirationTick::<Test>::get(),
                    current_tick_plus_storage_request_ttl,
                );

				// Calculate the deposit that the user is going to have to pay to issue this storage request.
				let mut storage_request_deposit = calculate_storage_request_deposit();

                // Issue a storage request for a file in the created bucket.
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

				// Compute the file's key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                ).unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file.
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

				// Update the deposit paid by the user since a BSP has used part of it.
				let amount_to_pay = <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
					&<Test as crate::Config>::WeightInfo::bsp_volunteer(),
				);
				storage_request_deposit -= amount_to_pay;

                // Dispatch the BSP confirm storing.
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

                // Dispatch the MSP accept storing the file.
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

                // Assert that the storage request was updated.
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
						deposit_paid: storage_request_deposit
                    })
                );

                // Assert that the capacity used by the MSP was updated.
				let new_msp_used_capacity = pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
					.expect("MSP should exist in storage")
					.capacity_used;
                assert_eq!(new_msp_used_capacity, size);

                // Now that the MSP has accepted storing, we can simulate the user being insolvent
                // previously, the MSP having already deleted its payment stream with him, and the user
				// becoming solvent again.
				assert_ok!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(&msp_id, &owner_account_id));
                assert_ok!(FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                    msp_signed.clone(),
                    bucket_id,
                ));

				// Check that the collection under the collection ID does not exist anymore.
				assert!(!<<Test as crate::Config>::CollectionInspector as shp_traits::InspectCollections>::collection_exists(&maybe_collection_id.unwrap()));

				// Check that the used capacity of the MSP has decreased by the bucket's size.
				assert_eq!(
					pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id)
						.expect("MSP should exist in storage")
						.capacity_used,
					new_msp_used_capacity - size
				);

				// Check that the bucket was deleted from the system.
				assert!(pallet_storage_providers::Buckets::<Test>::get(bucket_id).is_none());
				assert!(pallet_storage_providers::MainStorageProviderIdsToBuckets::<Test>::get(msp_id, &bucket_id).is_none());
				assert_eq!(pallet_storage_providers::MainStorageProviders::<Test>::get(msp_id).unwrap().amount_of_buckets, 0);
				assert_eq!(<Test as crate::Config>::Currency::balance_on_hold(&pallet_storage_providers::HoldReason::BucketDeposit.into(), &owner_account_id), 0);


                // Assert that the correct event was deposited
                System::assert_last_event(
                    Event::MspStopStoringBucketInsolventUser {
						msp_id,
						owner: owner_account_id,
						bucket_id,
					}
                    .into(),
                );
            });
        }
    }

    mod failure {

        use super::*;

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_fails_if_caller_not_a_provider() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Eve.to_account_id();
                let other_account_id = Keyring::Bob.to_account_id();
                let other_account_signed = RuntimeOrigin::signed(other_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();

                // Sign up an account as a Main Storage Provider.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Initially set Eve as solvent, acquiring the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                // Create the bucket.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Note: the user does not even have to be insolvent since the check to make sure the provider is a MSP is done
                // previous to the check of the user's solvency in the extrinsic.

                // Try to stop storing the bucket for the insolvent user as a regular, not registered user.
                // To do this, we need to release the read lock on the Eve flag, wait until we can write the new
                // value and acquire a new read lock.
                drop(eve_flag_read_lock);
                let eve_flag_read_lock = set_eve_insolvent(true);
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                        other_account_signed.clone(),
                        bucket_id,
                    ),
                    Error::<Test>::NotAMsp
                );

                // Drop the read lock so other tests that use it can continue.
                drop(eve_flag_read_lock);
            });
        }

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_fails_if_caller_not_a_msp() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Eve.to_account_id();
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();

                // Sign up an account as a Main Storage Provider.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Sign up an account as a Backup Storage Provider.
                let storage_amount: StorageDataUnit<Test> = 50;
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Initially set Eve as solvent, acquiring the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                // Create the bucket.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Note: the user does not even have to be insolvent since the check to make sure the provider is a MSP is done
                // previous to the check of the user's solvency in the extrinsic.

                // Try to stop storing the bucket for the insolvent user as the BSP.
                // To do this, we need to release the read lock on the Eve flag, wait until we can write the new
                // value and acquire a new read lock.
                drop(eve_flag_read_lock);
                let eve_flag_read_lock = set_eve_insolvent(true);
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                        bsp_signed.clone(),
                        bucket_id,
                    ),
                    Error::<Test>::NotAMsp
                );

                // Drop the read lock so other tests that use it can continue.
                drop(eve_flag_read_lock);
            });
        }

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_fails_if_user_not_insolvent() {
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
                let storage_amount: StorageDataUnit<Test> = 50;

                // Sign up an account as a Main Storage Provider.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Initially set Eve as solvent, acquiring the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                // Create the bucket.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up an account as a Backup Storage Provider.
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Issue a storage request for a file in the created bucket.
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

                // Compute the file's key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file.
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

                // Dispatch the BSP confirm storing.
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

                // Dispatch the MSP accept storing the file.
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

                // Now that the MSP has accepted storing, we can simulate the user being solvent
                // and the MSP trying to stop storing the bucket of the user.
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                        msp_signed.clone(),
                        bucket_id,
                    ),
                    Error::<Test>::UserNotInsolvent
                );

                // Drop the read lock so other tests that use it can continue.
                drop(eve_flag_read_lock);
            });
        }

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_fails_if_bucket_does_not_exist() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Eve.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let msp_signed = RuntimeOrigin::signed(msp.clone());

                // Sign up an account as a Main Storage Provider.
                let _ = add_msp_to_provider_storage(&msp);

                // Initially set Eve as solvent, acquiring the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                // Create a bucket ID of a bucket that does not exist.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <Test as file_system::Config>::Providers::derive_bucket_id(
                    &owner_account_id,
                    name.clone(),
                );

                // Simulate the MSP trying to stop storing a bucket that doesn't exist for an insolvent user.
                // To do this, we need to release the read lock on the Eve flag, wait until we can write the new
                // value and acquire a new read lock.
                drop(eve_flag_read_lock);
                let eve_flag_read_lock = set_eve_insolvent(true);
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                        msp_signed.clone(),
                        bucket_id,
                    ),
                    Error::<Test>::BucketNotFound
                );

                // Drop the read lock so other tests that use it can continue.
                drop(eve_flag_read_lock);
            });
        }

        #[test]
        fn msp_stop_storing_bucket_for_insolvent_user_fails_if_caller_not_storing_the_bucket() {
            new_test_ext().execute_with(|| {
                let owner_account_id = Keyring::Eve.to_account_id();
                let owner_signed = RuntimeOrigin::signed(owner_account_id.clone());
                let bsp_account_id = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account_id.clone());
                let msp = Keyring::Charlie.to_account_id();
                let another_msp = Keyring::Dave.to_account_id();
                let another_msp_signed = RuntimeOrigin::signed(another_msp.clone());
                let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
                let size = 4;
                let fingerprint = H256::zero();
                let peer_id = BoundedVec::try_from(vec![1]).unwrap();
                let peer_ids: PeerIds<Test> = BoundedVec::try_from(vec![peer_id]).unwrap();
                let storage_amount: StorageDataUnit<Test> = 50;

                // Sign up an account as a Main Storage Provider.
                let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

                // Sign up another account as a Main Storage Provider.
                let (_another_msp_id, _) = add_msp_to_provider_storage(&another_msp);

                // Initially set Eve as solvent, acquiring the read lock.
                let eve_flag_read_lock = set_eve_insolvent(false);

                // Create the bucket.
                let name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                // Sign up an account as a Backup Storage Provider.
                assert_ok!(bsp_sign_up(bsp_signed.clone(), storage_amount));

                // Issue a storage request for a file in the created bucket.
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

                // Compute the file's key.
                let file_key = FileSystem::compute_file_key(
                    owner_account_id.clone(),
                    bucket_id,
                    location.clone(),
                    size,
                    fingerprint,
                )
                .unwrap();

                // Calculate in how many ticks the BSP can volunteer for the file.
                let current_tick = ProofsDealer::get_current_tick();
                let tick_when_bsp_can_volunteer = FileSystem::query_earliest_file_volunteer_tick(
                    Providers::get_provider_id(&bsp_account_id).unwrap(),
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

                // Dispatch the BSP confirm storing.
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

                // Dispatch the MSP accept storing the file.
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

                // Now that the MSP has accepted storing, we can simulate the user being insolvent
                // and the other MSP trying to stop storing the bucket of the user.
                // To do this, we need to release the read lock on the Eve flag, wait until we can write the new
                // value and acquire a new read lock.
                drop(eve_flag_read_lock);
                let eve_flag_read_lock = set_eve_insolvent(true);
                assert_noop!(
                    FileSystem::msp_stop_storing_bucket_for_insolvent_user(
                        another_msp_signed.clone(),
                        bucket_id,
                    ),
                    Error::<Test>::MspNotStoringBucket
                );

                // Drop the read lock so other tests that use it can continue.
                drop(eve_flag_read_lock);
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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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
                let (bucket_id, _) = create_bucket(
                    &owner_account_id,
                    name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

                let another_name = BoundedVec::try_from(b"another_bucket".to_vec()).unwrap();
                let (_another_bucket_id, _) = create_bucket(
                    &owner_account_id,
                    another_name.clone(),
                    msp_id,
                    value_prop_id,
                    false,
                );

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

        #[test]
        fn msp_stop_storing_bucket_with_incomplete_storage_request_allows_bucket_deletion() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // BSP volunteers and confirms storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // MSP accepts storage request
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

                // Verify both BSP and MSP have confirmed storing
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);
                assert_eq!(storage_request.msp, Some((msp_id, true)));

                // Owner revokes storage request
                assert_ok!(FileSystem::revoke_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    file_key
                ));

                // Verify incomplete storage request was created
                assert!(
                    crate::IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    crate::IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert_eq!(incomplete_storage_request.pending_bucket_removal, true);

                // Get initial bucket state
                let initial_bucket_size = Providers::get_bucket_size(&bucket_id).unwrap();
                let initial_bucket_root = Providers::get_root_bucket(&bucket_id).unwrap();

                // MSP stops storing the bucket
                assert_ok!(FileSystem::msp_stop_storing_bucket(
                    RuntimeOrigin::signed(msp.clone()),
                    bucket_id
                ));

                // Verify bucket is no longer stored by MSP
                assert_eq!(
                    Providers::get_msp_of_bucket(&bucket_id).unwrap(),
                    None,
                    "Bucket should not have an MSP after stop storing"
                );

                // Verify incomplete storage request still exists
                assert!(
                    crate::IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should still exist"
                );

                // Delete file from bucket (no MSP provided)
                // Create an inclusion proof for the file in the bucket
                // The mock verifier returns encoded_nodes as proven keys
                let bucket_forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    None, // No MSP ID provided since bucket has no MSP
                    bucket_forest_proof_delete,
                ));

                // Verify bucket size decreased
                let final_bucket_size = Providers::get_bucket_size(&bucket_id).unwrap();
                assert_eq!(
                    final_bucket_size,
                    initial_bucket_size - size,
                    "Bucket size should decrease by file size"
                );

                // Verify bucket root changed (in mock, it becomes the file_key)
                let final_bucket_root = Providers::get_root_bucket(&bucket_id).unwrap();
                // In the mock implementation, apply_delta returns the last mutation key
                // which is the file_key for both add and remove operations
                assert_eq!(
                    final_bucket_root, file_key,
                    "Bucket root should be the file_key in mock implementation"
                );

                // Verify incomplete storage request still exists (BSP still pending)
                assert!(
                    crate::IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should still exist after bucket deletion"
                );
                let incomplete_storage_request =
                    crate::IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id],
                    "BSP should still be pending removal"
                );
                assert!(
                    !incomplete_storage_request.pending_bucket_removal,
                    "Bucket removal should be marked as complete"
                );

                // Verify event was emitted
                System::assert_has_event(
                    Event::BucketFileDeletionCompleted {
                        user: owner.clone(),
                        file_key,
                        file_size: size,
                        bucket_id,
                        msp_id: None, // No MSP since bucket has no MSP
                        old_root: initial_bucket_root,
                        new_root: final_bucket_root,
                    }
                    .into(),
                );

                // Delete file from BSP to complete cleanup
                let bsp_forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp_id),
                    bsp_forest_proof_delete,
                ));

                // Verify incomplete storage request was completely removed
                assert!(
                    crate::IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed after all providers deleted"
                );
            });
        }
    }
}

mod request_file_deletion {
    use super::*;

    #[test]
    fn file_owner_can_request_file_deletion() {
        new_test_ext().execute_with(|| {
            // 1. Setup: Create account and get keypair
            let alice_pair = Keyring::Alice.pair();
            let alice_account = Keyring::Alice.to_account_id();
            let alice_origin = RuntimeOrigin::signed(alice_account.clone());

            // Setup MSP and bucket
            let msp = Keyring::Charlie.to_account_id();
            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            let bucket_name =
                BucketNameFor::<Test>::try_from("test-bucket".as_bytes().to_vec()).unwrap();
            let (bucket_id, _) =
                create_bucket(&alice_account, bucket_name, msp_id, value_prop_id, false);

            // Test file metadata
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let file_content = b"buen_fla".to_vec();
            let size = 4;
            let fingerprint = BlakeTwo256::hash(&file_content);

            // Compute file key as done in the actual implementation
            let file_key = FileSystem::compute_file_key(
                alice_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint,
            )
            .unwrap();

            // 2. Construct the file deletion intention
            let signed_delete_intention = FileOperationIntention::<Test> {
                file_key,
                operation: FileOperation::Delete,
            };

            // 3. Sign the intention
            let signed_delete_intention_encoded = signed_delete_intention.encode();
            let signature_bytes = alice_pair.sign(&signed_delete_intention_encoded);
            let signature = MultiSignature::Sr25519(signature_bytes);

            // 4. Call the extrinsic to request the file deletion
            assert_ok!(FileSystem::request_delete_file(
                alice_origin,
                signed_delete_intention.clone(),
                signature.clone(),
                bucket_id,
                location,
                size,
                fingerprint
            ));

            // 5. Verify the event was emitted
            System::assert_last_event(
                Event::FileDeletionRequested {
                    signed_delete_intention,
                    signature,
                }
                .into(),
            );
        });
    }

    #[test]
    fn caller_owner_but_message_not_signed_by_owner() {
        new_test_ext().execute_with(|| {
            // 1. Setup: Create owner account
            let alice_account = Keyring::Alice.to_account_id();
            let alice_origin = RuntimeOrigin::signed(alice_account.clone());

            // Setup MSP and bucket
            let msp = Keyring::Charlie.to_account_id();
            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            let bucket_name =
                BucketNameFor::<Test>::try_from("test-bucket".as_bytes().to_vec()).unwrap();
            let (bucket_id, _) =
                create_bucket(&alice_account, bucket_name, msp_id, value_prop_id, false);

            // Test file metadata
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let file_content = b"buen_fla".to_vec();
            let size = 4;
            let fingerprint = BlakeTwo256::hash(&file_content);

            // Compute file key as done in the actual implementation
            let file_key = FileSystem::compute_file_key(
                alice_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint,
            )
            .unwrap();

            // 2. Construct the file deletion intention
            let signed_delete_intention = FileOperationIntention::<Test> {
                file_key,
                operation: FileOperation::Delete,
            };

            // 3. Non owner signs the intention
            let non_owner_pair = Keyring::Bob.pair();
            let signed_delete_intention_encoded = signed_delete_intention.encode();
            let non_owner_signature_bytes = non_owner_pair.sign(&signed_delete_intention_encoded);
            let non_owner_signature = MultiSignature::Sr25519(non_owner_signature_bytes);

            // 4. Call the extrinsic - caller is the owner, but the intention was signed by other account.
            assert_noop!(
                FileSystem::request_delete_file(
                    alice_origin,
                    signed_delete_intention.clone(),
                    non_owner_signature.clone(),
                    bucket_id,
                    location,
                    size,
                    fingerprint
                ),
                Error::<Test>::InvalidSignature
            );
        });
    }

    #[test]
    fn message_signed_by_owner_but_caller_is_different_account() {
        new_test_ext().execute_with(|| {
            // 1. Setup: Create owner account
            let alice_pair = Keyring::Alice.pair();
            let alice_account = Keyring::Alice.to_account_id();

            // Setup MSP and bucket
            let msp = Keyring::Charlie.to_account_id();
            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            let bucket_name =
                BucketNameFor::<Test>::try_from("test-bucket".as_bytes().to_vec()).unwrap();
            let (bucket_id, _) =
                create_bucket(&alice_account, bucket_name, msp_id, value_prop_id, false);

            // Test file metadata
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let file_content = b"buen_fla".to_vec();
            let size = 4;
            let fingerprint = BlakeTwo256::hash(&file_content);

            // Compute file key as done in the actual implementation
            let file_key = FileSystem::compute_file_key(
                alice_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint,
            )
            .unwrap();

            // 2. Construct the file deletion intention
            let signed_delete_intention = FileOperationIntention::<Test> {
                file_key,
                operation: FileOperation::Delete,
            };

            // 3. Owner signs the intention (valid signature)
            let signed_delete_intention_encoded = signed_delete_intention.encode();
            let owner_signature_bytes = alice_pair.sign(&signed_delete_intention_encoded);
            let owner_signature = MultiSignature::Sr25519(owner_signature_bytes);

            // 4. Different account calls the extrinsic
            let non_owner_account = Keyring::Bob.to_account_id();
            let non_owner_origin = RuntimeOrigin::signed(non_owner_account.clone());

            // 5. Call the extrinsic - signature is valid but caller is not the owner
            assert_noop!(
                FileSystem::request_delete_file(
                    non_owner_origin,
                    signed_delete_intention.clone(),
                    owner_signature.clone(),
                    bucket_id,
                    location,
                    size,
                    fingerprint
                ),
                Error::<Test>::NotBucketOwner
            );
        });
    }

    #[test]
    fn message_signed_by_caller_but_no_owner() {
        new_test_ext().execute_with(|| {
            // 1. Setup: Create owner account
            let alice_account = Keyring::Alice.to_account_id();

            // Setup MSP and bucket
            let msp = Keyring::Charlie.to_account_id();
            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            let bucket_name =
                BucketNameFor::<Test>::try_from("test-bucket".as_bytes().to_vec()).unwrap();
            let (bucket_id, _) =
                create_bucket(&alice_account, bucket_name, msp_id, value_prop_id, false);

            // Test file metadata
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let file_content = b"buen_fla".to_vec();
            let size = 4;
            let fingerprint = BlakeTwo256::hash(&file_content);

            // Compute file key as done in the actual implementation
            let file_key = FileSystem::compute_file_key(
                alice_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint,
            )
            .unwrap();

            // 2. Construct the file deletion intention
            let signed_delete_intention = FileOperationIntention::<Test> {
                file_key,
                operation: FileOperation::Delete,
            };

            // 3. Non owner signs the intention
            let non_owner_pair = Keyring::Bob.pair();
            let non_owner_account = Keyring::Bob.to_account_id();
            let non_owner_origin = RuntimeOrigin::signed(non_owner_account.clone());
            let signed_delete_intention_encoded = signed_delete_intention.encode();
            let non_owner_signature_bytes = non_owner_pair.sign(&signed_delete_intention_encoded);
            let non_owner_signature = MultiSignature::Sr25519(non_owner_signature_bytes);

            // 4. Call the extrinsic - non owner calls and signs the intention.
            assert_noop!(
                FileSystem::request_delete_file(
                    non_owner_origin,
                    signed_delete_intention.clone(),
                    non_owner_signature.clone(),
                    bucket_id,
                    location,
                    size,
                    fingerprint
                ),
                Error::<Test>::NotBucketOwner
            );
        });
    }

    #[test]
    fn insolvent_user_cannot_request_file_deletion() {
        new_test_ext().execute_with(|| {
            // 1. Setup: Create Eve account (the one that can be made insolvent) and get keypair
            let eve_pair = Keyring::Eve.pair();
            let eve_account = Keyring::Eve.to_account_id();
            let eve_origin = RuntimeOrigin::signed(eve_account.clone());

            // Make Eve NOT insolvent initially so we can create a bucket
            let _guard = set_eve_insolvent(false);

            // Setup MSP and bucket
            let msp = Keyring::Charlie.to_account_id();
            let (msp_id, value_prop_id) = add_msp_to_provider_storage(&msp);

            let bucket_name =
                BucketNameFor::<Test>::try_from("test-bucket".as_bytes().to_vec()).unwrap();
            let (bucket_id, _) =
                create_bucket(&eve_account, bucket_name, msp_id, value_prop_id, false);

            // Test file metadata
            let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
            let file_content = b"buen_fla".to_vec();
            let size = 4;
            let fingerprint = BlakeTwo256::hash(&file_content);

            // Compute file key as done in the actual implementation
            let file_key = FileSystem::compute_file_key(
                eve_account.clone(),
                bucket_id,
                location.clone(),
                size,
                fingerprint,
            )
            .unwrap();

            // 2. Construct the file deletion intention
            let signed_delete_intention = FileOperationIntention::<Test> {
                file_key,
                operation: FileOperation::Delete,
            };

            // 3. Sign the intention with Eve's key (valid signature)
            let signed_delete_intention_encoded = signed_delete_intention.encode();
            let signature_bytes = eve_pair.sign(&signed_delete_intention_encoded);
            let signature = MultiSignature::Sr25519(signature_bytes);

            // 4. Drop the first guard and make Eve insolvent
            drop(_guard);
            let _guard = set_eve_insolvent(true);

            // 5. Call the extrinsic - should fail due to insolvency
            assert_noop!(
                FileSystem::request_delete_file(
                    eve_origin,
                    signed_delete_intention.clone(),
                    signature.clone(),
                    bucket_id,
                    location,
                    size,
                    fingerprint
                ),
                Error::<Test>::OperationNotAllowedWithInsolventUser
            );
        });
    }
}

mod delete_file_tests {
    use super::*;

    mod success {
        use super::*;
        use pallet_payment_streams::types::UnitsProvidedFor;
        use shp_traits::{ProofsDealerInterface, TrieRemoveMutation};

        #[test]
        fn msp_can_delete_file_with_valid_forest_proof() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let (bucket_id, file_key, location, size, fingerprint, msp_id, value_prop_id) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Log the payment stream value
                let initial_payment_stream_value = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &alice);

                // Calculate expected payment stream rate
                let initial_bucket_size = <<Test as crate::Config>::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id).unwrap();
                let value_prop = pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<Test>::get(&msp_id, &value_prop_id).unwrap();
                let price_per_giga_unit_of_data_per_block = value_prop.price_per_giga_unit_of_data_per_block;
                let zero_sized_bucket_rate: u128 = <Test as pallet_storage_providers::Config>::ZeroSizeBucketFixedRate::get();
                // Convert bucket size from bytes to giga-units
                let initial_bucket_size_in_giga_units = initial_bucket_size / (shp_constants::GIGAUNIT as u64);
                let expected_initial_payment_stream_rate = (initial_bucket_size_in_giga_units as u128) * price_per_giga_unit_of_data_per_block + zero_sized_bucket_rate;

                assert_eq!(initial_payment_stream_value, Some(expected_initial_payment_stream_rate));
                // Create signature
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);

                // Get the current bucket root before deletion
                let old_bucket_root = <<Test as crate::Config>::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id).unwrap();

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Precalculate expected new root
                let expected_new_root = <<Test as crate::Config>::ProofDealer as ProofsDealerInterface>::generic_apply_delta(
                    &old_bucket_root,
                    &[(file_key, TrieRemoveMutation::default().into())],
                    &forest_proof,
                    Some(bucket_id.encode()),
                ).unwrap();

                // This is incorrect behaviour given that apply_delta returns the mutation provided (in our case, the value we pass as `file_key`
                // to `delete_file_for_incomplete_storage_request`) instead of the new root calculated after the mutation.
                // TODO: Consider using the real `apply_delta` from the `file_system` pallet to enable correct testing.
                assert_eq!(expected_new_root, file_key);

                assert_ok!(FileSystem::delete_file(
                    RuntimeOrigin::signed(alice.clone()),
                    alice.clone(),
                    signed_delete_intention,
                    signature,
                    bucket_id,
                    location,
                    size,
                    fingerprint,
                    None,
                    forest_proof,
                ));

                // Verify BucketFileDeletionCompleted event was emitted
                System::assert_last_event(
                    Event::BucketFileDeletionCompleted {
                        user: alice.clone(),
                        file_key,
                        file_size: size,
                        bucket_id,
                        msp_id: Some(msp_id),
                        old_root: old_bucket_root,
                        new_root: expected_new_root,
                    }
                    .into(),
                );

                let payment_stream_value: Option<BalanceOf<Test>> = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &alice);
                assert_eq!(payment_stream_value, Some(1));

                let used_capacity = <Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);
                // The only file being stored was removed
                assert!(used_capacity == 0);
            });
        }

        #[test]
        fn bsp_delete_file_with_valid_forest_proof_payment_stream_finish_if_no_more_files_stored() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let bsp = Keyring::Bob.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Sign up BSP
                let bsp_signed = RuntimeOrigin::signed(bsp.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), 100));
                let bsp_id = Providers::get_provider_id(&bsp).unwrap();

                // Create bucket for Alice (BSP test still need valid buckets for ownership checks)
                let (bucket_id, file_key, location, size, fingerprint, _, _) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Increase the data used by the registered bsp, to simulate that it is indeed storing the file
                assert_ok!(Providers::increase_capacity_used(&bsp_id, size));

                // Create and increase payment stream, to simulate that BSP is indeed storing the file
                let amount_provided = UnitsProvidedFor::<Test>::from(size);
                assert_ok!(PaymentStreams::create_dynamic_rate_payment_stream(
                    frame_system::RawOrigin::Root.into(),
                    bsp_id,
                    alice.clone(),
                    amount_provided,
                ));

                // Check initial capacity and payment stream state before deletion
                let initial_capacity_used = Providers::get_used_capacity(&bsp_id);
                assert_eq!(
                    initial_capacity_used, size,
                    "BSP should have capacity used equal to file size"
                );

                // Verify payment stream exists before deletion
                let payment_stream =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &alice);
                assert!(
                    payment_stream.is_ok(),
                    "Payment stream should exist before deletion"
                );
                assert_eq!(
                    payment_stream.unwrap().amount_provided,
                    amount_provided,
                    "Payment stream should have correct amount provided"
                );

                // Create signature and proof
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Get current BSP root before deletion
                let old_bsp_root = <<Test as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();

                // Precalculate expected new root
                let expected_new_root = <<Test as crate::Config>::ProofDealer as ProofsDealerInterface>::generic_apply_delta(
                    &old_bsp_root,
                    &[(file_key, TrieRemoveMutation::default().into())],
                    &forest_proof,
                    Some(bsp_id.encode()),
                ).unwrap();

                // This is incorrect behaviour given that apply_delta returns the mutation provided (in our case, the value we pass as `file_key`
                // to `delete_file_for_incomplete_storage_request`) instead of the new root calculated after the mutation.
                // TODO: Consider using the real `apply_delta` from the `file_system` pallet to enable correct testing.
                assert_eq!(expected_new_root, file_key);

                assert_ok!(FileSystem::delete_file(
                    RuntimeOrigin::signed(alice.clone()),
                    alice.clone(),
                    signed_delete_intention,
                    signature,
                    bucket_id,
                    location,
                    size,
                    fingerprint,
                    Some(bsp_id),
                    forest_proof,
                ));

                // Verify BSP event
                System::assert_last_event(
                    Event::BspFileDeletionCompleted {
                        user: alice.clone(),
                        file_key,
                        file_size: size,
                        bsp_id,
                        old_root: old_bsp_root,
                        new_root: expected_new_root,
                    }
                    .into(),
                );

                // Check capacity and payment stream state after deletion
                let final_capacity_used = Providers::get_used_capacity(&bsp_id);
                assert_eq!(
                    final_capacity_used,
                    initial_capacity_used - size,
                    "BSP capacity should have decreased by file size after deletion"
                );

                // Verify payment stream was removed after deletion
                let payment_stream_after =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &alice);
                assert!(
                    payment_stream_after.is_err(),
                    "Payment stream should be removed after file deletion"
                );
            });
        }

        #[test]
        fn bsp_delete_file_with_two_files_stored_payment_stream_updated_not_deleted() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let bsp = Keyring::Bob.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Sign up BSP
                let bsp_signed = RuntimeOrigin::signed(bsp.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), 200));
                let bsp_id = Providers::get_provider_id(&bsp).unwrap();

                // Create bucket for Alice (BSP test still need valid buckets for ownership checks)
                let (bucket_id, file_key, location, size, fingerprint, _, _) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Simulate storing 2 files
                assert_ok!(Providers::increase_capacity_used(&bsp_id, 2 * size));

                // Create and increase payment stream twice, to simulate that BSP is storing 2 files
                let amount_provided_per_file = UnitsProvidedFor::<Test>::from(size);
                let amount_provided = 2 * amount_provided_per_file;

                assert_ok!(PaymentStreams::create_dynamic_rate_payment_stream(
                    frame_system::RawOrigin::Root.into(),
                    bsp_id,
                    alice.clone(),
                    amount_provided,
                ));

                // Check initial capacity and payment stream state before deletion
                let initial_capacity_used = Providers::get_used_capacity(&bsp_id);
                assert_eq!(
                    initial_capacity_used,
                    size * 2,
                    "BSP should have capacity used equal to 2 times file size"
                );

                // Verify payment stream exists before deletion with correct total amount
                let payment_stream =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &alice);
                assert!(
                    payment_stream.is_ok(),
                    "Payment stream should exist before deletion"
                );
                assert_eq!(
                    payment_stream.unwrap().amount_provided,
                    amount_provided,
                    "Payment stream should have correct total amount provided for 2 files"
                );

                // Create signature and proof
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Get current BSP root before deletion
                let old_bsp_root = <<Test as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();

                // Precalculate expected new root
                let expected_new_root = <<Test as crate::Config>::ProofDealer as ProofsDealerInterface>::generic_apply_delta(
                    &old_bsp_root,
                    &[(file_key, TrieRemoveMutation::default().into())],
                    &forest_proof,
                    Some(bsp_id.encode()),
                ).unwrap();

                // This is incorrect behaivour given that apply_delta returns the mutation provided (in our case, the value we pass as `file_key`
                // to `delete_file_for_incomplete_storage_request`) instead of the new root calculated after the mutation.
                // TODO: Consider using the real `apply_delta` from the `file_system` pallet to enable correct testing.
                assert_eq!(expected_new_root, file_key);

                assert_ok!(FileSystem::delete_file(
                    RuntimeOrigin::signed(alice.clone()),
                    alice.clone(),
                    signed_delete_intention,
                    signature,
                    bucket_id,
                    location,
                    size,
                    fingerprint,
                    Some(bsp_id),
                    forest_proof,
                ));

                // Verify BSP event
                System::assert_last_event(
                    Event::BspFileDeletionCompleted {
                        user: alice.clone(),
                        file_key,
                        file_size: size,
                        bsp_id,
                        old_root: old_bsp_root,
                        new_root: expected_new_root,
                    }
                    .into(),
                );

                // Check capacity and payment stream state after deletion
                let final_capacity_used = Providers::get_used_capacity(&bsp_id);
                assert_eq!(
                    final_capacity_used,
                    initial_capacity_used - size,
                    "BSP capacity should have decreased by one file size after deletion"
                );

                // Verify payment stream still exists but is updated (decreased by one file amount)
                let payment_stream_after =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &alice);
                assert!(
                    payment_stream_after.is_ok(),
                    "Payment stream should still exist after deleting one of two files"
                );
                assert_eq!(
                    payment_stream_after.unwrap().amount_provided,
                    amount_provided_per_file,
                    "Payment stream should be updated to reflect remaining file amount"
                );
            });
        }

        #[test]
        fn delete_file_works_with_any_caller() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let (bucket_id, file_key, location, size, fingerprint, msp_id, value_prop_id) =
                    setup_file_in_msp_bucket(&alice, &msp);

                let initial_payment_stream_value = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &alice);
                // Calculate expected payment stream rate
                let initial_bucket_size = <<Test as crate::Config>::Providers as ReadBucketsInterface>::get_bucket_size(&bucket_id).unwrap();
                let value_prop = pallet_storage_providers::MainStorageProviderIdsToValuePropositions::<Test>::get(&msp_id, &value_prop_id).unwrap();
                let price_per_giga_unit_of_data_per_block = value_prop.price_per_giga_unit_of_data_per_block;
                let zero_sized_bucket_rate: u128 = <Test as pallet_storage_providers::Config>::ZeroSizeBucketFixedRate::get();
                // Convert bucket size from bytes to giga-units
                let initial_bucket_size_in_giga_units = initial_bucket_size / (shp_constants::GIGAUNIT as u64);
                let expected_initial_payment_stream_rate = (initial_bucket_size_in_giga_units as u128) * price_per_giga_unit_of_data_per_block + zero_sized_bucket_rate;
                assert_eq!(initial_payment_stream_value, Some(expected_initial_payment_stream_rate));
                // Alice signs the deletion message
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);

                // But Bob (fisherman) calls the extrinsic
                let bob = Keyring::Bob.to_account_id();

                // Get the current bucket root before deletion
                let old_bucket_root = <<Test as crate::Config>::Providers as ReadBucketsInterface>::get_root_bucket(&bucket_id).unwrap();

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Precalculate expected new root
                let expected_new_root = <<Test as crate::Config>::ProofDealer as ProofsDealerInterface>::generic_apply_delta(
                    &old_bucket_root,
                    &[(file_key, TrieRemoveMutation::default().into())],
                    &forest_proof,
                    Some(bucket_id.encode()),
                ).unwrap();

                // This is incorrect behaviour given that apply_delta returns the mutation provided (in our case, the value we pass as `file_key`
                // to `delete_file_for_incomplete_storage_request`) instead of the new root calculated after the mutation.
                // TODO: Consider using the real `apply_delta` from the `file_system` pallet to enable correct testing.
                assert_eq!(expected_new_root, file_key);

                assert_ok!(FileSystem::delete_file(
                    RuntimeOrigin::signed(bob),
                    alice.clone(),
                    signed_delete_intention,
                    signature,
                    bucket_id,
                    location,
                    size,
                    fingerprint,
                    None,
                    forest_proof,
                ));

                // Verify event shows Alice as the user
                System::assert_last_event(
                    Event::BucketFileDeletionCompleted {
                        user: alice.clone(),
                        file_key,
                        file_size: size,
                        bucket_id,
                        msp_id: Some(msp_id),
                        old_root: old_bucket_root,
                        new_root: expected_new_root,
                    }
                    .into(),
                );

                let payment_stream_value: Option<BalanceOf<Test>> = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &alice);
                assert_eq!(payment_stream_value, Some(1));

                let used_capacity = <Providers as ReadStorageProvidersInterface>::get_used_capacity(&msp_id);
                // The only file being stored was removed
                assert!(used_capacity == 0);
            });
        }
    }

    mod failure {
        use super::*;

        #[test]
        fn delete_file_fails_with_invalid_signature() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let (bucket_id, file_key, location, size, fingerprint, _msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Wrong signer (Bob instead of Alice)
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Bob, file_key);

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file(
                        RuntimeOrigin::signed(alice.clone()),
                        alice.clone(),
                        signed_delete_intention,
                        signature,
                        bucket_id,
                        location,
                        size,
                        fingerprint,
                        None,
                        forest_proof,
                    ),
                    Error::<Test>::InvalidSignature
                );
            });
        }

        #[test]
        fn delete_file_fails_with_invalid_forest_proof() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let (bucket_id, file_key, location, size, fingerprint, _msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Alice signs the deletion message
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);

                // But Bob (fisherman) calls the extrinsic
                let bob = Keyring::Bob.to_account_id();

                // Create invalid forest proof
                let invalid_forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file(
                        RuntimeOrigin::signed(bob),
                        alice.clone(),
                        signed_delete_intention,
                        signature,
                        bucket_id,
                        location,
                        size,
                        fingerprint,
                        None,
                        invalid_forest_proof,
                    ),
                    Error::<Test>::ExpectedInclusionProof
                );
            });
        }

        #[test]
        fn delete_file_fails_for_insolvent_user() {
            new_test_ext().execute_with(|| {
                // Setup with Eve (insolvent user) - reusing existing pattern
                let eve = Keyring::Eve.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Create bucket while Eve is solvent
                let _guard = set_eve_insolvent(false);
                let (bucket_id, file_key, location, size, fingerprint, _msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&eve, &msp);
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Eve, file_key);
                drop(_guard);

                // Make Eve insolvent
                let _guard = set_eve_insolvent(true);

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file(
                        RuntimeOrigin::signed(eve.clone()),
                        eve.clone(),
                        signed_delete_intention,
                        signature,
                        bucket_id,
                        location,
                        size,
                        fingerprint,
                        None,
                        forest_proof,
                    ),
                    Error::<Test>::OperationNotAllowedWithInsolventUser
                );
            });
        }

        #[test]
        fn delete_file_fails_when_not_bucket_owner() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let bob = Keyring::Bob.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Alice owns the bucket and file
                let (bucket_id, file_key, location, size, fingerprint, _msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Bob tries to delete Alice's file (wrong owner)
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Bob, file_key);

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Instead of returning `NotBucketOwner` error after checking is_bucket_owner(&file_owner, &bucket_id)?
                // In the extrinsic, we defer the check to the `compute_file_key` function, which returns `InvalidFileKeyMetadata` error
                // This check is essentially the same as the file_owner is part of the metadata needed to compute the file_key
                // Which also then is ensured to be part of the Merkle tree of the given bucket
                // Context: this was chenged to avoid an edge case where:
                // - File is deleted from the bucket
                // - User deletes the bucket (needs to be empty)
                // - Now the owner of the file is no longer the owner of the bucket (it doesn't exist anymore)
                // - BSPs remaining will no longer be able to delete the file, because the file_owner is no longer the owner of the bucket.
                assert_noop!(
                    FileSystem::delete_file(
                        RuntimeOrigin::signed(bob.clone()),
                        bob.clone(), // Wrong file_owner
                        signed_delete_intention,
                        signature,
                        bucket_id,
                        location,
                        size,
                        fingerprint,
                        None,
                        forest_proof,
                    ),
                    Error::<Test>::InvalidFileKeyMetadata
                );
            });
        }

        #[test]
        fn delete_file_fails_with_invalid_provider() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let (bucket_id, file_key, location, size, fingerprint, _, _) =
                    setup_file_in_msp_bucket(&alice, &msp);

                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);

                // Use non-existent provider ID
                let invalid_provider_id = H256::from_low_u64_be(99999);

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file(
                        RuntimeOrigin::signed(alice.clone()),
                        alice.clone(),
                        signed_delete_intention,
                        signature,
                        bucket_id,
                        location,
                        size,
                        fingerprint,
                        Some(invalid_provider_id), // Invalid BSP ID
                        forest_proof,
                    ),
                    Error::<Test>::InvalidProviderID
                );
            });
        }

        #[test]
        fn delete_file_fails_with_file_key_mismatch() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let (bucket_id, file_key, location, size, fingerprint, _msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&alice, &msp);

                // Create a different file key for the signed message
                let wrong_file_key = H256::from_low_u64_be(12345);
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, wrong_file_key);

                // Create forest proof
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file(
                        RuntimeOrigin::signed(alice.clone()),
                        alice.clone(),
                        signed_delete_intention,
                        signature,
                        bucket_id,
                        location,
                        size,
                        fingerprint,
                        None,
                        forest_proof,
                    ),
                    Error::<Test>::InvalidFileKeyMetadata
                );
            });
        }

        #[test]
        fn delete_file_succeeds_when_msp_not_storing_bucket() {
            new_test_ext().execute_with(|| {
                let alice = Keyring::Alice.to_account_id();
                let msp_storing_file = Keyring::Charlie.to_account_id();
                let msp_not_storing_file = Keyring::Dave.to_account_id();

                // Set up MSP that will store the file
                let (
                    bucket_id,
                    file_key,
                    location,
                    size,
                    fingerprint,
                    _msp_storing_file_id,
                    _value_prop_id,
                ) = setup_file_in_msp_bucket(&alice, &msp_storing_file);

                // Set up MSP that does NOT store the file
                let (_msp_not_storing_file_id, _dave_value_prop_id) =
                    add_msp_to_provider_storage(&msp_not_storing_file);

                // Alice signs the deletion message (valid signature)
                let (signed_delete_intention, signature) =
                    create_file_deletion_signature(&Keyring::Alice, file_key);

                // Create valid forest proof for the file stored by MSP
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // This should succeed with InvalidProviderID error
                assert_ok!(FileSystem::delete_file(
                    RuntimeOrigin::signed(alice.clone()),
                    alice.clone(),
                    signed_delete_intention,
                    signature,
                    bucket_id,
                    location,
                    size,
                    fingerprint,
                    None,
                    forest_proof,
                ),);
            });
        }
    }
}

mod delete_file_for_incomplete_storage_request_tests {
    use super::*;
    // Note: as for now we only amend processing expired storage requests
    // This test suite only involves BSPs (not MSPs) which files need to be deleted
    // As, if the MSP accepts the file, the storage request gets accepted (no deletion is needed)
    mod success {
        use crate::{
            types::{RejectedStorageRequest, RejectedStorageRequestReason},
            IncompleteStorageRequests,
        };

        use super::*;
        use pallet_payment_streams::types::UnitsProvidedFor;

        #[test]
        fn single_bsp_expired_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_account = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                // BSP needs to have enough capacity to volunteer for the file
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                let initial_bsp_root = <<Test as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
                let default_root = <<Test as crate::Config>::Providers as shp_traits::ReadProvidersInterface>::get_default_root();
                // Check initial root is the default root
                assert_eq!(initial_bsp_root, default_root);

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id, location.clone(), fingerprint, size, msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // BSP volunteers and confirms
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof { encoded_nodes: vec![file_key.as_ref().to_vec()] },
                };
                let forest_proof = CompactProof { encoded_nodes: vec![H256::default().as_ref().to_vec()] };

                // BSP confirm storing the file
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed, forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Root is correctly updated
                let bsp_root_after_confirming = <<Test as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
                assert_eq!(bsp_root_after_confirming, file_key);

                // BSP should have increased used capacity and payment stream
                let initial_capacity = Providers::get_used_capacity(&bsp_id);
                assert!(initial_capacity == size);

                // Check the payment stream has been created with the correct amount provided
                let payment_stream = PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &owner);
                assert!(payment_stream.is_ok());
                assert_eq!(payment_stream.unwrap().amount_provided, UnitsProvidedFor::<Test>::from(size));

                // Storage request is not rejected and it has 1 confirmed storage provider
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);

                // BSP is in StorageRequestBsps
                let initial_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(initial_bsps.len(), 1, "Should have 1 BSP associated with storage request initially");
                assert_eq!(initial_bsps[0].0, bsp_id, "BSP should be associated with storage request");

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Fisherman should be listening to this event
                System::assert_has_event(
                    Event::StorageRequestRejected {
                        file_key,
                        reason: RejectedStorageRequestReason::RequestExpired,
                    }
                    .into(),
                );

                // Verify the all storage request related data was removed from the storage
                assert!(StorageRequests::<Test>::get(&file_key).is_none(), "Storage request should be removed");
                // Storage request should be removed from bucket associations
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_none(),
                    "Storage request should be removed from bucket associations"
                );
                // All BSPs should be removed from storage request associations
                let final_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert!(
                    final_bsps.is_empty(),
                    "No BSPs should remain associated with the storage request after deletion"
                );

                // Verify the incomplete storage request was created
                assert!(IncompleteStorageRequests::<Test>::get(&file_key).is_some(), "Incomplete storage request should be created");

                // Verify the incomplete storage request has the correct BSPs
                let incomplete_storage_request = IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(incomplete_storage_request.pending_bsp_removals, vec![bsp_id]);
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Create forest proof showing BSP stores the file
                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Call delete_file_for_incomplete_storage_request
                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()), // Any caller
                    file_key,
                    Some(bsp_id),
                    forest_proof,
                ));

                // Warning
                // We can't perform this check that after deleting the only file that a BSP is storing, the root
                // should revert to the default root, because the `apply_delta` function defined in the mock runtime
                // returns the mutation provided (in our case, the value we pass as `file_key` to
                // `delete_file_for_incomplete_storage_request`) instead of the new root calculated after the mutation.
                // Reference: https://github.com/Moonsong-Labs/storage-hub/blob/main/pallets/file-system/src/mock.rs#L568C5-L586
                //
                // This check represents the real functionality of the extrinsic (but we can’t run it here):
                // let bsp_root_after_deleting = <<Test as crate::Config>::Providers as ReadProvidersInterface>::get_root(bsp_id).unwrap();
                // assert_eq!(bsp_root_after_deleting, default_root);
                //
                // A similar issue exists in the `bsp_confirm_stop_storing` test. There, the `default_root` is passed
                // to the extrinsic (and a custom `PendingStopStoringRequests` is injected). We can’t do that here,
                // since we check the integrity of the `file_key` against the `StorageRequest`.
                //
                // This limitation means we can’t test whether the challenge cycles stop.
                // TODO: Consider using the real `apply_delta` from the `file_system` pallet to enable correct testing.

                // Verify the BspFileDeletionCompleted event was emitted
                System::assert_has_event(
                    Event::BspFileDeletionCompleted {
                        user: owner.clone(),
                        file_key,
                        file_size: size,
                        bsp_id,
                        old_root: file_key,
                        new_root: file_key, // This should be default_key
                    }
                    .into(),
                );

                // Verify the FileDeletedFromIncompleteStorageRequest event was emitted
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp_id),
                    }
                    .into(),
                );

                // Verify BSP capacity decreased
                let final_capacity = Providers::get_used_capacity(&bsp_id);
                assert_eq!(final_capacity, initial_capacity - size);

                // Verify payment stream was removed after deletion
                let payment_stream_after =
                PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &owner);
                assert!(
                    payment_stream_after.is_err(),
                    "Payment stream should be removed after file deletion"
                );

                // Incomplete storage request should be completely removed after all providers removed their files
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be completely removed after all providers removed their files"
                );

                // TODO: Check that the challenge cycles stop (we need a proper implementation of apply_delta to be able to test this)
            });
        }

        #[test]
        fn single_bsp_expired_storage_request_with_two_files() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket for first file
                let (bucket_id, file_key1, location1, size, fingerprint1, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup second file
                let location2 = FileLocation::<Test>::try_from(b"test2".to_vec()).unwrap();
                let fingerprint2 = BlakeTwo256::hash(&b"test_content2".to_vec());
                let file_key2 = FileSystem::compute_file_key(
                    owner.clone(),
                    bucket_id,
                    location2.clone(),
                    size,
                    fingerprint2,
                )
                .unwrap();

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 3));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue and complete first storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location1.clone(),
                    fingerprint1,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Issue second storage request that will expire
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location2.clone(),
                    fingerprint2,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key1));
                let file_key1_with_proof = FileKeyWithProof {
                    file_key: file_key1,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key1.as_ref().to_vec()],
                    },
                };
                let forest_proof1 = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    forest_proof1,
                    BoundedVec::try_from(vec![file_key1_with_proof]).unwrap(),
                ));

                // BSP volunteers and confirms second file
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key2));
                let file_key2_with_proof = FileKeyWithProof {
                    file_key: file_key2,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key2.as_ref().to_vec()],
                    },
                };
                let forest_proof2 = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof2,
                    BoundedVec::try_from(vec![file_key2_with_proof]).unwrap(),
                ));

                // Verify BSP now has 2 files and correct payment stream
                let capacity_with_two_files = Providers::get_used_capacity(&bsp_id);
                assert_eq!(
                    capacity_with_two_files,
                    size * 2,
                    "BSP should store 2 files"
                );

                let payment_stream_before =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &owner);
                assert!(payment_stream_before.is_ok(), "Payment stream should exist");
                let amount_provided_file = UnitsProvidedFor::<Test>::from(size);
                let total_amount_before_deletion = amount_provided_file * 2;
                assert_eq!(
                    payment_stream_before.unwrap().amount_provided,
                    total_amount_before_deletion,
                    "Payment stream should reflect both files"
                );

                // Verify storage request has 1 confirmed BSP
                let storage_request = StorageRequests::<Test>::get(&file_key2).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);

                // Verify BSP is in StorageRequestBsps for file2
                let initial_bsps_file2: Vec<_> =
                    file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key2).collect();
                assert_eq!(
                    initial_bsps_file2.len(),
                    1,
                    "Should have 1 BSP associated with file2 storage request initially"
                );
                assert_eq!(
                    initial_bsps_file2[0].0, bsp_id,
                    "BSP should be associated with file2 storage request"
                );

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Verify all Storage request related data was removed from the storage (from file1)
                assert!(
                    StorageRequests::<Test>::get(&file_key1).is_none(),
                    "Storage request should be removed"
                );
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key1)
                        .is_none(),
                    "Storage request should be removed from bucket associations"
                );
                let final_bsps: Vec<_> =
                    file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key1).collect();
                assert!(
                    final_bsps.is_empty(),
                    "No BSPs should remain associated with the storage request after deletion"
                );

                assert!(
                    StorageRequests::<Test>::get(&file_key2).is_none(),
                    "Storage request should be removed"
                );
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key2)
                        .is_none(),
                    "Storage request should be removed from bucket associations"
                );
                let final_bsps: Vec<_> =
                    file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key2).collect();
                assert!(
                    final_bsps.is_empty(),
                    "No BSPs should remain associated with the storage request after deletion"
                );

                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key1).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key1).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key2).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key2).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Verify second storage request was marked as rejected
                System::assert_has_event(
                    Event::StorageRequestRejected {
                        file_key: file_key2,
                        reason: RejectedStorageRequestReason::RequestExpired,
                    }
                    .into(),
                );

                // Delete the expired storage request (we only do it for file 2)
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key2.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key2,
                    Some(bsp_id),
                    forest_proof_delete,
                ));

                // Verify incomplete storage request was removed as bsp was the only provider storing the file
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key2).is_none(),
                    "Incomplete storage request should be removed"
                );

                // Verify deletion events
                System::assert_has_event(
                    Event::BspFileDeletionCompleted {
                        user: owner.clone(),
                        file_key: file_key2,
                        file_size: size,
                        bsp_id,
                        old_root: file_key2,
                        new_root: file_key2,
                    }
                    .into(),
                );

                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key: file_key2,
                        bsp_id: Some(bsp_id),
                    }
                    .into(),
                );

                // Check capacity decreased by second file size (from 2 files to 1 file)
                let final_capacity = Providers::get_used_capacity(&bsp_id);
                assert_eq!(
                    final_capacity, size,
                    "BSP capacity should be back to 1 file after deleting expired file"
                );

                // Verify payment stream still exists but is updated (back to 1 file worth)
                let payment_stream_after =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp_id, &owner);
                assert!(
                    payment_stream_after.is_ok(),
                    "Payment stream should still exist after deleting one of two files"
                );
                assert_eq!(
                    payment_stream_after.unwrap().amount_provided,
                    amount_provided_file,
                    "Payment stream should be updated to reflect remaining file"
                );
            });
        }

        #[test]
        fn two_bsps_expired_storage_request_with_full_cleanup() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp1_account = Keyring::Bob.to_account_id();
                let bsp2_account = Keyring::Dave.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup first BSP
                let bsp1_signed = RuntimeOrigin::signed(bsp1_account.clone());
                assert_ok!(bsp_sign_up(bsp1_signed.clone(), size * 2));
                let bsp1_id = Providers::get_provider_id(&bsp1_account).unwrap();

                // Setup second BSP
                let bsp2_signed = RuntimeOrigin::signed(bsp2_account.clone());
                assert_ok!(bsp_sign_up(bsp2_signed.clone(), size * 2));
                let bsp2_id = Providers::get_provider_id(&bsp2_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Both BSPs volunteer for the storage request
                assert_ok!(FileSystem::bsp_volunteer(bsp1_signed.clone(), file_key));
                assert_ok!(FileSystem::bsp_volunteer(bsp2_signed.clone(), file_key));

                // Both BSPs confirm storing the file
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp1_signed,
                    forest_proof.clone(),
                    BoundedVec::try_from(vec![file_key_with_proof.clone()]).unwrap(),
                ));

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp2_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Both BSPs have the file and payment streams
                let bsp1_capacity = Providers::get_used_capacity(&bsp1_id);
                let bsp2_capacity = Providers::get_used_capacity(&bsp2_id);
                assert_eq!(bsp1_capacity, size);
                assert_eq!(bsp2_capacity, size);

                let payment_stream_bsp1 =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp1_id, &owner);
                let payment_stream_bsp2 =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp2_id, &owner);
                assert!(payment_stream_bsp1.is_ok());
                assert!(payment_stream_bsp2.is_ok());
                assert_eq!(
                    payment_stream_bsp1.unwrap().amount_provided,
                    UnitsProvidedFor::<Test>::from(size)
                );
                assert_eq!(
                    payment_stream_bsp2.unwrap().amount_provided,
                    UnitsProvidedFor::<Test>::from(size)
                );

                // Storage request has 2 confirmed BSPs
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    storage_request.bsps_confirmed, 2,
                    "Storage request should have 2 confirmed BSPs"
                );

                // Both BSPs are in StorageRequestBsps
                let initial_bsps: Vec<_> =
                    file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(
                    initial_bsps.len(),
                    2,
                    "Should have 2 BSPs associated with storage request initially"
                );
                let bsp_ids: Vec<_> = initial_bsps.iter().map(|(bsp_id, _)| *bsp_id).collect();
                assert!(
                    bsp_ids.contains(&bsp1_id),
                    "BSP1 should be associated with storage request"
                );
                assert!(
                    bsp_ids.contains(&bsp2_id),
                    "BSP2 should be associated with storage request"
                );

                // Verify incomplete storage request for this file_key does not exist
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist"
                );

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Verify all Storage request related data was removed from the storage
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be removed"
                );
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key)
                        .is_none(),
                    "Storage request should be removed from bucket associations"
                );
                let final_bsps: Vec<_> =
                    file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert!(
                    final_bsps.is_empty(),
                    "No BSPs should remain associated with the storage request after deletion"
                );

                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp2_id, bsp1_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Storage request was marked as rejected
                System::assert_has_event(
                    Event::StorageRequestRejected {
                        file_key,
                        reason: RejectedStorageRequestReason::RequestExpired,
                    }
                    .into(),
                );

                // First BSP deletion
                let forest_proof_delete1 = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp1_id),
                    forest_proof_delete1,
                ));

                // Verify first BSP deletion events and state changes
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp1_id),
                    }
                    .into(),
                );

                // Verify first BSP capacity is reduced and payment stream removed
                let bsp1_capacity_after_delete = Providers::get_used_capacity(&bsp1_id);
                assert_eq!(
                    bsp1_capacity_after_delete, 0,
                    "BSP1 should have no capacity after deletion"
                );

                let payment_stream_bsp1_after =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp1_id, &owner);
                assert!(
                    payment_stream_bsp1_after.is_err(),
                    "Payment stream for BSP1 should be removed"
                );

                // Verify incomplete storage request still exists with 1 confirmed BSP
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp2_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Second BSP deletion - this should trigger full cleanup
                let forest_proof_delete2 = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp2_id),
                    forest_proof_delete2,
                ));

                // Verify second BSP deletion events and state changes
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp2_id),
                    }
                    .into(),
                );

                // Verify second BSP capacity is reduced and payment stream removed
                let bsp2_capacity_after_delete = Providers::get_used_capacity(&bsp2_id);
                assert_eq!(
                    bsp2_capacity_after_delete, 0,
                    "BSP2 should have no capacity after deletion"
                );

                let payment_stream_bsp2_after =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&bsp2_id, &owner);
                assert!(
                    payment_stream_bsp2_after.is_err(),
                    "Payment stream for BSP2 should be removed"
                );

                // Verify incomplete storage request was removed as bsp was the only provider storing the file
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed"
                );
            });
        }

        #[test]
        fn revoke_with_confirmed_bsp_creates_incomplete_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // BSP volunteers and confirms storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Verify BSP has confirmed storing
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);

                // Verify incomplete storage request does not exist initially
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist initially"
                );

                // Owner revokes storage request
                assert_ok!(FileSystem::revoke_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    file_key
                ));

                // Verify storage request was cleaned up
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be removed after revoke"
                );

                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Verify can successfully cleanup with delete_file_for_incomplete_storage_request
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp_id),
                    forest_proof_delete,
                ));

                // Verify incomplete storage request was removed after cleanup
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed after cleanup"
                );

                // Verify events were emitted
                System::assert_has_event(Event::StorageRequestRevoked { file_key }.into());
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp_id),
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn revoke_with_confirmed_msp_creates_incomplete_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // MSP accepts storage request
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

                // Verify MSP has accepted storing (no BSPs confirmed)
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 0);
                assert_eq!(storage_request.msp, Some((msp_id, true)));

                // Verify incomplete storage request does not exist initially
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist initially"
                );

                // Owner revokes storage request
                assert_ok!(FileSystem::revoke_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    file_key
                ));

                // Verify storage request was cleaned up
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be removed after revoke"
                );

                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert!(incomplete_storage_request.pending_bsp_removals.is_empty());
                assert_eq!(incomplete_storage_request.pending_bucket_removal, true);

                // Verify can successfully cleanup with delete_file_for_incomplete_storage_request
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    None,
                    forest_proof_delete,
                ));

                // Verify incomplete storage request was removed after cleanup
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed after cleanup"
                );

                // Verify events were emitted
                System::assert_has_event(Event::StorageRequestRevoked { file_key }.into());
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: None,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn revoke_with_both_msp_and_bsp_confirmed_creates_incomplete_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // BSP volunteers and confirms storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // MSP accepts storage request
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

                // Verify both BSP and MSP have confirmed storing
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);
                assert_eq!(storage_request.msp, Some((msp_id, true)));

                // Verify incomplete storage request does not exist initially
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist initially"
                );

                // Owner revokes storage request
                assert_ok!(FileSystem::revoke_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    file_key
                ));

                // Verify storage request was cleaned up
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be removed after revoke"
                );

                // Verify incomplete storage request was created with both MSP and BSP
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert_eq!(incomplete_storage_request.pending_bucket_removal, true);

                // Delete file from MSP first
                let msp_forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    None,
                    msp_forest_proof_delete,
                ));

                // Verify incomplete storage request still exists but MSP is removed
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should still exist after MSP deletion"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Delete file from BSP (last provider)
                let bsp_forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp_id),
                    bsp_forest_proof_delete,
                ));

                // Verify incomplete storage request was completely removed after BSP deletion
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed after all providers deleted"
                );

                // Verify events were emitted
                System::assert_has_event(Event::StorageRequestRevoked { file_key }.into());
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: None,
                    }
                    .into(),
                );
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp_id),
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn revoke_with_both_msp_and_bsp_confirmed_creates_incomplete_storage_request_delete_bsp_first(
        ) {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // BSP volunteers and confirms storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // MSP accepts storage request
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

                // Verify both BSP and MSP have confirmed storing
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);
                assert_eq!(storage_request.msp, Some((msp_id, true)));

                // Verify incomplete storage request does not exist initially
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist initially"
                );

                // Owner revokes storage request
                assert_ok!(FileSystem::revoke_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    file_key
                ));

                // Verify storage request was cleaned up
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be removed after revoke"
                );

                // Verify incomplete storage request was created with both MSP and BSP
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert_eq!(incomplete_storage_request.pending_bucket_removal, true);

                // Delete file from BSP first
                let bsp_forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp_id),
                    bsp_forest_proof_delete,
                ));

                // Verify incomplete storage request still exists but BSP is removed
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should still exist after BSP deletion"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert!(incomplete_storage_request.pending_bsp_removals.is_empty());
                assert_eq!(incomplete_storage_request.pending_bucket_removal, true);

                // Delete file from MSP (last provider)
                let msp_forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    None,
                    msp_forest_proof_delete,
                ));

                // Verify incomplete storage request was completely removed after MSP deletion
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed after all providers deleted"
                );

                // Verify events were emitted
                System::assert_has_event(Event::StorageRequestRevoked { file_key }.into());
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp_id),
                    }
                    .into(),
                );
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: None,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn msp_reject_with_confirmed_bsp_creates_incomplete_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // BSP volunteers and confirms storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Verify BSP has confirmed storing
                let storage_request = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(storage_request.bsps_confirmed, 1);

                // Verify incomplete storage request does not exist initially
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist initially"
                );

                // MSP rejects storage request
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: None,
                        reject: vec![RejectedStorageRequest {
                            file_key,
                            reason: RejectedStorageRequestReason::ReachedMaximumCapacity,
                        }],
                    }],
                ));

                // Verify storage request was cleaned up
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be removed after MSP reject"
                );

                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Verify can successfully cleanup with delete_file_for_incomplete_storage_request
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp_id),
                    forest_proof_delete,
                ));

                // Verify incomplete storage request was removed after cleanup
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should be removed after cleanup"
                );

                // Verify events were emitted
                System::assert_has_event(
                    Event::StorageRequestRejected {
                        file_key,
                        reason: RejectedStorageRequestReason::ReachedMaximumCapacity,
                    }
                    .into(),
                );
                System::assert_has_event(
                    Event::FileDeletedFromIncompleteStorageRequest {
                        file_key,
                        bsp_id: Some(bsp_id),
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn no_bsp_confirmations_triggers_full_cleanup_on_expiration() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id, location.clone(), fingerprint, size, msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Verify storage request exists initially
                let initial_request = StorageRequests::<Test>::get(&file_key);
                assert!(initial_request.is_some(), "Storage request should exist initially");
                let initial_request = initial_request.unwrap();
                assert_eq!(initial_request.bsps_confirmed, 0, "No BSPs should be confirmed initially");

                // Verify storage request is in BucketsWithStorageRequests
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_some(),
                    "File key should be in bucket storage requests"
                );

                // BSP volunteers but do not confirm storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed, file_key));

                // Verify BSP volunteered
                let volunteered_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(volunteered_bsps.len(), 1, "BSP should have volunteered");
                assert_eq!(volunteered_bsps[0].0, bsp_id, "Correct BSP should have volunteered");

                // Storage request should still show 0 confirmed BSPs
                let request_before_expiry = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(request_before_expiry.bsps_confirmed, 0, "No BSPs should be confirmed before expiry");

                // Trigger storage request expiration: as no provider confirmed storing,
                // the storage request should be rejected and cleaned up.
                trigger_storage_request_expiration();

                // Verify full cleanup occurred:
                // 1. Storage request should be completely removed
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be completely cleaned up after expiration with no confirmations"
                );

                // 2. BSP associations should be completely cleaned up
                let final_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(final_bsps.len(), 0, "BSP associations should be completely cleaned up");

                // 3. Bucket storage requests should be cleaned up
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_none(),
                    "File key should be removed from bucket storage requests after cleanup"
                );

                // Verify incomplete storage request for this file_key does not exist
                assert!(IncompleteStorageRequests::<Test>::get(&file_key).is_none(), "Incomplete storage request should not exist");

                // Verify the StorageRequestRejected event was emitted
                System::assert_has_event(
                    Event::StorageRequestRejected {
                        file_key,
                        reason: RejectedStorageRequestReason::RequestExpired,
                    }.into()
                );
            });
        }

        #[test]
        fn revoke_with_no_confirmations_triggers_full_cleanup() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Verify storage request exists initially
                let initial_request = StorageRequests::<Test>::get(&file_key);
                assert!(initial_request.is_some(), "Storage request should exist initially");
                let initial_request = initial_request.unwrap();
                assert_eq!(initial_request.bsps_confirmed, 0, "No BSPs should be confirmed initially");

                // Verify storage request is in BucketsWithStorageRequests
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_some(),
                    "File key should be in bucket storage requests"
                );

                // Owner revokes storage request
                assert_ok!(FileSystem::revoke_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    file_key
                ));

                // Verify full cleanup occurred:
                // 1. Storage request should be completely removed
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be completely cleaned up after revoke with no confirmations"
                );

                // 2. BSP associations should be completely cleaned up
                let final_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(final_bsps.len(), 0, "BSP associations should be completely cleaned up");

                // 3. Bucket storage requests should be cleaned up
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_none(),
                    "File key should be removed from bucket storage requests after cleanup"
                );

                // 4. Verify incomplete storage request was NOT created (since no providers confirmed)
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not be created when no providers confirmed"
                );

                // Verify the StorageRequestRevoked event was emitted
                System::assert_has_event(Event::StorageRequestRevoked { file_key }.into());
            });
        }

        #[test]
        fn msp_reject_with_no_confirmations_triggers_full_cleanup() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Verify storage request exists initially
                let initial_request = StorageRequests::<Test>::get(&file_key);
                assert!(initial_request.is_some(), "Storage request should exist initially");
                let initial_request = initial_request.unwrap();
                assert_eq!(initial_request.bsps_confirmed, 0, "No BSPs should be confirmed initially");

                // Verify storage request is in BucketsWithStorageRequests
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_some(),
                    "File key should be in bucket storage requests"
                );

                // MSP rejects storage request
                assert_ok!(FileSystem::msp_respond_storage_requests_multiple_buckets(
                    RuntimeOrigin::signed(msp.clone()),
                    vec![StorageRequestMspBucketResponse {
                        bucket_id,
                        accept: None,
                        reject: vec![RejectedStorageRequest {
                            file_key,
                            reason: RejectedStorageRequestReason::ReachedMaximumCapacity,
                        }],
                    }],
                ));

                // Verify full cleanup occurred:
                // 1. Storage request should be completely removed
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be completely cleaned up after MSP reject with no confirmations"
                );

                // 2. BSP associations should be completely cleaned up
                let final_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(final_bsps.len(), 0, "BSP associations should be completely cleaned up");

                // 3. Bucket storage requests should be cleaned up
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_none(),
                    "File key should be removed from bucket storage requests after cleanup"
                );

                // 4. Verify incomplete storage request was NOT created (since no providers confirmed)
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not be created when no providers confirmed"
                );

                // Verify the StorageRequestRejected event was emitted
                System::assert_has_event(
                    Event::StorageRequestRejected {
                        file_key,
                        reason: RejectedStorageRequestReason::ReachedMaximumCapacity,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn msp_accept_with_no_bsp_threshold_reached_is_fulfilled_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id, location.clone(), fingerprint, size, msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Verify storage request exists initially
                let initial_request = StorageRequests::<Test>::get(&file_key);
                assert!(initial_request.is_some(), "Storage request should exist initially");
                assert_eq!(initial_request.unwrap().bsps_confirmed, 0, "No BSPs should be confirmed initially");

                // Verify storage request is in BucketsWithStorageRequests
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_some(),
                    "File key should be in bucket storage requests"
                );

                // MSP accepts storage request (but no BSPs confirm)
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

                // Verify MSP accepted but no BSPs confirmed
                let request_before_expiry = StorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(request_before_expiry.bsps_confirmed, 0, "No BSPs should be confirmed");
                assert_eq!(request_before_expiry.msp, Some((msp_id, true)), "MSP should have accepted");

                // Trigger storage request expiration: MSP accepted but no BSPs confirmed,
                // We treat this as an fulfilled storage request.
                // This will cleanup the storage request completely and no incomplete storage request will be created.
                trigger_storage_request_expiration();

                // Verify full cleanup occurred:
                // 1. Storage request should be completely removed
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be completely cleaned up after expiration with MSP accept but no BSP confirmations"
                );

                // 2. BSP associations should be completely cleaned up
                let final_bsps: Vec<_> = file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(final_bsps.len(), 0, "BSP associations should be completely cleaned up");

                // 3. Bucket storage requests should be cleaned up
                assert!(
                    file_system::BucketsWithStorageRequests::<Test>::get(&bucket_id, &file_key).is_none(),
                    "File key should be removed from bucket storage requests after cleanup"
                );

                // 4. Verify incomplete storage request was not created as we treat this as an fulfilled storage request
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_none(),
                    "Incomplete storage request should not exist when no BSPs confirmed, even if MSP accepted"
                );

                // Verify the expired storage request event was emitted
                System::assert_has_event(Event::StorageRequestExpired { file_key }.into());
            });
        }
    }

    mod failure {
        use crate::{types::IncompleteStorageRequestMetadata, IncompleteStorageRequests};

        use super::*;

        #[test]
        fn storage_request_not_found() {
            new_test_ext().execute_with(|| {
                let non_existent_file_key = H256::from_low_u64_be(99999);
                let bsp_account = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed, 100));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                let forest_proof = CompactProof {
                    encoded_nodes: vec![non_existent_file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Alice.to_account_id()),
                        non_existent_file_key,
                        Some(bsp_id),
                        forest_proof,
                    ),
                    Error::<Test>::IncompleteStorageRequestNotFound
                );
            });
        }

        #[test]
        fn storage_request_not_rejected() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location,
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Verify storage request exists
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_some(),
                    "Storage request should exist"
                );

                // BSP volunteers but do not confirm storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed, file_key));

                // Verify BSP volunteered
                let volunteered_bsps: Vec<_> =
                    file_system::StorageRequestBsps::<Test>::iter_prefix(&file_key).collect();
                assert_eq!(volunteered_bsps.len(), 1, "BSP should have volunteered");
                assert_eq!(
                    volunteered_bsps[0].0, bsp_id,
                    "Correct BSP should have volunteered"
                );

                let forest_proof = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                // Try to delete from active (non-rejected) storage request
                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Alice.to_account_id()),
                        file_key,
                        Some(bsp_id),
                        forest_proof,
                    ),
                    Error::<Test>::IncompleteStorageRequestNotFound
                );
            });
        }

        #[test]
        fn provider_not_associated_with_storage_request() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp1_account = Keyring::Bob.to_account_id();
                let bsp2_account = Keyring::Dave.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup two BSPs
                let bsp1_signed = RuntimeOrigin::signed(bsp1_account.clone());
                assert_ok!(bsp_sign_up(bsp1_signed.clone(), size * 2));
                let bsp1_id = Providers::get_provider_id(&bsp1_account).unwrap();

                let bsp2_signed = RuntimeOrigin::signed(bsp2_account.clone());
                assert_ok!(bsp_sign_up(bsp2_signed.clone(), size * 2));
                let bsp2_id = Providers::get_provider_id(&bsp2_account).unwrap();

                // Issue storage request
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Only BSP1 volunteers and confirms
                assert_ok!(FileSystem::bsp_volunteer(bsp1_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp1_signed,
                    forest_proof.clone(),
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Verify storage request was deleted
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be deleted"
                );
                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp1_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Try to delete from BSP2
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                        file_key,
                        Some(bsp2_id), // BSP2 never volunteered
                        forest_proof_delete,
                    ),
                    Error::<Test>::ProviderNotStoringFile
                );

                // Verify incomplete storage request was not impacted
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp1_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);
            });
        }

        #[test]
        fn invalid_provider_id() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request and have BSP volunteer/confirm
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Try to delete using non-existent provider ID
                let non_existent_provider_id = H256::from_low_u64_be(99999);
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                        file_key,
                        Some(non_existent_provider_id), // Non-existent provider
                        forest_proof_delete,
                    ),
                    Error::<Test>::ProviderNotStoringFile
                );

                // Verify incomplete storage request was not impacted
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);
            });
        }

        #[test]
        fn file_already_deleted_from_bsp() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();
                let bsp2_account = Keyring::Dave.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSPs
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                let bsp2_signed = RuntimeOrigin::signed(bsp2_account.clone());
                assert_ok!(bsp_sign_up(bsp2_signed.clone(), size * 2));
                let bsp2_id = Providers::get_provider_id(&bsp2_account).unwrap();

                // Issue storage request and have BSP volunteer/confirm
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                // Both BSPs volunteer and confirm storing
                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                assert_ok!(FileSystem::bsp_volunteer(bsp2_signed.clone(), file_key));

                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed.clone(),
                    forest_proof.clone(),
                    BoundedVec::try_from(vec![file_key_with_proof.clone()]).unwrap(),
                ));
                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp2_signed.clone(),
                    forest_proof.clone(),
                    BoundedVec::try_from(vec![file_key_with_proof.clone()]).unwrap(),
                ));

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Verify storage request was deleted
                assert!(
                    StorageRequests::<Test>::get(&file_key).is_none(),
                    "Storage request should be deleted"
                );
                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp2_id, bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // First deletion succeeds
                let forest_proof_delete = CompactProof {
                    encoded_nodes: vec![file_key.as_ref().to_vec()],
                };

                assert_ok!(FileSystem::delete_file_for_incomplete_storage_request(
                    RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                    file_key,
                    Some(bsp_id),
                    forest_proof_delete.clone(),
                ));

                // Verify incomplete storage request was updated
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp2_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Try to delete again - should fail because this BSP no longer stores the file
                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                        file_key,
                        Some(bsp_id), // Same BSP, same file
                        forest_proof_delete,
                    ),
                    Error::<Test>::ProviderNotStoringFile
                );
            });
        }

        #[test]
        fn invalid_forest_proof_verification() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();
                let bsp_account = Keyring::Bob.to_account_id();

                // Setup MSP and bucket
                let (bucket_id, file_key, location, size, fingerprint, msp_id, _value_prop_id) =
                    setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed.clone(), size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Issue storage request and have BSP volunteer/confirm
                assert_ok!(FileSystem::issue_storage_request(
                    RuntimeOrigin::signed(owner.clone()),
                    bucket_id,
                    location.clone(),
                    fingerprint,
                    size,
                    msp_id,
                    PeerIds::<Test>::try_from(vec![]).unwrap(),
                    ReplicationTarget::Basic,
                ));

                assert_ok!(FileSystem::bsp_volunteer(bsp_signed.clone(), file_key));
                let file_key_with_proof = FileKeyWithProof {
                    file_key,
                    proof: CompactProof {
                        encoded_nodes: vec![file_key.as_ref().to_vec()],
                    },
                };
                let forest_proof = CompactProof {
                    encoded_nodes: vec![H256::default().as_ref().to_vec()],
                };

                assert_ok!(FileSystem::bsp_confirm_storing(
                    bsp_signed,
                    forest_proof,
                    BoundedVec::try_from(vec![file_key_with_proof]).unwrap(),
                ));

                // Trigger storage request expiration
                trigger_storage_request_expiration();

                // Verify incomplete storage request was created
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);

                // Try to delete with invalid forest proof
                let invalid_forest_proof = CompactProof {
                    encoded_nodes: vec![H256::from_low_u64_be(99999).as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Ferdie.to_account_id()),
                        file_key,
                        Some(bsp_id),
                        invalid_forest_proof,
                    ),
                    Error::<Test>::ExpectedInclusionProof
                );

                // Verify incomplete storage request was not impacted
                assert!(
                    IncompleteStorageRequests::<Test>::get(&file_key).is_some(),
                    "Incomplete storage request should be created"
                );
                let incomplete_storage_request =
                    IncompleteStorageRequests::<Test>::get(&file_key).unwrap();
                assert_eq!(
                    incomplete_storage_request.pending_bsp_removals,
                    vec![bsp_id]
                );
                assert!(!incomplete_storage_request.pending_bucket_removal);
            });
        }

        #[test]
        fn file_key_mismatch() {
            new_test_ext().execute_with(|| {
                let owner = Keyring::Alice.to_account_id();
                let msp = Keyring::Charlie.to_account_id();

                // Setup MSP and bucket
                let (
                    bucket_id,
                    _correct_file_key,
                    location,
                    size,
                    fingerprint,
                    _msp_id,
                    _value_prop_id,
                ) = setup_file_in_msp_bucket(&owner, &msp);

                // Setup BSP
                let bsp_account = Keyring::Bob.to_account_id();
                let bsp_signed = RuntimeOrigin::signed(bsp_account.clone());
                assert_ok!(bsp_sign_up(bsp_signed, size * 2));
                let bsp_id = Providers::get_provider_id(&bsp_account).unwrap();

                // Insert a fake incomplete storage request under a wrong file_key
                let wrong_file_key = H256::from_low_u64_be(99999);

                IncompleteStorageRequests::<Test>::insert(
                    wrong_file_key,
                    IncompleteStorageRequestMetadata {
                        owner: owner.clone(),
                        bucket_id,
                        location: location.clone(),
                        fingerprint,
                        file_size: size,
                        pending_bsp_removals: BoundedVec::try_from(vec![bsp_id]).unwrap(),
                        pending_bucket_removal: false,
                    },
                );

                // Forest proof uses the wrong key
                let forest_proof = CompactProof {
                    encoded_nodes: vec![wrong_file_key.as_ref().to_vec()],
                };

                assert_noop!(
                    FileSystem::delete_file_for_incomplete_storage_request(
                        RuntimeOrigin::signed(Keyring::Alice.to_account_id()),
                        wrong_file_key,
                        Some(bsp_id),
                        forest_proof,
                    ),
                    Error::<Test>::FileKeyMismatch
                );
            });
        }
    }
}

/// Helper function that registers an account as a Backup Storage Provider
fn bsp_sign_up(
    bsp_signed: RuntimeOrigin,
    storage_amount: StorageDataUnit<Test>,
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
        amount_of_buckets: 0,
        amount_of_value_props: 1,
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
    private: bool,
) -> (BucketIdFor<Test>, Option<CollectionIdFor<Test>>) {
    let bucket_id =
        <Test as file_system::Config>::Providers::derive_bucket_id(&owner, name.clone());

    let origin = RuntimeOrigin::signed(owner.clone());

    // Dispatch a signed extrinsic.
    assert_ok!(FileSystem::create_bucket(
        origin,
        msp_id,
        name.clone(),
        private,
        value_prop_id
    ));

    // Get the collection ID of the bucket.
    let maybe_collection_id = pallet_storage_providers::Buckets::<Test>::get(bucket_id)
        .expect("Bucket should exist in storage")
        .read_access_group_id;

    // Assert bucket was created
    assert_eq!(
        pallet_storage_providers::Buckets::<Test>::get(bucket_id),
        Some(Bucket {
            root: <Test as pallet_storage_providers::pallet::Config>::DefaultMerkleRoot::get(),
            user_id: owner.clone(),
            msp_id: Some(msp_id),
            private,
            read_access_group_id: maybe_collection_id,
            size: 0,
            value_prop_id,
        })
    );

    assert!(<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(&msp_id, &owner));
    assert!(
        <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(&msp_id, &owner).is_some()
    );

    (bucket_id, maybe_collection_id)
}

fn calculate_storage_request_deposit() -> BalanceOf<Test> {
    let number_of_bsps =
        <<Test as crate::Config>::Providers as ReadStorageProvidersInterface>::get_number_of_bsps();
    let number_of_bsps_balance_typed =
        <Test as crate::Config>::ReplicationTargetToBalance::convert(number_of_bsps);
    let storage_request_deposit =
        <<Test as crate::Config>::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
            &<Test as crate::Config>::WeightInfo::bsp_volunteer(),
        )
        .saturating_mul(number_of_bsps_balance_typed)
        .saturating_add(<Test as crate::Config>::BaseStorageRequestCreationDeposit::get());

    storage_request_deposit
}

fn calculate_upfront_amount_to_pay(
    replication_target: ReplicationTarget<Test>,
    size: StorageDataUnit<Test>,
) -> BalanceOf<Test> {
    let replication_target = match replication_target {
        ReplicationTarget::Basic => <Test as crate::Config>::BasicReplicationTarget::get(),
        ReplicationTarget::Standard => <Test as crate::Config>::StandardReplicationTarget::get(),
        ReplicationTarget::HighSecurity => {
            <Test as crate::Config>::HighSecurityReplicationTarget::get()
        }
        ReplicationTarget::SuperHighSecurity => {
            <Test as crate::Config>::SuperHighSecurityReplicationTarget::get()
        }
        ReplicationTarget::UltraHighSecurity => {
            <Test as crate::Config>::UltraHighSecurityReplicationTarget::get()
        }
        ReplicationTarget::Custom(replication_target) => replication_target,
    };
    <<Test as crate::Config>::PaymentStreams as PricePerGigaUnitPerTickInterface>::get_price_per_giga_unit_per_tick()
				.saturating_mul(<Test as crate::Config>::TickNumberToBalance::convert(<Test as crate::Config>::UpfrontTicksToPay::get()))
				.saturating_mul(<Test as crate::Config>::ReplicationTargetToBalance::convert(replication_target))
				.saturating_mul(<Test as crate::Config>::StorageDataUnitToBalance::convert(size))
				.checked_div(shp_constants::GIGAUNIT.into())
				.unwrap_or_default()
}

/// Setup file stored in MSP bucket
fn setup_file_in_msp_bucket(
    owner: &sp_runtime::AccountId32,
    msp_account: &sp_runtime::AccountId32,
) -> (
    BucketIdFor<Test>,
    crate::types::MerkleHash<Test>,
    FileLocation<Test>,
    StorageDataUnit<Test>,
    crate::types::Fingerprint<Test>,
    ProviderIdFor<Test>,
    ValuePropId<Test>,
) {
    let (msp_id, value_prop_id) = add_msp_to_provider_storage(msp_account);
    let bucket_name = BucketNameFor::<Test>::try_from("test-bucket".as_bytes().to_vec()).unwrap();
    let (bucket_id, _) = create_bucket(owner, bucket_name, msp_id, value_prop_id, false);

    // Standard file metadata (reusing existing patterns)
    let location = FileLocation::<Test>::try_from(b"test".to_vec()).unwrap();
    // We need giga units to see significant changes in payment streams
    let size = 4 * (shp_constants::GIGAUNIT as u64);
    let fingerprint = BlakeTwo256::hash(&b"test_content".to_vec());
    let file_key = FileSystem::compute_file_key(
        owner.clone(),
        bucket_id,
        location.clone(),
        size,
        fingerprint,
    )
    .unwrap();

    // Increase bucket size to simulate it storing the file
    assert_ok!(
        <<Test as crate::Config>::Providers as MutateBucketsInterface>::increase_bucket_size(
            &bucket_id, size
        )
    );

    // Increase the used capacity of the MSP
    assert_ok!(<<Test as crate::Config>::Providers as MutateStorageProvidersInterface>::increase_capacity_used(&msp_id, size));

    (
        bucket_id,
        file_key,
        location,
        size,
        fingerprint,
        msp_id,
        value_prop_id,
    )
}

/// Create deletion intention and signature
fn create_file_deletion_signature(
    file_owner: &sp_keyring::sr25519::Keyring,
    file_key: crate::types::MerkleHash<Test>,
) -> (FileOperationIntention<Test>, MultiSignature) {
    let signed_delete_intention = FileOperationIntention::<Test> {
        file_key,
        operation: FileOperation::Delete,
    };

    let signed_delete_intention_encoded = signed_delete_intention.encode();
    let pair = file_owner.pair();
    let signature_bytes = pair.sign(&signed_delete_intention_encoded);
    let signature = MultiSignature::Sr25519(signature_bytes);

    (signed_delete_intention, signature)
}

/// Trigger storage request expiration by advancing time and calling on_idle
fn trigger_storage_request_expiration() {
    let storage_request_ttl: u32 = StorageRequestTtl::<Test>::get();
    let storage_request_ttl: TickNumber<Test> = storage_request_ttl.into();
    let expiration_tick = <<Test as crate::Config>::ProofDealer as shp_traits::ProofsDealerInterface>::get_current_tick() + storage_request_ttl;

    // Roll to expiration + 1 to trigger processing
    roll_to(expiration_tick + 1);
}

use crate::{
    log_bucket_created, log_bucket_deleted, log_bucket_move_requested, log_bucket_privacy_updated,
    log_storage_request_issued, log_storage_request_revoked,
    mock::{
        new_test_ext, PCall, Precompiles, PrecompilesValue, Providers, RuntimeCall, RuntimeOrigin,
        Test, FILE_SYSTEM_PRECOMPILE_ADDRESS,
    },
};

use fp_account::AccountId20 as AccountId;
use frame_support::assert_ok;
use pallet_evm::Call as EvmCall;
use pallet_file_system::{types::BucketNameFor, PendingMoveBucketRequests, StorageRequests};
use pallet_storage_providers::{pallet::Buckets, types::ValueProposition};
use precompile_utils::{prelude::Address, testing::*};
use shp_traits::ReadBucketsInterface;
use sp_core::{Encode, Hasher, H160, H256, U256};
use sp_runtime::{
    traits::{BlakeTwo256, Dispatchable},
    BoundedVec,
};

fn precompiles() -> Precompiles<Test> {
    PrecompilesValue::get()
}

fn evm_call(from: impl Into<H160>, input: Vec<u8>) -> EvmCall<Test> {
    EvmCall::call {
        source: from.into(),
        target: H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
        input,
        value: U256::zero(),
        gas_limit: u64::max_value(),
        max_fee_per_gas: 0.into(),
        max_priority_fee_per_gas: Some(U256::zero()),
        nonce: None,
        access_list: Vec::new(),
    }
}

mod unit_tests {
    use super::*;
    mod create_bucket {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn create_bucket_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP
                    let bob_msp_id = setup_msp(bob);

                    // Get Bob's value proposition ID
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket owned by Alice with Bob as the MSP
                    let bucket_name = b"test-bucket".to_vec();

                    // Calculate the expected ID of the bucket to be created
                    let expected_bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.clone().try_into().unwrap(),
                        );
                    let expected_bucket_id_h256 = H256::from_slice(expected_bucket_id.as_ref());

                    // Call the precompile to create the bucket and verify both functionality and event emission
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .expect_log(log_bucket_created(
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            alice,
                            expected_bucket_id_h256,
                            bob_msp_id,
                        ))
                        .execute_returns(());

                    // Verify that the bucket was actually created in storage
                    assert!(Buckets::<Test>::contains_key(&expected_bucket_id));
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn create_bucket_fails_with_invalid_msp() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Non-existent MSP and value proposition IDs
                    let invalid_msp_id = H256::from([99u8; 32]);
                    let value_prop_id = H256::from([3u8; 32]);

                    // This call will revert with NotAMsp error because the MSP ID is invalid
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: invalid_msp_id,
                                name: b"should-fail".to_vec().into(),
                                private: false,
                                value_prop_id,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("NotAMsp")
                        });
                });
            }

            #[test]
            fn create_bucket_fails_with_too_long_name() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Bucket name that's too long (over 100 bytes)
                    let long_name = vec![b'a'; 101];

                    // This call will revert with "Value is too large for length" error because the bucket name exceeds 100 bytes
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: long_name.into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("Value is too large for length")
                        });
                });
            }
        }
    }

    mod request_move_bucket {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn request_move_bucket_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let charlie = AccountId::from([3u8; 20]);
                    let dave = AccountId::from([4u8; 20]);

                    // Setup Charlie and Dave as MSPs
                    let charlie_msp_id = setup_msp(charlie.clone());
                    let dave_msp_id = setup_msp(dave);

                    // Get Charlie's value proposition ID
                    let value_prop_charlie = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let charlie_value_prop_id = value_prop_charlie.derive_id();

                    // Dave's value proposition ID is the same as Charlie's
                    let dave_value_prop_id = charlie_value_prop_id;

                    // Create a bucket with Charlie as its MSP
                    let bucket_name: BucketNameFor<Test> =
                        b"move-test-bucket".to_vec().try_into().unwrap();
                    assert_ok!(pallet_file_system::Pallet::<Test>::create_bucket(
                        RuntimeOrigin::signed(alice.clone()),
                        charlie_msp_id,
                        bucket_name.clone(),
                        false,
                        charlie_value_prop_id,
                    ));

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name,
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Verify that the bucket is stored by Charlie
                    assert!(
                        pallet_storage_providers::Pallet::<Test>::is_bucket_stored_by_msp(
                            &charlie_msp_id,
                            &bucket_id
                        ),
                        "Bucket should be stored by Charlie"
                    );

                    // Request to move the bucket to Dave using the precompile and verify both functionality and event emission
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::request_move_bucket {
                                bucket_id: bucket_id_h256,
                                new_msp_id: dave_msp_id,
                                new_value_prop_id: H256::from_slice(&dave_value_prop_id.as_ref()),
                            },
                        )
                        .expect_log(log_bucket_move_requested(
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            alice,
                            bucket_id_h256,
                            dave_msp_id,
                        ))
                        .execute_returns(());

                    // Verify that the move request was created
                    let pending_move = PendingMoveBucketRequests::<Test>::get(&bucket_id);
                    assert!(
                        pending_move.is_some(),
                        "Move bucket request should be pending"
                    );

                    // Verify that the move request has the correct metadata
                    let move_metadata = pending_move.unwrap();
                    assert_eq!(move_metadata.requester, alice);
                    assert_eq!(move_metadata.new_msp_id, dave_msp_id);
                    assert_eq!(move_metadata.new_value_prop_id, dave_value_prop_id);
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn request_move_bucket_fails_with_non_existent_bucket() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let charlie = AccountId::from([3u8; 20]);

                    // Setup Charlie as an MSP and get his value proposition ID
                    let charlie_msp_id = setup_msp(charlie);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Non-existent bucket ID
                    let non_existent_bucket_id = H256::from([42u8; 32]);

                    // This call will revert with BucketNotFound error because the bucket does not exist
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::request_move_bucket {
                                bucket_id: non_existent_bucket_id,
                                new_msp_id: charlie_msp_id,
                                new_value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("BucketNotFound")
                                || output_str.contains("does not exist")
                        });
                });
            }

            #[test]
            fn request_move_bucket_fails_with_invalid_new_msp() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"move-fail-test".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Try to move to a non-existent MSP
                    let invalid_msp_id = H256::from([99u8; 32]);

                    // This call will revert with NotAMsp error because the new MSP ID is invalid
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::request_move_bucket {
                                bucket_id: bucket_id_h256,
                                new_msp_id: invalid_msp_id,
                                new_value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("NotAMsp")
                        });
                });
            }
        }
    }

    mod update_bucket_privacy {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn update_bucket_privacy_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a public bucket with Bob as its MSP
                    let bucket_name = b"privacy-test-bucket".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Verify initial state (public) by checking the bucket metadata
                    let bucket_metadata = Buckets::<Test>::get(&bucket_id).unwrap();
                    assert_eq!(bucket_metadata.private, false);

                    // Update the bucket to be private and verify both functionality and event emission
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::update_bucket_privacy {
                                bucket_id: bucket_id_h256,
                                private: true,
                            },
                        )
                        .expect_log(log_bucket_privacy_updated(
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            alice,
                            bucket_id_h256,
                            true,
                        ))
                        .execute_returns(());

                    // Verify privacy was updated by checking the bucket metadata
                    let bucket_metadata = Buckets::<Test>::get(&bucket_id).unwrap();
                    assert_eq!(bucket_metadata.private, true);
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn update_bucket_privacy_fails_with_non_existent_bucket() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Non-existent bucket ID
                    let non_existent_bucket_id = H256::from([42u8; 32]);

                    // This call will revert with BucketNotFound error because the bucket does not exist
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::update_bucket_privacy {
                                bucket_id: non_existent_bucket_id,
                                private: true,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("BucketNotFound")
                                || output_str.contains("does not exist")
                        });
                });
            }
        }
    }

    mod create_and_associate_collection_with_bucket {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn create_and_associate_collection_with_bucket_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"collection-test-bucket".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id = <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                        &alice,
                        bucket_name.try_into().unwrap(),
                    );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Create and associate a NFT collection with the bucket
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_and_associate_collection_with_bucket {
                                bucket_id: bucket_id_h256,
                            },
                        )
                        .execute_returns(());

                    // Verify collection was created and associated
                    let collection_id = <Providers as shp_traits::ReadBucketsInterface>::get_read_access_group_id_of_bucket(&bucket_id);
                    assert!(collection_id.is_ok() && collection_id.unwrap().is_some());
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn create_and_associate_collection_fails_with_non_existent_bucket() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Non-existent bucket ID
                    let non_existent_bucket_id = H256::from([42u8; 32]);

                    // This call will revert with BucketNotFound error because the bucket does not exist
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_and_associate_collection_with_bucket {
                                bucket_id: non_existent_bucket_id,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("BucketNotFound")
                                || output_str.contains("does not exist")
                        });
                });
            }
        }
    }

    mod delete_bucket {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn delete_bucket_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"delete-test-bucket".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Verify bucket exists before deletion
                    assert!(Buckets::<Test>::contains_key(&bucket_id));

                    // Delete the bucket and verify both functionality and event emission
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::delete_bucket {
                                bucket_id: bucket_id_h256,
                            },
                        )
                        .expect_log(log_bucket_deleted(
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            alice,
                            bucket_id_h256,
                        ))
                        .execute_returns(());

                    // Verify that the bucket has been deleted
                    assert!(!Buckets::<Test>::contains_key(&bucket_id));
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn delete_bucket_fails_with_non_existent_bucket() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Non-existent bucket ID
                    let non_existent_bucket_id = H256::from([42u8; 32]);

                    // This call will revert with BucketNotFound error because the bucket does not exist
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::delete_bucket {
                                bucket_id: non_existent_bucket_id,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("BucketNotFound")
                                || output_str.contains("does not exist")
                        });
                });
            }
        }
    }

    mod utils {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn get_pending_file_deletion_requests_count_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Initially should be 0
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::get_pending_file_deletion_requests_count {
                                user_address: Address(alice.into()),
                            },
                        )
                        .execute_returns(0u32);
                });
            }

            #[test]
            fn derive_bucket_id_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bucket_name = b"test-derive-bucket".to_vec();

                    // Calculate the expected bucket ID
                    let expected_bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.clone().try_into().unwrap(),
                        );
                    let expected_bucket_id_h256: H256 =
                        H256::from_slice(expected_bucket_id.as_ref());

                    // Call the precompile to derive the bucket ID
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::derive_bucket_id {
                                owner: Address(alice.into()),
                                name: bucket_name.into(),
                            },
                        )
                        .execute_returns(expected_bucket_id_h256);
                });
            }

            #[test]
            fn test_solidity_interface_has_all_function_selectors_documented_and_implemented() {
                // This test checks that all functions defined in the Solidity interface
                // are implemented in the precompile.
                check_precompile_implements_solidity_interfaces(
                    &["FileSystem.sol"],
                    PCall::supports_selector,
                );
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn derive_bucket_id_fails_with_too_long_name() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Bucket name that's too long (over 100 bytes)
                    let long_name = vec![b'a'; 101];

                    // This call will revert with "Value is too large for length" error because the bucket name exceeds 100 bytes
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::derive_bucket_id {
                                owner: Address(alice.into()),
                                name: long_name.into(),
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("Value is too large for length")
                        });
                });
            }
        }
    }

    mod issue_storage_request {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn issue_storage_request_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"storage-request-test".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Issue a storage request
                    let location = b"test/file.txt".to_vec();
                    let fingerprint = H256::from([3u8; 32]);
                    let size = 1024u64;
                    let peer_ids = vec![b"peer1".to_vec().into()];

                    // Calculate the expected file key for log verification
                    let expected_file_key = pallet_file_system::Pallet::<Test>::compute_file_key(
                        alice.clone(),
                        bucket_id,
                        location.clone().try_into().unwrap(),
                        size.into(),
                        fingerprint.into(),
                    )
                    .unwrap();
                    let expected_file_key_h256: H256 = expected_file_key.into();

                    // Issue storage request and verify both functionality and event emission
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::issue_storage_request {
                                bucket_id: bucket_id_h256,
                                location: location.into(),
                                fingerprint,
                                size,
                                msp_id: bob_msp_id,
                                peer_ids,
                                replication_target: 1, // Standard
                                custom_replication_target: 0,
                            },
                        )
                        .expect_log(log_storage_request_issued(
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            alice,
                            expected_file_key_h256,
                            bucket_id_h256,
                        ))
                        .execute_returns(());

                    // Verify that the storage request was created
                    assert!(StorageRequests::<Test>::contains_key(&expected_file_key));
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn issue_storage_request_fails_with_non_existent_bucket() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);

                    // Non-existent bucket ID
                    let non_existent_bucket_id = H256::from([42u8; 32]);

                    // This call will revert with BucketNotFound error because the bucket does not exist
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::issue_storage_request {
                                bucket_id: non_existent_bucket_id,
                                location: b"test/file.txt".to_vec().into(),
                                fingerprint: H256::from([3u8; 32]),
                                size: 1024u64,
                                msp_id: bob_msp_id,
                                peer_ids: vec![b"peer1".to_vec().into()],
                                replication_target: 1,
                                custom_replication_target: 0,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("BucketNotFound")
                                || output_str.contains("does not exist")
                        });
                });
            }

            #[test]
            fn issue_storage_request_fails_with_invalid_replication_target() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"storage-fail-test".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Use invalid replication target
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::issue_storage_request {
                                bucket_id: bucket_id_h256,
                                location: b"test/file.txt".to_vec().into(),
                                fingerprint: H256::from([3u8; 32]),
                                size: 1024u64,
                                msp_id: bob_msp_id,
                                peer_ids: vec![b"peer1".to_vec().into()],
                                replication_target: 99, // Invalid
                                custom_replication_target: 0,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("Invalid replication target")
                        });
                });
            }

            #[test]
            fn issue_storage_request_fails_with_too_long_location() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"storage-path-test".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Location path that's too long (over 512 bytes)
                    let long_path = vec![b'a'; 513];

                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::issue_storage_request {
                                bucket_id: bucket_id_h256,
                                location: long_path.into(),
                                fingerprint: H256::from([3u8; 32]),
                                size: 1024u64,
                                msp_id: bob_msp_id,
                                peer_ids: vec![b"peer1".to_vec().into()],
                                replication_target: 1,
                                custom_replication_target: 0,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("Value is too large for length")
                        });
                });
            }
        }
    }

    mod revoke_storage_request {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn revoke_storage_request_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"revoke-test-bucket".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Issue a storage request
                    let location = b"test/revoke-file.txt".to_vec();
                    let fingerprint = H256::from([5u8; 32]);
                    let size = 512u64;
                    let peer_ids = vec![b"peer1".to_vec().into()];

                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::issue_storage_request {
                                bucket_id: bucket_id_h256,
                                location: location.clone().into(),
                                fingerprint,
                                size,
                                msp_id: bob_msp_id,
                                peer_ids,
                                replication_target: 0, // Basic
                                custom_replication_target: 0,
                            },
                        )
                        .execute_returns(());

                    // Calculate the file key
                    let file_key = pallet_file_system::Pallet::<Test>::compute_file_key(
                        alice.clone(),
                        bucket_id,
                        location.try_into().unwrap(),
                        size.into(),
                        fingerprint.into(),
                    )
                    .unwrap();
                    let file_key_h256: H256 = file_key.into();

                    // Verify that the storage request exists
                    assert!(StorageRequests::<Test>::contains_key(&file_key));

                    // Revoke the storage request and verify both functionality and event emission
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::revoke_storage_request {
                                file_key: file_key_h256,
                            },
                        )
                        .expect_log(log_storage_request_revoked(
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            file_key_h256,
                        ))
                        .execute_returns(());

                    // Verify that the storage request was revoked
                    assert!(!StorageRequests::<Test>::contains_key(&file_key));
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn revoke_storage_request_fails_with_non_existent_file() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);

                    // Non-existent file key
                    let non_existent_file_key = H256::from([42u8; 32]);

                    // This call will revert with StorageRequestNotFound error because the file key does not exist
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::revoke_storage_request {
                                file_key: non_existent_file_key,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("StorageRequestNotFound")
                                || output_str.contains("does not exist")
                        });
                });
            }
        }
    }

    mod request_delete_file {
        use super::*;
        mod success {
            use super::*;

            #[test]
            fn request_delete_file_works() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"delete-test-bucket".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    // Calculate the file key
                    let location = b"test/delete-file.txt".to_vec();
                    let fingerprint = H256::from([7u8; 32]);
                    let size = 1024u64;
                    let file_key = pallet_file_system::Pallet::<Test>::compute_file_key(
                        alice.clone(),
                        bucket_id,
                        location.clone().try_into().unwrap(),
                        size.into(),
                        fingerprint.into(),
                    )
                    .unwrap();
                    let file_key_h256: H256 = file_key.into();

                    // Create a mock signature (65 bytes for Ethereum signature)
                    let signature_bytes = vec![0u8; 65];

                    // Request file deletion - this will fail with invalid signature in the actual implementation
                    // We test that the precompile correctly validates the signature and reverts with InvalidSignature error
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::request_delete_file {
                                signed_intention: (file_key_h256, 0), // 0 = Delete operation
                                signature: signature_bytes.into(),
                                bucket_id: bucket_id_h256,
                                location: location.into(),
                                size,
                                fingerprint,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("InvalidSignature")
                        });
                });
            }
        }

        mod failure {
            use super::*;

            #[test]
            fn request_delete_file_fails_with_invalid_signature_length() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"delete-sig-test".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    let location = b"test/delete-file.txt".to_vec();
                    let fingerprint = H256::from([7u8; 32]);
                    let size = 1024u64;
                    let file_key = pallet_file_system::Pallet::<Test>::compute_file_key(
                        alice.clone(),
                        bucket_id,
                        location.clone().try_into().unwrap(),
                        size.into(),
                        fingerprint.into(),
                    )
                    .unwrap();
                    let file_key_h256: H256 = file_key.into();

                    // Invalid signature length (not 65 bytes)
                    let invalid_signature = vec![0u8; 64];

                    // This call will revert with "Invalid signature length" error because signature must be exactly 65 bytes
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::request_delete_file {
                                signed_intention: (file_key_h256, 0),
                                signature: invalid_signature.into(),
                                bucket_id: bucket_id_h256,
                                location: location.into(),
                                size,
                                fingerprint,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("Invalid signature length")
                        });
                });
            }

            #[test]
            fn request_delete_file_fails_with_invalid_operation() {
                new_test_ext().execute_with(|| {
                    let alice = AccountId::from([1u8; 20]);
                    let bob = AccountId::from([2u8; 20]);

                    // Setup Bob as an MSP and get his value proposition ID
                    let bob_msp_id = setup_msp(bob);
                    let value_prop = ValueProposition::<Test>::new(
                        1u128.into(),
                        BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                        10000u64.into(),
                    );
                    let value_prop_id = value_prop.derive_id();

                    // Create a bucket with Bob as its MSP
                    let bucket_name = b"delete-op-test".to_vec();
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::create_bucket {
                                msp_id: bob_msp_id,
                                name: bucket_name.clone().into(),
                                private: false,
                                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
                            },
                        )
                        .execute_returns(());

                    // Calculate the created bucket's ID
                    let bucket_id =
                        <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                            &alice,
                            bucket_name.try_into().unwrap(),
                        );
                    let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

                    let location = b"test/delete-file.txt".to_vec();
                    let fingerprint = H256::from([7u8; 32]);
                    let size = 1024u64;
                    let file_key = pallet_file_system::Pallet::<Test>::compute_file_key(
                        alice.clone(),
                        bucket_id,
                        location.clone().try_into().unwrap(),
                        size.into(),
                        fingerprint.into(),
                    )
                    .unwrap();
                    let file_key_h256: H256 = file_key.into();

                    // Create a mock signature (65 bytes for Ethereum signature)
                    let signature_bytes = vec![0u8; 65];

                    // Invalid operation (not 0 for Delete)
                    precompiles()
                        .prepare_test(
                            alice,
                            H160::from_low_u64_be(FILE_SYSTEM_PRECOMPILE_ADDRESS),
                            PCall::request_delete_file {
                                signed_intention: (file_key_h256, 99), // Invalid operation
                                signature: signature_bytes.into(),
                                bucket_id: bucket_id_h256,
                                location: location.into(),
                                size,
                                fingerprint,
                            },
                        )
                        .execute_reverts(|output| {
                            let output_str = String::from_utf8_lossy(output);
                            output_str.contains("Invalid file operation")
                        });
                });
            }
        }
    }
}

mod integration_tests {
    use super::*;

    #[test]
    fn create_bucket_via_evm() {
        new_test_ext().execute_with(|| {
            let alice = AccountId::from([1u8; 20]);
            let bob = AccountId::from([2u8; 20]);

            // Setup Bob as an MSP
            let bob_msp_id = setup_msp(bob);

            // Get Bob's value proposition ID
            let value_prop = ValueProposition::<Test>::new(
                1u128.into(),
                BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                10000u64.into(),
            );
            let value_prop_id = value_prop.derive_id();

            // Construct the function call to create the bucket
            let bucket_name = b"integration-test-bucket".to_vec();
            let input = PCall::create_bucket {
                msp_id: bob_msp_id,
                name: bucket_name.clone().into(),
                private: false,
                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
            }
            .into();

            // Dispatch the EVM call to create the bucket
            assert_ok!(RuntimeCall::Evm(evm_call(alice, input)).dispatch(RuntimeOrigin::root()));

            // Calculate the expected ID of the bucket
            let bucket_id = <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                &alice,
                bucket_name.try_into().unwrap(),
            );

            // Verify that the bucket exists in storage and has the correct metadata
            let bucket_metadata = Buckets::<Test>::get(&bucket_id);
            assert!(bucket_metadata.is_some(), "Bucket should exist in storage");
            let bucket_metadata = bucket_metadata.unwrap();
            assert_eq!(bucket_metadata.user_id, alice);
            assert_eq!(bucket_metadata.msp_id, Some(bob_msp_id));
            assert_eq!(bucket_metadata.private, false);
            assert_eq!(bucket_metadata.value_prop_id, value_prop_id);
        });
    }

    #[test]
    fn create_and_delete_bucket_flow() {
        new_test_ext().execute_with(|| {
            let alice = AccountId::from([1u8; 20]);
            let bob = AccountId::from([2u8; 20]);

            // Setup Bob as an MSP
            let bob_msp_id = setup_msp(bob);

            // Get Bob's value proposition ID
            let value_prop = ValueProposition::<Test>::new(
                1u128.into(),
                BoundedVec::try_from(vec![1u8, 2u8, 3u8]).unwrap(),
                10000u64.into(),
            );
            let value_prop_id = value_prop.derive_id();

            // Create a bucket owned by Alice with Bob as the MSP
            let bucket_name = b"temp-bucket".to_vec();
            let create_input = PCall::create_bucket {
                msp_id: bob_msp_id,
                name: bucket_name.clone().into(),
                private: true,
                value_prop_id: H256::from_slice(&value_prop_id.as_ref()),
            }
            .into();

            assert_ok!(
                RuntimeCall::Evm(evm_call(alice, create_input)).dispatch(RuntimeOrigin::root())
            );

            // Calculate the created bucket's ID
            let bucket_id = <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                &alice,
                bucket_name.try_into().unwrap(),
            );
            let bucket_id_h256: H256 = H256::from_slice(bucket_id.as_ref());

            // Verify that the bucket exists
            assert!(Buckets::<Test>::contains_key(&bucket_id));

            // Construct the function call to delete the bucket
            let delete_input = PCall::delete_bucket {
                bucket_id: bucket_id_h256,
            }
            .into();

            // Dispatch the EVM call to delete the bucket
            assert_ok!(
                RuntimeCall::Evm(evm_call(alice, delete_input)).dispatch(RuntimeOrigin::root())
            );

            // Verify that the bucket has been deleted
            assert!(
                !Buckets::<Test>::contains_key(&bucket_id),
                "Bucket should be deleted"
            );
        });
    }

    #[test]
    fn fail_create_bucket_invalid_msp() {
        new_test_ext().execute_with(|| {
            let alice = AccountId::from([1u8; 20]);

            // Non-existent MSP and value proposition IDs
            let invalid_msp_id = H256::from([99u8; 32]);
            let value_prop_id = H256::from([3u8; 32]);

            // Construct the function call to create the bucket with the invalid MSP
            let bucket_name = b"should-fail".to_vec();
            let input = PCall::create_bucket {
                msp_id: invalid_msp_id,
                name: bucket_name.clone().into(),
                private: false,
                value_prop_id,
            }
            .into();

            // This call should succeed at the dispatch level (EVM executes)
            // but the underlying pallet call should revert
            let result = RuntimeCall::Evm(evm_call(alice, input)).dispatch(RuntimeOrigin::root());
            assert!(result.is_ok(), "EVM call should succeed at dispatch level");

            // Verify that no bucket was created
            let bucket_id = <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                &alice,
                bucket_name.try_into().unwrap(),
            );
            assert!(
                !Buckets::<Test>::contains_key(&bucket_id),
                "No bucket should have been created with invalid MSP"
            );
        });
    }
}

/// Helper function to setup an MSP for testing using force_msp_sign_up
fn setup_msp(account: AccountId) -> H256 {
    // Calculate the MSP ID (which is the blake2 hash of the account)
    let msp_id: H256 = BlakeTwo256::hash(&account.encode()).into();

    // Register as MSP using force (root origin)
    let capacity = 1000u64;

    // Create multiaddresses as BoundedVec
    let multiaddr_bytes = b"/ip4/127.0.0.1/tcp/30333".to_vec();
    let multiaddr_bounded: BoundedVec<u8, _> = multiaddr_bytes.try_into().unwrap();
    let multiaddresses: BoundedVec<BoundedVec<u8, _>, _> =
        vec![multiaddr_bounded].try_into().unwrap();

    let value_prop_price = 1u128; // Price per giga unit of data per block
    let commitment: BoundedVec<u8, _> = vec![1u8, 2u8, 3u8].try_into().unwrap();
    let value_prop_max_data_limit = 10000u64;

    assert_ok!(pallet_storage_providers::Pallet::<Test>::force_msp_sign_up(
        RuntimeOrigin::root(),
        account,
        msp_id.into(),
        capacity.into(),
        multiaddresses,
        value_prop_price.into(),
        commitment,
        value_prop_max_data_limit.into(),
        account, // payment account same as provider account
    ));

    msp_id
}

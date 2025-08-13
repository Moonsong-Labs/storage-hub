use crate::{
    log_bucket_created,
    mock::{
        new_test_ext, PCall, Precompiles, PrecompilesValue, Providers, RuntimeCall, RuntimeOrigin,
        Test, FILE_SYSTEM_PRECOMPILE_ADDRESS,
    },
};

use fp_account::AccountId20 as AccountId;
use frame_support::assert_ok;
use pallet_evm::Call as EvmCall;
use pallet_file_system::PendingMoveBucketRequests;
use pallet_storage_providers::{pallet::Buckets, types::ValueProposition};
use precompile_utils::testing::*;
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

mod unit {

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

            // Try to create a bucket owned by Alice with Bob as the MSP
            let bucket_name = b"test-bucket".to_vec();
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

            // Verify that the bucket was created
            let bucket_id = <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                &alice,
                bucket_name.try_into().unwrap(),
            );
            assert!(Buckets::<Test>::contains_key(&bucket_id));
        });
    }

    #[test]
    fn create_bucket_emits_log() {
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

            let bucket_name = b"test-bucket".to_vec();

            // Calculate the expected ID of the bucket to be created
            let expected_bucket_id =
                <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
                    &alice,
                    bucket_name.clone().try_into().unwrap(),
                );
            let expected_bucket_id_h256 = H256::from_slice(expected_bucket_id.as_ref());

            // Call the precompile to create the bucket and check its event
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
        });
    }

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
            let bucket_name: BoundedVec<u8, _> = b"move-test-bucket".to_vec().try_into().unwrap();
            assert_ok!(pallet_file_system::Pallet::<Test>::create_bucket(
                RuntimeOrigin::signed(alice.clone()),
                charlie_msp_id,
                bucket_name.clone(),
                false,
                charlie_value_prop_id,
            ));

            // Calculate the created bucket's ID
            let bucket_id = <Providers as shp_traits::ReadBucketsInterface>::derive_bucket_id(
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

            // Request to move the bucket to Dave using the precompile
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

mod integration {

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

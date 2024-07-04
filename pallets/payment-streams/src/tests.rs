use crate::{mock::*, types::BalanceOf, Error, Event, RegisteredUsers};

use frame_support::assert_noop;
use frame_support::pallet_prelude::Weight;
use frame_support::traits::fungible::Mutate;
use frame_support::traits::{Get, OnFinalize, OnIdle, OnInitialize};
use frame_support::{assert_ok, BoundedVec};
use shp_traits::PaymentManager;
use shp_traits::PaymentStreamsInterface;
use shp_traits::ProvidersInterface;
use sp_core::H256;
use sp_runtime::traits::Convert;
use sp_runtime::DispatchError;

// `payment-streams` types:
type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;
pub type NewStreamDeposit = <Test as crate::Config>::NewStreamDeposit;
pub type BlockNumberToBalance = <Test as crate::Config>::BlockNumberToBalance;

// `storage-providers` types:
pub type StorageData<Test> = <Test as pallet_storage_providers::Config>::StorageData;
pub type SpMinDeposit = <Test as pallet_storage_providers::Config>::SpMinDeposit;
pub type DepositPerData = <Test as pallet_storage_providers::Config>::DepositPerData;
pub type SpMinCapacity = <Test as pallet_storage_providers::Config>::SpMinCapacity;
pub type MaxMultiAddressAmount<Test> =
    <Test as pallet_storage_providers::Config>::MaxMultiAddressAmount;
use pallet_storage_providers::types::MultiAddress;
pub type ValuePropId = <Test as pallet_storage_providers::Config>::ValuePropId;
use pallet_storage_providers::types::ValueProposition;

/// This module holds all tests for fixed-rate payment streams
mod fixed_rate_streams {
    use super::*;

    /// This module holds the tests for the creation of a payment stream
    mod create_stream {

        use super::*;

        #[test]
        fn create_payment_stream_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // The new balance of Bob should be his original balance minus `rate * NewStreamDeposit` (in this case 100)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();
                // The payment stream should be created with the correct rate
                assert_eq!(payment_stream_info.rate, rate);

                // The payment stream should be created with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );

                // Bob should have 1 payment stream open
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 1);

                // And that payment stream should be the one just created
                assert_eq!(
                    PaymentStreams::get_fixed_rate_payment_streams_of_user(&bob)[0],
                    (alice_msp_id, payment_stream_info)
                );

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::FixedRatePaymentStreamCreated {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        rate,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_stream_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Try to create a payment stream from Bob to Alice of 10 units per block again
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::PaymentStreamAlreadyExists
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_msp_account_has_not_registered_as_msp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to create a payment stream from Bob to a random not registered MSP of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &H256::random(),
                        &bob,
                        rate
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_user_is_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let charlie: AccountId = 2;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_msp_id,
                        System::block_number(),
						100
                    )
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for the 10th block and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Try to create a payment stream from Bob to Charlie of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_user_cant_pay_deposit() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let rate: BalanceOf<Test> = 10;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Transfer almost all of Bob's balance to Alice (Bob keeps `rate * NewStreamDeposit - 1` balance)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                assert_ok!(Balances::transfer(
                    &bob,
                    &alice,
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed + 1,
                    frame_support::traits::tokens::Preservation::Preserve
                ));

                // Try to create a payment stream from Bob to Alice of 10 units per block
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::CannotHoldDeposit
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_user_has_too_many_streams() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Set the amount of payment streams that Bob has to u32::MAX - 1
                RegisteredUsers::<Test>::insert(&bob, u32::MAX - 1);

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    ),
                );

                // Check that Bob's new balance is his initial balance minus the deposit
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_free_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_free_balance);

                // Check how many streams Bob has
                assert_eq!(
                    PaymentStreams::get_payment_streams_count_of_user(&bob),
                    u32::MAX
                );

                // Create a payment stream from Bob to Charlie of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        rate
                    ),
                    DispatchError::Arithmetic(sp_runtime::ArithmeticError::Overflow)
                );

                // Check that Bob still has u32::MAX payment streams open
                assert_eq!(
                    PaymentStreams::get_payment_streams_count_of_user(&bob),
                    u32::MAX
                );

                // Check that Bob has the payment stream from Bob to Alice open
                assert_eq!(
                    PaymentStreams::get_fixed_rate_payment_streams_of_user(&bob)[0],
                    (
                        alice_msp_id,
                        PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                            .unwrap()
                    )
                );

                // Check that the payment stream from Bob to Charlie was not created
                assert!(matches!(
                    PaymentStreams::get_fixed_rate_payment_stream_info(&charlie_msp_id, &bob),
                    Err(Error::<Test>::PaymentStreamNotFound)
                ));

                // Check that the deposit for Charlie's payment stream was not taken from Bob
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_free_balance);
            });
        }
    }

    /// This module holds the tests for updating a payment stream (right now, only the rate can be updated)
    mod update_stream {

        use super::*;

        #[test]
        fn update_payment_stream_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        new_rate
                    )
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::FixedRatePaymentStreamUpdated {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        new_rate,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_new_rate_is_zero() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Try to update the rate of the payment stream from Bob to Alice to 0 units per block
                let new_rate: BalanceOf<Test> = 0;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        new_rate
                    ),
                    Error::<Test>::RateCantBeZero
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_new_rate_is_equal_to_old_rate() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Try to update the rate of the payment stream from Bob to Alice to 10 units per block
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::UpdateRateToSameRate
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Try to update the rate of a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        10
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_msp_account_has_not_registered_as_msp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to update a payment stream from Bob to a random not registered MSP of 10 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &H256::random(),
                        &bob,
                        new_rate
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_user_is_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for the 10th block and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Try to update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        new_rate
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn updated_payment_stream_charges_pending_blocks_with_old_rate() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        new_rate
                    )
                );

                // Check that Bob's deposit was updated AND he was charged 10 blocks at the old 10 units/block rate after the payment stream was updated
                let bob_balance_updated_deposit =
                    bob_new_balance - (new_rate - rate) * new_stream_deposit_blocks_balance_typed;
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_balance_updated_deposit - 10 * rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                /* // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    payment_stream_info.last_chargeable_block,
                    System::block_number()
                ); */

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );
            });
        }
    }

    mod delete_stream {

        use super::*;

        #[test]
        fn delete_payment_stream_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Delete the payment stream from Bob to Alice
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob
                    )
                );

                // The payment stream should be deleted
                assert!(matches!(
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob),
                    Err(Error::<Test>::PaymentStreamNotFound)
                ));

                // Bob should have 0 payment streams open
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Bob should have his initial balance back
                assert_eq!(NativeBalance::free_balance(&bob), bob_initial_balance);

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::FixedRatePaymentStreamDeleted {
                        user_account: bob,
                        provider_id: alice_msp_id,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn delete_payment_stream_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Try to delete a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn delete_payment_stream_fails_if_msp_account_has_not_registered_as_msp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to delete a payment stream from Bob to a random not registered MSP
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                        &H256::random(),
                        &bob
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn delete_payment_stream_fails_if_user_is_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for the 10th block and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Try to delete the payment stream from Bob to Alice
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn delete_payment_stream_charges_pending_blocks() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Delete the payment stream from Bob to Alice
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob
                    )
                );

                // Check that Bob was returned his deposit AND charged 10 blocks at the 10 units/block rate after the payment stream was deleted
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance + rate * (new_stream_deposit_blocks_balance_typed - 10)
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // The payment stream should be deleted
                assert!(matches!(
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob),
                    Err(Error::<Test>::PaymentStreamNotFound)
                ));

                // Bob should have 0 payment streams open
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Bob should have his deposit back (but not the charged amount)
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_initial_balance - 10 * rate
                );
            });
        }
    }

    mod charge_stream {

        use crate::UsersWithoutFunds;

        use super::*;

        #[test]
        fn charge_payment_streams_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the 10 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_updates_last_charged_block_to_last_chargeable_block() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead, with a 10 units/block price index rate
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Advance some blocks so the last valid proof is not the same as the current block
                run_to_block(System::block_number() + 5);

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the 10 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

				// Get Alice's last chargeable information
				let alice_last_chargeable_info = PaymentStreams::get_last_chargeable_info(&alice_msp_id);

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    alice_last_chargeable_info.last_chargeable_block
                );

                // And not with the current block
                assert_ne!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_uses_the_latest_rate() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        new_rate
                    )
                );

                // Check that Bob's deposit has also been updated
                let bob_new_balance =
                    bob_new_balance - (new_rate - rate) * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the 20 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * new_rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * new_rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

				// Get Alice's last chargeable information
				let alice_last_chargeable_info = PaymentStreams::get_last_chargeable_info(&alice_msp_id);

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    alice_last_chargeable_info.last_chargeable_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_works_after_two_charges() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the 10 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 20 blocks ahead
                run_to_block(System::block_number() + 20);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 300)
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 20 blocks at the 10 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * rate - 20 * rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 20 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_streams_fails_if_msp_account_has_not_registered_as_msp() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Try to charge a payment stream from Bob to Alice without registering Alice as a MSP
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn charge_payment_streams_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data
                register_account_as_msp(alice, 100);

                // Try to charge a payment stream that does not exist
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn charge_payment_streams_fails_if_total_amount_would_overflow_balance_type() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Mint Bob enough tokens to pay for the deposit
                let maximum_amount_to_mint = u128::MAX - NativeBalance::total_issuance();
                assert_ok!(NativeBalance::mint_into(&bob, maximum_amount_to_mint));
                let bob_new_balance = NativeBalance::free_balance(&bob);
                assert_eq!(
                    bob_new_balance,
                    bob_initial_balance + maximum_amount_to_mint
                );

                // Create a payment stream from Bob to Alice of bob_new_balance / (NewStreamDeposit + 1) units per block (because we hold `rate * NewStreamDeposit`)
                let rate: BalanceOf<Test> = bob_new_balance / 11;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_balance_after_deposit =
                    bob_new_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_balance_after_deposit);

                // Set the last valid proof of the payment stream from Bob to Alice to 1000 blocks ahead
                run_to_block(System::block_number() + 1000);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Try to charge the payment stream from Bob to Alice
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::ChargeOverflow
                );

                // Check that Bob's balance has not changed
                assert_eq!(NativeBalance::free_balance(&bob), bob_balance_after_deposit);
            });
        }

        #[test]
        fn charge_payment_streams_correctly_flags_user_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Check that no funds where charged from Bob's account
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 0,
                    }
                    .into(),
                );

                // Try to create a new stream from Charlie to Bob
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn charge_payment_streams_unflags_user_if_it_now_has_enough_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data
                register_account_as_msp(charlie, 1000);

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let bob_new_balance =
                    bob_initial_balance - rate * new_stream_deposit_blocks_balance_typed;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Check that no funds where charged from Bob's account
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 0,
                    }
                    .into(),
                );

                // Deposit enough funds to Bob's account
                let deposit_amount = rate * 10;
                assert_ok!(NativeBalance::mint_into(&bob, deposit_amount));

                // Try to charge the payment stream again
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the (bob_initial_balance / 20 + 1) units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance + deposit_amount - 10 * rate
                );
                System::assert_last_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Check that Bob is no longer flagged as a user without funds (TODO: we should have an event about this if we leave this behavior in prod)
                assert!(!UsersWithoutFunds::<Test>::contains_key(bob));
            });
        }
    }
    mod update_last_chargeable_block {

        use super::*;

        #[test]
        fn update_last_chargeable_block_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

				// Get Alice's last chargeable information
				let alice_last_chargeable_info = PaymentStreams::get_last_chargeable_info(&alice_msp_id);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    alice_last_chargeable_info.last_chargeable_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn update_last_chargeable_block_fails_if_msp_account_has_not_registered_as_msp() {
            ExtBuilder::build().execute_with(|| {
                // Try to update the last valid proof of a payment stream from Bob to a random not registered MSP
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &H256::random(),
                        System::block_number(),
                        100
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn update_last_chargeable_block_fails_if_trying_to_set_greater_than_current_block_number() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Try to set the last valid proof of the payment stream from Bob to Alice to a block number greater than the current block number
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_msp_id,
                        System::block_number() + 1,
						100
                    ),
                    Error::<Test>::InvalidLastChargeableBlockNumber
                );
            });
        }

        #[test]
        fn update_last_chargeable_block_fails_if_trying_to_set_less_than_previous_value() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(&alice_msp_id, System::block_number(), 100)
                );

                // Try to set the last valid proof of the payment stream from Bob to Alice to a block number less than the last valid proof block number already set
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_msp_id,
                        System::block_number() - 1,
						100
                    ),
                    Error::<Test>::InvalidLastChargeableBlockNumber
                );
            });
        }
    }
}

/// This module holds all tests for dynamic-rate payment streams
mod dynamic_rate_streams {

    use super::*;

    /// This module holds the tests for the creation of a payment stream
    mod create_stream {

        use super::*;

        #[test]
        fn create_payment_stream_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // The new balance of Bob should be his original balance minus `current_price * amount_provided * NewStreamDeposit` (in this case 10 * 100 * 10 = 10000)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_initial_balance - deposit_amount
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();
                // The payment stream should be created with the correct amount provided
                assert_eq!(payment_stream_info.amount_provided, amount_provided);

                // The payment stream should be created with the correct price index when last charged (which should be the current one)
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );

                /*                 // The payment stream should be created with the correct price index at the last chargeable block (which should be the current one)
                assert_eq!(
                    payment_stream_info.price_index_at_last_chargeable_block,
                    current_price_index
                ); */

                // The payment stream should correctly track the user's deposit
                assert_eq!(payment_stream_info.user_deposit, deposit_amount);

                // Bob should have 1 payment stream open
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 1);

                // And that payment stream should be the one just created
                assert_eq!(
                    PaymentStreams::get_dynamic_rate_payment_streams_of_user(&bob)[0],
                    (alice_bsp_id, payment_stream_info)
                );

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::DynamicRatePaymentStreamCreated {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount_provided,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_stream_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Try to create a payment stream from Bob to Alice of 100 units provided again
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                    Error::<Test>::PaymentStreamAlreadyExists
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Try to create a payment stream from Bob to a random not registered BSP of 100 units provided
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &H256::random(),
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_user_is_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let charlie: AccountId = 2;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Create a payment stream from Bob to Alice of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // The new balance of Bob should be his original balance minus `current_price * amount_provided * NewStreamDeposit` (in this case 10 * 100 * 10 = 10000)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index to something that will make Bob run out of funds
                let current_price_index =
                    current_price_index + bob_new_balance / (amount_provided as u128) + 1;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for storage and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Try to create a payment stream from Bob to Charlie of 100 units provided
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_user_cant_pay_deposit() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Transfer almost all of Bob's balance to Alice (Bob keeps `deposit_amount - 1` balance)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                assert_ok!(Balances::transfer(
                    &bob,
                    &alice,
                    bob_initial_balance - deposit_amount + 1,
                    frame_support::traits::tokens::Preservation::Preserve
                ));

                // Try to create a payment stream from Bob to Alice of 100 units provided
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                    Error::<Test>::CannotHoldDeposit
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_user_has_too_many_streams() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Set the amount of payment streams that Bob has to u32::MAX - 1
                RegisteredUsers::<Test>::insert(&bob, u32::MAX - 1);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                );

                // Check that Bob's new balance is his initial balance minus the deposit
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_free_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_free_balance);

                // Check how many streams Bob has
                assert_eq!(
                    PaymentStreams::get_payment_streams_count_of_user(&bob),
                    u32::MAX
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                    DispatchError::Arithmetic(sp_runtime::ArithmeticError::Overflow)
                );

                // Check that Bob still has u32::MAX payment streams open
                assert_eq!(
                    PaymentStreams::get_payment_streams_count_of_user(&bob),
                    u32::MAX
                );

                // Check that Bob has the payment stream from Bob to Alice open
                assert_eq!(
                    PaymentStreams::get_dynamic_rate_payment_streams_of_user(&bob)[0],
                    (
                        alice_bsp_id,
                        PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                            .unwrap()
                    )
                );

                // Check that the payment stream from Bob to Charlie was not created
                assert!(matches!(
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&charlie_bsp_id, &bob),
                    Err(Error::<Test>::PaymentStreamNotFound)
                ));

                // Check that the deposit for Charlie's payment stream was not taken from Bob
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_free_balance);
            });
        }
    }

    /// This module holds the tests for updating a payment stream (right now, only the rate can be updated)
    mod update_stream {

        use super::*;

        #[test]
        fn update_payment_stream_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                        current_price,
                    )
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct amount_provided
                assert_eq!(payment_stream_info.amount_provided, new_amount_provided);

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::DynamicRatePaymentStreamUpdated {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        new_amount_provided,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_new_amount_is_zero() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Try to update the amount provided of the payment stream from Bob to Alice to 0 units
                let new_amount_provided = 0;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                        current_price
                    ),
                    Error::<Test>::AmountProvidedCantBeZero
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_new_amount_is_equal_to_old_amount() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Try to update the amount provided of the payment stream from Bob to Alice to 100 units per block (the same as the original amount)
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price
                    ),
                    Error::<Test>::UpdateAmountToSameAmount
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Try to update the rate of a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;
                let current_price = 10;

                // Try to update a payment stream from Bob to a random not registered BSP to 200 units per block
                let new_amount_provided = 200;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &H256::random(),
                        &bob,
                        &new_amount_provided,
                        current_price
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_user_is_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // The new balance of Bob should be his original balance minus `current_price * amount_provided * NewStreamDeposit` (in this case 10 * 100 * 10 = 10000)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index to something that will make Bob run out of funds
                let current_price_index =
                    current_price_index + bob_new_balance / (amount_provided as u128) + 1;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for his storage and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Try to update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                        current_price
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn updated_payment_stream_charges_pending_blocks_with_old_amount() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                        current_price
                    )
                );

                // Check that Bob's deposit was updated AND he was charged 10 blocks (at current price) at the old 100 units provided after the payment stream was updated
                let bob_balance_updated_deposit = bob_new_balance
                    - current_price
                        * u128::from(new_amount_provided - amount_provided)
                        * new_stream_deposit_blocks_balance_typed;
                let paid_for_storage = 10 * current_price * u128::from(amount_provided);
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_balance_updated_deposit - paid_for_storage
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: paid_for_storage,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct amount provided
                assert_eq!(payment_stream_info.amount_provided, new_amount_provided);

                /* // The payment stream should be updated with the correct price index at the last chargeable block
                assert_eq!(
                    payment_stream_info.price_index_at_last_chargeable_block,
                    current_price_index
                ); */

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );
            });
        }
    }

    mod delete_stream {

        use super::*;

        #[test]
        fn delete_payment_stream_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Delete the payment stream from Bob to Alice
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob
                    )
                );

                // The payment stream should be deleted
                assert!(matches!(
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob),
                    Err(Error::<Test>::PaymentStreamNotFound)
                ));

                // Bob should have 0 payment streams open
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Bob should have his initial balance back
                assert_eq!(NativeBalance::free_balance(&bob), bob_initial_balance);

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::DynamicRatePaymentStreamDeleted {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn delete_payment_stream_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Try to delete a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn delete_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to delete a payment stream from Bob to a random not registered BSP
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                        &H256::random(),
                        &bob
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn delete_payment_stream_fails_if_user_is_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // The new balance of Bob should be his original balance minus `current_price * amount_provided * NewStreamDeposit` (in this case 10 * 100 * 10 = 10000)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to something that will make Bob run out of funds
                let current_price_index =
                    current_price_index + bob_new_balance / (amount_provided as u128) + 1;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for his storage and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Try to delete the payment stream from Bob to Alice
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn delete_payment_stream_charges_pending_blocks() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                let amount_to_pay_for_storage = 10 * current_price * (amount_provided as u128);
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Delete the payment stream from Bob to Alice
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob
                    )
                );

                // Check that Bob was returned his deposit AND charged 10 blocks at the current price considering the amount provided before the payment stream was deleted
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance + deposit_amount - amount_to_pay_for_storage
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // The payment stream should be deleted
                assert!(matches!(
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob),
                    Err(Error::<Test>::PaymentStreamNotFound)
                ));

                // Bob should have 0 payment streams open
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Bob should have his deposit back (but not the charged amount)
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_initial_balance - amount_to_pay_for_storage
                );
            });
        }
    }

    mod charge_stream {

        use crate::UsersWithoutFunds;

        use super::*;

        #[test]
        fn charge_payment_streams_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                let amount_to_pay_for_storage = 10 * current_price * (amount_provided as u128);
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the current price with the correct amount provided
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - amount_to_pay_for_storage
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_updates_last_charged_price_index_to_last_chargeable_one(
        ) {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                let amount_to_pay_for_storage = 10 * current_price * (amount_provided as u128);
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the current price with the correct amount provided
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - amount_to_pay_for_storage
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

				// Get Alice's last chargeable info
				let alice_last_chargeable_info = PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    alice_last_chargeable_info.price_index
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_uses_the_latest_amount() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                        current_price
                    )
                );

                // Check that Bob's deposit has also been updated
                let new_deposit_amount = current_price
                    * (new_amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_new_balance - (new_deposit_amount - deposit_amount);
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                let amount_to_pay_for_storage = 10 * current_price * (new_amount_provided as u128);
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the current price with the new amount provided
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - amount_to_pay_for_storage
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

				// Get Alice's last chargeable info
				let alice_last_chargeable_info = PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );

                // The payment stream should be updated with the correct amount provided
                assert_eq!(payment_stream_info.amount_provided, new_amount_provided);

                // The payment stream should be updated with the correct last chargeable price index
                assert_eq!(
                    alice_last_chargeable_info.price_index,
                    current_price_index
                );
            });
        }

        #[test]
        fn charge_payment_works_after_two_charges() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                let amount_to_pay_for_storage = 10 * current_price * (amount_provided as u128);
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the current price with the correct amount provided
                let bob_new_balance = bob_new_balance - amount_to_pay_for_storage;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 20 blocks ahead
                let current_price_index = current_price_index + 20 * current_price;
                let amount_to_pay_for_storage = 20 * current_price * (amount_provided as u128);
                run_to_block(System::block_number() + 20);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 20 blocks at the current price with the correct amount provided
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - amount_to_pay_for_storage
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );
            });
        }

        #[test]
        fn charge_payment_streams_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Try to charge a payment stream from Bob to Alice without registering Alice as a BSP
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn charge_payment_streams_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data
                register_account_as_bsp(alice, 100);

                // Try to charge a payment stream that does not exist
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn charge_payment_streams_fails_if_total_amount_would_overflow_balance_type() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 100;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_balance_after_deposit = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_balance_after_deposit);

                // Set the last chargeable price index of the payment stream from Bob to Alice to something that would overflow the balance type
                let current_price_index =
                    current_price_index + u128::MAX / (amount_provided as u128) + 1;
                run_to_block(System::block_number() + 1000);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to charge the payment stream from Bob to Alice
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::ChargeOverflow
                );

                // Check that Bob's balance has not changed
                assert_eq!(NativeBalance::free_balance(&bob), bob_balance_after_deposit);
            });
        }

        #[test]
        fn charge_payment_streams_correctly_flags_user_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to something that will make Bob run out of funds
                let current_price_index =
                    current_price_index + bob_new_balance / (amount_provided as u128) + 1;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Check that no funds where charged from Bob's account
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                    }
                    .into(),
                );

                // Try to create a new stream from Charlie to Bob
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn charge_payment_streams_unflags_user_if_it_now_has_enough_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data
                register_account_as_bsp(charlie, 1000);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to something that will make Bob run out of funds
                let current_price_index =
                    current_price_index + bob_new_balance / (amount_provided as u128) + 1;
                let amount_to_pay_for_storage =
                    (amount_provided as u128) * (bob_new_balance / (amount_provided as u128) + 1);
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

                // Check that no funds where charged from Bob's account
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                    }
                    .into(),
                );

                // Mint enough funds to Bob's account
                let mint_amount = 10 * current_price * (amount_provided as u128);
                assert_ok!(NativeBalance::mint_into(&bob, mint_amount));

                // Try to charge the payment stream again
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged for the storage
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance + mint_amount - amount_to_pay_for_storage
                );
                System::assert_last_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                    }
                    .into(),
                );

                // Check that Bob is no longer flagged as a user without funds (TODO: we should have an event about this if we leave this behavior in prod)
                assert!(!UsersWithoutFunds::<Test>::contains_key(bob));
            });
        }
    }
    mod update_chargeable_price_index {

        use super::*;

        #[test]
        fn update_chargeable_price_index_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );
				// Get Alice's last chargeable info
				let alice_last_chargeable_info = PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last chargeable price index
                assert_eq!(
                    alice_last_chargeable_info.price_index,
                    current_price_index
                );
            });
        }

        #[test]
        fn update_chargeable_price_index_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                // Try to update the last valid proof of a payment stream from Bob to a random not registered BSP
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &H256::random(),
						System::block_number(),
                        10000
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn update_chargeable_price_index_fails_if_trying_to_set_less_than_previous_value() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                        current_price,
                        current_price_index
                    )
                );

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                let current_price_index = current_price_index + 10 * current_price;
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index
                    )
                );

                // Try to set the last chargeable price index of the payment stream from Bob to Alice to a price index smaller than the one just set
				run_to_block(System::block_number() + 1);
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block_and_price_index(
                        &alice_bsp_id,
						System::block_number(),
                        current_price_index - 1
                    ),
                    Error::<Test>::InvalidLastChargeablePriceIndex
                );
            });
        }
    }
}

/// Helper function that registers an account as a Backup Storage Provider, with storage_amount StorageData unit
fn register_account_as_bsp(account: AccountId, storage_amount: StorageData<Test>) {
    // Initialize variables:
    let mut multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
        BoundedVec::new();
    multiaddresses.force_push(
        "/ip4/127.0.0.1/udp/1234"
            .as_bytes()
            .to_vec()
            .try_into()
            .unwrap(),
    );

    // Get the deposit amount for the storage amount
    // The deposit for any amount of storage is be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
    let deposit_for_storage_amount: BalanceOf<Test> = <SpMinDeposit as Get<u128>>::get()
        .saturating_add(
            <DepositPerData as Get<u128>>::get()
                .saturating_mul((storage_amount - <SpMinCapacity as Get<u32>>::get()).into()),
        );

    // Check the balance of the account to make sure it has more than the deposit amount needed
    assert!(NativeBalance::free_balance(&account) >= deposit_for_storage_amount);

    // Request to sign up the account as a Backup Storage Provider
    assert_ok!(StorageProviders::request_bsp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        account
    ));

    // Advance enough blocks for randomness to be valid
    run_to_block(frame_system::Pallet::<Test>::block_number() + 4);

    // Confirm the sign up of the account as a Backup Storage Provider
    assert_ok!(StorageProviders::confirm_sign_up(
        RuntimeOrigin::signed(account),
        Some(account)
    ));
}

/// Helper function that registers an account as a Main Storage Provider, with storage_amount StorageData units
fn register_account_as_msp(account: AccountId, storage_amount: StorageData<Test>) {
    // Initialize variables:
    let mut multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
        BoundedVec::new();
    multiaddresses.force_push(
        "/ip4/127.0.0.1/udp/1234"
            .as_bytes()
            .to_vec()
            .try_into()
            .unwrap(),
    );
    let value_prop: ValueProposition<Test> = ValueProposition {
        identifier: ValuePropId::default(),
        data_limit: 10,
        protocols: BoundedVec::new(),
    };

    // Get the deposit amount for the storage amount
    // The deposit for any amount of storage is be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
    let deposit_for_storage_amount: BalanceOf<Test> = <SpMinDeposit as Get<u128>>::get()
        .saturating_add(
            <DepositPerData as Get<u128>>::get()
                .saturating_mul((storage_amount - <SpMinCapacity as Get<u32>>::get()).into()),
        );

    // Check the balance of the account to make sure it has more than the deposit amount needed
    assert!(NativeBalance::free_balance(&account) >= deposit_for_storage_amount);

    // Request to sign up the account as a Main Storage Provider
    assert_ok!(StorageProviders::request_msp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        value_prop,
        account
    ));

    // Advance enough blocks for randomness to be valid
    run_to_block(frame_system::Pallet::<Test>::block_number() + 4);

    // Confirm the sign up of the account as a Main Storage Provider
    assert_ok!(StorageProviders::confirm_sign_up(
        RuntimeOrigin::signed(account),
        Some(account)
    ));
}

/// Helper function that advances the blockchain until block n, executing the hooks for each block
fn run_to_block(n: u64) {
    assert!(n > System::block_number(), "Cannot go back in time");

    while System::block_number() < n {
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    }
}

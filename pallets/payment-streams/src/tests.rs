use crate::{mock::*, types::BalanceOf, Error, Event, RegisteredUsers};

use frame_support::assert_noop;
use frame_support::pallet_prelude::Weight;
use frame_support::traits::fungible::Mutate;
use frame_support::traits::{Get, OnFinalize, OnIdle, OnInitialize};
use frame_support::{assert_ok, BoundedVec};
use sp_core::H256;
use sp_runtime::traits::Convert;
use sp_runtime::DispatchError;
use storage_hub_traits::PaymentManager;
use storage_hub_traits::PaymentStreamsInterface;
use storage_hub_traits::ProvidersInterface;

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
pub type MaxMultiAddressSize<Test> =
    <Test as pallet_storage_providers::Config>::MaxMultiAddressSize;
pub type MultiAddress<T> = BoundedVec<u8, MaxMultiAddressSize<T>>;

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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();
                // The payment stream should be created with the correct rate
                assert_eq!(payment_stream_info.rate, rate);

                // The payment stream should be created with the correct last valid proof
                assert_eq!(
                    payment_stream_info.last_chargeable_block,
                    System::block_number()
                );

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
                    (alice_bsp_id, payment_stream_info)
                );

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::FixedRatePaymentStreamCreated {
                        user_account: bob,
                        provider_id: alice_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Try to create a payment stream from Bob to Alice of 10 units per block again
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::PaymentStreamAlreadyExists
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to create a payment stream from Bob to a random not registered BSP of 10 units per block
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 10` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 10; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
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
                        &charlie_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
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
                        &alice_bsp_id,
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

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    ),
                );

                // Check how many streams Bob has
                assert_eq!(
                    PaymentStreams::get_payment_streams_count_of_user(&bob),
                    u32::MAX
                );

                // Create a payment stream from Bob to Charlie of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        rate
                    ),
                    DispatchError::Arithmetic(sp_runtime::ArithmeticError::Overflow)
                );
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        new_rate
                    )
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The event should be emitted
                System::assert_last_event(
                    Event::<Test>::FixedRatePaymentStreamUpdated {
                        user_account: bob,
                        provider_id: alice_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Try to update the rate of the payment stream from Bob to Alice to 0 units per block
                let new_rate: BalanceOf<Test> = 0;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Try to update the rate of the payment stream from Bob to Alice to 10 units per block
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Try to update the rate of a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        10
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to update a payment stream from Bob to a random not registered BSP of 10 units per block
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 10` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 10; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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
                        &alice_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
                );

                // Update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        new_rate
                    )
                );

                // Check that Bob was charged 10 blocks at the old 10 units/block rate after the payment stream was updated
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * rate
                );
                System::assert_has_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    payment_stream_info.last_chargeable_block,
                    System::block_number()
                );

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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                        &alice_bsp_id,
                        &bob
                    )
                );

                // The payment stream should be deleted
                assert!(matches!(
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob),
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
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 10` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 10; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
                );

                // Delete the payment stream from Bob to Alice
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::delete_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                        provider_id: alice_bsp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // The payment stream should be deleted
                assert!(matches!(
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob),
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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
                        provider_id: alice_bsp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_updates_last_charged_proof_to_last_valid_proof() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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
                        provider_id: alice_bsp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    payment_stream_info.last_chargeable_block
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                        &alice_bsp_id,
                        &bob,
                        new_rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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
                        provider_id: alice_bsp_id,
                        amount: 10 * new_rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    payment_stream_info.last_chargeable_block,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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
                        provider_id: alice_bsp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 20 blocks ahead
                run_to_block(System::block_number() + 20);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
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
                        provider_id: alice_bsp_id,
                        amount: 20 * rate,
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_block,
                    System::block_number()
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of u128::MAX - 1 units per block
                let rate: BalanceOf<Test> = u128::MAX - 1;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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

                // Set the last valid proof of the payment stream from Bob to Alice to 2 blocks ahead
                run_to_block(System::block_number() + 2);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
                );

                // Try to charge the payment stream from Bob to Alice
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::ChargeOverflow
                );

                // Check that Bob's balance has not changed
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
            });
        }

        #[test]
        fn charge_payment_streams_correctly_flags_user_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(charlie).unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 10` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 10; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
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
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_bsp_id,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data
                register_account_as_bsp(charlie, 1000);

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 10` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 10; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
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
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
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

                // Deposit enough funds to Bob's account
                let deposit_amount = rate * 5;
                assert_ok!(NativeBalance::mint_into(&bob, deposit_amount));

                // Try to charge the payment stream again
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 10 blocks at the (bob_initial_balance / 10) units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance + deposit_amount - 10 * rate
                );
                System::assert_last_event(
                    Event::<Test>::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 10 * rate,
                    }
                    .into(),
                );

                // Check that Bob is no longer flagged as a user without funds (TODO: we should have an event about this if we leave this behavior in prod)
                assert!(!UsersWithoutFunds::<Test>::contains_key(bob));
            });
        }
    }
    mod update_last_valid_proof {

        use super::*;

        #[test]
        fn update_last_valid_proof_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    payment_stream_info.last_chargeable_block,
                    System::block_number()
                );
            });
        }

        #[test]
        fn update_last_valid_proof_fails_if_stream_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Try to update the last valid proof of a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn update_last_valid_proof_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to update the last valid proof of a payment stream from Bob to a random not registered BSP
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &H256::random(),
                        &bob,
                        System::block_number()
                    ),
                    Error::<Test>::NotAProvider
                );
            });
        }

        #[test]
        fn update_last_valid_proof_fails_if_trying_to_set_greater_than_current_block_number() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Try to set the last valid proof of the payment stream from Bob to Alice to a block number greater than the current block number
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number() + 1
                    ),
                    Error::<Test>::InvalidLastChargeableBlockNumber
                );
            });
        }

        #[test]
        fn update_last_valid_proof_fails_if_trying_to_set_less_than_previous_value() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ProvidersInterface>::get_provider_id(alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                assert_ok!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number()
                    )
                );

                // Try to set the last valid proof of the payment stream from Bob to Alice to a block number less than the last valid proof block number already set
                assert_noop!(
                    <PaymentStreams as PaymentManager>::update_last_chargeable_block(
                        &alice_bsp_id,
                        &bob,
                        System::block_number() - 1
                    ),
                    Error::<Test>::InvalidLastChargeableBlockNumber
                );
            });
        }
    }
}

/// Helper function that registers an account as a Backup Storage Provider, with storage_amount StorageData units
///
/// Returns the deposit amount that was utilized from the account's balance and the BSP information
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

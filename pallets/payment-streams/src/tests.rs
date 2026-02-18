use crate::{
    mock::*,
    types::{BalanceOf, ProviderLastChargeableInfo},
    AccumulatedPriceIndex, CurrentPricePerGigaUnitPerTick, DynamicRatePaymentStreams, Error, Event,
    LastChargeableInfo, RegisteredUsers, UsersWithoutFunds,
};

use frame_support::{
    assert_noop, assert_ok,
    pallet_prelude::Weight,
    traits::{
        fungible::{InspectHold, Mutate},
        Get, Hooks, OnFinalize, OnIdle, OnInitialize,
    },
    weights::WeightMeter,
    BoundedVec,
};
use pallet_storage_providers::types::StorageProviderId;
use shp_constants::GIGAUNIT;
use shp_traits::{PaymentStreamsInterface, ReadProvidersInterface};
use sp_core::H256;
use sp_runtime::{bounded_vec, traits::Convert, DispatchError};

// `payment-streams` types:
type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;
pub type NewStreamDeposit = <Test as crate::Config>::NewStreamDeposit;
pub type BaseDeposit = <Test as crate::Config>::BaseDeposit;
pub type UserWithoutFundsCooldown = <Test as crate::Config>::UserWithoutFundsCooldown;
pub type BlockNumberToBalance = <Test as crate::Config>::BlockNumberToBalance;

// `storage-providers` types:
pub type StorageData<Test> = <Test as pallet_storage_providers::Config>::StorageDataUnit;
pub type SpMinDeposit = <Test as pallet_storage_providers::Config>::SpMinDeposit;
pub type DepositPerData = <Test as pallet_storage_providers::Config>::DepositPerData;
pub type SpMinCapacity = <Test as pallet_storage_providers::Config>::SpMinCapacity;
pub type MaxMultiAddressAmount<Test> =
    <Test as pallet_storage_providers::Config>::MaxMultiAddressAmount;
use pallet_storage_providers::types::MultiAddress;

const GIGAUNIT_BALANCE: u128 = GIGAUNIT as u128;

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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_initial_balance
                        - rate * new_stream_deposit_blocks_balance_typed
                        - base_deposit
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();
                // The payment stream should be created with the correct rate
                assert_eq!(payment_stream_info.rate, rate);

                // The payment stream should be created with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
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
                    Event::FixedRatePaymentStreamCreated {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        rate,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_provider_is_insolvent() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(alice_msp_id),
                    (),
                );

                // Try to create a payment stream from Bob to Alice of 10 units per block again
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        10
                    ),
                    Error::<Test>::ProviderInsolvent
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Create a payment stream from Bob to Charlie of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: 100,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and the payment stream will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Try to create a payment stream from Bob to Alice of 10 units per block (since the original stream should have been deleted)
                let rate: BalanceOf<Test> = 10;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_free_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                let deposit = rate * <NewStreamDeposit as Get<u64>>::get() as u128
                    + <BaseDeposit as Get<BalanceOf<Test>>>::get();
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Check that the deposit is correct
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();
                assert_eq!(payment_stream_info.user_deposit, deposit);

                // Update the rate of the payment stream from Bob to Alice to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                let new_deposit = new_rate * <NewStreamDeposit as Get<u64>>::get() as u128
                    + <BaseDeposit as Get<BalanceOf<Test>>>::get();
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

                // The payment stream should be updated with the correct rate and deposit
                assert_eq!(payment_stream_info.rate, new_rate);
                assert_eq!(payment_stream_info.user_deposit, new_deposit);

                // The event should be emitted
                System::assert_last_event(
                    Event::FixedRatePaymentStreamUpdated {
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Create a payment stream from Bob to Charlie of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: 100,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and the payment stream will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Try to update the rate of the payment stream from Bob to Charlie to 20 units per block
                let new_rate: BalanceOf<Test> = 20;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_fixed_rate_payment_stream(
                        &charlie_msp_id,
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
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
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );
            });
        }

        #[test]
        fn updated_payment_stream_works_with_insolvent_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(alice_msp_id),
                    (),
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct rate and last charged tick
                assert_eq!(payment_stream_info.rate, new_rate);
                assert_eq!(
                    payment_stream_info.last_charged_tick,
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
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
                    Event::FixedRatePaymentStreamDeleted {
                        user_account: bob,
                        provider_id: alice_msp_id,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn delete_payment_stream_works_with_insolvent_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(alice_msp_id),
                    (),
                );

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
                    Event::FixedRatePaymentStreamDeleted {
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
        fn delete_payment_stream_charges_pending_blocks() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    bob_new_balance
                        + rate * (new_stream_deposit_blocks_balance_typed - 10)
                        + base_deposit
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
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

        #[test]
        fn delete_payment_stream_charges_pending_blocks_with_insolvent_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(alice_msp_id),
                    (),
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    bob_new_balance
                        + rate * (new_stream_deposit_blocks_balance_typed - 10)
                        + base_deposit
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_streams_with_awaited_top_up_from_provider_fails() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate awaited top up from provider 5 blocks before the last chargeable tick
                run_to_block(System::block_number() + 5);
                pallet_storage_providers::AwaitingTopUpFromProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(alice_msp_id),
                    pallet_storage_providers::types::TopUpMetadata {
                        started_at: System::block_number(),
                        end_tick_grace_period: System::block_number() + 10,
                    },
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 5 blocks ahead
                run_to_block(System::block_number() + 5);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::ProviderInsolvent
                );
            });
        }

        #[test]
        fn charge_payment_streams_with_insolvent_provider_no_charge_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::MainStorageProvider(alice_msp_id),
                    (),
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::ProviderInsolvent
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_updates_last_charged_tick_to_last_chargeable_tick() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead, with a 10 units/block price index rate
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Advance some blocks so the last valid proof is not the same as the current block
                run_to_block(System::block_number() + 5);

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that Bob was charged 15 blocks at the 15 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 15 * rate
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 15 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // Get Alice's last chargeable information
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info_with_privilege(&alice_msp_id);

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    alice_last_chargeable_info.last_chargeable_tick
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
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
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * new_rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // Get Alice's last chargeable information
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_msp_id);

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    alice_last_chargeable_info.last_chargeable_tick,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_payment_streams_correctly_works_for_privileged_providers() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Set any last chargeable info for Alice. It shouldn't be used since she is a privileged provider
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
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

                // Advance 10 blocks. Alice should be able to charge for them since she is a privileged provider
                run_to_block(System::block_number() + 10);

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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * new_rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged tick
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );

                // The payment stream should have been updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);
            });
        }

        #[test]
        fn charge_payment_streams_correctly_works_for_fixed_rate_streams_even_if_provider_is_not_privileged(
        ) {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Remove Alice from the privileged providers list
                <PaymentStreams as PaymentStreamsInterface>::remove_privileged_provider(
                    &alice_msp_id,
                );

                // Set Alice's last chargeable info to use since she's no longer a privileged provider
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
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
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * new_rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // Get Alice's last chargeable information
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_msp_id);

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );

                // The payment stream should be updated with the correct rate
                assert_eq!(payment_stream_info.rate, new_rate);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    alice_last_chargeable_info.last_chargeable_tick,
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );

                // Set the last valid proof of the payment stream from Bob to Alice to 20 blocks ahead
                run_to_block(System::block_number() + 20);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 300,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 20 * rate,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    payment_stream_info.last_charged_tick,
                    System::block_number()
                );
            });
        }

        #[test]
        fn charge_multiple_users_payment_streams_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let charlie: AccountId = 2;
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let bob_rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        bob_rate
                    )
                );

                // Create a payment stream from Charlie to Alice of 20 units per block
                let charlie_rate: BalanceOf<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &charlie,
                        charlie_rate
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - bob_rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check the new free balance of Charlie (after the new stream deposit)
                let charlie_new_balance = charlie_initial_balance
                    - charlie_rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&charlie), charlie_new_balance);

                // Set the last valid proof of the payment streams from Bob to Alice and from Charlie to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Charge both payment streams
                let user_accounts = vec![bob, charlie];
                assert_ok!(PaymentStreams::charge_multiple_users_payment_streams(
                    RuntimeOrigin::signed(alice),
                    user_accounts.clone().try_into().unwrap()
                ));

                // Check that Bob was charged 10 blocks at the 10 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * bob_rate
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * bob_rate,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Charlie was charged 10 blocks at the 20 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&charlie),
                    charlie_new_balance - 10 * charlie_rate
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: charlie,
                        provider_id: alice_msp_id,
                        amount: 10 * charlie_rate,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that the UsersCharged event was emitted
                System::assert_has_event(
                    Event::UsersCharged {
                        user_accounts: user_accounts.try_into().unwrap(),
                        provider_id: alice_msp_id,
                        charged_at_tick: PaymentStreams::get_current_tick(),
                    }
                    .into(),
                );

                // Get the payment stream information for Bob
                let bob_payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    bob_payment_stream_info.last_charged_tick,
                    PaymentStreams::get_current_tick()
                );

                // Check the same for Charlie
                let charlie_payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &charlie)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    charlie_payment_stream_info.last_charged_tick,
                    PaymentStreams::get_current_tick()
                );
            });
        }

        #[test]
        fn charge_three_users_with_different_payment_streams_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let charlie: AccountId = 2;
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);
                let dave: AccountId = 3;
                let dave_initial_balance = NativeBalance::free_balance(&dave);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a fixed-rate payment stream from Bob to Alice of 10 units per block
                let bob_rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        bob_rate
                    )
                );

                // Create a dynamic-rate payment stream from Charlie to Alice with 20 units of data
                let charlie_amount_provided: StorageData<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_msp_id,
                        &charlie,
                        &charlie_amount_provided
                    )
                );

                // Create both a fixed-rate and a dynamic-rate payment stream from Dave to Alice
                let dave_rate: BalanceOf<Test> = 5;
                let dave_amount_provided: StorageData<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &dave,
                        dave_rate
                    )
                );
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_msp_id,
                        &dave,
                        &dave_amount_provided
                    )
                );

                // Remove Alice from the privileged providers list so it charges according to her last chargeable info
                <PaymentStreams as PaymentStreamsInterface>::remove_privileged_provider(
                    &alice_msp_id,
                );

                // Get the current price for dynamic-rate payment streams from the runtime
                let current_storage_price: BalanceOf<Test> =
                    PaymentStreams::get_current_price_per_giga_unit_per_tick();

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - bob_rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check the new free balance of Charlie (after the new stream deposit)
                let charlie_new_balance: BalanceOf<Test> = charlie_initial_balance
                    - current_storage_price
                        * new_stream_deposit_blocks_balance_typed
                        * charlie_amount_provided as u128
                        / GIGAUNIT_BALANCE
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&charlie), charlie_new_balance);

                // Check the new free balance of Dave (after both new stream deposits)
                let dave_new_balance = dave_initial_balance
                    - dave_rate * new_stream_deposit_blocks_balance_typed
                    - current_storage_price
                        * new_stream_deposit_blocks_balance_typed
                        * dave_amount_provided as u128
                        / GIGAUNIT_BALANCE
                    - 2 * base_deposit;
                assert_eq!(NativeBalance::free_balance(&dave), dave_new_balance);

                // Set the last valid proof of the payment streams from Bob to Alice, from Charlie to Alice and from Dave to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                let last_chargeable_price_index = PaymentStreams::get_accumulated_price_index();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: last_chargeable_price_index,
                    },
                );

                // Charge the three users (four payment streams)
                let user_accounts = vec![bob, charlie, dave];
                assert_ok!(PaymentStreams::charge_multiple_users_payment_streams(
                    RuntimeOrigin::signed(alice),
                    user_accounts.clone().try_into().unwrap()
                ));

                // Check that Bob was charged 10 blocks at the 10 units/block rate
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance - 10 * bob_rate
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 10 * bob_rate,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Charlie was charged 10 blocks at the current_price * charlie_amount_provided rate
                assert_eq!(
                    NativeBalance::free_balance(&charlie),
                    charlie_new_balance
                        - 10 * current_storage_price * charlie_amount_provided as u128
                            / GIGAUNIT_BALANCE
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: charlie,
                        provider_id: alice_msp_id,
                        amount: 10 * current_storage_price * charlie_amount_provided as u128
                            / GIGAUNIT_BALANCE,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Dave was charged 10 blocks at the 5 units/block rate
                // and 10 blocks at the current_price * dave_amount_provided rate
                assert_eq!(
                    NativeBalance::free_balance(&dave),
                    dave_new_balance
                        - 10 * dave_rate
                        - 10 * current_storage_price * dave_amount_provided as u128
                            / GIGAUNIT_BALANCE
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: dave,
                        provider_id: alice_msp_id,
                        amount: 10 * dave_rate
                            + 10 * current_storage_price * dave_amount_provided as u128
                                / GIGAUNIT_BALANCE,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that the UsersCharged event was emitted
                System::assert_has_event(
                    Event::UsersCharged {
                        user_accounts: user_accounts.try_into().unwrap(),
                        provider_id: alice_msp_id,
                        charged_at_tick: PaymentStreams::get_current_tick(),
                    }
                    .into(),
                );

                // Get the payment stream information for Bob
                let bob_payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct last charged proof
                assert_eq!(
                    bob_payment_stream_info.last_charged_tick,
                    PaymentStreams::get_current_tick()
                );

                // Check the same for Charlie
                let charlie_payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_msp_id, &charlie)
                        .unwrap();

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    charlie_payment_stream_info.price_index_when_last_charged,
                    PaymentStreams::get_accumulated_price_index()
                );

                // Check the same for both payment streams of Dave
                let dave_fixed_payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &dave)
                        .unwrap();
                let dave_dynamic_payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_msp_id, &dave)
                        .unwrap();

                // The payment streams should be updated with the correct last charged proof or price index
                assert_eq!(
                    dave_fixed_payment_stream_info.last_charged_tick,
                    PaymentStreams::get_current_tick()
                );
                assert_eq!(
                    dave_dynamic_payment_stream_info.price_index_when_last_charged,
                    PaymentStreams::get_accumulated_price_index()
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Mint Bob enough tokens to pay for the deposit
                let maximum_amount_to_mint =
                    u128::MAX - pallet_balances::TotalIssuance::<Test>::get();
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
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_balance_after_deposit =
                    bob_new_balance - rate * new_stream_deposit_blocks_balance_typed - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_balance_after_deposit);

                // Set the last valid proof of the payment stream from Bob to Alice to 1000 blocks ahead
                run_to_block(System::block_number() + 1000);
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: 100,
                    },
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Create a payment stream from Bob to Charlie of 10 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        10
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - rate * new_stream_deposit_blocks_balance_typed
                    - 10 * new_stream_deposit_blocks_balance_typed
                    - 2 * base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and the payment stream will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 0,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Try to create a new stream from Bob to Alice (since the original one would have been deleted)
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn charge_payment_streams_emits_correct_event_for_insolvent_user_if_it_has_no_more_streams()
        {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a MSP with 1000 units of data and get his MSP ID
                register_account_as_msp(charlie, 1000);
                let charlie_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of `bob_initial_balance / 20 + 1` units per block
                let rate: BalanceOf<Test> = bob_initial_balance / 20 + 1; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        rate
                    )
                );

                // Create a payment stream from Bob to Charlie of 10 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &charlie_msp_id,
                        &bob,
                        10
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - (rate + 10) * new_stream_deposit_blocks_balance_typed
                    - 2 * base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_msp_id,
                        amount: 0,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Set the last valid proof of Charlie to the current block
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &charlie_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: 100,
                    },
                );

                // Try to charge the payment stream from Bob to Charlie
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(charlie),
                    bob
                ));

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: charlie_msp_id,
                        amount: 0,
                        last_tick_charged: System::block_number(),
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Bob has no remaining payment streams
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the UserPaidAllDebts event was emitted for Bob
                System::assert_has_event(Event::UserPaidAllDebts { who: bob }.into());
            });
        }

        #[test]
        fn charge_three_users_with_different_payment_streams_reverts_if_one_charge_fails() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let charlie: AccountId = 2;
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);
                let dave: AccountId = 3;

                // Dave is the one that will have the payment stream that fails to charge. For that,
                // we want to make sure the balance type will overflow when trying to charge him. For this,
                // we first mint the maximum amount of tokens possible to his account.
                let maximum_amount_to_mint =
                    u128::MAX - pallet_balances::TotalIssuance::<Test>::get();
                assert_ok!(NativeBalance::mint_into(&dave, maximum_amount_to_mint));
                let dave_initial_balance = NativeBalance::free_balance(&dave);

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_msp(alice, 100);
                let alice_msp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Get the tick and accumulated price index when the payment streams are going to be created
                let initial_tick = System::block_number();
                let initial_price_index = PaymentStreams::get_accumulated_price_index();

                // Create a fixed-rate payment stream from Bob to Alice of 10 units per block
                let bob_rate: BalanceOf<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &bob,
                        bob_rate
                    )
                );

                // Create a dynamic-rate payment stream from Charlie to Alice with 20 units of data
                let charlie_amount_provided: StorageData<Test> = 20;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_msp_id,
                        &charlie,
                        &charlie_amount_provided
                    )
                );

                // Create both a fixed-rate and a dynamic-rate payment stream from Dave to Alice
                let dave_rate: BalanceOf<Test> = 5;
                let dave_amount_provided: StorageData<Test> = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_fixed_rate_payment_stream(
                        &alice_msp_id,
                        &dave,
                        dave_rate
                    )
                );
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_msp_id,
                        &dave,
                        &dave_amount_provided
                    )
                );

                // Make it so Dave's fixed-rate payment stream will fail to charge, by using a rate that will overflow the balance type
                let dave_rate = u128::MAX / 11; // Since the deposit is 10 blocks, to make sure it can be held, we use a smaller rate
                assert_ok!(PaymentStreams::update_fixed_rate_payment_stream(
                    RuntimeOrigin::root(),
                    alice_msp_id,
                    dave,
                    dave_rate
                ));

                // Get the current price for dynamic-rate payment streams from the runtime
                let current_storage_price: BalanceOf<Test> =
                    PaymentStreams::get_current_price_per_giga_unit_per_tick();

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let bob_new_balance = bob_initial_balance
                    - bob_rate * new_stream_deposit_blocks_balance_typed
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check the new free balance of Charlie (after the new stream deposit)
                let charlie_new_balance: BalanceOf<Test> = charlie_initial_balance
                    - current_storage_price
                        * new_stream_deposit_blocks_balance_typed
                        * charlie_amount_provided as u128
                        / GIGAUNIT_BALANCE
                    - base_deposit;
                assert_eq!(NativeBalance::free_balance(&charlie), charlie_new_balance);

                // Check the new free balance of Dave (after both new stream deposits)
                let dave_new_balance = dave_initial_balance
                    - dave_rate * new_stream_deposit_blocks_balance_typed
                    - current_storage_price
                        * new_stream_deposit_blocks_balance_typed
                        * dave_amount_provided as u128
                        / GIGAUNIT_BALANCE
                    - 2 * base_deposit;
                assert_eq!(NativeBalance::free_balance(&dave), dave_new_balance);

                // Set the last valid proof of the payment streams from Bob to Alice, from Charlie to Alice and from Dave to Alice to 20 blocks ahead
                run_to_block(System::block_number() + 20);
                let last_chargeable_tick = System::block_number();
                let last_chargeable_price_index = PaymentStreams::get_accumulated_price_index();
                LastChargeableInfo::<Test>::insert(
                    &alice_msp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: last_chargeable_price_index,
                    },
                );

                // Charge the three users (four payment streams)
                let user_accounts = vec![bob, charlie, dave];
                assert_noop!(
                    PaymentStreams::charge_multiple_users_payment_streams(
                        RuntimeOrigin::signed(alice),
                        user_accounts.clone().try_into().unwrap()
                    ),
                    Error::<Test>::ChargeOverflow
                );

                // Check that Bob was not charged
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Charlie was not charged
                assert_eq!(NativeBalance::free_balance(&charlie), charlie_new_balance);

                // Check that Dave was not charged
                assert_eq!(NativeBalance::free_balance(&dave), dave_new_balance);

                // Get the payment stream information for Bob
                let bob_payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &bob)
                        .unwrap();

                // The payment stream should not be updated
                assert_eq!(bob_payment_stream_info.last_charged_tick, initial_tick);

                // Check the same for Charlie
                let charlie_payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_msp_id, &charlie)
                        .unwrap();

                // The payment stream should not be updated
                assert_eq!(
                    charlie_payment_stream_info.price_index_when_last_charged,
                    initial_price_index
                );

                // Check the same for both payment streams of Dave
                let dave_fixed_payment_stream_info =
                    PaymentStreams::get_fixed_rate_payment_stream_info(&alice_msp_id, &dave)
                        .unwrap();
                let dave_dynamic_payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_msp_id, &dave)
                        .unwrap();

                // The payment streams should not be updated
                assert_eq!(
                    dave_fixed_payment_stream_info.last_charged_tick,
                    initial_tick
                );
                assert_eq!(
                    dave_dynamic_payment_stream_info.price_index_when_last_charged,
                    initial_price_index
                );
            });
        }
    }
    mod update_last_chargeable_tick {

        use super::*;

        #[test]
        fn update_last_chargeable_tick_works() {
            ExtBuilder::build().execute_with(|| {
                let alice_on_poll: AccountId = 123;
                let bob: AccountId = 1;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice_on_poll, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice_on_poll)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 10 units provided
                let amount_provided: u32 = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided.into()
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to block 123 (represented by Alice's account ID)
                // Since we are using the mocked ProofSubmittersInterface, we can pass the account ID as the
                // block number to the `on_poll` hook and it will consider that Provider as one that submitted a proof
                run_to_block(alice_on_poll);

                // Get Alice's last chargeable information
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last valid proof
                assert_eq!(
                    alice_last_chargeable_info.last_chargeable_tick,
                    System::block_number() - 1
                );
            });
        }

        #[test]
        fn update_last_chargeable_tick_with_submitters_paused_works() {
            ExtBuilder::build().execute_with(|| {
                let alice_on_poll: AccountId = 123;
                let bob: AccountId = 1;

                // Register Alice as a MSP with 100 units of data and get her MSP ID
                register_account_as_bsp(alice_on_poll, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice_on_poll)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 10 units per block
                let amount_provided: u32 = 10;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided.into()
                    )
                );

                // Set the last valid proof of the payment stream from Bob to Alice to block 123 (represented by Alice's account ID)
                // Since we are using the mocked ProofSubmittersInterface, we can pass the account ID as the
                // block number to the `on_poll` hook and it will consider that Provider as one that submitted a proof.
                // We advance to one block before Alice's turn.
                run_to_block(alice_on_poll - 1);

                // Mock blocks advancing where the Proofs Submitters' tick is stopped.
                let tick_before = crate::OnPollTicker::<Test>::get();
                for _ in 0..10 {
                    // For that, we run `on_poll` hooks of this Payment Stream's pallet, without actually advancing blocks.
                    PaymentStreams::on_poll(System::block_number(), &mut WeightMeter::new());
                }
                let tick_after = crate::OnPollTicker::<Test>::get();
                assert!(tick_before == tick_after - 10);

                // Now the Proof Submitters pallet "resumes", so we advance one more block to when
                // it is Alice's turn to be in the valid proofs submitters set.
                run_to_block(alice_on_poll);

                // Get Alice's last chargeable information
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_bsp_id);
                let current_tick = crate::OnPollTicker::<Test>::get();

                // The payment stream should be updated and considering that the last chargeable tick
                // is the current tick for the Payment Streams pallet.
                assert_eq!(
                    alice_last_chargeable_info.last_chargeable_tick,
                    current_tick - 1
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // The new balance of Bob should be his original balance minus `current_price * amount_provided * NewStreamDeposit + BaseDeposit` (in this case 10 * 100 * 10 + 10 = 10010)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
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
                    payment_stream_info.price_index_at_last_chargeable_tick,
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
                    Event::DynamicRatePaymentStreamCreated {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount_provided,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_insolvent_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10;
                let current_price_index = 10000;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(alice_bsp_id),
                    (),
                );

                // Try to create a payment stream from Bob to Alice of 100 units provided again
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    ),
                    Error::<Test>::ProviderInsolvent
                );
            });
        }

        #[test]
        fn create_payment_stream_fails_if_stream_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Try to create a payment stream from Bob to Alice of 100 units provided again
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Try to create a payment stream from Bob to a random not registered BSP of 100 units provided
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &H256::random(),
                        &bob,
                        &amount_provided,
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Create a payment stream from Bob to Alice of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // The new balance of Bob should be his original balance minus `current_price * amount_provided * NewStreamDeposit` (in this case 10 * 100 * 10 = 10000)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index to something that will make Bob run out of funds
                let current_price_index = current_price_index
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                run_to_block(System::block_number() + 10);
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: current_price_index,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and the payment stream will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Try to create a payment stream from Bob to Alice of 100 units provided (since the original stream would have been deleted)
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Transfer almost all of Bob's balance to Alice (Bob keeps `deposit_amount - 1` balance)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE;
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;
                let bob_initial_balance = NativeBalance::free_balance(&bob);

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Set the amount of payment streams that Bob has to u32::MAX - 1
                RegisteredUsers::<Test>::insert(&bob, u32::MAX - 1);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    ),
                );

                // Check that Bob's new balance is his initial balance minus the deposit
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
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
                let amount_provided: u64 = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit = current_price
                    * amount_provided as u128
                    * BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get())
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check that the deposit was correct
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();
                assert_eq!(payment_stream_info.user_deposit, deposit);

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided: u64 = 200;
                let new_deposit = current_price
                    * new_amount_provided as u128
                    * BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get())
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                    )
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct amount_provided and deposit
                assert_eq!(payment_stream_info.amount_provided, new_amount_provided);
                assert_eq!(payment_stream_info.user_deposit, new_deposit);

                // The event should be emitted
                System::assert_last_event(
                    Event::DynamicRatePaymentStreamUpdated {
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Try to update the amount provided of the payment stream from Bob to Alice to 0 units
                let new_amount_provided = 0;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Try to update the amount provided of the payment stream from Bob to Alice to 100 units per block (the same as the original amount)
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
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

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Try to update the rate of a payment stream that does not exist
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    ),
                    Error::<Test>::PaymentStreamNotFound
                );
            });
        }

        #[test]
        fn update_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
            ExtBuilder::build().execute_with(|| {
                let bob: AccountId = 1;

                // Try to update a payment stream from Bob to a random not registered BSP to 200 units per block
                let new_amount_provided = 200;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &H256::random(),
                        &bob,
                        &new_amount_provided,
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
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // The new balance of Bob should be his original balance minus `2 * current_price * amount_provided * NewStreamDeposit` (in this case 2 * 10 * 100 * 10 = 20000)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index to something that will make Bob run out of funds
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                run_to_block(System::block_number() + 10);
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: current_price_index,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and the payment stream will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Try to update the amount provided of the payment stream from Bob to Charlie to 200 units
                let new_amount_provided = 200;
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &new_amount_provided,
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                    )
                );

                // Check that Bob's deposit was updated AND he was charged 10 blocks (at current price) at the old 100 units provided after the payment stream was updated
                let bob_balance_updated_deposit = bob_new_balance
                    - current_price
                        * u128::from(new_amount_provided - amount_provided)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE;
                let paid_for_storage =
                    10 * current_price * u128::from(amount_provided) / GIGAUNIT_BALANCE;
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_balance_updated_deposit - paid_for_storage
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: paid_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
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
                    payment_stream_info.price_index_at_last_chargeable_tick,
                    current_price_index
                ); */

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );
            });
        }

        #[test]
        fn updated_payment_stream_works_with_insolvent_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(alice_bsp_id),
                    (),
                );

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                    )
                );

                // Check that Bob's deposit was updated AND he was charged 10 blocks (at current price) at the old 100 units provided after the payment stream was updated
                let bob_balance_updated_deposit = bob_new_balance
                    - current_price
                        * u128::from(new_amount_provided - amount_provided)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE;
                let paid_for_storage =
                    10 * current_price * u128::from(amount_provided) / GIGAUNIT_BALANCE;
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_balance_updated_deposit - paid_for_storage
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: paid_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // The payment stream should be updated with the correct amount provided and last charged price index
                assert_eq!(payment_stream_info.amount_provided, new_amount_provided);
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
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
                    Event::DynamicRatePaymentStreamDeleted {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                    }
                    .into(),
                );
            });
        }

        #[test]
        fn delete_payment_stream_no_charge_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(alice_bsp_id),
                    (),
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
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
                    Event::DynamicRatePaymentStreamDeleted {
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
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

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
        fn delete_payment_stream_charges_pending_blocks() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    10 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
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

        #[test]
        fn delete_payment_stream_charges_pending_blocks_with_insolvent_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Simulate insolvent provider
                pallet_storage_providers::InsolventProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(alice_bsp_id),
                    (),
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    10 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
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
        use super::*;

        #[test]
        fn charge_payment_streams_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    10 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
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
        fn charge_payment_streams_with_awaited_top_up_from_provider_fails() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Simulate awaited top up from provider 5 blocks before the last chargeable tick
                run_to_block(System::block_number() + 5);
                pallet_storage_providers::AwaitingTopUpFromProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(alice_bsp_id),
                    pallet_storage_providers::types::TopUpMetadata {
                        started_at: System::block_number(),
                        end_tick_grace_period: System::block_number() + 10,
                    },
                );
                let current_price_index = AccumulatedPriceIndex::<Test>::get();

                // Set the last chargeable price index of the payment stream from Bob to Alice to 5 blocks ahead
                run_to_block(System::block_number() + 5);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_noop!(
                    PaymentStreams::charge_payment_streams(RuntimeOrigin::signed(alice), bob),
                    Error::<Test>::ProviderInsolvent
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    10 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // Get Alice's last chargeable info
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Update the amount provided of the payment stream from Bob to Alice to 200 units
                let new_amount_provided = 200;
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::update_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &new_amount_provided,
                    )
                );

                // Check that Bob's deposit has also been updated
                let new_deposit_amount = current_price
                    * (new_amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_new_balance - (new_deposit_amount - deposit_amount);
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    10 * current_price * (new_amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the payment stream information
                let payment_stream_info =
                    PaymentStreams::get_dynamic_rate_payment_stream_info(&alice_bsp_id, &bob)
                        .unwrap();

                // Get Alice's last chargeable info
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last charged price index
                assert_eq!(
                    payment_stream_info.price_index_when_last_charged,
                    current_price_index
                );

                // The payment stream should be updated with the correct amount provided
                assert_eq!(payment_stream_info.amount_provided, new_amount_provided);

                // The payment stream should be updated with the correct last chargeable price index
                assert_eq!(alice_last_chargeable_info.price_index, current_price_index);
            });
        }

        #[test]
        fn charge_payment_works_after_two_charges() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to 10 blocks ahead
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    10 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
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
                run_to_block(System::block_number() + 20);
                let current_price_index = AccumulatedPriceIndex::<Test>::get();
                let amount_to_pay_for_storage =
                    20 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
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
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: amount_to_pay_for_storage,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = current_price
                    * (amount_provided as u128)
                    * new_stream_deposit_blocks_balance_typed
                    / GIGAUNIT_BALANCE
                    + base_deposit;
                let bob_balance_after_deposit = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_balance_after_deposit);

                // Set the last chargeable price index of the payment stream from Bob to Alice to something that would overflow the balance type
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + u128::MAX / (amount_provided as u128)
                    + 1;
                run_to_block(System::block_number() + 1000);
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: current_price_index,
                    },
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
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new stream deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of the payment stream from Bob to Alice to something that will make Bob run out of funds
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and the payment stream will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Try to create a new stream from Bob to Alice (since the original stream should have been deleted)
                assert_noop!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    ),
                    Error::<Test>::UserWithoutFunds
                );
            });
        }

        #[test]
        fn charge_payment_streams_emits_correct_event_for_insolvent_user_if_it_has_no_more_streams()
        {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                run_to_block(System::block_number() + 10);
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Set the last chargeable price index of Charlie to the equivalent of one block ahead
                LastChargeableInfo::<Test>::insert(
                    &charlie_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + current_price,
                    },
                );

                // Try to charge the payment stream (Bob will not have enough balance to pay for it and will get flagged as without funds)
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Get the deposit of the payment stream from Bob to Charlie
                let charlie_stream_deposit =
                    DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;

                // Try to charge the payment stream from Bob to Charlie
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(charlie),
                    bob
                ));

                // Check that no funds were charged from Bob's free balance, and he got the rest of his deposit (minus the block he paid)
                assert_eq!(
                    NativeBalance::free_balance(&bob),
                    bob_new_balance + charlie_stream_deposit
                        - current_price * amount_provided as u128 / GIGAUNIT_BALANCE
                );
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: charlie_bsp_id,
                        amount: 0,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Bob has no remaining payment streams
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the UserPaidAllDebts event was emitted for Bob
                System::assert_has_event(Event::UserPaidAllDebts { who: bob }.into());
            });
        }
    }
    mod update_last_chargeable_price_index {

        use super::*;

        #[test]
        fn update_last_chargeable_price_index_works() {
            ExtBuilder::build().execute_with(|| {
                let alice_on_poll: AccountId = 123;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let initial_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(initial_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice_on_poll, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice_on_poll)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Set the last chargeable price index of the payment stream from Bob to Alice to block 123
                let blocks_to_advance = alice_on_poll - System::block_number();
                let price_index_increment = current_price * (blocks_to_advance - 1) as u128; // We substract one since price index is updated after the chargeable info (since this one is for the previous block)
                let current_price_index =
                    AccumulatedPriceIndex::<Test>::get() + price_index_increment;
                run_to_block(alice_on_poll);

                // Get Alice's last chargeable info
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last chargeable price index
                assert_eq!(alice_last_chargeable_info.price_index, current_price_index);
            });
        }

        #[test]
        fn update_last_chargeable_price_index_works_with_awaited_top_up_provider() {
            ExtBuilder::build().execute_with(|| {
                let alice_on_poll: AccountId = 123;
                let bob: AccountId = 1;
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let initial_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(initial_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice_on_poll, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice_on_poll)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units per block
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Set the last chargeable price index of the payment stream from Bob to Alice to block 123
                let blocks_to_advance = alice_on_poll - System::block_number();
                let price_index_increment = current_price * (blocks_to_advance - 1) as u128; // We substract one since price index is updated after the chargeable info (since this one is for the previous block)
                let current_price_index =
                    AccumulatedPriceIndex::<Test>::get() + price_index_increment;

                pallet_storage_providers::AwaitingTopUpFromProviders::<Test>::insert(
                    StorageProviderId::<Test>::BackupStorageProvider(alice_bsp_id),
                    pallet_storage_providers::types::TopUpMetadata {
                        started_at: System::block_number(),
                        end_tick_grace_period: System::block_number() + 1,
                    },
                );

                run_to_block(alice_on_poll);

                // Get Alice's last chargeable info
                let alice_last_chargeable_info =
                    PaymentStreams::get_last_chargeable_info(&alice_bsp_id);

                // The payment stream should be updated with the correct last chargeable price index
                assert_eq!(alice_last_chargeable_info.price_index, current_price_index);
            });
        }
    }
}

mod user_without_funds {

    use super::*;

    mod pay_outstanding_debt {

        use super::*;

        #[test]
        fn pay_outstanding_debt_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID and free balance
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();
                let alice_initial_balance = NativeBalance::free_balance(&alice);

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let total_deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - total_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that the deposit of Bob to Alice is half of the total deposit
                let alice_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(alice_deposit_amount, total_deposit_amount / 2);

                // And that Charlie has the other half
                let charlie_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(charlie_deposit_amount, total_deposit_amount / 2);

                // Set the last chargeable price index of Charlie to the equivalent of one block ahead
                LastChargeableInfo::<Test>::insert(
                    &charlie_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + current_price,
                    },
                );

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Bob is flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the payment stream from Bob to Alice does not exist anymore
                assert_eq!(
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob),
                    None
                );

                // Check that the payment stream from Bob to Charlie still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob).is_some());

                // Check that Bob's free balance has not changed but it's deposit to Alice has been transferred to her
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                assert_eq!(
                    NativeBalance::free_balance(&alice),
                    alice_initial_balance + alice_deposit_amount
                );

                // Check that Bob still has its deposit with Charlie
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    charlie_deposit_amount
                );

                // Pay the outstanding debt of Bob
                assert_ok!(PaymentStreams::pay_outstanding_debt(
                    RuntimeOrigin::signed(bob),
                    vec![charlie_bsp_id]
                ));

                // Check that Bob's balance has been updated with the correct amount after paying Charlie
                let amount_to_pay_for_storage =
                    current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let bob_new_balance =
                    bob_new_balance - amount_to_pay_for_storage + charlie_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Charlie has been paid
                assert_eq!(
                    NativeBalance::free_balance(&charlie),
                    charlie_initial_balance + amount_to_pay_for_storage
                );

                // Check that Bob no longer has any deposits
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    0
                );

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the UserPaidAllDebts event was emitted for Bob
                System::assert_has_event(Event::UserPaidAllDebts { who: bob }.into());

                // Check that Bob has no remaining payment streams
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);
            });
        }

        #[test]
        fn pay_outstanding_debt_works_with_multiple_streams() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let david: AccountId = 3;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID and free balance
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();
                let alice_initial_balance = NativeBalance::free_balance(&alice);

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);

                // Register David as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(david, 1000);
                let david_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&david).unwrap();
                let david_initial_balance = NativeBalance::free_balance(&david);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to David of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &david_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let total_deposit_amount = 3
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - total_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that the deposit of Bob to Alice is one third of the total deposit
                let alice_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(alice_deposit_amount, total_deposit_amount / 3);

                // And that Charlie has the other third
                let charlie_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(charlie_deposit_amount, total_deposit_amount / 3);

                // And that David has the last third
                let david_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&david_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(david_deposit_amount, total_deposit_amount / 3);

                // Set the last chargeable price index of David to the equivalent of two blocks ahead
                LastChargeableInfo::<Test>::insert(
                    &david_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&david_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + 2 * current_price,
                    },
                );

                // Set the last chargeable price index of Charlie to the equivalent of one block ahead
                LastChargeableInfo::<Test>::insert(
                    &charlie_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + current_price,
                    },
                );

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Bob is flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the payment stream from Bob to Alice does not exist anymore
                assert_eq!(
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob),
                    None
                );

                // Check that the payment stream from Bob to Charlie still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob).is_some());

                // Check that the payment stream from Bob to David still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&david_bsp_id, &bob).is_some());

                // Check that Bob's free balance has not changed but it's deposit to Alice has been transferred to her
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                assert_eq!(
                    NativeBalance::free_balance(&alice),
                    alice_initial_balance + alice_deposit_amount
                );

                // Check that Bob still has its deposit with Charlie and David
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    charlie_deposit_amount + david_deposit_amount
                );

                // Pay the outstanding debt of Bob
                assert_ok!(PaymentStreams::pay_outstanding_debt(
                    RuntimeOrigin::signed(bob),
                    vec![charlie_bsp_id, david_bsp_id]
                ));

                // Check that Bob's balance has been updated with the correct amount after paying charlie and david
                let amount_to_pay_for_storage =
                    3 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let bob_new_balance = bob_new_balance - amount_to_pay_for_storage
                    + charlie_deposit_amount
                    + david_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Charlie has been paid
                assert_eq!(
                    NativeBalance::free_balance(&charlie),
                    charlie_initial_balance + amount_to_pay_for_storage / 3
                );

                // Check that David has been paid
                assert_eq!(
                    NativeBalance::free_balance(&david),
                    david_initial_balance + 2 * amount_to_pay_for_storage / 3
                );

                // Check that Bob no longer has any deposits nor payment streams
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    0
                );
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the UserPaidAllDebts event was emitted for Bob
                System::assert_has_event(Event::UserPaidAllDebts { who: bob }.into());
            });
        }

        #[test]
        fn pay_outstanding_debt_works_when_only_paying_partial_debt() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let david: AccountId = 3;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID and free balance
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();
                let alice_initial_balance = NativeBalance::free_balance(&alice);

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);

                // Register David as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(david, 1000);
                let david_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&david).unwrap();
                let david_initial_balance = NativeBalance::free_balance(&david);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to David of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &david_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let total_deposit_amount = 3
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - total_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that the deposit of Bob to Alice is one third of the total deposit
                let alice_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(alice_deposit_amount, total_deposit_amount / 3);

                // And that Charlie has the other third
                let charlie_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(charlie_deposit_amount, total_deposit_amount / 3);

                // And that David has the last third
                let david_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&david_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(david_deposit_amount, total_deposit_amount / 3);

                // Set the last chargeable price index of David to the equivalent of two blocks ahead
                LastChargeableInfo::<Test>::insert(
                    &david_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&david_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + 2 * current_price,
                    },
                );

                // Set the last chargeable price index of Charlie to the equivalent of one block ahead
                LastChargeableInfo::<Test>::insert(
                    &charlie_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + current_price,
                    },
                );

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Bob is flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the payment stream from Bob to Alice does not exist anymore
                assert_eq!(
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob),
                    None
                );

                // Check that the payment stream from Bob to Charlie still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob).is_some());

                // Check that the payment stream from Bob to David still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&david_bsp_id, &bob).is_some());

                // Check that Bob's free balance has not changed but it's deposit to Alice has been transferred to her
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                assert_eq!(
                    NativeBalance::free_balance(&alice),
                    alice_initial_balance + alice_deposit_amount
                );

                // Check that Bob still has its deposit with Charlie and David
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    charlie_deposit_amount + david_deposit_amount
                );

                // Pay the outstanding debt of Bob, but only to Charlie
                assert_ok!(PaymentStreams::pay_outstanding_debt(
                    RuntimeOrigin::signed(bob),
                    vec![charlie_bsp_id]
                ));

                // Check that Bob's balance has been updated with the correct amount after paying Charlie (but not David)
                let amount_to_pay_for_storage_charlie =
                    1 * current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let bob_new_balance =
                    bob_new_balance - amount_to_pay_for_storage_charlie + charlie_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Charlie has been paid
                assert_eq!(
                    NativeBalance::free_balance(&charlie),
                    charlie_initial_balance + amount_to_pay_for_storage_charlie
                );

                // Check that David has NOT been paid
                assert_eq!(NativeBalance::free_balance(&david), david_initial_balance);

                // Check that Bob still has the deposit with David
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    david_deposit_amount
                );

                // Check that Bob still has a payment stream
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 1);

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the UserPaidSomeDebts event was emitted for Bob
                System::assert_has_event(Event::UserPaidSomeDebts { who: bob }.into());
            });
        }

        #[test]
        fn pay_outstanding_debt_fails_if_user_is_not_flagged_as_without_funds() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;
                let amount_provided = 100;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Try to pay the outstanding debt of Bob without him being flagged as a user without funds
                assert_noop!(
                    PaymentStreams::pay_outstanding_debt(
                        RuntimeOrigin::signed(bob),
                        vec![alice_bsp_id]
                    ),
                    Error::<Test>::UserNotFlaggedAsWithoutFunds
                );
            });
        }
    }

    mod clear_insolvent_flag {

        use super::*;

        #[test]
        fn clear_insolvent_flag_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID and free balance
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();
                let alice_initial_balance = NativeBalance::free_balance(&alice);

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();
                let charlie_initial_balance = NativeBalance::free_balance(&charlie);

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let total_deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - total_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that the deposit of Bob to Alice is half of the total deposit
                let alice_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(alice_deposit_amount, total_deposit_amount / 2);

                // And that Charlie has the other half
                let charlie_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(charlie_deposit_amount, total_deposit_amount / 2);

                // Set the last chargeable price index of Charlie to the equivalent of one block ahead
                LastChargeableInfo::<Test>::insert(
                    &charlie_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + current_price,
                    },
                );

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Bob is flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the payment stream from Bob to Alice does not exist anymore
                assert_eq!(
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob),
                    None
                );

                // Check that the payment stream from Bob to Charlie still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob).is_some());

                // Check that Bob's free balance has not changed but it's deposit to Alice has been transferred to her
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                assert_eq!(
                    NativeBalance::free_balance(&alice),
                    alice_initial_balance + alice_deposit_amount
                );

                // Check that Bob still has its deposit with Charlie
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    charlie_deposit_amount
                );

                // Pay the outstanding debt of Bob
                assert_ok!(PaymentStreams::pay_outstanding_debt(
                    RuntimeOrigin::signed(bob),
                    vec![charlie_bsp_id]
                ));

                // Check that Bob's balance has been updated with the correct amount after paying Charlie
                let amount_to_pay_for_storage =
                    current_price * (amount_provided as u128) / GIGAUNIT_BALANCE;
                let bob_new_balance =
                    bob_new_balance - amount_to_pay_for_storage + charlie_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that Charlie has been paid
                assert_eq!(
                    NativeBalance::free_balance(&charlie),
                    charlie_initial_balance + amount_to_pay_for_storage
                );

                // Check that Bob no longer has any deposits
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    0
                );

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the UserPaidAllDebts event was emitted for Bob
                System::assert_has_event(Event::UserPaidAllDebts { who: bob }.into());

                // Check that Bob has no remaining payment streams
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 0);

                // Wait enough blocks to allow Bob to clear himself as an insolvent user
                run_to_block(
                    System::block_number() + <UserWithoutFundsCooldown as Get<u64>>::get() + 1,
                );

                // Clear the insolvent flag of Bob
                assert_ok!(PaymentStreams::clear_insolvent_flag(RuntimeOrigin::signed(
                    bob
                )));

                // Check that Bob is no longer flagged as a user without funds
                assert!(!UsersWithoutFunds::<Test>::contains_key(bob));
            });
        }

        #[test]
        fn clear_insolvent_flag_fails_if_there_are_remaining_payment_streams() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let bob_initial_balance = NativeBalance::free_balance(&bob);
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID and free balance
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();
                let alice_initial_balance = NativeBalance::free_balance(&alice);

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Check the new free balance of Bob (after the new user deposit)
                let new_stream_deposit_blocks_balance_typed =
                    BlockNumberToBalance::convert(<NewStreamDeposit as Get<u64>>::get());
                let base_deposit = <BaseDeposit as Get<BalanceOf<Test>>>::get();
                let total_deposit_amount = 2
                    * (current_price
                        * (amount_provided as u128)
                        * new_stream_deposit_blocks_balance_typed
                        / GIGAUNIT_BALANCE
                        + base_deposit);
                let bob_new_balance = bob_initial_balance - total_deposit_amount;
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);

                // Check that the deposit of Bob to Alice is half of the total deposit
                let alice_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(alice_deposit_amount, total_deposit_amount / 2);

                // And that Charlie has the other half
                let charlie_deposit_amount =
                    DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                        .unwrap()
                        .user_deposit;
                assert_eq!(charlie_deposit_amount, total_deposit_amount / 2);

                // Set the last chargeable price index of Charlie to the equivalent of one block ahead
                LastChargeableInfo::<Test>::insert(
                    &charlie_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob)
                            .unwrap()
                            .price_index_when_last_charged
                            + current_price,
                    },
                );

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                let last_chargeable_tick = System::block_number();
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick,
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that no funds were charged from Bob's free balance
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                System::assert_has_event(
                    Event::PaymentStreamCharged {
                        user_account: bob,
                        provider_id: alice_bsp_id,
                        amount: 0,
                        last_tick_charged: last_chargeable_tick,
                        charged_at_tick: System::block_number(),
                    }
                    .into(),
                );

                // Check that Bob is flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Check that the payment stream from Bob to Alice does not exist anymore
                assert_eq!(
                    DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob),
                    None
                );

                // Check that the payment stream from Bob to Charlie still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob).is_some());

                // Check that Bob's free balance has not changed but it's deposit to Alice has been transferred to her
                assert_eq!(NativeBalance::free_balance(&bob), bob_new_balance);
                assert_eq!(
                    NativeBalance::free_balance(&alice),
                    alice_initial_balance + alice_deposit_amount
                );

                // Check that Bob still has its deposit with Charlie
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &RuntimeHoldReason::PaymentStreams(crate::HoldReason::PaymentStreamDeposit),
                        &bob
                    ),
                    charlie_deposit_amount
                );

                // Wait enough blocks to allow Bob to clear himself as an insolvent user
                run_to_block(
                    System::block_number() + <UserWithoutFundsCooldown as Get<u64>>::get() + 1,
                );

                // Try to clear the insolvent flag of Bob. It'll fail since there's still a payment stream
                assert_noop!(
                    PaymentStreams::clear_insolvent_flag(RuntimeOrigin::signed(bob)),
                    Error::<Test>::UserHasRemainingDebt
                );

                // Check that Bob still has a remaining payment stream
                assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 1);

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));
            });
        }

        #[test]
        fn clear_insolvent_flag_fails_if_the_cooldown_period_has_not_passed() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = 0;
                let bob: AccountId = 1;
                let charlie: AccountId = 2;
                let amount_provided = 100;
                let current_price = 10 * GIGAUNIT_BALANCE;
                let current_price_index = 10000 * GIGAUNIT_BALANCE;

                // Update the current price and current price index
                CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
                AccumulatedPriceIndex::<Test>::put(current_price_index);

                // Register Alice as a BSP with 100 units of data and get her BSP ID and free balance
                register_account_as_bsp(alice, 100);
                let alice_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

                // Register Charlie as a BSP with 1000 units of data and get his BSP ID and free balance
                register_account_as_bsp(charlie, 1000);
                let charlie_bsp_id =
                    <StorageProviders as ReadProvidersInterface>::get_provider_id(&charlie)
                        .unwrap();

                // Create a payment stream from Bob to Alice of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &alice_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Create a payment stream from Bob to Charlie of 100 units provided
                assert_ok!(
                    <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                        &charlie_bsp_id,
                        &bob,
                        &amount_provided,
                    )
                );

                // Get Bob's free balance after the deposits for the payment streams
                let bob_new_balance = NativeBalance::free_balance(&bob);

                // Set the last chargeable price index of Alice to something that will make Bob run out of funds
                run_to_block(System::block_number() + 10);
                let current_price_index = AccumulatedPriceIndex::<Test>::get()
                    + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided as u128)
                    + 1;
                LastChargeableInfo::<Test>::insert(
                    &alice_bsp_id,
                    ProviderLastChargeableInfo {
                        last_chargeable_tick: System::block_number(),
                        price_index: current_price_index,
                    },
                );

                // Charge the payment stream from Bob to Alice
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Advance enough blocks for Bob to be flagged as a user without funds
                run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
                assert_ok!(PaymentStreams::charge_payment_streams(
                    RuntimeOrigin::signed(alice),
                    bob
                ));

                // Check that the UserWithoutFunds event was emitted for Bob
                System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

                // Check that Bob is flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));

                // Wait less than what would be enough blocks to allow Bob to clear himself as an insolvent user
                run_to_block(
                    System::block_number() + <UserWithoutFundsCooldown as Get<u64>>::get() - 10,
                );

                // Clear the insolvent flag of Bob
                assert_noop!(
                    PaymentStreams::clear_insolvent_flag(RuntimeOrigin::signed(bob)),
                    Error::<Test>::CooldownPeriodNotPassed
                );

                // Check that the payment stream between Bob and Charlie still exists
                assert!(DynamicRatePaymentStreams::<Test>::get(&charlie_bsp_id, &bob).is_some());

                // Check that Bob is still flagged as a user without funds
                assert!(UsersWithoutFunds::<Test>::contains_key(bob));
            });
        }
    }
}

mod users_with_debt_over_threshold {

    use super::*;

    #[test]
    fn get_users_with_debt_over_threshold_correctly_returns_list_of_users() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let bob: AccountId = 1;
            let charlie: AccountId = 2;
            let dave: AccountId = 3;
            let amount_provided_bob = 100;
            let amount_provided_charlie = 1;
            let current_price = 10 * GIGAUNIT_BALANCE;
            let current_price_index = 10000 * GIGAUNIT_BALANCE;
            let empty_account_id_vector: Vec<AccountId> = Vec::new();

            // Update the current price and current price index
            CurrentPricePerGigaUnitPerTick::<Test>::put(current_price);
            AccumulatedPriceIndex::<Test>::put(current_price_index);

            // Register Alice as a BSP with 100 units of data and get her BSP ID
            register_account_as_bsp(alice, 100);
            let alice_bsp_id =
                <StorageProviders as ReadProvidersInterface>::get_provider_id(&alice).unwrap();

            // Register Dave as a BSP with 1000 units of data and get his BSP ID
            register_account_as_bsp(dave, 1000);
            let dave_bsp_id =
                <StorageProviders as ReadProvidersInterface>::get_provider_id(&dave).unwrap();

            // Create a payment stream from Bob to Alice of 100 units provided
            assert_ok!(
                <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                    &alice_bsp_id,
                    &bob,
                    &amount_provided_bob,
                )
            );

            // Create a payment stream from Bob to Dave of 100 units provided
            assert_ok!(
                <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                    &dave_bsp_id,
                    &bob,
                    &amount_provided_bob,
                )
            );

            // Create a payment stream from Charlie to Alice of 200 units provided
            assert_ok!(
                <PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(
                    &alice_bsp_id,
                    &charlie,
                    &amount_provided_charlie,
                )
            );

            // Check that `get_users_with_debt_over_threshold` for Alice returns Bob and Charlie if threshold is 0
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&alice_bsp_id, 0).unwrap(),
                vec![bob, charlie]
            );

            // And it returns an empty vector if threshold is > 0
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&alice_bsp_id, 1).unwrap(),
                empty_account_id_vector
            );

            // Check that `get_users_with_debt_over_threshold` for Dave returns Bob if threshold is 0
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&dave_bsp_id, 0).unwrap(),
                vec![bob]
            );

            // And it returns an empty vector if threshold is > 0
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&dave_bsp_id, 1).unwrap(),
                empty_account_id_vector
            );

            // Set the last chargeable price index of Alice to the equivalent of one block ahead
            LastChargeableInfo::<Test>::insert(
                &alice_bsp_id,
                ProviderLastChargeableInfo {
                    last_chargeable_tick: System::block_number(),
                    price_index: DynamicRatePaymentStreams::<Test>::get(&alice_bsp_id, &bob)
                        .unwrap()
                        .price_index_when_last_charged
                        + current_price,
                },
            );

            // Same with Dave
            LastChargeableInfo::<Test>::insert(
                &dave_bsp_id,
                ProviderLastChargeableInfo {
                    last_chargeable_tick: System::block_number(),
                    price_index: DynamicRatePaymentStreams::<Test>::get(&dave_bsp_id, &bob)
                        .unwrap()
                        .price_index_when_last_charged
                        + current_price,
                },
            );

            // Check that now `get_users_with_debt_over_threshold` for Alice returns Bob and Charlie with a threshold of 1
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&alice_bsp_id, 1).unwrap(),
                vec![bob, charlie]
            );

            // And for Dave returns Bob with a threshold of 1
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&dave_bsp_id, 1).unwrap(),
                vec![bob]
            );

            // Set the last chargeable price index of Alice to something that will make Bob run out of funds
            run_to_block(System::block_number() + 10);
            let bob_new_balance = NativeBalance::free_balance(&bob);
            let current_price_index = AccumulatedPriceIndex::<Test>::get()
                + bob_new_balance * GIGAUNIT_BALANCE / (amount_provided_bob as u128)
                + 1;
            LastChargeableInfo::<Test>::insert(
                &alice_bsp_id,
                ProviderLastChargeableInfo {
                    last_chargeable_tick: System::block_number(),
                    price_index: current_price_index,
                },
            );

            // Charge the payment stream from Bob to Alice
            assert_ok!(PaymentStreams::charge_payment_streams(
                RuntimeOrigin::signed(alice),
                bob
            ));

            // Advance enough blocks for Bob to be flagged as a user without funds
            run_to_block(System::block_number() + <NewStreamDeposit as Get<u64>>::get() + 1);
            assert_ok!(PaymentStreams::charge_payment_streams(
                RuntimeOrigin::signed(alice),
                bob
            ));

            // Check that the UserWithoutFunds event was emitted for Bob
            System::assert_has_event(Event::UserWithoutFunds { who: bob }.into());

            // Check that Bob is flagged as a user without funds
            assert!(UsersWithoutFunds::<Test>::contains_key(bob));

            // Check that now `get_users_with_debt_over_threshold` for Alice returns only Charlie with a threshold of 1
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&alice_bsp_id, 1).unwrap(),
                vec![charlie]
            );

            // And returns an empty vector for Dave, since Bob has been flagged as without funds
            assert_eq!(
                PaymentStreams::get_users_with_debt_over_threshold(&dave_bsp_id, 1).unwrap(),
                empty_account_id_vector
            );
        });
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
                .saturating_mul((storage_amount - <SpMinCapacity as Get<u64>>::get()).into()),
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

    // Get the deposit amount for the storage amount
    // The deposit for any amount of storage is be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
    let deposit_for_storage_amount: BalanceOf<Test> = <SpMinDeposit as Get<u128>>::get()
        .saturating_add(
            <DepositPerData as Get<u128>>::get()
                .saturating_mul((storage_amount - <SpMinCapacity as Get<u64>>::get()).into()),
        );

    // Check the balance of the account to make sure it has more than the deposit amount needed
    assert!(NativeBalance::free_balance(&account) >= deposit_for_storage_amount);

    // Request to sign up the account as a Main Storage Provider
    assert_ok!(StorageProviders::request_msp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        1,
        bounded_vec![],
        10,
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
        PaymentStreams::on_poll(System::block_number(), &mut WeightMeter::new());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    }
}

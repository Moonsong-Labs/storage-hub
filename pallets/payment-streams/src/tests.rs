use crate::{mock::*, types::BalanceOf, Error, Event, RegisteredUsers};

use frame_support::pallet_prelude::Weight;
use frame_support::traits::fungible::Mutate;
use frame_support::traits::{Get, OnFinalize, OnIdle, OnInitialize};
use frame_support::{assert_ok, BoundedVec};
use storage_hub_traits::ProvidersInterface;

// `payment-streams` types:
type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;
pub type NewUserDeposit = <Test as crate::Config>::NewUserDeposit;

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

/// This module holds the tests for the creation of a payment stream
mod create_stream {
    use frame_support::assert_noop;
    use sp_runtime::DispatchError;
    use storage_hub_traits::PaymentStreamsInterface;

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
                <StorageProviders as ProvidersInterface>::get_provider(alice).unwrap();

            // Create a payment stream from Bob to Alice of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_ok!(PaymentStreams::create_payment_stream(
                RuntimeOrigin::root(),
                alice,
                bob,
                rate
            ));

            // The new balance of Bob should be his original balance - 10 (deposit to be a user)
            assert_eq!(
                NativeBalance::free_balance(&bob),
                bob_initial_balance - <NewUserDeposit as Get<BalanceOf<Test>>>::get()
            );

            // Get the payment stream information
            let payment_stream_info =
                PaymentStreams::get_payment_stream_info(&alice_bsp_id, &bob).unwrap();
            // The payment stream should be created with the correct rate
            assert_eq!(payment_stream_info.rate, rate);

            // The payment stream should be created with the correct last valid proof
            assert_eq!(payment_stream_info.last_valid_proof, System::block_number());

            // The payment stream should be created with the correct last charged proof
            assert_eq!(
                payment_stream_info.last_charged_proof,
                System::block_number()
            );

            // Bob should have 1 payment stream open
            assert_eq!(PaymentStreams::get_payment_streams_count_of_user(&bob), 1);

            // And that payment stream should be the one just created
            assert_eq!(
                PaymentStreams::get_payment_streams_of_user(&bob)[0],
                (alice_bsp_id, payment_stream_info)
            );

            // The event should be emitted
            System::assert_last_event(
                Event::<Test>::PaymentStreamCreated {
                    user_account: bob,
                    backup_storage_provider_id: alice_bsp_id,
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

            // Register Alice as a BSP with 100 units of data
            register_account_as_bsp(alice, 100);

            // Create a payment stream from Bob to Alice of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_ok!(PaymentStreams::create_payment_stream(
                RuntimeOrigin::root(),
                alice,
                bob,
                rate
            ));

            // Try to create a payment stream from Bob to Alice of 10 units per block again
            assert_noop!(
                PaymentStreams::create_payment_stream(RuntimeOrigin::root(), alice, bob, rate),
                Error::<Test>::PaymentStreamAlreadyExists
            );
        });
    }

    #[test]
    fn create_payment_stream_fails_if_bsp_account_has_not_registered_as_bsp() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let bob: AccountId = 1;

            // Try to create a payment stream from Bob to Alice of 10 units per block without registering Alice as a BSP
            let rate: BalanceOf<Test> = 10;
            assert_noop!(
                PaymentStreams::create_payment_stream(RuntimeOrigin::root(), alice, bob, rate),
                Error::<Test>::NotABackupStorageProvider
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

            // Register Alice as a BSP with 100 units of data
            register_account_as_bsp(alice, 100);

            // Register Charlie as a BSP with 1000 units of data
            register_account_as_bsp(charlie, 1000);

            // Create a payment stream from Bob to Alice of `bob_initial_balance / 10` units per block
            let rate: BalanceOf<Test> = bob_initial_balance / 10; // Bob will have enough balance to pay for only 9 blocks, will come short on the 10th because of the deposit
            assert_ok!(PaymentStreams::create_payment_stream(
                RuntimeOrigin::root(),
                alice,
                bob,
                rate
            ));

            // Set the last valid proof of the payment stream from Bob to Alice to 10 blocks ahead
            run_to_block(System::block_number() + 10);
            assert_ok!(
                <PaymentStreams as PaymentStreamsInterface>::update_last_valid_proof(
                    &alice,
                    &bob,
                    System::block_number()
                )
            );

            // Try to charge the payment stream (Bob will not have enough balance to pay for the 10th block and will get flagged as without funds)
            assert_ok!(PaymentStreams::charge_payment_stream(
                RuntimeOrigin::signed(alice),
                bob
            ));

            // Check that the UserWithoutFunds event was emitted for Bob
            System::assert_has_event(Event::<Test>::UserWithoutFunds { who: bob }.into());

            // Try to create a payment stream from Bob to Charlie of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_noop!(
                PaymentStreams::create_payment_stream(RuntimeOrigin::root(), charlie, bob, rate),
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

            // Register Alice as a BSP with 100 units of data
            register_account_as_bsp(alice, 100);

            // Transfer almost all of Bob's balance to Alice (Bob keeps `NewUserDeposit - 1` balance)
            assert_ok!(Balances::transfer(
                &bob,
                &alice,
                bob_initial_balance - <NewUserDeposit as Get<BalanceOf<Test>>>::get() + 1,
                frame_support::traits::tokens::Preservation::Preserve
            ));

            // Try to create a payment stream from Bob to Alice of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_noop!(
                PaymentStreams::create_payment_stream(RuntimeOrigin::root(), alice, bob, rate),
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

            // Register Alice as a BSP with 100 units of data
            register_account_as_bsp(alice, 100);

            // Register Charlie as a BSP with 1000 units of data
            register_account_as_bsp(charlie, 1000);

            // Set the amount of payment streams that Bob has to u32::MAX - 1
            RegisteredUsers::<Test>::insert(&bob, u32::MAX - 1);

            // Create a payment stream from Bob to Alice of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_ok!(PaymentStreams::create_payment_stream(
                RuntimeOrigin::root(),
                alice,
                bob,
                rate
            ),);

            // Create a payment stream from Bob to Charlie of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_noop!(
                PaymentStreams::create_payment_stream(RuntimeOrigin::root(), charlie, bob, rate),
                DispatchError::Arithmetic(sp_runtime::ArithmeticError::Overflow)
            );
        });
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

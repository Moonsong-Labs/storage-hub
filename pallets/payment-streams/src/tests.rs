use crate::{mock::*, types::BalanceOf, Event};

use frame_support::pallet_prelude::Weight;
use frame_support::traits::{Get, OnFinalize, OnIdle, OnInitialize};
use frame_support::{assert_ok, BoundedVec};
use storage_hub_traits::ProvidersInterface;

type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;

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
    use super::*;

    #[test]
    fn create_payment_stream_works() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let bob: AccountId = 1;
            let bob_initial_balance = NativeBalance::free_balance(&bob);

            // We register Alice as a BSP with 100 units of data and get her BSP ID
            register_account_as_bsp(alice, 100);
            let alice_bsp_id =
                <StorageProviders as ProvidersInterface>::get_provider(alice).unwrap();

            // We now try to create a payment stream from Bob to Alice of 10 units per block
            let rate: BalanceOf<Test> = 10;
            assert_ok!(PaymentStreams::create_payment_stream(
                RuntimeOrigin::root(),
                alice,
                bob,
                rate
            ));

            // The new balance of Bob should be his original balance - 10 (deposit to be a user)
            assert_eq!(NativeBalance::free_balance(&bob), bob_initial_balance - 10);

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

            // The event should be emitted
            System::assert_last_event(
                Event::<Test>::PaymentStreamCreated {
                    user_id: bob,
                    backup_storage_provider_id: alice_bsp_id,
                    rate,
                }
                .into(),
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

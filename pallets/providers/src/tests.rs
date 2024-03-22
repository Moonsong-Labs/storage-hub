use crate::{
    mock::*,
    types::{StorageData, ValuePropId, ValueProposition},
    Error, Event,
};
use frame_support::pallet_prelude::Weight;
use frame_support::traits::{fungible::InspectHold, Get, OnFinalize, OnIdle, OnInitialize};
use frame_support::BoundedVec;
use frame_support::{assert_noop, assert_ok};
use storage_hub_traits::ProvidersInterface;

use crate::types::{BalanceOf, MaxMultiAddressAmount, MultiAddress};

type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;

type SpMinDeposit = <Test as crate::Config>::SpMinDeposit;
type SpMinCapacity = <Test as crate::Config>::SpMinCapacity;
type DepositPerData = <Test as crate::Config>::DepositPerData;

/// Helper functions:
///
/// This function advances the blockchain until block n, executing the hooks for each block
fn _run_to_block(n: u64) {
    assert!(n > System::block_number(), "Cannot go back in time");

    while System::block_number() < n {
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    }
}

/// This module holds the test cases for the signup of Main Storage Providers and Backup Storage Providers
mod sign_up {

    use super::*;

    #[test]
    fn msp_sign_up_works() {
        ExtBuilder::build().execute_with(|| {
            // Initialize variables:
            let multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
                BoundedVec::new();
            let value_prop: ValueProposition<Test> = ValueProposition {
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice and check its balance
            let alice: AccountId = 0;
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Alice is going to sign up as a Main Storage Provider with 100 StorageData units
            // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
            // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
            let deposit_for_storage_amount: BalanceOf<Test> = <SpMinDeposit as Get<u128>>::get()
                .saturating_add(
                    <DepositPerData as Get<u128>>::get().saturating_mul(
                        (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                    ),
                );

            // Sign up Alice as a Main Storage Provider
            assert_ok!(StorageProviders::msp_sign_up(
                RuntimeOrigin::signed(alice),
                storage_amount,
                multiaddresses.clone(),
                value_prop.clone()
            ));

            // Check the new free balance of Alice
            assert_eq!(
                NativeBalance::free_balance(&alice),
                5_000_000 - deposit_for_storage_amount
            );
            // Check the new held balance of Alice
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                deposit_for_storage_amount
            );

            // Check the event was emitted
            System::assert_has_event(
                Event::<Test>::MspSignUpSuccess {
                    who: alice,
                    multiaddresses,
                    total_data: storage_amount,
                    value_prop,
                }
                .into(),
            );
        });
    }

    #[test]
    fn bsp_sign_up_works() {
        ExtBuilder::build().execute_with(|| {
            // Initialize variables:
            let multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
                BoundedVec::new();
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice and check its balance
            let alice: AccountId = 0;
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Alice is going to sign up as a Main Storage Provider with 100 StorageData units
            // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
            // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
            let deposit_for_storage_amount: BalanceOf<Test> = <SpMinDeposit as Get<u128>>::get()
                .saturating_add(
                    <DepositPerData as Get<u128>>::get().saturating_mul(
                        (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                    ),
                );

            // Sign up Alice as a Backup Storage Provider
            assert_ok!(StorageProviders::bsp_sign_up(
                RuntimeOrigin::signed(alice),
                storage_amount,
                multiaddresses.clone(),
            ));

            // Check the new free balance of Alice
            assert_eq!(
                NativeBalance::free_balance(&alice),
                5_000_000 - deposit_for_storage_amount
            );
            // Check the new held balance of Alice
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                deposit_for_storage_amount
            );

            // Check that Alice is a Backup Storage Provider
            let alice_sp_id = StorageProviders::get_provider(alice);
            assert!(alice_sp_id.is_some());
            assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

            // Check the event was emitted
            System::assert_has_event(
                Event::<Test>::BspSignUpSuccess {
                    who: alice,
                    multiaddresses,
                    total_data: storage_amount,
                }
                .into(),
            );
        });
    }

    #[test]
    fn msp_and_bsp_sign_up_fails_when_already_registered_as_msp() {
        ExtBuilder::build().execute_with(|| {
            // Initialize variables:
            let multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
                BoundedVec::new();
            let value_prop: ValueProposition<Test> = ValueProposition {
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice
            let alice: AccountId = 0;

            // Sign up Alice as a Main Storage Provider
            assert_ok!(StorageProviders::msp_sign_up(
                RuntimeOrigin::signed(alice),
                storage_amount,
                multiaddresses.clone(),
                value_prop.clone()
            ));

            // Try to sign up Alice again as a Main Storage Provider
            // We use assert_noop to make sure that it not only returns the specific
            // error, but it also does not modify any storage
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::AlreadyRegistered
            );

            // We try to register her as a BSP now
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::AlreadyRegistered
            );
        });
    }

    #[test]
    fn msp_and_bsp_sign_up_fails_when_already_registered_as_bsp() {
        ExtBuilder::build().execute_with(|| {
            // Initialize variables:
            let multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
                BoundedVec::new();
            let value_prop: ValueProposition<Test> = ValueProposition {
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice
            let alice: AccountId = 0;

            // Sign up Alice as a Backup Storage Provider
            assert_ok!(StorageProviders::bsp_sign_up(
                RuntimeOrigin::signed(alice),
                storage_amount,
                multiaddresses.clone(),
            ));

            // Try to sign up Alice now as a Main Storage Provider
            // We use assert_noop to make sure that it not only returns the specific
            // error, but it also does not modify any storage
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::AlreadyRegistered
            );

            // We try to register her again as a BSP now
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::AlreadyRegistered
            );
        });
    }
}

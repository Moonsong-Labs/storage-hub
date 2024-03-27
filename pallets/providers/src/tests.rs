use crate::{
    mock::*,
    types::{StorageData, ValuePropId, ValueProposition},
    Error, Event,
};
use frame_support::pallet_prelude::Weight;
use frame_support::traits::{
    fungible::{InspectHold, Mutate},
    Get, OnFinalize, OnIdle, OnInitialize,
};
use frame_support::BoundedVec;
use frame_support::{assert_noop, assert_ok};
use storage_hub_traits::ProvidersInterface;

use crate::types::{BalanceOf, MaxMultiAddressAmount, MultiAddress};

type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;

// Pallet constants:
type SpMinDeposit = <Test as crate::Config>::SpMinDeposit;
type SpMinCapacity = <Test as crate::Config>::SpMinCapacity;
type DepositPerData = <Test as crate::Config>::DepositPerData;
type MaxMsps = <Test as crate::Config>::MaxMsps;
type MaxBsps = <Test as crate::Config>::MaxBsps;

/// This module holds the test cases for the signup of Main Storage Providers and Backup Storage Providers
mod sign_up {
    use super::*;

    #[test]
    fn msp_sign_up_works() {
        ExtBuilder::build().execute_with(|| {
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

            // Check that Alice is now a Storage Provider
            let alice_sp_id = StorageProviders::get_provider(alice);
            assert!(alice_sp_id.is_some());
            assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

            // Check the event was emitted
            System::assert_has_event(
                Event::<Test>::MspSignUpSuccess {
                    who: alice,
                    multiaddresses,
                    capacity: storage_amount,
                    value_prop,
                }
                .into(),
            );
        });
    }

    #[test]
    fn multiple_users_can_sign_up_as_msp() {
        ExtBuilder::build().execute_with(|| {
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
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount_alice: StorageData<Test> = 100;
            let storage_amount_bob: StorageData<Test> = 300;

            // Get the Account Id of Alice and check its balance
            let alice: AccountId = 0;
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Do the same for Bob
            let bob: AccountId = 1;
            assert_eq!(NativeBalance::free_balance(&bob), 10_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                0
            );

            // Alice is going to sign up as a Main Storage Provider with 100 StorageData units
            // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
            // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
            let deposit_for_storage_amount_alice: BalanceOf<Test> =
                <SpMinDeposit as Get<u128>>::get().saturating_add(
                    <DepositPerData as Get<u128>>::get().saturating_mul(
                        (storage_amount_alice - <SpMinCapacity as Get<u32>>::get()).into(),
                    ),
                );

            // Bob is going to sign up as a Main Storage Provider with 300 StorageData units
            // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
            // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
            let deposit_for_storage_amount_bob: BalanceOf<Test> =
                <SpMinDeposit as Get<u128>>::get().saturating_add(
                    <DepositPerData as Get<u128>>::get().saturating_mul(
                        (storage_amount_bob - <SpMinCapacity as Get<u32>>::get()).into(),
                    ),
                );

            // Sign up Alice as a Main Storage Provider
            assert_ok!(StorageProviders::msp_sign_up(
                RuntimeOrigin::signed(alice),
                storage_amount_alice,
                multiaddresses.clone(),
                value_prop.clone()
            ));

            // Sign up Bob as a Main Storage Provider
            assert_ok!(StorageProviders::msp_sign_up(
                RuntimeOrigin::signed(bob),
                storage_amount_bob,
                multiaddresses.clone(),
                value_prop.clone()
            ));

            // Check the new free balance of Alice
            assert_eq!(
                NativeBalance::free_balance(&alice),
                5_000_000 - deposit_for_storage_amount_alice
            );
            // Check the new held balance of Alice
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                deposit_for_storage_amount_alice
            );

            // Check the new free balance of Bob
            assert_eq!(
                NativeBalance::free_balance(&bob),
                10_000_000 - deposit_for_storage_amount_bob
            );
            // Check the new held balance of Bob
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                deposit_for_storage_amount_bob
            );

            // Check that Alice's event was emitted
            System::assert_has_event(
                Event::<Test>::MspSignUpSuccess {
                    who: alice,
                    multiaddresses: multiaddresses.clone(),
                    capacity: storage_amount_alice,
                    value_prop: value_prop.clone(),
                }
                .into(),
            );

            // Check that Bob's event was emitted
            System::assert_has_event(
                Event::<Test>::MspSignUpSuccess {
                    who: bob,
                    multiaddresses,
                    capacity: storage_amount_bob,
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
            let mut multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
                BoundedVec::new();
            multiaddresses.force_push(
                "/ip4/127.0.0.1/udp/1234"
                    .as_bytes()
                    .to_vec()
                    .try_into()
                    .unwrap(),
            );
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice and check its balance
            let alice: AccountId = 0;
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Alice is going to sign up as a Backup Storage Provider with 100 StorageData units
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

            // Check that the total capacity of the Backup Storage Providers has increased
            assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

            // Check the event was emitted
            System::assert_has_event(
                Event::<Test>::BspSignUpSuccess {
                    who: alice,
                    multiaddresses,
                    capacity: storage_amount,
                }
                .into(),
            );
        });
    }

    #[test]
    fn multiple_users_can_sign_up_as_bsp() {
        ExtBuilder::build().execute_with(|| {
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
            let storage_amount_alice: StorageData<Test> = 100;
            let storage_amount_bob: StorageData<Test> = 300;

            // Get the Account Id of Alice and check its balance
            let alice: AccountId = 0;
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Get the Account Id of Bob and check its balance
            let bob: AccountId = 1;
            assert_eq!(NativeBalance::free_balance(&bob), 10_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                0
            );

            // Alice is going to sign up as a Backup Storage Provider with 100 StorageData units
            // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
            // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
            let deposit_for_storage_amount_alice: BalanceOf<Test> =
                <SpMinDeposit as Get<u128>>::get().saturating_add(
                    <DepositPerData as Get<u128>>::get().saturating_mul(
                        (storage_amount_alice - <SpMinCapacity as Get<u32>>::get()).into(),
                    ),
                );

            // Bob is going to sign up as a Backup Storage Provider with 300 StorageData units
            // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
            // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
            let deposit_for_storage_amount_bob: BalanceOf<Test> =
                <SpMinDeposit as Get<u128>>::get().saturating_add(
                    <DepositPerData as Get<u128>>::get().saturating_mul(
                        (storage_amount_bob - <SpMinCapacity as Get<u32>>::get()).into(),
                    ),
                );

            // Sign up Alice as a Backup Storage Provider
            assert_ok!(StorageProviders::bsp_sign_up(
                RuntimeOrigin::signed(alice),
                storage_amount_alice,
                multiaddresses.clone(),
            ));

            // Check the new free balance of Alice
            assert_eq!(
                NativeBalance::free_balance(&alice),
                5_000_000 - deposit_for_storage_amount_alice
            );
            // Check the new held balance of Alice
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                deposit_for_storage_amount_alice
            );

            // Check that Alice is a Backup Storage Provider
            let alice_sp_id = StorageProviders::get_provider(alice);
            assert!(alice_sp_id.is_some());
            assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

            // Check that the total capacity of the Backup Storage Providers has increased
            assert_eq!(
                StorageProviders::get_total_bsp_capacity(),
                storage_amount_alice
            );

            // Check that Alice registration event has been emitted
            System::assert_has_event(
                Event::<Test>::BspSignUpSuccess {
                    who: alice,
                    multiaddresses: multiaddresses.clone(),
                    capacity: storage_amount_alice,
                }
                .into(),
            );

            // Check that Bob is not a Backup Storage Provider
            let bob_sp_id = StorageProviders::get_provider(bob);
            assert!(bob_sp_id.is_none());

            // Sign up Bob as a Backup Storage Provider
            assert_ok!(StorageProviders::bsp_sign_up(
                RuntimeOrigin::signed(bob),
                storage_amount_bob,
                multiaddresses.clone(),
            ));

            // Check the new free balance of Bob
            assert_eq!(
                NativeBalance::free_balance(&bob),
                10_000_000 - deposit_for_storage_amount_bob
            );
            // Check the new held balance of Bob
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                deposit_for_storage_amount_bob
            );

            // Check that Bob is now a Backup Storage Provider
            let bob_sp_id = StorageProviders::get_provider(bob);
            assert!(bob_sp_id.is_some());
            assert!(StorageProviders::is_provider(bob_sp_id.unwrap()));

            // Check that the total capacity of the Backup Storage Providers has increased
            assert_eq!(
                StorageProviders::get_total_bsp_capacity(),
                storage_amount_alice + storage_amount_bob
            );

            // Check that Bob registration event has been emitted
            System::assert_has_event(
                Event::<Test>::BspSignUpSuccess {
                    who: bob,
                    multiaddresses,
                    capacity: storage_amount_bob,
                }
                .into(),
            );
        });
    }

    #[test]
    fn msp_and_bsp_sign_up_fails_when_already_registered_as_msp() {
        ExtBuilder::build().execute_with(|| {
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

    #[test]
    fn msp_sign_up_fails_when_max_amount_of_msps_reached() {
        ExtBuilder::build().execute_with(|| {
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
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice
            let alice: AccountId = 0;

            // Sign up the maximum amount of Main Storage Providers
            for i in 1..<MaxMsps as Get<u32>>::get() + 1 {
                let account_id = i as AccountId;
                let account_new_balance = 1_000_000_000_000_000;
                assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
                    &account_id,
                    account_new_balance
                ));
                assert_ok!(StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(account_id),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ));
            }

            // Try to sign up Alice as a Main Storage Provider
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::MaxMspsReached
            );
        });
    }

    #[test]
    fn bsp_sign_up_fails_when_max_amount_of_bsps_reached() {
        ExtBuilder::build().execute_with(|| {
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
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice
            let alice: AccountId = 0;

            // Sign up the maximum amount of Main Storage Providers
            for i in 1..<MaxBsps as Get<u32>>::get() + 1 {
                let account_id = i as AccountId;
                let account_new_balance = 1_000_000_000_000_000;
                assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
                    &account_id,
                    account_new_balance
                ));
                assert_ok!(StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(account_id),
                    storage_amount,
                    multiaddresses.clone(),
                ));
            }

            // Try to sign up Alice as a Main Storage Provider
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::MaxBspsReached
            );
        });
    }

    #[test]
    fn msp_and_bsp_sign_up_fails_when_under_min_capacity() {
        ExtBuilder::build().execute_with(|| {
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
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 1;

            // Get the Account Id of Alice
            let alice: AccountId = 0;

            // Try to sign up Alice as a Main Storage Provider with less than the minimum capacity
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::StorageTooLow
            );

            // Try to sign up Alice as a Backup Storage Provider with less than the minimum capacity
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::StorageTooLow
            );
        });
    }

    #[test]
    fn msp_and_bsp_sign_up_fails_when_under_needed_balance() {
        ExtBuilder::build().execute_with(|| {
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
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Helen (who has no balance)
            let helen: AccountId = 7;

            // Try to sign up Helen as a Main Storage Provider
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(helen),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::NotEnoughBalance
            );

            // Try to sign up Helen as a Backup Storage Provider
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(helen),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::NotEnoughBalance
            );
        });
    }

    #[test]
    fn msp_and_bsp_sign_up_fails_when_passing_no_multiaddresses() {
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

            // Try to sign up Alice as a Main Storage Provider with no multiaddresses
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::NoMultiAddress
            );

            // Try to sign up Alice as a Backup Storage Provider with no multiaddresses
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::NoMultiAddress
            );
        });
    }

    // TODO: Test invalid multiaddresses (after developing the multiaddress checking logic)
    /* #[test]
    fn msp_and_bsp_sign_up_fails_when_passing_an_invalid_multiaddress() {
        ExtBuilder::build().execute_with(|| {
            // Initialize variables:
            let mut multiaddresses: BoundedVec<MultiAddress<Test>, MaxMultiAddressAmount<Test>> =
                BoundedVec::new();
            let valid_multiaddress: Multiaddr = "/ip4/127.0.0.1/udp/1234".parse().unwrap();
            let invalid_multiaddress = "/ip4/127.0.0.1/udp/1234".as_bytes().to_vec();
            multiaddresses.force_push(valid_multiaddress.to_vec().try_into().unwrap());
            multiaddresses.force_push(invalid_multiaddress.try_into().unwrap());

            let value_prop: ValueProposition<Test> = ValueProposition {
                identifier: ValuePropId::<Test>::default(),
                data_limit: 10,
                protocols: BoundedVec::new(),
            };
            let storage_amount: StorageData<Test> = 100;

            // Get the Account Id of Alice
            let alice: AccountId = 0;

            // Try to sign up Alice as a Main Storage Provider with an invalid multiaddress
            assert_noop!(
                StorageProviders::msp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                    value_prop.clone()
                ),
                Error::<Test>::InvalidMultiAddress
            );

            // Try to sign up Alice as a Backup Storage Provider with an invalid multiaddress
            assert_noop!(
                StorageProviders::bsp_sign_up(
                    RuntimeOrigin::signed(alice),
                    storage_amount,
                    multiaddresses.clone(),
                ),
                Error::<Test>::InvalidMultiAddress
            );
        });
    } */
}

/// This module holds the test cases for the sign-off of Main Storage Providers and Backup Storage Providers
mod sign_off {
    use super::*;

    #[test]
    fn msp_sign_off_works() {
        ExtBuilder::build().execute_with(|| {
            // Register Alice as MSP:
            let alice: AccountId = 0;
            let storage_amount: StorageData<Test> = 100;
            let deposit_amount = register_account_as_msp(alice, storage_amount);

            // Check the new free and held balance of Alice
            assert_eq!(
                NativeBalance::free_balance(&alice),
                5_000_000 - deposit_amount
            );
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                deposit_amount
            );

            // Sign off Alice as a Main Storage Provider
            assert_ok!(StorageProviders::msp_sign_off(RuntimeOrigin::signed(alice)));

            // Check the new free and held balance of Alice
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Check that Alice is not a Main Storage Provider anymore
            let alice_sp_id = StorageProviders::get_provider(alice);
            assert!(alice_sp_id.is_none());

            // Check the MSP Sign Off event was emitted
            System::assert_has_event(Event::<Test>::MspSignOffSuccess { who: alice }.into());
        });
    }

    #[test]
    fn bsp_sign_off_works() {
        ExtBuilder::build().execute_with(|| {
            // Register Alice as BSP:
            let alice: AccountId = 0;
            let storage_amount: StorageData<Test> = 100;
            let deposit_amount = register_account_as_bsp(alice, storage_amount);

            // Check the new free and held balance of Alice
            assert_eq!(
                NativeBalance::free_balance(&alice),
                5_000_000 - deposit_amount
            );
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                deposit_amount
            );

            // TODO: Check the capacity of all the BSPs

            // Sign off Alice as a Backup Storage Provider
            assert_ok!(StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)));

            // TODO: Check the new capacity of all BSPs

            // Check the new free and held balance of Alice
            assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
            assert_eq!(
                NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                0
            );

            // Check that Alice is not a Backup Storage Provider anymore
            let alice_sp_id = StorageProviders::get_provider(alice);
            assert!(alice_sp_id.is_none());

            // Check the BSP Sign Off event was emitted
            System::assert_has_event(Event::<Test>::BspSignOffSuccess { who: alice }.into());
        });
    }
}

// Helper functions for testing:

/// Helper function that registers an account as a Main Storage Provider, with storage_amount StorageData units
///
/// Returns the deposit amount that was utilized from the account's balance
fn register_account_as_msp(
    account: AccountId,
    storage_amount: StorageData<Test>,
) -> BalanceOf<Test> {
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
        identifier: ValuePropId::<Test>::default(),
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

    // Sign up the account as a Main Storage Provider
    assert_ok!(StorageProviders::msp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        value_prop.clone()
    ));

    // Check that the sign up event was emitted
    System::assert_last_event(
        Event::<Test>::MspSignUpSuccess {
            who: account,
            multiaddresses,
            capacity: storage_amount,
            value_prop,
        }
        .into(),
    );

    // Return the deposit amount that was utilized from the account's balance
    deposit_for_storage_amount
}

/// Helper function that registers an account as a Backup Storage Provider, with storage_amount StorageData units
///
/// Returns the deposit amount that was utilized from the account's balance
fn register_account_as_bsp(
    account: AccountId,
    storage_amount: StorageData<Test>,
) -> BalanceOf<Test> {
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

    // Sign up the account as a Backup Storage Provider
    assert_ok!(StorageProviders::bsp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
    ));

    // Check that the sign up event was emitted
    System::assert_last_event(
        Event::<Test>::BspSignUpSuccess {
            who: account,
            multiaddresses,
            capacity: storage_amount,
        }
        .into(),
    );

    // Return the deposit amount that was utilized from the account's balance
    deposit_for_storage_amount
}

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

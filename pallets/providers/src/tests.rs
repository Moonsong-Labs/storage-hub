use crate::{
    mock::*,
    types::{
        BackupStorageProvider, BalanceOf, MainStorageProvider, MaxMultiAddressAmount, MultiAddress,
        StorageData, StorageProvider, ValuePropId, ValueProposition,
    },
    Error, Event,
};

use frame_support::pallet_prelude::Weight;
use frame_support::traits::{
    fungible::{InspectHold, Mutate},
    Get, OnFinalize, OnIdle, OnInitialize,
};
use frame_support::{assert_noop, assert_ok, dispatch::Pays, BoundedVec};
use frame_system::pallet_prelude::BlockNumberFor;
use shp_traits::MutateProvidersInterface;
use shp_traits::ProvidersInterface;

type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;

// Pallet constants:
type SpMinDeposit = <Test as crate::Config>::SpMinDeposit;
type SpMinCapacity = <Test as crate::Config>::SpMinCapacity;
type DepositPerData = <Test as crate::Config>::DepositPerData;
type MaxMsps = <Test as crate::Config>::MaxMsps;
type MaxBsps = <Test as crate::Config>::MaxBsps;
type MinBlocksBetweenCapacityChanges = <Test as crate::Config>::MinBlocksBetweenCapacityChanges;

// Runtime constants:
// This is the duration of an epoch in blocks, a constant from the runtime configuration that we mock here
const EPOCH_DURATION_IN_BLOCKS: BlockNumberFor<Test> = 10;

// Extra constants:
// This is the amount of blocks that we need to advance to have a valid randomness value. In an actual runtime, this would be dependent on BABE
const BLOCKS_BEFORE_RANDOMNESS_VALID: BlockNumberFor<Test> = 3;

/// This module holds the test cases for the signup of Main Storage Providers and Backup Storage Providers
mod sign_up {

    use super::*;

    /// This module holds the tests cases for signing up that result in successful registrations
    mod success {
        use super::*;

        /// This module holds the success cases for Main Storage Providers
        mod msp {
            use super::*;
            #[test]
            fn msp_request_sign_up_works() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
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

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check the event was emitted
                    System::assert_has_event(
                        Event::<Test>::MspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount,
                            value_prop: value_prop.clone(),
                        }
                        .into(),
                    );

                    // Check that Alice's request is in the requests list and matches the info provided
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.is_ok());
                    assert_eq!(
                        alice_sign_up_request.unwrap(),
                        (
                            StorageProvider::MainStorageProvider(MainStorageProvider {
                                buckets: BoundedVec::new(),
                                capacity: storage_amount,
                                data_used: 0,
                                multiaddresses,
                                value_prop,
                                last_capacity_change: current_block,
                            }),
                            current_block
                        )
                    );
                });
            }

            #[test]
            fn msp_confirm_sign_up_works_when_passing_provider_account() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
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

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the account as a Main Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice)
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
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
            fn msp_confirm_sign_up_works_when_not_passing_provider_account() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
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

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the account as a Main Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        None
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
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
            fn multiple_users_can_request_to_sign_up_as_msp() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Alice is going to request to sign up as a Main Storage Provider with 100 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Main Storage Provider with 300 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request sign up Bob as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
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
                        Event::<Test>::MspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                        }
                        .into(),
                    );

                    // Check that Bob's event was emitted
                    System::assert_has_event(
                        Event::<Test>::MspRequestSignUpSuccess {
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
            fn msp_cancel_sign_up_works() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Check that Alice's request to sign up as a Main Storage Provider exists and is the one we just created
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.as_ref().is_ok_and(|request| request.0
                        == StorageProvider::MainStorageProvider(MainStorageProvider {
                            buckets: BoundedVec::new(),
                            capacity: storage_amount,
                            data_used: 0,
                            multiaddresses: multiaddresses.clone(),
                            value_prop: value_prop.clone(),
                            last_capacity_change: current_block
                        })));
                    assert!(alice_sign_up_request.is_ok_and(|request| request.1 == current_block));

                    // Cancel the sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(
                        alice
                    )));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that Alice's sign up request no longer exists
                    assert!(StorageProviders::get_sign_up_request(&alice)
                        .is_err_and(|err| { matches!(err, Error::<Test>::SignUpNotRequested) }));

                    // Check that the cancel MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::SignUpRequestCanceled { who: alice }.into(),
                    );
                });
            }
        }

        /// This module holds the success cases for Backup Storage Providers
        mod bsp {
            use super::*;

            #[test]
            fn bsp_request_sign_up_works() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
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

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that the total capacity of the Backup Storage Providers has NOT yet increased
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), 0);

                    // Check the event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount,
                        }
                        .into(),
                    );

                    // Check that Alice's request is in the requests list and matches the info provided
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.is_ok());
                    assert_eq!(
                        alice_sign_up_request.unwrap(),
                        (
                            StorageProvider::BackupStorageProvider(BackupStorageProvider {
                                root: Default::default(),
                                capacity: storage_amount,
                                data_used: 0,
                                multiaddresses,
                                last_capacity_change: current_block,
                            }),
                            current_block
                        )
                    );
                });
            }

            #[test]
            fn bsp_confirm_sign_up_works_when_passing_provider_account() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
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

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that the total capacity of the Backup Storage Providers has NOT yet increased
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), 0);

                    // Check the event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount,
                        }
                        .into(),
                    );

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice)
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the total capacity of the Backup Storage Providers has now increased
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

                    // Check that the confirm BSP sign up event was emitted
                    System::assert_last_event(
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
            fn bsp_confirm_sign_up_works_when_not_passing_provider_account() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
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

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that the total capacity of the Backup Storage Providers has NOT yet increased
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), 0);

                    // Check the event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount,
                        }
                        .into(),
                    );

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        None
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the total capacity of the Backup Storage Providers has now increased
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

                    // Check that the confirm BSP sign up event was emitted
                    System::assert_last_event(
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
            fn multiple_users_can_request_to_sign_up_as_bsp() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), 10_000_000);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Alice is going to request to sign up as a Backup Storage Provider with 100 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Backup Storage Provider with 300 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
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
                        Event::<Test>::BspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                        }
                        .into(),
                    );

                    // Check that Bob's event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspRequestSignUpSuccess {
                            who: bob,
                            multiaddresses,
                            capacity: storage_amount_bob,
                        }
                        .into(),
                    );
                });
            }

            #[test]
            fn bsp_cancel_sign_up_works() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                    ));

                    // Check that Alice's request to sign up as a Backup Storage Provider exists and is the one we just created
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.as_ref().is_ok_and(|request| request.0
                        == StorageProvider::BackupStorageProvider(BackupStorageProvider {
                            capacity: storage_amount,
                            data_used: 0,
                            multiaddresses: multiaddresses.clone(),
                            root: Default::default(),
                            last_capacity_change: current_block
                        })));
                    assert!(alice_sign_up_request.is_ok_and(|request| request.1 == current_block));

                    // Cancel the sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(
                        alice
                    )));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that Alice's sign up request no longer exists
                    assert!(StorageProviders::get_sign_up_request(&alice)
                        .is_err_and(|err| { matches!(err, Error::<Test>::SignUpNotRequested) }));

                    // Check that the cancel BSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::SignUpRequestCanceled { who: alice }.into(),
                    );
                });
            }
        }
        /// This module holds the success cases for functions that test both Main Storage Providers and Backup Storage Providers
        mod msp_and_bsp {
            use super::*;

            #[test]
            fn multiple_users_can_request_to_sign_up_as_msp_and_bsp_at_the_same_time() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Alice is going to request to sign up as a Main Storage Provider with 100 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Backup Storage Provider with 300 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
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
                        Event::<Test>::MspRequestSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                        }
                        .into(),
                    );

                    // Check that Bob's event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspRequestSignUpSuccess {
                            who: bob,
                            multiaddresses,
                            capacity: storage_amount_bob,
                        }
                        .into(),
                    );
                });
            }

            #[test]
            fn multiple_users_can_request_to_sign_up_and_one_can_confirm() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                    ));

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the account as a Main Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice)
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                        }
                        .into(),
                    );

                    // Check that Bob is still NOT a Storage Provider
                    let bob_sp_id = StorageProviders::get_provider(bob);
                    assert!(bob_sp_id.is_none());
                });
            }

            #[test]
            fn multiple_users_can_request_to_sign_up_and_multiple_can_confirm() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                    ));

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the account as a Main Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice)
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                        }
                        .into(),
                    );

                    // Confirm the sign up of the account as a Backup Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(bob),
                        Some(bob)
                    ));

                    // Check that Bob is now a Storage Provider
                    let bob_sp_id = StorageProviders::get_provider(bob);
                    assert!(bob_sp_id.is_some());
                    assert!(StorageProviders::is_provider(bob_sp_id.unwrap()));

                    // Check that the confirm BSP sign up event was emitted
                    System::assert_last_event(
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
            fn multiple_users_can_request_to_sign_up_and_one_confirm_other_cancel() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                    ));

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the account as a Main Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice)
                    ));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                        }
                        .into(),
                    );

                    // Cancel the sign up of Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(bob)));

                    // Check that Bob is still not a Storage Provider
                    let bob_sp_id = StorageProviders::get_provider(bob);
                    assert!(bob_sp_id.is_none());

                    // Check that Bob's request no longer exists
                    assert!(StorageProviders::get_sign_up_request(&bob)
                        .is_err_and(|err| { matches!(err, Error::<Test>::SignUpNotRequested) }));

                    // Check that the cancel BSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::SignUpRequestCanceled { who: bob }.into(),
                    );
                });
            }

            #[test]
            fn confirm_sign_up_is_free_if_successful() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the account as a Main Storage Provider
                    let confirm_result = StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice),
                    );
                    assert_eq!(confirm_result, Ok(Pays::No.into()));

                    // Check that Alice is now a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
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
            fn msp_or_bsp_can_cancel_expired_sign_up_request() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Alice is going to request to sign up as a Main Storage Provider with 100 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Main Storage Provider with 300 StorageData units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u32>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone()
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

                    // Advance enough blocks for randomness to be too old (expiring the request)
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + (EPOCH_DURATION_IN_BLOCKS * 2),
                    );

                    // Try to confirm the sign up of Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::SignUpRequestExpired
                    );

                    // Try to confirm the sign up of Bob as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(RuntimeOrigin::signed(bob), Some(bob)),
                        Error::<Test>::SignUpRequestExpired
                    );

                    // Cancel the sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(
                        alice
                    )));

                    // Check Alice's new free balance
                    assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
                    // Check Alice's new held balance
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Check that Alice's sign up request no longer exists
                    assert!(StorageProviders::get_sign_up_request(&alice)
                        .is_err_and(|err| { matches!(err, Error::<Test>::SignUpNotRequested) }));

                    // Check that the cancel MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::SignUpRequestCanceled { who: alice }.into(),
                    );

                    // Cancel the sign up of Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(bob)));

                    // Check Bob's new free balance
                    assert_eq!(NativeBalance::free_balance(&bob), 10_000_000);
                    // Check Bob's new held balance
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Check that Bob's sign up request no longer exists
                    assert!(StorageProviders::get_sign_up_request(&bob)
                        .is_err_and(|err| { matches!(err, Error::<Test>::SignUpNotRequested) }));

                    // Check that the cancel BSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::SignUpRequestCanceled { who: bob }.into(),
                    );
                });
            }
        }
    }

    /// This module holds the tests cases for signing up that result in failed registrations
    mod failure {
        use super::*;

        /// This module holds the failure cases for Main Storage Providers
        mod msp {
            use super::*;

            #[test]
            fn msp_confirm_sign_up_fails_if_randomness_request_is_too_recent() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Advance blocks but not enough for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID
                            - 1,
                    );

                    // Try to confirm the sign up of the account as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::RandomnessNotValidYet
                    );

                    // Check that Alice is still not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());
                });
            }

            #[test]
            fn msp_confirm_sign_up_fails_if_randomness_is_too_old() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Advance enough blocks for randomness to be too old (expiring the request)
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + (EPOCH_DURATION_IN_BLOCKS * 2),
                    );

                    // Try to confirm the sign up of the account as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::SignUpRequestExpired
                    );

                    // Check that Alice is still not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());
                });
            }

            #[test]
            fn msp_request_sign_up_fails_when_another_request_pending() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request to sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Try to request to sign up Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ),
                        Error::<Test>::SignUpRequestPending
                    );
                });
            }

            #[test]
            fn msp_request_sign_up_fails_when_max_amount_of_msps_reached() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request to sign up the maximum amount of Main Storage Providers
                    for i in 1..<MaxMsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        let account_new_balance = 1_000_000_000_000_000;
                        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
                            &account_id,
                            account_new_balance
                        ));
                        assert_ok!(StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(account_id),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ));
                    }

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the maximum amount of Main Storage Providers
                    for i in 1..<MaxMsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        assert_ok!(StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(account_id),
                            Some(account_id)
                        ));
                    }

                    // Try to request to sign up Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
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
            fn msp_confirm_sign_up_fails_when_max_amount_of_msps_reached() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request to sign up Alice
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request to sign up the maximum amount of Main Storage Providers
                    for i in 1..<MaxMsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        let account_new_balance = 1_000_000_000_000_000;
                        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
                            &account_id,
                            account_new_balance
                        ));
                        assert_ok!(StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(account_id),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ));
                    }

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the maximum amount of Main Storage Providers
                    for i in 1..<MaxMsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        assert_ok!(StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(account_id),
                            Some(account_id)
                        ));
                    }

                    // Try to confirm the sign up of Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::MaxMspsReached
                    );
                });
            }
        }

        /// This module holds the failure cases for Backup Storage Providers
        mod bsp {
            use super::*;

            #[test]
            fn bsp_confirm_sign_up_fails_if_randomness_request_is_too_recent() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Advance blocks but not enough for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID
                            - 1,
                    );

                    // Try to confirm the sign up of the account as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::RandomnessNotValidYet
                    );

                    // Check that Alice is still not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());
                });
            }

            #[test]
            fn bsp_confirm_sign_up_fails_if_randomness_is_too_old() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Advance enough blocks for randomness to be too old (expiring the request)
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + (EPOCH_DURATION_IN_BLOCKS * 2),
                    );

                    // Try to confirm the sign up of Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::SignUpRequestExpired
                    );

                    // Check that Alice is still not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());
                });
            }

            #[test]
            fn bsp_request_sign_up_fails_when_another_request_pending() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request to sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                    ));

                    // Try to request to sign up Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                        ),
                        Error::<Test>::SignUpRequestPending
                    );
                });
            }

            #[test]
            fn bsp_request_sign_up_fails_when_max_amount_of_bsps_reached() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request to sign up the maximum amount of Backup Storage Providers
                    for i in 1..<MaxBsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        let account_new_balance = 1_000_000_000_000_000;
                        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
                            &account_id,
                            account_new_balance
                        ));
                        assert_ok!(StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(account_id),
                            storage_amount,
                            multiaddresses.clone(),
                        ));
                    }

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the maximum amount of Backup Storage Providers
                    for i in 1..<MaxBsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        assert_ok!(StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(account_id),
                            Some(account_id)
                        ));
                    }

                    // Try to request to sign up Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                        ),
                        Error::<Test>::MaxBspsReached
                    );
                });
            }

            #[test]
            fn bsp_confirm_sign_up_fails_when_max_amount_of_bsps_reached() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request to sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                    ));

                    // Request to sign up the maximum amount of Backup Storage Providers
                    for i in 1..<MaxBsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        let account_new_balance = 1_000_000_000_000_000;
                        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
                            &account_id,
                            account_new_balance
                        ));
                        assert_ok!(StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(account_id),
                            storage_amount,
                            multiaddresses.clone(),
                        ));
                    }

                    // Advance enough blocks for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID,
                    );

                    // Confirm the sign up of the maximum amount of Backup Storage Providers
                    for i in 1..<MaxBsps as Get<u32>>::get() + 1 {
                        let account_id = i as AccountId;
                        assert_ok!(StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(account_id),
                            Some(account_id)
                        ));
                    }

                    // Try to confirm to sign up Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::MaxBspsReached
                    );
                });
            }
        }

        /// This module holds the failure cases for functions that test both Main Storage Providers and Backup Storage Providers
        mod msp_and_bsp {
            use super::*;

            #[test]
            fn confirm_sign_up_is_not_free_if_it_fails() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Advance blocks but not enough for randomness to be valid
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + BLOCKS_BEFORE_RANDOMNESS_VALID
                            - 1,
                    );

                    // Try to confirm the sign up of the account as a Main Storage Provider
                    let confirm_result = StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(alice),
                        Some(alice),
                    );
                    assert!(
                        confirm_result.is_err_and(|result| result.post_info.pays_fee == Pays::Yes)
                    );

                    // Check that Alice is still not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());
                });
            }

            #[test]
            fn confirm_sign_up_fails_if_request_does_not_exist() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice
                    let alice: AccountId = 0;

                    // Check that Alice does not have a pending sign up request
                    assert!(StorageProviders::get_sign_up_request(&alice)
                        .is_err_and(|err| { matches!(err, Error::<Test>::SignUpNotRequested) }));

                    // Try to confirm the sign up of Alice
                    assert_noop!(
                        StorageProviders::confirm_sign_up(
                            RuntimeOrigin::signed(alice),
                            Some(alice)
                        ),
                        Error::<Test>::SignUpNotRequested
                    );
                });
            }

            #[test]
            fn msp_and_bsp_request_sign_up_fails_when_already_registered() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice and Bob
                    let alice: AccountId = 0;
                    let bob: AccountId = 1;

                    // Register Alice as a Main Storage Provider
                    let (_alice_deposit, alice_msp) = register_account_as_msp(alice, 100);
                    // Register Bob as a Backup Storage Provider
                    let (_bob_deposit, bob_bsp) = register_account_as_bsp(bob, 100);

                    // Try to request to sign up Alice again as a Main Storage Provider
                    // We use assert_noop to make sure that it not only returns the specific
                    // error, but it also does not modify any storage
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            alice_msp.capacity,
                            alice_msp.multiaddresses.clone(),
                            alice_msp.value_prop.clone()
                        ),
                        Error::<Test>::AlreadyRegistered
                    );

                    // We try to request to sign her up as a BSP now
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            alice_msp.capacity,
                            alice_msp.multiaddresses.clone(),
                        ),
                        Error::<Test>::AlreadyRegistered
                    );

                    // Try to request to sign up Bob as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(bob),
                            bob_bsp.capacity,
                            bob_bsp.multiaddresses.clone(),
                            ValueProposition {
                                identifier: ValuePropId::<Test>::default(),
                                data_limit: 10,
                                protocols: BoundedVec::new(),
                            }
                        ),
                        Error::<Test>::AlreadyRegistered
                    );

                    // And as a BSP
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(bob),
                            bob_bsp.capacity,
                            bob_bsp.multiaddresses.clone(),
                        ),
                        Error::<Test>::AlreadyRegistered
                    );
                });
            }

            #[test]
            fn msp_and_bsp_request_sign_up_fails_when_already_requested() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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

                    // Get the Account Id of Alice and Bob
                    let alice: AccountId = 0;
                    let bob: AccountId = 1;

                    // Request to sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone()
                    ));

                    // Request to sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount,
                        multiaddresses.clone(),
                    ));

                    // Try to request to sign up Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                        ),
                        Error::<Test>::SignUpRequestPending
                    );

                    // Try to request to sign up Bob as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(bob),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ),
                        Error::<Test>::SignUpRequestPending
                    );
                });
            }

            #[test]
            fn msp_and_bsp_request_sign_up_fails_when_under_min_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ),
                        Error::<Test>::StorageTooLow
                    );

                    // Try to sign up Alice as a Backup Storage Provider with less than the minimum capacity
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                        ),
                        Error::<Test>::StorageTooLow
                    );
                });
            }

            #[test]
            fn msp_and_bsp_request_sign_up_fails_when_under_needed_balance() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let mut multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(helen),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ),
                        Error::<Test>::NotEnoughBalance
                    );

                    // Try to sign up Helen as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(helen),
                            storage_amount,
                            multiaddresses.clone(),
                        ),
                        Error::<Test>::NotEnoughBalance
                    );
                });
            }

            #[test]
            fn msp_and_bsp_request_sign_up_fails_when_passing_no_multiaddresses() {
                ExtBuilder::build().execute_with(|| {
                    // Initialize variables:
                    let multiaddresses: BoundedVec<
                        MultiAddress<Test>,
                        MaxMultiAddressAmount<Test>,
                    > = BoundedVec::new();
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
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone()
                        ),
                        Error::<Test>::NoMultiAddress
                    );

                    // Try to sign up Alice as a Backup Storage Provider with no multiaddresses
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
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
    }
}

/// This module holds the test cases for the sign-off of Main Storage Providers and Backup Storage Providers
mod sign_off {

    use super::*;

    /// This module holds the success cases for signing off Main Storage Providers and Backup Storage Providers
    mod success {
        use super::*;

        /// This module holds the success cases for Main Storage Providers
        mod msp {
            use super::*;

            #[test]
            fn msp_sign_off_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let storage_amount: StorageData<Test> = 100;
                    let (deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the counter of registered MSPs
                    assert_eq!(StorageProviders::get_msp_count(), 1);

                    // Sign off Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::msp_sign_off(RuntimeOrigin::signed(alice)));

                    // Check the new free and held balance of Alice
                    assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Check that the counter of registered MSPs has decreased
                    assert_eq!(StorageProviders::get_msp_count(), 0);

                    // Check that Alice is not a Main Storage Provider anymore
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check the MSP Sign Off event was emitted
                    System::assert_has_event(
                        Event::<Test>::MspSignOffSuccess { who: alice }.into(),
                    );
                });
            }
        }

        /// This module holds the success cases for Backup Storage Providers
        mod bsp {
            use super::*;

            #[test]
            fn bsp_sign_off_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let storage_amount: StorageData<Test> = 100;
                    let (deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the capacity of all the BSPs
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

                    // Check the counter of registered BSPs
                    assert_eq!(StorageProviders::get_bsp_count(), 1);

                    // Sign off Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)));

                    // Check the new capacity of all BSPs
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), 0);

                    // Check the new free and held balance of Alice
                    assert_eq!(NativeBalance::free_balance(&alice), 5_000_000);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Check that Alice is not a Backup Storage Provider anymore
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that the counter of registered BSPs has decreased
                    assert_eq!(StorageProviders::get_bsp_count(), 0);

                    // Check the BSP Sign Off event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspSignOffSuccess { who: alice }.into(),
                    );
                });
            }
        }
    }

    /// This module holds the failure cases for signing off Main Storage Providers and Backup Storage Providers
    mod failure {
        use super::*;

        /// This module holds the failure cases for Main Storage Providers
        mod msp {
            use super::*;

            #[test]
            fn msp_sign_off_fails_when_not_registered_as_msp() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice
                    let alice: AccountId = 0;

                    // Try to sign off Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::msp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::NotRegistered
                    );
                });
            }

            #[test]
            fn msp_sign_off_fails_when_it_still_has_used_storage() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let storage_amount: StorageData<Test> = 100;
                    let (deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the counter of registered MSPs
                    assert_eq!(StorageProviders::get_msp_count(), 1);

                    // Check that Alice does not have any used storage
                    let alice_sp_id = StorageProviders::get_provider(alice).unwrap();
                    assert_eq!(
                        StorageProviders::get_used_storage_of_msp(&alice_sp_id).unwrap(),
                        0
                    );

                    // Add used storage to Alice (simulating that she has accepted to store a file)
                    assert_ok!(
                        <StorageProviders as MutateProvidersInterface>::increase_data_used(
                            &alice, 10
                        )
                    );

                    // Try to sign off Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::msp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::StorageStillInUse
                    );

                    // Make sure that Alice is still registered as a Main Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the counter of registered MSPs has not changed
                    assert_eq!(StorageProviders::get_msp_count(), 1);
                });
            }
        }

        /// This module holds the failure cases for Backup Storage Providers
        mod bsp {
            use super::*;

            #[test]
            fn bsp_sign_off_fails_when_not_registered_as_bsp() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice
                    let alice: AccountId = 0;

                    // Try to sign off Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::NotRegistered
                    );
                });
            }

            #[test]
            fn bsp_sign_off_fails_when_it_still_has_used_storage() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let storage_amount: StorageData<Test> = 100;
                    let (deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the counter of registered BSPs
                    assert_eq!(StorageProviders::get_bsp_count(), 1);

                    // Check the total capacity of the Backup Storage Providers
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

                    // Check that Alice does not have any used storage
                    let alice_sp_id = StorageProviders::get_provider(alice).unwrap();
                    assert_eq!(
                        StorageProviders::get_used_storage_of_bsp(&alice_sp_id).unwrap(),
                        0
                    );

                    // Add used storage to Alice (simulating that she has accepted to store a file)
                    assert_ok!(
                        <StorageProviders as MutateProvidersInterface>::increase_data_used(
                            &alice, 10
                        )
                    );

                    // Try to sign off Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::StorageStillInUse
                    );

                    // Make sure that Alice is still registered as a Backup Storage Provider
                    let alice_sp_id = StorageProviders::get_provider(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Make sure the total capacity of the Backup Storage Providers has not changed
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

                    // Check that the counter of registered BSPs has not changed
                    assert_eq!(StorageProviders::get_bsp_count(), 1);
                });
            }
        }
    }
}

mod change_capacity {

    use super::*;

    /// This module holds the success cases for changing the capacity of Main Storage Providers and Backup Storage Providers
    mod success {
        use super::*;

        /// This module holds the success cases for changing the capacity of Main Storage Providers
        mod msp {
            use super::*;

            #[test]
            fn msp_increase_change_capacity_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let increased_storage_amount: StorageData<Test> = 200;
                    let (old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - old_deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        old_deposit_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Change the capacity of Alice
                    assert_ok!(StorageProviders::change_capacity(
                        RuntimeOrigin::signed(alice),
                        increased_storage_amount
                    ));

                    // Get the deposit amount for the new storage
                    let deposit_for_increased_storage: BalanceOf<Test> = <SpMinDeposit as Get<
                        u128,
                    >>::get(
                    )
                    .saturating_add(<DepositPerData as Get<u128>>::get().saturating_mul(
                        (increased_storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_for_increased_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_increased_storage
                    );

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            old_capacity: old_storage_amount,
                            new_capacity: increased_storage_amount,
                            next_block_when_change_allowed:
                                frame_system::Pallet::<Test>::block_number()
                                    + <MinBlocksBetweenCapacityChanges as Get<
                                        BlockNumberFor<Test>,
                                    >>::get(),
                        }
                        .into(),
                    );
                });
            }

            #[test]
            fn msp_decrease_change_capacity_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let decreased_storage_amount: StorageData<Test> = 50;
                    let (old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - old_deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        old_deposit_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Change the capacity of Alice
                    assert_ok!(StorageProviders::change_capacity(
                        RuntimeOrigin::signed(alice),
                        decreased_storage_amount
                    ));

                    // Get the deposit amount for the new storage
                    let deposit_for_decreased_storage: BalanceOf<Test> = <SpMinDeposit as Get<
                        u128,
                    >>::get(
                    )
                    .saturating_add(<DepositPerData as Get<u128>>::get().saturating_mul(
                        (decreased_storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_for_decreased_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_decreased_storage
                    );

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            old_capacity: old_storage_amount,
                            new_capacity: decreased_storage_amount,
                            next_block_when_change_allowed:
                                frame_system::Pallet::<Test>::block_number()
                                    + <MinBlocksBetweenCapacityChanges as Get<
                                        BlockNumberFor<Test>,
                                    >>::get(),
                        }
                        .into(),
                    );
                });
            }

            #[test]
            fn msp_decrease_change_capacity_to_exactly_minimum_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 500;
                    let minimum_storage_amount: StorageData<Test> =
                        <SpMinCapacity as Get<u32>>::get();
                    let (old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - old_deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        old_deposit_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Change the capacity of Alice
                    assert_ok!(StorageProviders::change_capacity(
                        RuntimeOrigin::signed(alice),
                        minimum_storage_amount
                    ));

                    // Get the deposit amount for the minimum storage
                    let deposit_for_minimum_storage: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get();

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_for_minimum_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_minimum_storage
                    );

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            old_capacity: old_storage_amount,
                            new_capacity: minimum_storage_amount,
                            next_block_when_change_allowed:
                                frame_system::Pallet::<Test>::block_number()
                                    + <MinBlocksBetweenCapacityChanges as Get<
                                        BlockNumberFor<Test>,
                                    >>::get(),
                        }
                        .into(),
                    );
                });
            }
        }
        /// This module holds the success cases for changing the capacity of Backup Storage Providers
        mod bsp {
            use super::*;

            #[test]
            fn bsp_increase_change_capacity_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let increased_storage_amount: StorageData<Test> = 200;
                    let (old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - old_deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        old_deposit_amount
                    );

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Change the capacity of Alice
                    assert_ok!(StorageProviders::change_capacity(
                        RuntimeOrigin::signed(alice),
                        increased_storage_amount
                    ));

                    // Get the deposit amount for the new storage
                    let deposit_for_increased_storage: BalanceOf<Test> = <SpMinDeposit as Get<
                        u128,
                    >>::get(
                    )
                    .saturating_add(<DepositPerData as Get<u128>>::get().saturating_mul(
                        (increased_storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_for_increased_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_increased_storage
                    );

                    // Check the new total capacity of the network (all BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        increased_storage_amount
                    );

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            old_capacity: old_storage_amount,
                            new_capacity: increased_storage_amount,
                            next_block_when_change_allowed:
                                frame_system::Pallet::<Test>::block_number()
                                    + <MinBlocksBetweenCapacityChanges as Get<
                                        BlockNumberFor<Test>,
                                    >>::get(),
                        }
                        .into(),
                    );
                });
            }

            #[test]
            fn bsp_decrease_change_capacity_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let decreased_storage_amount: StorageData<Test> = 50;
                    let (old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - old_deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        old_deposit_amount
                    );

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Change the capacity of Alice
                    assert_ok!(StorageProviders::change_capacity(
                        RuntimeOrigin::signed(alice),
                        decreased_storage_amount
                    ));

                    // Get the deposit amount for the new storage
                    let deposit_for_decreased_storage: BalanceOf<Test> = <SpMinDeposit as Get<
                        u128,
                    >>::get(
                    )
                    .saturating_add(<DepositPerData as Get<u128>>::get().saturating_mul(
                        (decreased_storage_amount - <SpMinCapacity as Get<u32>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_for_decreased_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_decreased_storage
                    );

                    // Check the new total capacity of the network (all BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        decreased_storage_amount
                    );

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            old_capacity: old_storage_amount,
                            new_capacity: decreased_storage_amount,
                            next_block_when_change_allowed:
                                frame_system::Pallet::<Test>::block_number()
                                    + <MinBlocksBetweenCapacityChanges as Get<
                                        BlockNumberFor<Test>,
                                    >>::get(),
                        }
                        .into(),
                    );
                });
            }

            #[test]
            fn bsp_decrease_change_capacity_to_exactly_minimum_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 500;
                    let minimum_storage_amount: StorageData<Test> =
                        <SpMinCapacity as Get<u32>>::get();
                    let (old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - old_deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        old_deposit_amount
                    );

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Change the capacity of Alice to the minimum
                    assert_ok!(StorageProviders::change_capacity(
                        RuntimeOrigin::signed(alice),
                        minimum_storage_amount
                    ));

                    // Get the deposit amount for the new storage
                    let deposit_for_minimum_storage: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get();

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        5_000_000 - deposit_for_minimum_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_minimum_storage
                    );

                    // Check the new total capacity of the network (all BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        minimum_storage_amount
                    );

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            old_capacity: old_storage_amount,
                            new_capacity: minimum_storage_amount,
                            next_block_when_change_allowed:
                                frame_system::Pallet::<Test>::block_number()
                                    + <MinBlocksBetweenCapacityChanges as Get<
                                        BlockNumberFor<Test>,
                                    >>::get(),
                        }
                        .into(),
                    );
                });
            }
        }
    }

    /// This module holds the failure cases for changing the capacity of Main Storage Providers and Backup Storage Providers
    mod failure {
        use super::*;

        /// This module holds the failure cases for changing the capacity of Main Storage Providers
        mod msp {
            use super::*;

            #[test]
            fn msp_change_capacity_fails_when_not_registered_as_msp() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice
                    let alice: AccountId = 0;

                    // Try to change the capacity of Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::change_capacity(RuntimeOrigin::signed(alice), 100),
                        Error::<Test>::NotRegistered
                    );
                });
            }

            #[test]
            fn msp_change_capacity_fails_if_not_enough_time_passed() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let new_storage_amount: StorageData<Test> = 200;
                    let (_old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Try to change the capacity of Alice before enough time has passed
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            new_storage_amount
                        ),
                        Error::<Test>::NotEnoughTimePassed
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn msp_change_capacity_fails_when_changing_to_zero() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let zero_storage_amount: StorageData<Test> = 0;
                    let (_old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Try to change the capacity of Alice to zero
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            zero_storage_amount
                        ),
                        Error::<Test>::NewCapacityCantBeZero
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn msp_change_capacity_fails_when_using_same_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let new_storage_amount: StorageData<Test> = old_storage_amount;
                    let (_old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Try to change the capacity of Alice to the same as before
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            new_storage_amount
                        ),
                        Error::<Test>::NewCapacityEqualsCurrentCapacity
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn msp_change_capacity_fails_when_under_min_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let decreased_storage_amount: StorageData<Test> = 1;
                    let (_old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Try to change the capacity of Alice to a value under the minimum capacity
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            decreased_storage_amount
                        ),
                        Error::<Test>::StorageTooLow
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn msp_change_capacity_fails_when_under_used_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let decreased_storage_amount: StorageData<Test> = 50;
                    let (_old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Change used storage to be more than the new capacity
                    assert_ok!(
                        <StorageProviders as MutateProvidersInterface>::increase_data_used(
                            &alice, 60
                        )
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Try to change the capacity of Alice to a value under the used storage
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            decreased_storage_amount
                        ),
                        Error::<Test>::NewCapacityLessThanUsedStorage
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn msp_change_capacity_fails_when_not_enough_funds() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let new_storage_amount: StorageData<Test> =
                        (5_000_000 / <DepositPerData as Get<u128>>::get() + 1)
                            .try_into()
                            .unwrap();
                    let (_old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Try to change the capacity of Alice to a value under the used storage
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            new_storage_amount
                        ),
                        Error::<Test>::NotEnoughBalance
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );
                });
            }
        }

        /// This module holds the failure cases for changing the capacity of Backup Storage Providers
        mod bsp {
            use super::*;

            #[test]
            fn bsp_change_capacity_fails_when_not_registered_as_bsp() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice
                    let alice: AccountId = 0;

                    // Try to change the capacity of Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::change_capacity(RuntimeOrigin::signed(alice), 100),
                        Error::<Test>::NotRegistered
                    );
                });
            }

            #[test]
            fn bsp_change_capacity_fails_if_not_enough_time_passed() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let new_storage_amount: StorageData<Test> = 200;
                    let (_old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Try to change the capacity of Alice before enough time has passed
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            new_storage_amount
                        ),
                        Error::<Test>::NotEnoughTimePassed
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );

                    // Make sure that the total capacity of the network has not changed
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn bsp_change_capacity_fails_when_changing_to_zero() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let zero_storage_amount: StorageData<Test> = 0;
                    let (_old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Try to change the capacity of Alice to zero
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            zero_storage_amount
                        ),
                        Error::<Test>::NewCapacityCantBeZero
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );

                    // Make sure that the total capacity of the network has not changed
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn bsp_change_capacity_fails_when_using_same_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let new_storage_amount: StorageData<Test> = old_storage_amount;
                    let (_old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Try to change the capacity of Alice to the same as before
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            new_storage_amount
                        ),
                        Error::<Test>::NewCapacityEqualsCurrentCapacity
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );

                    // Make sure that the total capacity of the network has not changed
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn bsp_change_capacity_fails_when_under_min_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let decreased_storage_amount: StorageData<Test> = 1;
                    let (_old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Try to change the capacity of Alice to a value under the minimum capacity
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            decreased_storage_amount
                        ),
                        Error::<Test>::StorageTooLow
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );

                    // Make sure that the total capacity of the network has not changed
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn bsp_change_capacity_fails_when_under_used_capacity() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let decreased_storage_amount: StorageData<Test> = 50;
                    let (_old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Change used storage to be more than the new capacity
                    assert_ok!(
                        <StorageProviders as MutateProvidersInterface>::increase_data_used(
                            &alice, 60
                        )
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Try to change the capacity of Alice to a value under the used storage
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            decreased_storage_amount
                        ),
                        Error::<Test>::NewCapacityLessThanUsedStorage
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );

                    // Make sure that the total capacity of the network has not changed
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );
                });
            }

            #[test]
            fn bsp_change_capacity_fails_when_not_enough_funds() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = 0;
                    let old_storage_amount: StorageData<Test> = 100;
                    let new_storage_amount: StorageData<Test> =
                        (5_000_000 / <DepositPerData as Get<u128>>::get() + 1)
                            .try_into()
                            .unwrap();
                    let (_old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    // Advance enough blocks to allow Alice to change her capacity
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number()
                            + <MinBlocksBetweenCapacityChanges as Get<BlockNumberFor<Test>>>::get(),
                    );

                    // Try to change the capacity of Alice to a value under the used storage
                    assert_noop!(
                        StorageProviders::change_capacity(
                            RuntimeOrigin::signed(alice),
                            new_storage_amount
                        ),
                        Error::<Test>::NotEnoughBalance
                    );

                    // Make sure that the capacity of Alice has not changed
                    assert_eq!(
                        StorageProviders::get_total_capacity_of_sp(&alice).unwrap(),
                        old_storage_amount
                    );

                    // Make sure that the total capacity of the network has not changed
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );
                });
            }
        }
    }
}

// Helper functions for testing:

/// Helper function that registers an account as a Main Storage Provider, with storage_amount StorageData units
///
/// Returns the deposit amount that was utilized from the account's balance and the MSP information
fn register_account_as_msp(
    account: AccountId,
    storage_amount: StorageData<Test>,
) -> (BalanceOf<Test>, MainStorageProvider<Test>) {
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

    // Request to sign up the account as a Main Storage Provider
    assert_ok!(StorageProviders::request_msp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        value_prop.clone()
    ));

    // Check that the request sign up event was emitted
    System::assert_last_event(
        Event::<Test>::MspRequestSignUpSuccess {
            who: account,
            multiaddresses: multiaddresses.clone(),
            capacity: storage_amount,
            value_prop: value_prop.clone(),
        }
        .into(),
    );

    // Advance enough blocks for randomness to be valid
    run_to_block(frame_system::Pallet::<Test>::block_number() + BLOCKS_BEFORE_RANDOMNESS_VALID);

    // Confirm the sign up of the account as a Main Storage Provider
    assert_ok!(StorageProviders::confirm_sign_up(
        RuntimeOrigin::signed(account),
        Some(account)
    ));

    // Check that the confirm MSP sign up event was emitted
    System::assert_last_event(
        Event::<Test>::MspSignUpSuccess {
            who: account,
            multiaddresses: multiaddresses.clone(),
            capacity: storage_amount,
            value_prop: value_prop.clone(),
        }
        .into(),
    );

    // Return the deposit amount that was utilized from the account's balance and the MSP information
    (
        deposit_for_storage_amount,
        MainStorageProvider {
            buckets: BoundedVec::new(),
            capacity: storage_amount,
            data_used: 0,
            multiaddresses,
            value_prop,
            last_capacity_change: frame_system::Pallet::<Test>::block_number(),
        },
    )
}

/// Helper function that registers an account as a Backup Storage Provider, with storage_amount StorageData units
///
/// Returns the deposit amount that was utilized from the account's balance and the BSP information
fn register_account_as_bsp(
    account: AccountId,
    storage_amount: StorageData<Test>,
) -> (BalanceOf<Test>, BackupStorageProvider<Test>) {
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

    // Check that the request sign up event was emitted
    System::assert_last_event(
        Event::<Test>::BspRequestSignUpSuccess {
            who: account,
            multiaddresses: multiaddresses.clone(),
            capacity: storage_amount,
        }
        .into(),
    );

    // Advance enough blocks for randomness to be valid
    run_to_block(frame_system::Pallet::<Test>::block_number() + 4);

    // Confirm the sign up of the account as a Backup Storage Provider
    assert_ok!(StorageProviders::confirm_sign_up(
        RuntimeOrigin::signed(account),
        Some(account)
    ));

    // Check that the confirm BSP sign up event was emitted
    System::assert_last_event(
        Event::<Test>::BspSignUpSuccess {
            who: account,
            multiaddresses: multiaddresses.clone(),
            capacity: storage_amount,
        }
        .into(),
    );

    // Return the deposit amount that was utilized from the account's balance
    (
        deposit_for_storage_amount,
        BackupStorageProvider {
            capacity: storage_amount,
            data_used: 0,
            multiaddresses,
            root: Default::default(),
            last_capacity_change: frame_system::Pallet::<Test>::block_number(),
        },
    )
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

/// This module is just a test to make sure the MockRandomness trait works. TODO: remove it alongside the test_randomness_output function
mod randomness {
    use super::*;

    #[test]
    fn test_randomness() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let (sp_id, block_number) = test_randomness_output(&alice);
            println!(
                "current block_number: {:?}",
                frame_system::Pallet::<Test>::block_number()
            );
            println!("sp_id: {:?}", sp_id);
            println!("block_number: {:?}", block_number);
            assert_eq!(
                block_number,
                frame_system::Pallet::<Test>::block_number()
                    .saturating_sub(BLOCKS_BEFORE_RANDOMNESS_VALID)
            );

            run_to_block(10);
            let (sp_id, block_number) = test_randomness_output(&alice);
            println!(
                "current block_number: {:?}",
                frame_system::Pallet::<Test>::block_number()
            );
            println!("sp_id: {:?}", sp_id);
            println!("block_number: {:?}", block_number);
            assert_eq!(
                block_number,
                frame_system::Pallet::<Test>::block_number()
                    .saturating_sub(BLOCKS_BEFORE_RANDOMNESS_VALID)
            );
        });
    }
}

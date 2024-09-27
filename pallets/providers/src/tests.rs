use crate::{
    mock::*,
    types::{
        BackupStorageProvider, BalanceOf, Bucket, MainStorageProvider, MainStorageProviderId,
        MaxBuckets, MaxMultiAddressAmount, MultiAddress, StorageDataUnit, StorageProvider,
        ValuePropId, ValueProposition,
    },
    Error, Event,
};

use frame_support::{assert_err, assert_noop, assert_ok, dispatch::Pays, BoundedVec};
use frame_support::{
    pallet_prelude::Weight,
    traits::{fungible::InspectHold, Get, OnFinalize, OnIdle, OnInitialize},
};
use frame_system::pallet_prelude::BlockNumberFor;
use shp_traits::{
    MutateBucketsInterface, MutateStorageProvidersInterface, ReadBucketsInterface,
    ReadProvidersInterface,
};

type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;

// Pallet constants:
type SpMinDeposit = <Test as crate::Config>::SpMinDeposit;
type SpMinCapacity = <Test as crate::Config>::SpMinCapacity;
type DepositPerData = <Test as crate::Config>::DepositPerData;
type MinBlocksBetweenCapacityChanges = <Test as crate::Config>::MinBlocksBetweenCapacityChanges;
type DefaultMerkleRoot = <Test as crate::Config>::DefaultMerkleRoot;
type BucketDeposit = <Test as crate::Config>::BucketDeposit;

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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Alice is going to sign up as a Main Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount
                    );

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                                capacity_used: 0,
                                multiaddresses,
                                value_prop,
                                last_capacity_change: current_block,
                                owner_account: alice,
                                payment_account: alice
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Alice is going to sign up as a Main Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount
                    );

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses,
                            capacity: storage_amount,
                            value_prop,
                            msp_id: alice_sp_id.unwrap(),
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Alice is going to sign up as a Main Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount
                    );

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses,
                            capacity: storage_amount,
                            value_prop,
                            msp_id: alice_sp_id.unwrap(),
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Alice is going to request to sign up as a Main Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Main Storage Provider with 300 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        bob
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount_alice
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount_alice
                    );

                    // Check the new free balance of Bob
                    assert_eq!(
                        NativeBalance::free_balance(&bob),
                        accounts::BOB.1 - deposit_for_storage_amount_bob
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Check that Alice's request to sign up as a Main Storage Provider exists and is the one we just created
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.as_ref().is_ok_and(|request| request.0
                        == StorageProvider::MainStorageProvider(MainStorageProvider {
                            buckets: BoundedVec::new(),
                            capacity: storage_amount,
                            capacity_used: 0,
                            multiaddresses: multiaddresses.clone(),
                            value_prop: value_prop.clone(),
                            last_capacity_change: current_block,
                            owner_account: alice,
                            payment_account: alice
                        })));
                    assert!(alice_sign_up_request.is_ok_and(|request| request.1 == current_block));

                    // Cancel the sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(
                        alice
                    )));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Alice is going to sign up as a Backup Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount
                    );

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                                root: DefaultMerkleRoot::get(),
                                capacity: storage_amount,
                                capacity_used: 0,
                                multiaddresses,
                                last_capacity_change: current_block,
                                owner_account: alice,
                                payment_account: alice,
                                reputation_weight:
                                    <Test as crate::Config>::StartingReputationWeight::get(),
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Alice is going to sign up as a Backup Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount
                    );

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                            bsp_id: alice_sp_id.unwrap(),
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Alice is going to sign up as a Backup Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount
                    );

                    // Check that Alice is still NOT a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                            bsp_id: alice_sp_id.unwrap(),
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Alice is going to request to sign up as a Backup Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Backup Storage Provider with 300 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        bob
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount_alice
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount_alice
                    );

                    // Check the new free balance of Bob
                    assert_eq!(
                        NativeBalance::free_balance(&bob),
                        accounts::BOB.1 - deposit_for_storage_amount_bob
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Check that Alice's request to sign up as a Backup Storage Provider exists and is the one we just created
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.as_ref().is_ok_and(|request| request.0
                        == StorageProvider::BackupStorageProvider(BackupStorageProvider {
                            capacity: storage_amount,
                            capacity_used: 0,
                            multiaddresses: multiaddresses.clone(),
                            root: DefaultMerkleRoot::get(),
                            last_capacity_change: current_block,
                            owner_account: alice,
                            payment_account: alice,
                            reputation_weight:
                                <Test as crate::Config>::StartingReputationWeight::get(),
                        })));
                    assert!(alice_sign_up_request.is_ok_and(|request| request.1 == current_block));

                    // Cancel the sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(
                        alice
                    )));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Alice is going to request to sign up as a Main Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Backup Storage Provider with 300 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        bob
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount_alice
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount_alice
                    );

                    // Check the new free balance of Bob
                    assert_eq!(
                        NativeBalance::free_balance(&bob),
                        accounts::BOB.1 - deposit_for_storage_amount_bob
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        bob
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                            msp_id: alice_sp_id.unwrap(),
                        }
                        .into(),
                    );

                    // Check that Bob is still NOT a Storage Provider
                    let bob_sp_id = StorageProviders::get_provider_id(bob);
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        bob
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                            msp_id: alice_sp_id.unwrap(),
                        }
                        .into(),
                    );

                    // Confirm the sign up of the account as a Backup Storage Provider
                    assert_ok!(StorageProviders::confirm_sign_up(
                        RuntimeOrigin::signed(bob),
                        Some(bob)
                    ));

                    // Check that Bob is now a Storage Provider
                    let bob_sp_id = StorageProviders::get_provider_id(bob);
                    assert!(bob_sp_id.is_some());
                    assert!(StorageProviders::is_provider(bob_sp_id.unwrap()));

                    // Check that the confirm BSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::BspSignUpSuccess {
                            who: bob,
                            multiaddresses,
                            capacity: storage_amount_bob,
                            bsp_id: bob_sp_id.unwrap(),
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        bob
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses: multiaddresses.clone(),
                            capacity: storage_amount_alice,
                            value_prop: value_prop.clone(),
                            msp_id: alice_sp_id.unwrap(),
                        }
                        .into(),
                    );

                    // Cancel the sign up of Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::cancel_sign_up(RuntimeOrigin::signed(bob)));

                    // Check that Bob is still not a Storage Provider
                    let bob_sp_id = StorageProviders::get_provider_id(bob);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

                    // Check that the confirm MSP sign up event was emitted
                    System::assert_last_event(
                        Event::<Test>::MspSignUpSuccess {
                            who: alice,
                            multiaddresses,
                            capacity: storage_amount,
                            value_prop,
                            msp_id: alice_sp_id.unwrap(),
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
                    let storage_amount_alice: StorageDataUnit<Test> = 100;
                    let storage_amount_bob: StorageDataUnit<Test> = 300;

                    // Get the Account Id of Alice and check its balance
                    let alice: AccountId = accounts::ALICE.0;
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Do the same for Bob
                    let bob: AccountId = 1;
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                        0
                    );

                    // Alice is going to request to sign up as a Main Storage Provider with 100 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (100 - 1) = 208
                    let deposit_for_storage_amount_alice: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_alice - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Bob is going to request to sign up as a Main Storage Provider with 300 StorageDataUnit units
                    // The deposit for any amount of storage would be MinDeposit + DepositPerData * (storage_amount - MinCapacity)
                    // In this case, the deposit would be 10 + 2 * (300 - 1) = 608
                    let deposit_for_storage_amount_bob: BalanceOf<Test> =
                        <SpMinDeposit as Get<u128>>::get().saturating_add(
                            <DepositPerData as Get<u128>>::get().saturating_mul(
                                (storage_amount_bob - <SpMinCapacity as Get<u64>>::get()).into(),
                            ),
                        );

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount_alice,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        bob
                    ));

                    // Check the new free balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_storage_amount_alice
                    );
                    // Check the new held balance of Alice
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_storage_amount_alice
                    );

                    // Check the new free balance of Bob
                    assert_eq!(
                        NativeBalance::free_balance(&bob),
                        accounts::BOB.1 - deposit_for_storage_amount_bob
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
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
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
                    assert_eq!(NativeBalance::free_balance(&bob), accounts::BOB.1);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request to sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Try to request to sign up Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone(),
                            alice
                        ),
                        Error::<Test>::SignUpRequestPending
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Check that Alice is not a Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request to sign up Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        alice
                    ));

                    // Try to request to sign up Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            alice
                        ),
                        Error::<Test>::SignUpRequestPending
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_none());
                });
            }

            #[test]
            fn confirm_sign_up_fails_if_request_does_not_exist() {
                ExtBuilder::build().execute_with(|| {
                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

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
                    let alice: AccountId = accounts::ALICE.0;
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
                            alice_msp.value_prop.clone(),
                            alice
                        ),
                        Error::<Test>::AlreadyRegistered
                    );

                    // We try to request to sign her up as a BSP now
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            alice_msp.capacity,
                            alice_msp.multiaddresses.clone(),
                            alice
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
                            },
                            bob
                        ),
                        Error::<Test>::AlreadyRegistered
                    );

                    // And as a BSP
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(bob),
                            bob_bsp.capacity,
                            bob_bsp.multiaddresses.clone(),
                            bob
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and Bob
                    let alice: AccountId = accounts::ALICE.0;
                    let bob: AccountId = 1;

                    // Request to sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.clone(),
                        alice
                    ));

                    // Request to sign up Bob as a Backup Storage Provider
                    assert_ok!(StorageProviders::request_bsp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount,
                        multiaddresses.clone(),
                        bob
                    ));

                    // Try to request to sign up Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            alice
                        ),
                        Error::<Test>::SignUpRequestPending
                    );

                    // Try to request to sign up Bob as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(bob),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone(),
                            bob
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
                    let storage_amount: StorageDataUnit<Test> = 1;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Try to sign up Alice as a Main Storage Provider with less than the minimum capacity
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone(),
                            alice
                        ),
                        Error::<Test>::StorageTooLow
                    );

                    // Try to sign up Alice as a Backup Storage Provider with less than the minimum capacity
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            alice
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Helen (who has no balance)
                    let helen: AccountId = 7;

                    // Try to sign up Helen as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(helen),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone(),
                            helen
                        ),
                        Error::<Test>::NotEnoughBalance
                    );

                    // Try to sign up Helen as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(helen),
                            storage_amount,
                            multiaddresses.clone(),
                            helen
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Try to sign up Alice as a Main Storage Provider with no multiaddresses
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.clone(),
                            alice
                        ),
                        Error::<Test>::NoMultiAddress
                    );

                    // Try to sign up Alice as a Backup Storage Provider with no multiaddresses
                    assert_noop!(
                        StorageProviders::request_bsp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            alice
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
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

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
                    let alice: AccountId = accounts::ALICE.0;
                    let storage_amount: StorageDataUnit<Test> = 100;
                    let (deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the counter of registered MSPs
                    assert_eq!(StorageProviders::get_msp_count(), 1);

                    // Get the MSP ID of Alice
                    let alice_msp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Sign off Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::msp_sign_off(RuntimeOrigin::signed(alice)));

                    // Check the new free and held balance of Alice
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Check that the counter of registered MSPs has decreased
                    assert_eq!(StorageProviders::get_msp_count(), 0);

                    // Check that Alice is not a Main Storage Provider anymore
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_none());

                    // Check the MSP Sign Off event was emitted
                    System::assert_has_event(
                        Event::<Test>::MspSignOffSuccess {
                            who: alice,
                            msp_id: alice_msp_id,
                        }
                        .into(),
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
                    let alice: AccountId = accounts::ALICE.0;
                    let storage_amount: StorageDataUnit<Test> = 100;
                    let (deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the capacity of all the BSPs
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), storage_amount);

                    // Check the counter of registered BSPs
                    assert_eq!(StorageProviders::get_bsp_count(), 1);

                    // Get the BSP ID of Alice
                    let alice_bsp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Sign off Alice as a Backup Storage Provider
                    assert_ok!(StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)));

                    // Check the new capacity of all BSPs
                    assert_eq!(StorageProviders::get_total_bsp_capacity(), 0);

                    // Check the new free and held balance of Alice
                    assert_eq!(NativeBalance::free_balance(&alice), accounts::ALICE.1);
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        0
                    );

                    // Check that Alice is not a Backup Storage Provider anymore
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_none());

                    // Check that the counter of registered BSPs has decreased
                    assert_eq!(StorageProviders::get_bsp_count(), 0);

                    // Check the BSP Sign Off event was emitted
                    System::assert_has_event(
                        Event::<Test>::BspSignOffSuccess {
                            who: alice,
                            bsp_id: alice_bsp_id,
                        }
                        .into(),
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
                    let alice: AccountId = accounts::ALICE.0;

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
                    let alice = 0;
                    let storage_amount: StorageDataUnit<Test> = 100;
                    let (deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_amount
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_amount
                    );

                    // Check the counter of registered MSPs
                    assert_eq!(StorageProviders::get_msp_count(), 1);

                    // Check that Alice does not have any used storage
                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();
                    assert_eq!(
                        StorageProviders::get_used_storage_of_msp(&alice_sp_id).unwrap(),
                        0
                    );

                    let alice_msp_id =
                        crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                    // Add used storage to Alice (simulating that she has accepted to store a file)
                    assert_ok!(
                        <StorageProviders as MutateStorageProvidersInterface>::increase_capacity_used(
                            &alice_msp_id,
                            10
                        )
                    );

                    // Try to sign off Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::msp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::StorageStillInUse
                    );

                    // Make sure that Alice is still registered as a Main Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
                    let alice: AccountId = accounts::ALICE.0;

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
                    let alice: AccountId = accounts::ALICE.0;
                    let storage_amount: StorageDataUnit<Test> = 100;
                    let (deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_amount
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
                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();
                    assert_eq!(
                        StorageProviders::get_used_storage_of_bsp(&alice_sp_id).unwrap(),
                        0
                    );

                    // Add used storage to Alice (simulating that she has accepted to store a file)
                    assert_ok!(
                        <StorageProviders as MutateStorageProvidersInterface>::increase_capacity_used(
                            &alice_sp_id,
                            10
                        )
                    );

                    // Try to sign off Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::StorageStillInUse
                    );

                    // Make sure that Alice is still registered as a Backup Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
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
            use crate::types::StorageProviderId;

            use super::*;

            #[test]
            fn msp_increase_change_capacity_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as MSP:
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let increased_storage_amount: StorageDataUnit<Test> = 200;
                    let (old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - old_deposit_amount
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
                        (increased_storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_increased_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_increased_storage
                    );

                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            provider_id: StorageProviderId::MainStorageProvider(alice_sp_id),
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let decreased_storage_amount: StorageDataUnit<Test> = 50;
                    let (old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - old_deposit_amount
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
                        (decreased_storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_decreased_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_decreased_storage
                    );

                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            provider_id: StorageProviderId::MainStorageProvider(alice_sp_id),
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 500;
                    let minimum_storage_amount: StorageDataUnit<Test> =
                        <SpMinCapacity as Get<u64>>::get();
                    let (old_deposit_amount, _alice_msp) =
                        register_account_as_msp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - old_deposit_amount
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
                        accounts::ALICE.1 - deposit_for_minimum_storage
                    );
                    assert_eq!(
                        NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                        deposit_for_minimum_storage
                    );

                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            provider_id: StorageProviderId::MainStorageProvider(alice_sp_id),
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
            use crate::types::StorageProviderId;

            use super::*;

            #[test]
            fn bsp_increase_change_capacity_works() {
                ExtBuilder::build().execute_with(|| {
                    // Register Alice as BSP:
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let increased_storage_amount: StorageDataUnit<Test> = 200;
                    let (old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - old_deposit_amount
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
                        (increased_storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_increased_storage
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

                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            provider_id: StorageProviderId::BackupStorageProvider(alice_sp_id),
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let decreased_storage_amount: StorageDataUnit<Test> = 50;
                    let (old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - old_deposit_amount
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
                        (decreased_storage_amount - <SpMinCapacity as Get<u64>>::get()).into(),
                    ));

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - deposit_for_decreased_storage
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

                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            provider_id: StorageProviderId::BackupStorageProvider(alice_sp_id),
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 500;
                    let minimum_storage_amount: StorageDataUnit<Test> =
                        <SpMinCapacity as Get<u64>>::get();
                    let (old_deposit_amount, _alice_bsp) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the new free and held balance of Alice
                    assert_eq!(
                        NativeBalance::free_balance(&alice),
                        accounts::ALICE.1 - old_deposit_amount
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
                        accounts::ALICE.1 - deposit_for_minimum_storage
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

                    let alice_sp_id = StorageProviders::get_provider_id(alice).unwrap();

                    // Check that the capacity changed event was emitted
                    System::assert_has_event(
                        Event::<Test>::CapacityChanged {
                            who: alice,
                            provider_id: StorageProviderId::BackupStorageProvider(alice_sp_id),
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
                    let alice: AccountId = accounts::ALICE.0;

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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let new_storage_amount: StorageDataUnit<Test> = 200;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let zero_storage_amount: StorageDataUnit<Test> = 0;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let new_storage_amount: StorageDataUnit<Test> = old_storage_amount;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let decreased_storage_amount: StorageDataUnit<Test> = 1;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let decreased_storage_amount: StorageDataUnit<Test> = 50;
                    let (_old_deposit_amount, _alice_sp_id) =
                        register_account_as_msp(alice, old_storage_amount);

                    let alice_msp_id =
                        crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                    // Change used storage to be more than the new capacity
                    assert_ok!(
                        <StorageProviders as MutateStorageProvidersInterface>::increase_capacity_used(
                            &alice_msp_id,
                            60
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let new_storage_amount: StorageDataUnit<Test> =
                        (accounts::ALICE.1 / <DepositPerData as Get<u128>>::get() + 1)
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
                    let alice: AccountId = accounts::ALICE.0;

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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let new_storage_amount: StorageDataUnit<Test> = 200;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let zero_storage_amount: StorageDataUnit<Test> = 0;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let new_storage_amount: StorageDataUnit<Test> = old_storage_amount;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let decreased_storage_amount: StorageDataUnit<Test> = 1;
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let decreased_storage_amount: StorageDataUnit<Test> = 50;
                    let (_old_deposit_amount, _alice_bsp_id) =
                        register_account_as_bsp(alice, old_storage_amount);

                    // Check the total capacity of the network (BSPs)
                    assert_eq!(
                        StorageProviders::get_total_bsp_capacity(),
                        old_storage_amount
                    );

                    let alice_bsp_id =
                        crate::AccountIdToBackupStorageProviderId::<Test>::get(&alice).unwrap();

                    // Change used storage to be more than the new capacity
                    assert_ok!(
                        <StorageProviders as MutateStorageProvidersInterface>::increase_capacity_used(
                            &alice_bsp_id,
                            60
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
                    let alice: AccountId = accounts::ALICE.0;
                    let old_storage_amount: StorageDataUnit<Test> = 100;
                    let new_storage_amount: StorageDataUnit<Test> =
                        (accounts::ALICE.1 / <DepositPerData as Get<u128>>::get() + 1)
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

mod change_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn change_bucket_fails_when_bucket_id_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &msp_id,
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    msp_id,
                    bucket_owner,
                    bucket_id,
                    false,
                    None
                ));

                // Try to change the bucket for Alice with the same bucket id
                assert_noop!(
                    StorageProviders::add_bucket(msp_id, bucket_owner, bucket_id, false, None),
                    Error::<Test>::BucketAlreadyExists
                );
            });
        }
    }
}

mod add_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn add_bucket_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &msp_id,
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    msp_id,
                    bucket_owner,
                    bucket_id,
                    false,
                    None
                ));

                // Try to add the bucket for Alice with the same bucket id
                assert_noop!(
                    StorageProviders::add_bucket(msp_id, bucket_owner, bucket_id, false, None),
                    Error::<Test>::BucketAlreadyExists
                );
            });
        }

        #[test]
        fn add_bucket_msp_not_registered() {
            ExtBuilder::build().execute_with(|| {
                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &MainStorageProviderId::<Test>::default(),
                    &bucket_owner,
                    bucket_name,
                );

                // Try to add a bucket to a non-registered MSP
                assert_noop!(
                    StorageProviders::add_bucket(
                        MainStorageProviderId::<Test>::default(),
                        bucket_owner,
                        bucket_id,
                        false,
                        None
                    ),
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn add_bucket_passed_max_bucket_msp_capacity() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &msp_id,
                    &bucket_owner,
                    bucket_name,
                );

                // Add the maximum amount of buckets for Alice
                for i in 0..MaxBuckets::<Test>::get() {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &msp_id,
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        msp_id,
                        bucket_owner,
                        bucket_id,
                        false,
                        None
                    ));
                }

                // Try to add another bucket for Alice
                assert_err!(
                    StorageProviders::add_bucket(msp_id, bucket_owner, bucket_id, false, None),
                    Error::<Test>::AppendBucketToMspFailed
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn add_bucket() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &msp_id,
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    msp_id,
                    bucket_owner,
                    bucket_id,
                    false,
                    None
                ));

                assert_eq!(
                    NativeBalance::free_balance(&bucket_owner),
                    accounts::BOB.1 - <BucketDeposit as Get<u128>>::get()
                );

                assert_eq!(
                    NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                    BucketDeposit::get()
                );

                let buckets = crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id).unwrap();

                assert_eq!(buckets.len(), 1);

                let bucket = crate::Buckets::<Test>::get(&bucket_id).unwrap();

                assert_eq!(
                    bucket,
                    Bucket::<Test> {
                        root: DefaultMerkleRoot::get(),
                        user_id: bucket_owner,
                        msp_id,
                        private: false,
                        read_access_group_id: None,
                        size: 0,
                    }
                );
            });
        }

        #[test]
        fn add_buckets_to_max_capacity() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;

                // Add the maximum amount of buckets for Alice
                for i in 0..MaxBuckets::<Test>::get() {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &msp_id,
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        msp_id,
                        bucket_owner,
                        bucket_id,
                        false,
                        None
                    ));

                    let expected_hold_amount =
                        (i + 1) as u128 * <BucketDeposit as Get<u128>>::get();
                    assert_eq!(
                        NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                        expected_hold_amount
                    );
                }

                let buckets = crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id).unwrap();

                let max_buckets: u32 = MaxBuckets::<Test>::get();
                assert_eq!(buckets.len(), max_buckets as usize);
            });
        }
    }
}

mod remove_root_bucket {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn remove_root_bucket_when_bucket_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &MainStorageProviderId::<Test>::default(),
                    &bucket_owner,
                    bucket_name,
                );

                // Try to remove a bucket that does not exist
                assert_noop!(
                    StorageProviders::remove_root_bucket(bucket_id),
                    Error::<Test>::BucketNotFound
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn remove_root_bucket() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &msp_id,
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    msp_id,
                    bucket_owner,
                    bucket_id,
                    false,
                    None
                ));

                // Check that the bucket was added to the MSP
                assert_eq!(
                    crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id).unwrap(),
                    vec![bucket_id]
                );

                // Remove the bucket
                assert_ok!(StorageProviders::remove_root_bucket(bucket_id));

                // Check that the bucket deposit is returned to the bucket owner
                assert_eq!(NativeBalance::free_balance(&bucket_owner), accounts::BOB.1);

                // Check that the bucket deposit is no longer on hold
                assert_eq!(
                    NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                    0
                );

                // Check that the bucket was removed
                assert_eq!(crate::Buckets::<Test>::get(&bucket_id), None);

                // Check that the bucket was removed from the MSP
                assert_eq!(
                    crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id),
                    None
                );
            });
        }

        #[test]
        fn remove_root_buckets_multiple() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;

                // Add the maximum amount of buckets for Alice
                for i in 0..MaxBuckets::<Test>::get() {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &msp_id,
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        msp_id,
                        bucket_owner,
                        bucket_id,
                        false,
                        None
                    ));

                    let expected_hold_amount =
                        (i + 1) as u128 * <BucketDeposit as Get<u128>>::get();
                    assert_eq!(
                        NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                        expected_hold_amount
                    );
                }

                let buckets = crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id).unwrap();

                let max_buckets: u32 = MaxBuckets::<Test>::get();
                assert_eq!(buckets.len(), max_buckets as usize);

                // Remove all the buckets
                for i in 0..MaxBuckets::<Test>::get() {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &msp_id,
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::remove_root_bucket(bucket_id));
                }

                // Check that the bucket deposits are returned to the bucket owner
                assert_eq!(NativeBalance::free_balance(&bucket_owner), accounts::BOB.1);

                // Check that the bucket deposits are no longer on hold
                assert_eq!(
                    NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                    0
                );

                // Check that all the buckets were removed
                assert_eq!(
                    crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id),
                    None
                );
            });
        }
    }
}

mod slash {
    use super::*;
    mod failure {
        use sp_core::H256;

        use super::*;

        #[test]
        fn slash_when_storage_provider_not_registered() {
            ExtBuilder::build().execute_with(|| {
                let caller = accounts::BOB.0;

                // Try to slash a provider that is not registered
                assert_noop!(
                    StorageProviders::slash(RuntimeOrigin::signed(caller), H256::default()),
                    Error::<Test>::ProviderNotSlashable
                );
            });
        }

        #[test]
        fn slash_when_storage_provider_not_slashable() {
            ExtBuilder::build().execute_with(|| {
                // register msp
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let provider_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let caller = accounts::BOB.0;

                // Try to slash the provider
                assert_noop!(
                    StorageProviders::slash(RuntimeOrigin::signed(caller), provider_id),
                    Error::<Test>::ProviderNotSlashable
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn slash_storage_provider() {
            ExtBuilder::build().execute_with(|| {
                // register msp
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let provider_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                // Set proofs-dealer storage to have a slashable provider
                pallet_proofs_dealer::SlashableProviders::<Test>::insert(&provider_id, 1);

                let deposit_on_hold =
                    NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice);

                let caller = accounts::BOB.0;

                let treasury_balance =
                    NativeBalance::free_balance(&<Test as crate::Config>::Treasury::get());

                let slash_factor: BalanceOf<Test> =
                    StorageProviders::compute_worst_case_scenario_slashable_amount(&provider_id)
                        .expect("Failed to compute slashable amount");

                // Slash the provider
                assert_ok!(StorageProviders::slash(
                    RuntimeOrigin::signed(caller),
                    provider_id
                ));

                // Check that the provider is no longer slashable
                assert_eq!(
                    pallet_proofs_dealer::SlashableProviders::<Test>::get(&provider_id),
                    None
                );

                // Check that the held deposit of the provider has been reduced by slash factor
                assert_eq!(
                    NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                    deposit_on_hold.saturating_sub(slash_factor)
                );

                // If the slash factor is greater than the deposit on hold, the slash amount is the deposit on hold
                let actual_slashed_amount = slash_factor.min(deposit_on_hold);

                // Check that the Treasury has received the slash amount
                assert_eq!(
                    NativeBalance::free_balance(&<Test as crate::Config>::Treasury::get()),
                    treasury_balance + actual_slashed_amount
                );
            });
        }

        #[test]
        fn slash_multiple_storage_providers() {
            ExtBuilder::build().execute_with(|| {
                // register msp and bsp
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp) = register_account_as_msp(alice, storage_amount);

                let bob: AccountId = accounts::BOB.0;
                let (_deposit_amount, _bob_bsp) = register_account_as_bsp(bob, storage_amount);

                let alice_provider_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();
                let bob_provider_id =
                    crate::AccountIdToBackupStorageProviderId::<Test>::get(&bob).unwrap();

                let alice_accrued_failed_proof_submissions = 5;
                let bob_accrued_failed_proof_submissions = 10;

                // Set proofs-dealer storage to have a slashable provider
                pallet_proofs_dealer::SlashableProviders::<Test>::insert(
                    &alice_provider_id,
                    alice_accrued_failed_proof_submissions,
                );
                pallet_proofs_dealer::SlashableProviders::<Test>::insert(
                    &bob_provider_id,
                    bob_accrued_failed_proof_submissions,
                );

                let alice_deposit_on_hold =
                    NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice);
                let bob_deposit_on_hold =
                    NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob);

                let caller = accounts::CHARLIE.0;

                let treasury_balance =
                    NativeBalance::free_balance(&<Test as crate::Config>::Treasury::get());

                // Check that the held deposit of the providers has been reduced by slash factor
                let alice_slash_amount: BalanceOf<Test> =
                    StorageProviders::compute_worst_case_scenario_slashable_amount(
                        &alice_provider_id,
                    )
                    .expect("Failed to compute slashable amount");

                let bob_slash_amount: BalanceOf<Test> =
                    StorageProviders::compute_worst_case_scenario_slashable_amount(
                        &bob_provider_id,
                    )
                    .expect("Failed to compute slashable amount");

                // Slash the providers
                assert_ok!(StorageProviders::slash(
                    RuntimeOrigin::signed(caller),
                    alice_provider_id
                ));
                assert_ok!(StorageProviders::slash(
                    RuntimeOrigin::signed(caller),
                    bob_provider_id
                ));

                // Check that the providers are no longer slashable
                assert_eq!(
                    pallet_proofs_dealer::SlashableProviders::<Test>::get(&alice_provider_id),
                    None
                );
                assert_eq!(
                    pallet_proofs_dealer::SlashableProviders::<Test>::get(&bob_provider_id),
                    None
                );

                assert_eq!(
                    NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &alice),
                    alice_deposit_on_hold.saturating_sub(alice_slash_amount)
                );

                assert_eq!(
                    NativeBalance::balance_on_hold(&StorageProvidersHoldReason::get(), &bob),
                    bob_deposit_on_hold.saturating_sub(bob_slash_amount)
                );

                // If slash amount is greater than deposit then the actual slash amount should be the deposit amount
                let actual_alice_slashed_amount = alice_slash_amount.min(alice_deposit_on_hold);
                let actual_bob_slashed_amount = bob_slash_amount.min(bob_deposit_on_hold);

                // Check that the Treasury has received the slash amount
                assert_eq!(
                    NativeBalance::free_balance(&<Test as crate::Config>::Treasury::get()),
                    treasury_balance + actual_alice_slashed_amount + actual_bob_slashed_amount
                );
            });
        }
    }
}

// Helper functions for testing:

/// Helper function that registers an account as a Main Storage Provider, with storage_amount StorageDataUnit units
///
/// Returns the deposit amount that was utilized from the account's balance and the MSP information
fn register_account_as_msp(
    account: AccountId,
    storage_amount: StorageDataUnit<Test>,
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
                .saturating_mul((storage_amount - <SpMinCapacity as Get<u64>>::get()).into()),
        );

    // Check the balance of the account to make sure it has more than the deposit amount needed
    assert!(NativeBalance::free_balance(&account) >= deposit_for_storage_amount);

    // Request to sign up the account as a Main Storage Provider
    assert_ok!(StorageProviders::request_msp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        value_prop.clone(),
        account
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

    let msp_id = StorageProviders::get_provider_id(account).unwrap();

    // Check that the confirm MSP sign up event was emitted
    System::assert_last_event(
        Event::<Test>::MspSignUpSuccess {
            who: account,
            msp_id,
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
            capacity_used: 0,
            multiaddresses,
            value_prop,
            last_capacity_change: frame_system::Pallet::<Test>::block_number(),
            owner_account: account,
            payment_account: account,
        },
    )
}

/// Helper function that registers an account as a Backup Storage Provider, with storage_amount StorageDataUnit units
///
/// Returns the deposit amount that was utilized from the account's balance and the BSP information
fn register_account_as_bsp(
    account: AccountId,
    storage_amount: StorageDataUnit<Test>,
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

    let bsp_id = StorageProviders::get_provider_id(account).unwrap();

    // Check that the confirm BSP sign up event was emitted
    System::assert_last_event(
        Event::<Test>::BspSignUpSuccess {
            who: account,
            bsp_id,
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
            capacity_used: 0,
            multiaddresses,
            root: DefaultMerkleRoot::get(),
            last_capacity_change: frame_system::Pallet::<Test>::block_number(),
            owner_account: account,
            payment_account: account,
            reputation_weight: <Test as crate::Config>::StartingReputationWeight::get(),
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
            let alice: AccountId = accounts::ALICE.0;
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

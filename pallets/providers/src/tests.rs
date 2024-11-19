use crate::{
    mock::*,
    types::{
        BackupStorageProvider, BalanceOf, Bucket, HashId, MainStorageProvider,
        MainStorageProviderId, MaxMultiAddressAmount, MultiAddress, RelayBlockGetter,
        SignUpRequestSpParams, StorageDataUnit, StorageProviderId, TopUpMetadata, ValueProposition,
        ValuePropositionWithId,
    },
    Error, Event, MainStorageProviders,
};

use frame_support::{assert_noop, assert_ok, dispatch::Pays, BoundedVec};
use frame_support::{
    pallet_prelude::Weight,
    traits::{
        fungible::{InspectHold, Mutate},
        Get, OnFinalize, OnIdle, OnInitialize,
    },
};
use frame_system::pallet_prelude::BlockNumberFor;
use shp_traits::{
    MutateBucketsInterface, MutateStorageProvidersInterface, PaymentStreamsInterface,
    ReadBucketsInterface, ReadProvidersInterface,
};
use sp_runtime::bounded_vec;
use sp_runtime::traits::{BlockNumberProvider, Convert};

type NativeBalance = <Test as crate::Config>::NativeBalance;
type AccountId = <Test as frame_system::Config>::AccountId;

// Pallet constants:
type SpMinDeposit = <Test as crate::Config>::SpMinDeposit;
type SpMinCapacity = <Test as crate::Config>::SpMinCapacity;
type DepositPerData = <Test as crate::Config>::DepositPerData;
type MinBlocksBetweenCapacityChanges = <Test as crate::Config>::MinBlocksBetweenCapacityChanges;
type DefaultMerkleRoot = <Test as crate::Config>::DefaultMerkleRoot;
type BucketDeposit = <Test as crate::Config>::BucketDeposit;
type TopUpGracePeriod = <Test as crate::Config>::TopUpGracePeriod;

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
            use crate::types::{MainStorageProviderSignUpRequest, SignUpRequest};

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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);

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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                        }
                        .into(),
                    );

                    // Check that Alice's request is in the requests list and matches the info provided
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice);
                    assert!(alice_sign_up_request.is_ok());
                    assert_eq!(
                        alice_sign_up_request.unwrap(),
                        SignUpRequest::<Test> {
                            sp_sign_up_request: SignUpRequestSpParams::MainStorageProvider(
                                MainStorageProviderSignUpRequest {
                                    msp_info: MainStorageProvider {
                                        capacity: storage_amount,
                                        capacity_used: 0,
                                        multiaddresses,
                                        last_capacity_change: current_block,
                                        owner_account: alice,
                                        payment_account: alice,
                                        sign_up_block: current_block
                                    },
                                    value_prop
                                }
                            ),
                            at: current_block
                        }
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            msp_id: alice_sp_id.unwrap(),
                            value_prop: ValuePropositionWithId {
                                id: value_prop.derive_id(),
                                value_prop,
                            },
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            value_prop: ValuePropositionWithId {
                                id: value_prop.derive_id(),
                                value_prop,
                            },
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
                        alice
                    ));

                    // Request sign up Bob as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(bob),
                        storage_amount_bob,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                        }
                        .into(),
                    );

                    // Check that Bob's event was emitted
                    System::assert_has_event(
                        Event::<Test>::MspRequestSignUpSuccess {
                            who: bob,
                            multiaddresses,
                            capacity: storage_amount_bob,
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
                        alice
                    ));

                    // Check that Alice's request to sign up as a Main Storage Provider exists and is the one we just created
                    let current_block = frame_system::Pallet::<Test>::block_number();
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice)
                        .expect("Alice's sign up request should exist after requesting to sign up");
                    assert!(
                        alice_sign_up_request.sp_sign_up_request
                            == SignUpRequestSpParams::MainStorageProvider(
                                MainStorageProviderSignUpRequest {
                                    msp_info: MainStorageProvider {
                                        capacity: storage_amount,
                                        capacity_used: 0,
                                        multiaddresses: multiaddresses.clone(),
                                        last_capacity_change: current_block,
                                        owner_account: alice,
                                        payment_account: alice,
                                        sign_up_block: current_block
                                    },
                                    value_prop
                                }
                            )
                    );
                    assert!(alice_sign_up_request.at == current_block);

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
            use crate::types::SignUpRequest;

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
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice)
                        .expect("Alice's sign up request should exist after requesting to sign up");
                    assert_eq!(
                        alice_sign_up_request,
                        SignUpRequest::<Test> {
                            sp_sign_up_request: SignUpRequestSpParams::BackupStorageProvider(
                                BackupStorageProvider {
                                    root: DefaultMerkleRoot::get(),
                                    capacity: storage_amount,
                                    capacity_used: 0,
                                    multiaddresses,
                                    last_capacity_change: current_block,
                                    owner_account: alice,
                                    payment_account: alice,
                                    reputation_weight:
                                        <Test as crate::Config>::StartingReputationWeight::get(),
                                    sign_up_block: current_block
                                }
                            ),
                            at: current_block
                        }
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
                    let alice_sign_up_request = StorageProviders::get_sign_up_request(&alice)
                        .expect("Alice's sign up request should exist after requesting to sign up");
                    assert!(
                        alice_sign_up_request.sp_sign_up_request
                            == SignUpRequestSpParams::BackupStorageProvider(
                                BackupStorageProvider {
                                    capacity: storage_amount,
                                    capacity_used: 0,
                                    multiaddresses: multiaddresses.clone(),
                                    root: DefaultMerkleRoot::get(),
                                    last_capacity_change: current_block,
                                    owner_account: alice,
                                    payment_account: alice,
                                    reputation_weight:
                                        <Test as crate::Config>::StartingReputationWeight::get(),
                                    sign_up_block: current_block
                                }
                            )
                    );
                    assert!(alice_sign_up_request.at == current_block);

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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            msp_id: alice_sp_id.unwrap(),
                            value_prop: ValuePropositionWithId {
                                id: value_prop.derive_id(),
                                value_prop,
                            },
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            msp_id: alice_sp_id.unwrap(),
                            value_prop: ValuePropositionWithId {
                                id: value_prop.derive_id(),
                                value_prop,
                            },
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            msp_id: alice_sp_id.unwrap(),
                            value_prop: ValuePropositionWithId {
                                id: value_prop.derive_id(),
                                value_prop,
                            },
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            value_prop: ValuePropositionWithId {
                                id: value_prop.derive_id(),
                                value_prop,
                            },
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
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
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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

                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request to sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
                        alice
                    ));

                    // Try to request to sign up Alice as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.price_per_unit_of_data_per_block,
                            value_prop.commitment.clone(),
                            value_prop.bucket_data_limit,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Request sign up of Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                    let (_alice_deposit, alice_msp, _) =
                        register_account_as_msp(alice, 100, None, None);
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
                            1,
                            bounded_vec![],
                            10,
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
                            1,
                            bounded_vec![],
                            10,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice and Bob
                    let alice: AccountId = accounts::ALICE.0;
                    let bob: AccountId = 1;

                    // Request to sign up Alice as a Main Storage Provider
                    assert_ok!(StorageProviders::request_msp_sign_up(
                        RuntimeOrigin::signed(alice),
                        storage_amount,
                        multiaddresses.clone(),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit,
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
                            value_prop.price_per_unit_of_data_per_block,
                            value_prop.commitment.clone(),
                            value_prop.bucket_data_limit,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 1;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Try to sign up Alice as a Main Storage Provider with less than the minimum capacity
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.price_per_unit_of_data_per_block,
                            value_prop.commitment.clone(),
                            value_prop.bucket_data_limit,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Helen (who has no balance)
                    let helen: AccountId = 7;

                    // Try to sign up Helen as a Main Storage Provider
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(helen),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.price_per_unit_of_data_per_block,
                            value_prop.commitment.clone(),
                            value_prop.bucket_data_limit,
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
                    let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);
                    let storage_amount: StorageDataUnit<Test> = 100;

                    // Get the Account Id of Alice
                    let alice: AccountId = accounts::ALICE.0;

                    // Try to sign up Alice as a Main Storage Provider with no multiaddresses
                    assert_noop!(
                        StorageProviders::request_msp_sign_up(
                            RuntimeOrigin::signed(alice),
                            storage_amount,
                            multiaddresses.clone(),
                            value_prop.price_per_unit_of_data_per_block,
                            value_prop.commitment.clone(),
                            value_prop.bucket_data_limit,
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

                    let value_prop = (<Test as crate::Config>::ValuePropId::default(), ValueProposition::<Test>::new(1, 10));
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
                    let (deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, storage_amount, None, None);

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

                    // Advance enough blocks for the BSP to sign off
                    let bsp_sign_up_lock_period: u64 =
                        <Test as crate::Config>::BspSignUpLockPeriod::get();
                    run_to_block(
                        frame_system::Pallet::<Test>::block_number() + bsp_sign_up_lock_period,
                    );

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
                    let (deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, storage_amount, None, None);

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

            #[test]
            fn bsp_sign_up_fails_when_lock_period_not_passed() {
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

                    // Try to sign off Alice as a Backup Storage Provider
                    assert_noop!(
                        StorageProviders::bsp_sign_off(RuntimeOrigin::signed(alice)),
                        Error::<Test>::SignOffPeriodNotPassed
                    );

                    // Make sure that Alice is still registered as a Backup Storage Provider
                    let alice_sp_id = StorageProviders::get_provider_id(alice);
                    assert!(alice_sp_id.is_some());
                    assert!(StorageProviders::is_provider(alice_sp_id.unwrap()));

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
                    let (old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (_old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (_old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (_old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (_old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (_old_deposit_amount, _alice_sp_id, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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
                    let (_old_deposit_amount, _alice_msp, _) =
                        register_account_as_msp(alice, old_storage_amount, None, None);

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

mod add_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn add_bucket_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                // Try to add the bucket for Alice with the same bucket id
                assert_noop!(
                    StorageProviders::add_bucket(
                        Some(msp_id),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(value_prop_id)
                    ),
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
                    &bucket_owner,
                    bucket_name,
                );

                // Try to add a bucket to a non-registered MSP
                assert_noop!(
                    StorageProviders::add_bucket(
                        Some(MainStorageProviderId::<Test>::default()),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(HashId::<Test>::default())
                    ),
                    Error::<Test>::NotRegistered
                );
            });
        }
    }

    mod success {
        use crate::Config;

        use super::*;

        #[test]
        fn add_bucket() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                // Check payment stream was added
                assert!(
                    <<Test as Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &msp_id,
                        &bucket_owner
                    )
                );

                let new_stream_deposit: u64 = <Test as pallet_payment_streams::Config>::NewStreamDeposit::get();
                assert_eq!(
                    NativeBalance::free_balance(&bucket_owner),
                    accounts::BOB.1 - <BucketDeposit as Get<u128>>::get() - new_stream_deposit as u128
                );

                let new_rate = <<Test as Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                    &msp_id,
                    &bucket_owner
                ).unwrap_or_default();

                let zero_size_bucket_rate: u128 = <Test as Config>::ZeroSizeBucketFixedRate::get();

                // Check that the fixed rate payment stream increased by 10 zero size bucket rates
                assert_eq!(zero_size_bucket_rate, new_rate);

                assert_eq!(
                    NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                    BucketDeposit::get()
                );

                assert!(
                    crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id, bucket_id)
                    .is_some()
                );

                let bucket = crate::Buckets::<Test>::get(&bucket_id).unwrap();

                assert_eq!(
                    bucket,
                    Bucket::<Test> {
                        root: DefaultMerkleRoot::get(),
                        user_id: bucket_owner,
                        msp_id: Some(msp_id),
                        private: false,
                        read_access_group_id: None,
                        size: 0,
                        value_prop_id: Some(value_prop_id),
                    }
                );

            });
        }

        #[test]
        fn add_multiple_buckets() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;

                // Add the maximum amount of buckets for Alice
                let num_buckets = 10;
                for i in 0..10 {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        Some(msp_id),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(value_prop_id)
                    ));

                    let expected_hold_amount =
                        (i + 1) as u128 * <BucketDeposit as Get<u128>>::get();
                    assert_eq!(
                        NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                        expected_hold_amount
                    );


                    // Check that the fixed rate payment stream matches the expected zero size bucket rate * the current number of buckets
                    let zero_size_bucket_rate: u128 = <Test as crate::Config>::ZeroSizeBucketFixedRate::get();
                    let expected_fixed_payment_stream_value = zero_size_bucket_rate * (i + 1) as u128;
                    let fixed_payment_stream_value = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();
                    assert_eq!(fixed_payment_stream_value, expected_fixed_payment_stream_value);
                }

                let buckets =
                    crate::MainStorageProviderIdsToBuckets::<Test>::iter_key_prefix(&msp_id)
                        .collect::<Vec<_>>();

                assert_eq!(buckets.len(), num_buckets);
            });
        }
    }
}

mod unassign_msp_from_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn bucket_not_found() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Try to change a bucket that does not exist
                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::unassign_msp_from_bucket(
                        &bucket_id
                    ),
                    Error::<Test>::BucketNotFound
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn unassign_msp_from_bucket_works() {
            ExtBuilder::build().execute_with(|| {
                // Register Alice as MSP
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                // Create bucket
                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                assert!(
                    <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &msp_id,
                        &bucket_owner
                    )
                );

                assert_ok!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::unassign_msp_from_bucket(&bucket_id),
                );

                assert!(
                    !<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &msp_id,
                        &bucket_owner
                    )
                );

                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::unassign_msp_from_bucket(&bucket_id),
                    Error::<Test>::BucketMustHaveMspForOperation
                );
            });
        }
    }
}

mod assign_msp_to_bucket {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn bucket_not_found() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Try to change a bucket that does not exist
                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::assign_msp_to_bucket(
                        &bucket_id, &msp_id,
                    ),
                    Error::<Test>::BucketNotFound
                );
            });
        }

        #[test]
        fn msp_already_assigned_to_bucket() {
            ExtBuilder::build().execute_with(|| {
                // Register Alice as MSP
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                // Create bucket
                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add bucket
                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::assign_msp_to_bucket(
                        &bucket_id, &msp_id,
                    ),
                    Error::<Test>::MspAlreadyAssignedToBucket
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn assign_msp_to_bucket() {
            ExtBuilder::build().execute_with(|| {
                // Register Alice as MSP
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let alice_msp_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                // Register Charlie as MSP
                let charlie: AccountId = accounts::CHARLIE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _charlie_msp, _) =
                    register_account_as_msp(charlie, storage_amount, None, None);

                let charlie_msp_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&charlie).unwrap();

                // Create bucket
                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add bucket
                assert_ok!(StorageProviders::add_bucket(
                    Some(alice_msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                // check payment stream exists for alice
                assert!(
                    <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &alice_msp_id,
                        &bucket_owner
                    )
                );

                // Change MSP of bucket
                assert_ok!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::assign_msp_to_bucket(
                        &bucket_id,
                        &charlie_msp_id,
                    )
                );

                // Check that the bucket was removed from alice
                assert!(crate::MainStorageProviderIdsToBuckets::<Test>::get(
                    &alice_msp_id,
                    bucket_id
                )
                .is_none());

                // Check that the bucket was added to the default MSP
                assert!(crate::MainStorageProviderIdsToBuckets::<Test>::get(
                    &charlie_msp_id,
                    bucket_id
                )
                .is_some());

                // Check that the bucket was updated
                let bucket = crate::Buckets::<Test>::get(&bucket_id).unwrap();
                assert_eq!(
                    bucket.msp_id,
                    Some(charlie_msp_id)
                );

                // check payment stream exists for charlie
                assert!(
                    <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &charlie_msp_id,
                        &bucket_owner
                    )
                );

                // check payment stream does not exist for alice
                assert!(
                    !<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &alice_msp_id,
                        &bucket_owner
                    )
                );
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
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
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
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                // Check that the bucket was added to the MSP
                assert!(
                    crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id, bucket_id)
                        .is_some()
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
                assert!(
                    crate::MainStorageProviderIdsToBuckets::<Test>::get(&msp_id, bucket_id)
                        .is_none()
                );

                // Check payment stream was removed
                assert!(
                    !<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &msp_id,
                        &bucket_owner
                    )
                );
            });
        }

        #[test]
        fn remove_root_buckets_multiple() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;

                // Add the maximum amount of buckets for Alice
                let num_buckets = 10;
                for i in 0..10 {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        Some(msp_id),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(value_prop_id)
                    ));

                    let expected_hold_amount =
                        (i + 1) as u128 * <BucketDeposit as Get<u128>>::get();
                    assert_eq!(
                        NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                        expected_hold_amount
                    );
                }

                let buckets =
                    crate::MainStorageProviderIdsToBuckets::<Test>::iter_key_prefix(&msp_id)
                        .collect::<Vec<_>>();

                assert_eq!(buckets.len(), num_buckets);

                // Remove all the buckets
                for i in 0..num_buckets {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::remove_root_bucket(bucket_id));
                    if i < num_buckets - 1 {
                        // Check that the payment streams still exists if we haven't removed the last bucket
                        assert!(
                            <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                                &msp_id,
                                &bucket_owner
                            )
                        );

                        // Check that the fixed rate payment stream matches the expected zero size bucket rate * the current number of buckets
                        let zero_size_bucket_rate: u128 = <Test as crate::Config>::ZeroSizeBucketFixedRate::get();
                        let expected_fixed_payment_stream_value = zero_size_bucket_rate * (num_buckets - i - 1) as u128;
                        let fixed_payment_stream_value = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                            &msp_id,
                            &bucket_owner
                        ).unwrap_or_default();
                        assert_eq!(fixed_payment_stream_value, expected_fixed_payment_stream_value);
                    }
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
                    crate::MainStorageProviderIdsToBuckets::<Test>::iter_key_prefix(&msp_id)
                        .count(),
                    0
                );

                // Check that the payment streams was removed
                assert!(
                    !<<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::fixed_rate_payment_stream_exists(
                        &msp_id,
                        &bucket_owner
                    )
                );
            });
        }
    }
}

mod increase_bucket_size {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn bucket_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Try to remove a bucket that does not exist
                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::increase_bucket_size(
                        &bucket_id, 100
                    ),
                    Error::<Test>::BucketNotFound
                );
            });
        }

        #[test]
        fn bucket_must_have_msp_for_operation() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                // Remove the MSP from the bucket
                assert_ok!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::unassign_msp_from_bucket(
                        &bucket_id
                    )
                );

                // Try to increase the size of a bucket that does not have an MSP
                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::increase_bucket_size(
                        &bucket_id, 100
                    ),
                    Error::<Test>::BucketMustHaveMspForOperation
                );
            });
        }
    }

    mod success {
        use crate::MainStorageProviderIdsToValuePropositions;

        use super::*;

        #[test]
        fn increase_bucket_size_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;

                let num_buckets = 10;
                let delta_increase = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, Some(10), Some(num_buckets * delta_increase));

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;

                // Add the maximum amount of buckets for Alice
                for i in 0..10 {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        Some(msp_id),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(value_prop_id)
                    ));

                    let expected_hold_amount =
                        (i + 1) as u128 * <BucketDeposit as Get<u128>>::get();
                    assert_eq!(
                        NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                        expected_hold_amount
                    );
                }

                let buckets =
                    crate::MainStorageProviderIdsToBuckets::<Test>::iter_key_prefix(&msp_id)
                        .collect::<Vec<_>>();

                assert_eq!(buckets.len(), num_buckets as usize);

                // Remove all the buckets
                for i in 0..num_buckets {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );

                    let current_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();

                    assert_ok!(<crate::Pallet<Test> as MutateBucketsInterface>::increase_bucket_size(
                        &bucket_id,
                        delta_increase
                    ));

                    // Check that the fixed rate payment stream matches the expected rate
                    let value_prop = MainStorageProviderIdsToValuePropositions::<Test>::get(&msp_id, value_prop_id).unwrap();
                    let delta_rate = value_prop.price_per_unit_of_data_per_block * delta_increase as u128;

                    let expected_rate = current_rate + delta_rate;
                    let actual_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();

                    assert_eq!(actual_rate, expected_rate);
                }
            });
        }
    }
}

mod decrease_bucket_size {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn bucket_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Try to remove a bucket that does not exist
                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::decrease_bucket_size(
                        &bucket_id, 100
                    ),
                    Error::<Test>::BucketNotFound
                );
            });
        }

        #[test]
        fn bucket_must_have_msp_for_operation() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Add a bucket for Alice
                assert_ok!(StorageProviders::add_bucket(
                    Some(msp_id),
                    bucket_owner,
                    bucket_id,
                    false,
                    None,
                    Some(value_prop_id)
                ));

                // Remove the MSP from the bucket
                assert_ok!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::unassign_msp_from_bucket(
                        &bucket_id
                    )
                );

                // Try to increase the size of a bucket that does not have an MSP
                assert_noop!(
                    <crate::Pallet<Test> as MutateBucketsInterface>::decrease_bucket_size(
                        &bucket_id, 100
                    ),
                    Error::<Test>::BucketMustHaveMspForOperation
                );
            });
        }
    }

    mod success {
        use crate::MainStorageProviderIdsToValuePropositions;

        use super::*;

        #[test]
        fn increase_bucket_size_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;

                let num_buckets = 10;
                let delta_increase = 100;
                let (_deposit_amount, _alice_msp, value_prop_id) =
                    register_account_as_msp(alice, storage_amount, Some(10), Some(num_buckets * delta_increase));

                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let bucket_owner = accounts::BOB.0;

                // Add the maximum amount of buckets for Alice
                for i in 0..10 {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );
                    assert_ok!(StorageProviders::add_bucket(
                        Some(msp_id),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(value_prop_id)
                    ));

                    let expected_hold_amount =
                        (i + 1) as u128 * <BucketDeposit as Get<u128>>::get();
                    assert_eq!(
                        NativeBalance::balance_on_hold(&BucketHoldReason::get(), &bucket_owner),
                        expected_hold_amount
                    );
                }

                let buckets =
                    crate::MainStorageProviderIdsToBuckets::<Test>::iter_key_prefix(&msp_id)
                        .collect::<Vec<_>>();

                assert_eq!(buckets.len(), num_buckets as usize);

                // Increase the bucket size of all buckets
                for i in 0..num_buckets {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );

                    let current_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();

                    assert_ok!(<crate::Pallet<Test> as MutateBucketsInterface>::increase_bucket_size(
                        &bucket_id,
                        delta_increase
                    ));

                    // Check that the fixed rate payment stream matches the expected rate
                    let value_prop = MainStorageProviderIdsToValuePropositions::<Test>::get(&msp_id, value_prop_id).unwrap();
                    let delta_rate = value_prop.price_per_unit_of_data_per_block * delta_increase as u128;

                    let expected_rate = current_rate + delta_rate;
                    let actual_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();

                    assert_eq!(actual_rate, expected_rate);
                }

                // Decrease the bucket size of all buckets
                for i in 0..num_buckets {
                    let bucket_name =
                        BoundedVec::try_from(format!("bucket{}", i).as_bytes().to_vec()).unwrap();
                    let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                        &bucket_owner,
                        bucket_name,
                    );

                    let current_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();

                    assert_ok!(<crate::Pallet<Test> as MutateBucketsInterface>::decrease_bucket_size(
                        &bucket_id,
                        delta_increase
                    ));

                    // Check that the fixed rate payment stream matches the expected rate
                    let value_prop = MainStorageProviderIdsToValuePropositions::<Test>::get(&msp_id, value_prop_id).unwrap();
                    let delta_rate = value_prop.price_per_unit_of_data_per_block * delta_increase as u128;

                    let expected_rate = current_rate - delta_rate;
                    let actual_rate = <<Test as crate::Config>::PaymentStreams as PaymentStreamsInterface>::get_inner_fixed_rate_payment_stream_value(
                        &msp_id,
                        &bucket_owner
                    ).unwrap_or_default();

                    assert_eq!(actual_rate, expected_rate);
                }
            });
        }
    }
}

mod slash_and_top_up {
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
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn slash_when_storage_provider_not_slashable() {
            ExtBuilder::build().execute_with(|| {
                // register msp
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let provider_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                let caller = accounts::BOB.0;

                // Try to slash an unslashable provider
                assert_noop!(
                    StorageProviders::slash(RuntimeOrigin::signed(caller), provider_id),
                    Error::<Test>::ProviderNotSlashable
                );
            });
        }

        #[test]
        fn top_up_when_provider_not_registered() {
            ExtBuilder::build().execute_with(|| {
                let caller = accounts::BOB.0;

                // Try to top up a provider that is not registered
                assert_noop!(
                    StorageProviders::top_up_deposit(RuntimeOrigin::signed(caller),),
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn top_up_when_not_enough_for_held_deposit() {
            ExtBuilder::build().execute_with(|| {
                // register msp
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let alice_msp_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();

                // Manually set a capacity deficit to avoid having to slash the provider
                MainStorageProviders::<Test>::mutate(alice_msp_id, |p| {
                    let p = p.as_mut().unwrap();
                    p.capacity_used = 100;
                    p.capacity = 0;
                });

                // Set provider's balance to existential deposit to simulate the provider not having enough balance to cover the held deposit
                NativeBalance::set_balance(&alice, ExistentialDeposit::get());

                // Try to top up a provider that does not have enough balance to cover the held deposit
                assert_noop!(
                    StorageProviders::top_up_deposit(RuntimeOrigin::signed(alice)),
                    Error::<Test>::CannotHoldDeposit
                );
            });
        }
    }

    mod success {
        use core::u32;

        use sp_runtime::traits::ConvertBack;

        use crate::{AwaitingTopUpFromProviders, GracePeriodToSlashedProviders};

        use super::*;

        struct TestSetup {
            account: AccountId,
            provider_id: HashId<Test>,
            /// Sets accrued slashes to have the deposit be slashed completely
            slash_held_deposit: bool,
            /// Sets the provider's balance to cover the top up required amount
            automatic_top_up: bool,
        }

        impl Default for TestSetup {
            fn default() -> Self {
                let alice: AccountId = accounts::ALICE.0;
                Self::new(alice)
            }
        }

        impl TestSetup {
            fn new(account: AccountId) -> Self {
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(account, storage_amount, None, None);

                let provider_id =
                    crate::AccountIdToMainStorageProviderId::<Test>::get(&account).unwrap();

                MainStorageProviders::<Test>::mutate(provider_id, |p| {
                    p.as_mut().unwrap().capacity_used = 50;
                });

                Self {
                    account,
                    provider_id,
                    slash_held_deposit: false,
                    automatic_top_up: false,
                }
            }

            fn slash_and_verify(&self) {
                // Utility function to calculate the required hold amount needed to cover a potential capacity deficit
                let deposit_needed_for_capacity_used_fn = || -> BalanceOf<Test> {
                    let provider = MainStorageProviders::<Test>::get(self.provider_id).unwrap();
                    let deposit_needed_for_capacity_used: BalanceOf<Test> =
                        StorageProviders::compute_deposit_needed_for_capacity(
                            provider.capacity_used,
                        )
                        .unwrap()
                        .into();
                    deposit_needed_for_capacity_used.saturating_sub(NativeBalance::balance_on_hold(
                        &StorageProvidersHoldReason::get(),
                        &self.account,
                    ))
                };

                // Set proofs-dealer storage to have a slashable provider with a accrued slashes based on the test setup
                pallet_proofs_dealer::SlashableProviders::<Test>::insert(
                    &self.provider_id,
                    if self.slash_held_deposit { u32::MAX } else { 1 },
                );

                let computed_slash_amount =
                    StorageProviders::compute_worst_case_scenario_slashable_amount(
                        &self.provider_id,
                    )
                    .unwrap();

                let pre_state_held_deposit = NativeBalance::balance_on_hold(
                    &StorageProvidersHoldReason::get(),
                    &self.account,
                );

                // Amount expected to be slashed from the held deposit
                // Since `do_slash` will slash on a BestEffort basis, the slash amount is either the entire computed slash amount or the amount in held deposit
                let expected_slash_amount = pre_state_held_deposit.min(computed_slash_amount);

                let pre_state_balance = if self.automatic_top_up {
                    // Provider will be able to top up capacity deficit
                    NativeBalance::set_balance(&self.account, expected_slash_amount * 2)
                } else {
                    // Provider will not be able to top up any capacity deficit
                    NativeBalance::set_balance(&self.account, ExistentialDeposit::get())
                };

                // Get the pre-state values
                let pre_state_required_hold_amount = deposit_needed_for_capacity_used_fn();
                let pre_state_treasury_balance =
                    NativeBalance::free_balance(&<Test as crate::Config>::Treasury::get());
                let pre_state_provider =
                    MainStorageProviders::<Test>::get(self.provider_id).unwrap();

                // Slash the provider
                assert_ok!(StorageProviders::slash(
                    RuntimeOrigin::signed(self.account),
                    self.provider_id
                ));

                // Verify the slash event
                let last_slashed_event = System::events()
                    .iter()
                    .rev()
                    .find_map(|event| {
                        if let RuntimeEvent::StorageProviders(Event::<Test>::Slashed {
                            provider_id,
                            amount,
                        }) = event.event
                        {
                            Some((provider_id, amount))
                        } else {
                            None
                        }
                    })
                    .expect("Expected Slashed event");

                assert_eq!(last_slashed_event.0, self.provider_id);
                assert_eq!(last_slashed_event.1, expected_slash_amount);

                let grace_period: u32 = TopUpGracePeriod::get();
                let end_block_grace_period =
                    RelayBlockGetter::<Test>::current_block_number() + grace_period;

                // Verify post-slash state
                let post_state_provider =
                    MainStorageProviders::<Test>::get(self.provider_id).unwrap();

                if self.automatic_top_up {
                    let required_held_amount = deposit_needed_for_capacity_used_fn();

                    // There should be no more required held amount (i.e. no more capacity deficit)
                    assert_eq!(required_held_amount, 0);

                    System::assert_has_event(
                        Event::<Test>::TopUpFulfilled {
                            provider_id: self.provider_id,
                            amount: pre_state_required_hold_amount + expected_slash_amount,
                        }
                        .into(),
                    );

                    // Free balance should be reduced by the amount needed to cover the outstanding top up slash amount
                    assert_eq!(
                        NativeBalance::free_balance(self.account),
                        pre_state_balance - pre_state_required_hold_amount - expected_slash_amount
                    );

                    // Check that the held deposit of the provider has been automatically topped up to the used capacity required
                    assert_eq!(
                        NativeBalance::balance_on_hold(
                            &StorageProvidersHoldReason::get(),
                            &self.account
                        ),
                        StorageProviders::compute_deposit_needed_for_capacity(
                            post_state_provider.capacity_used,
                        )
                        .unwrap()
                        .into()
                    );

                    // Check that the storage has been cleared
                    assert!(GracePeriodToSlashedProviders::<Test>::get(
                        end_block_grace_period,
                        self.provider_id
                    )
                    .is_none());
                    assert!(AwaitingTopUpFromProviders::<Test>::get(self.provider_id).is_none());

                    // Check that the provider's capacity is equal to used capacity
                    assert_eq!(
                        post_state_provider.capacity_used,
                        post_state_provider.capacity
                    );
                } else if deposit_needed_for_capacity_used_fn() > 0 {
                    System::assert_has_event(
                        Event::<Test>::AwaitingTopUp {
                            provider_id: self.provider_id,
                            top_up_metadata: TopUpMetadata {
                                end_block_grace_period,
                            },
                        }
                        .into(),
                    );

                    // Check that the held deposit of the provider has been slashed and not automatically topped up
                    assert_eq!(
                        NativeBalance::balance_on_hold(
                            &StorageProvidersHoldReason::get(),
                            &self.account
                        ),
                        pre_state_held_deposit.saturating_sub(expected_slash_amount)
                    );

                    // Check that the provider's free balance hasn't been reduced since there was not enough to top up
                    assert_eq!(NativeBalance::free_balance(self.account), pre_state_balance);

                    // Check that the AwaitingTopUpFromProviders storage has been updated
                    let top_up_metadata =
                        AwaitingTopUpFromProviders::<Test>::get(self.provider_id).unwrap();
                    assert_eq!(
                        top_up_metadata.end_block_grace_period,
                        end_block_grace_period
                    );

                    // Check that the provider's capacity was reduced by the converted slash amount (storage data units)
                    let expected_capacity_delta =
                        StorageDataUnitAndBalanceConverter::convert_back(expected_slash_amount);
                    assert_eq!(
                        post_state_provider.capacity,
                        pre_state_provider
                            .capacity
                            .saturating_sub(expected_capacity_delta)
                    );
                }

                // Check that the Treasury has received the slash amount
                assert_eq!(
                    NativeBalance::free_balance(&<Test as crate::Config>::Treasury::get()),
                    pre_state_treasury_balance + expected_slash_amount
                );
            }

            fn top_up(&self) {
                let grace_period: u32 = TopUpGracePeriod::get();
                let end_block_grace_period =
                    RelayBlockGetter::<Test>::current_block_number() + grace_period;

                let pre_state_provider =
                    MainStorageProviders::<Test>::get(self.provider_id).unwrap();

                let pre_state_held_amount = NativeBalance::balance_on_hold(
                    &StorageProvidersHoldReason::get(),
                    &self.account,
                );

                let capacity_deficit =
                    pre_state_provider.capacity_used - pre_state_provider.capacity;

                let expected_delta_hold_amount =
                    StorageDataUnitAndBalanceConverter::convert(capacity_deficit);

                let existential_deposit = ExistentialDeposit::get();
                // Set provider's balance to cover the expected_delta_hold_amount
                NativeBalance::set_balance(
                    &self.account,
                    expected_delta_hold_amount + existential_deposit,
                );

                // Top up the provider
                assert_ok!(StorageProviders::top_up_deposit(RuntimeOrigin::signed(
                    self.account
                ),));

                // Check event TopUpFulfilled is emitted
                System::assert_has_event(
                    Event::<Test>::TopUpFulfilled {
                        provider_id: self.provider_id,
                        amount: expected_delta_hold_amount,
                    }
                    .into(),
                );

                // Check that the held deposit covers the used capacity
                assert_eq!(
                    NativeBalance::balance_on_hold(
                        &StorageProvidersHoldReason::get(),
                        &self.account
                    ),
                    pre_state_held_amount + expected_delta_hold_amount
                );

                // Check that the storage has been cleared
                assert!(GracePeriodToSlashedProviders::<Test>::get(
                    end_block_grace_period,
                    self.provider_id
                )
                .is_none());
                assert!(AwaitingTopUpFromProviders::<Test>::get(self.provider_id).is_none());
            }
        }

        #[test]
        fn slash_storage_provider_automatic_top_up() {
            ExtBuilder::build().execute_with(|| {
                let mut test_setup = TestSetup::default();

                test_setup.slash_held_deposit = true;
                test_setup.slash_and_verify();

                test_setup.automatic_top_up = true;
                test_setup.slash_and_verify();
            });
        }

        #[test]
        fn slash_storage_provider_manual_top_up() {
            ExtBuilder::build().execute_with(|| {
                let mut test_setup = TestSetup::default();

                test_setup.slash_held_deposit = true;
                test_setup.slash_and_verify();

                test_setup.top_up();
            });
        }

        #[test]
        fn automatic_top_up_after_many_slashes() {
            ExtBuilder::build().execute_with(|| {
                let mut test_setup = TestSetup::default();
                // Test accrued slashes
                test_setup.slash_and_verify();
                test_setup.slash_held_deposit = true;
                test_setup.slash_and_verify();

                // Test automatic top up when provider is slashed
                test_setup.automatic_top_up = true;
                test_setup.slash_and_verify();
            });
        }

        #[test]
        fn automatic_top_up_after_many_slashes_of_many_storage_providers() {
            ExtBuilder::build().execute_with(|| {
                let mut alice_test_setup = TestSetup::default();
                let mut bob_test_setup = TestSetup::new(accounts::BOB.0);

                alice_test_setup.slash_and_verify();

                bob_test_setup.slash_and_verify();

                alice_test_setup.slash_held_deposit = true;
                alice_test_setup.slash_and_verify();

                bob_test_setup.slash_held_deposit = true;
                bob_test_setup.slash_and_verify();

                alice_test_setup.automatic_top_up = true;
                alice_test_setup.slash_and_verify();

                bob_test_setup.automatic_top_up = true;
                bob_test_setup.slash_and_verify();
            });
        }

        #[test]
        fn manual_top_up_after_many_slashes_of_many_storage_providers() {
            ExtBuilder::build().execute_with(|| {
                let mut alice_test_setup = TestSetup::default();
                let mut bob_test_setup = TestSetup::new(accounts::BOB.0);

                alice_test_setup.slash_and_verify();

                bob_test_setup.slash_and_verify();

                alice_test_setup.slash_held_deposit = true;
                alice_test_setup.slash_and_verify();

                bob_test_setup.slash_held_deposit = true;
                bob_test_setup.slash_and_verify();

                alice_test_setup.top_up();
                bob_test_setup.top_up();
            });
        }
    }
}

mod multiaddresses {
    use super::*;

    mod failure {
        use super::*;

        #[test]
        fn add_multiaddress_fails_when_provider_not_registered() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let new_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();

                // Try to add a multiaddress to an account that is not registered as an MSP
                assert_noop!(
                    StorageProviders::add_multiaddress(
                        RuntimeOrigin::signed(alice),
                        new_multiaddress
                    ),
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn add_multiaddress_fails_if_multiaddress_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let new_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();

                // Add a multiaddress to Alice
                assert_ok!(StorageProviders::add_multiaddress(
                    RuntimeOrigin::signed(alice),
                    new_multiaddress.clone()
                ));

                // Try to add the same multiaddress to Alice
                assert_noop!(
                    StorageProviders::add_multiaddress(
                        RuntimeOrigin::signed(alice),
                        new_multiaddress
                    ),
                    Error::<Test>::MultiAddressAlreadyExists
                );
            });
        }

        #[test]
        fn add_multiaddress_fails_if_max_multiaddresses_reached() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                // Add the maximum amount of multiaddresses for Alice (we start at 1 since Alice already has a MultiAddress)
                for i in 1..MaxMultiAddressAmount::<Test>::get() {
                    let multiaddress: MultiAddress<Test> = format!(
                        "/ip4/127.0.0.1/udp/1234/new/multiaddress/{}",
                        i.to_string().as_str()
                    )
                    .as_bytes()
                    .to_vec()
                    .try_into()
                    .unwrap();
                    assert_ok!(StorageProviders::add_multiaddress(
                        RuntimeOrigin::signed(alice),
                        multiaddress
                    ));
                }

                let multiaddress_over_limit: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();

                // Try to add another multiaddress for Alice
                assert_noop!(
                    StorageProviders::add_multiaddress(
                        RuntimeOrigin::signed(alice),
                        multiaddress_over_limit
                    ),
                    Error::<Test>::MultiAddressesMaxAmountReached
                );
            });
        }

        #[test]
        fn remove_multiaddress_fails_when_provider_not_registered() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let new_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();

                // Try to remove a multiaddress from an account that is not registered as an MSP
                assert_noop!(
                    StorageProviders::remove_multiaddress(
                        RuntimeOrigin::signed(alice),
                        new_multiaddress
                    ),
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn remove_multiaddress_fails_when_multiaddress_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                // Add a new multiaddress to Alice
                let new_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();
                assert_ok!(StorageProviders::add_multiaddress(
                    RuntimeOrigin::signed(alice),
                    new_multiaddress.clone()
                ));

                // Get a multiaddress that does not exist
                let non_saved_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/no/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();

                // Try to remove a multiaddress that does not exist
                assert_noop!(
                    StorageProviders::remove_multiaddress(
                        RuntimeOrigin::signed(alice),
                        non_saved_multiaddress
                    ),
                    Error::<Test>::MultiAddressNotFound
                );
            });
        }

        #[test]
        fn remove_multiaddress_fails_if_multiaddress_is_the_last_one() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                // Try to remove the only multiaddress of Alice
                assert_noop!(
                    StorageProviders::remove_multiaddress(
                        RuntimeOrigin::signed(alice),
                        "/ip4/127.0.0.1/udp/1234"
                            .as_bytes()
                            .to_vec()
                            .try_into()
                            .unwrap()
                    ),
                    Error::<Test>::LastMultiAddressCantBeRemoved
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn add_multiaddress() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let new_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                // Add a multiaddress to Alice
                assert_ok!(StorageProviders::add_multiaddress(
                    RuntimeOrigin::signed(alice),
                    new_multiaddress.clone()
                ));

                // Check that the multiaddress was added to the MSP
                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();
                let msp_info = crate::MainStorageProviders::<Test>::get(&msp_id).unwrap();

                assert_eq!(msp_info.multiaddresses.len(), 2);
                assert_eq!(msp_info.multiaddresses[1], new_multiaddress);
            });
        }

        #[test]
        fn add_multiaddress_to_max_multiaddresses() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                // Add the maximum amount of multiaddresses for Alice (we start at 1 since Alice already has a MultiAddress)
                for i in 1usize..<MaxMultiAddressAmount<Test> as Get<u32>>::get() as usize {
                    let multiaddress: MultiAddress<Test> = format!(
                        "/ip4/127.0.0.1/udp/1234/new/multiaddress/{}",
                        i.to_string().as_str()
                    )
                    .as_bytes()
                    .to_vec()
                    .try_into()
                    .unwrap();
                    assert_ok!(StorageProviders::add_multiaddress(
                        RuntimeOrigin::signed(alice),
                        multiaddress.clone()
                    ));

                    let msp_id =
                        crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();
                    let msp_info = crate::MainStorageProviders::<Test>::get(&msp_id).unwrap();

                    assert_eq!(msp_info.multiaddresses[i], multiaddress);
                    assert_eq!(msp_info.multiaddresses.len(), i + 1);
                }
            });
        }

        #[test]
        fn remove_multiaddress() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _value_prop_id) =
                    register_account_as_msp(alice, storage_amount, None, None);

                // We first add a multiaddress to Alice
                let new_multiaddress: MultiAddress<Test> =
                    "/ip4/127.0.0.1/udp/1234/new/multiaddress"
                        .as_bytes()
                        .to_vec()
                        .try_into()
                        .unwrap();

                assert_ok!(StorageProviders::add_multiaddress(
                    RuntimeOrigin::signed(alice),
                    new_multiaddress.clone()
                ));

                // Check that the multiaddress was added to the MSP
                let msp_id = crate::AccountIdToMainStorageProviderId::<Test>::get(&alice).unwrap();
                let msp_info = crate::MainStorageProviders::<Test>::get(&msp_id).unwrap();
                assert_eq!(msp_info.multiaddresses.len(), 2);
                assert_eq!(msp_info.multiaddresses[1], new_multiaddress);

                // Remove the original multiaddress from Alice
                let initial_multiaddress = msp_info.multiaddresses[0].clone();
                assert_ok!(StorageProviders::remove_multiaddress(
                    RuntimeOrigin::signed(alice),
                    initial_multiaddress.clone()
                ));

                // Check that the multiaddress was removed from the MSP
                let msp_info = crate::MainStorageProviders::<Test>::get(&msp_id).unwrap();
                assert_eq!(msp_info.multiaddresses.len(), 1);
                assert_eq!(msp_info.multiaddresses[0], new_multiaddress);
            });
        }
    }
}

mod add_value_prop {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn account_is_not_a_registered_msp() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);

                // Try to add a value proposition to an account that is not a registered MSP
                assert_noop!(
                    StorageProviders::add_value_prop(
                        RuntimeOrigin::signed(alice),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit
                    ),
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn value_prop_already_exists() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let value_prop = ValueProposition::<Test>::new(999, bounded_vec![], 999);

                assert_ok!(StorageProviders::add_value_prop(
                    RuntimeOrigin::signed(alice),
                    value_prop.price_per_unit_of_data_per_block,
                    value_prop.commitment.clone(),
                    value_prop.bucket_data_limit
                ));

                assert_noop!(
                    StorageProviders::add_value_prop(
                        RuntimeOrigin::signed(alice),
                        value_prop.price_per_unit_of_data_per_block,
                        value_prop.commitment.clone(),
                        value_prop.bucket_data_limit
                    ),
                    Error::<Test>::ValuePropositionAlreadyExists
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn add_value_prop_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);
                let msp_id = StorageProviders::get_provider_id(alice).unwrap();

                let value_prop = ValueProposition::<Test>::new(999, bounded_vec![], 999);

                assert_ok!(StorageProviders::add_value_prop(
                    RuntimeOrigin::signed(alice),
                    value_prop.price_per_unit_of_data_per_block,
                    value_prop.commitment.clone(),
                    value_prop.bucket_data_limit
                ));

                let value_prop_id = value_prop.derive_id();

                // Check event is emitted
                System::assert_last_event(
                    Event::<Test>::ValuePropAdded {
                        msp_id,
                        value_prop_id,
                        value_prop: value_prop.clone(),
                    }
                    .into(),
                );

                assert_eq!(
                    crate::MainStorageProviderIdsToValuePropositions::<Test>::get(
                        &msp_id,
                        value_prop_id
                    ),
                    Some(value_prop)
                );
            });
        }
    }
}

mod make_value_prop_unavailable {
    use super::*;
    mod failure {
        use super::*;

        #[test]
        fn account_is_not_a_registered_msp() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let value_prop = ValueProposition::<Test>::new(1, bounded_vec![], 10);

                // Try to make a value proposition unavailable to an account that is not a registered MSP
                assert_noop!(
                    StorageProviders::make_value_prop_unavailable(
                        RuntimeOrigin::signed(alice),
                        value_prop.derive_id()
                    ),
                    Error::<Test>::NotRegistered
                );
            });
        }

        #[test]
        fn value_prop_does_not_exist() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let value_prop = ValueProposition::<Test>::new(999, bounded_vec![], 999);

                assert_noop!(
                    StorageProviders::make_value_prop_unavailable(
                        RuntimeOrigin::signed(alice),
                        value_prop.derive_id()
                    ),
                    Error::<Test>::ValuePropositionNotFound
                );
            });
        }
    }

    mod success {
        use super::*;

        #[test]
        fn make_value_prop_unavailable_works() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);
                let msp_id = StorageProviders::get_provider_id(alice).unwrap();

                let value_prop = ValueProposition::<Test>::new(999, bounded_vec![], 999);

                assert_ok!(StorageProviders::add_value_prop(
                    RuntimeOrigin::signed(alice),
                    value_prop.price_per_unit_of_data_per_block,
                    value_prop.commitment.clone(),
                    value_prop.bucket_data_limit
                ));

                let value_prop_id = value_prop.derive_id();

                assert_ok!(StorageProviders::make_value_prop_unavailable(
                    RuntimeOrigin::signed(alice),
                    value_prop_id
                ));

                // Check event is emitted
                System::assert_last_event(
                    Event::<Test>::ValuePropUnavailable {
                        msp_id,
                        value_prop_id,
                    }
                    .into(),
                );

                assert_eq!(
                    crate::MainStorageProviderIdsToValuePropositions::<Test>::get(
                        &msp_id,
                        value_prop_id
                    )
                    .unwrap(),
                    ValueProposition::<Test> {
                        price_per_unit_of_data_per_block: 999,
                        commitment: bounded_vec![],
                        bucket_data_limit: 999,
                        available: false
                    }
                );
            });
        }

        #[test]
        fn create_bucket_fails_when_value_prop_is_unavailable() {
            ExtBuilder::build().execute_with(|| {
                let alice: AccountId = accounts::ALICE.0;
                let storage_amount: StorageDataUnit<Test> = 100;
                let (_deposit_amount, _alice_msp, _) =
                    register_account_as_msp(alice, storage_amount, None, None);

                let msp_id = StorageProviders::get_provider_id(alice).unwrap();

                let value_prop = ValueProposition::<Test>::new(999, bounded_vec![], 999);

                assert_ok!(StorageProviders::add_value_prop(
                    RuntimeOrigin::signed(alice),
                    value_prop.price_per_unit_of_data_per_block,
                    value_prop.commitment.clone(),
                    value_prop.bucket_data_limit
                ));

                let value_prop_id = value_prop.derive_id();

                assert_ok!(StorageProviders::make_value_prop_unavailable(
                    RuntimeOrigin::signed(alice),
                    value_prop_id
                ));

                let bucket_owner = accounts::BOB.0;
                let bucket_name = BoundedVec::try_from(b"bucket".to_vec()).unwrap();
                let bucket_id = <StorageProviders as ReadBucketsInterface>::derive_bucket_id(
                    &bucket_owner,
                    bucket_name,
                );

                // Try to add a bucket with an unavailable value proposition
                assert_noop!(
                    StorageProviders::add_bucket(
                        Some(msp_id),
                        bucket_owner,
                        bucket_id,
                        false,
                        None,
                        Some(value_prop_id)
                    ),
                    Error::<Test>::ValuePropositionNotAvailable
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
    value_prop_price_per_unit_of_data_per_block: Option<BalanceOf<Test>>,
    value_prop_bucket_size_limit: Option<StorageDataUnit<Test>>,
) -> (BalanceOf<Test>, MainStorageProvider<Test>, HashId<Test>) {
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

    let value_prop = ValueProposition::<Test>::new(
        value_prop_price_per_unit_of_data_per_block.unwrap_or(1),
        bounded_vec![],
        value_prop_bucket_size_limit.unwrap_or(100),
    );

    // Request to sign up the account as a Main Storage Provider
    assert_ok!(StorageProviders::request_msp_sign_up(
        RuntimeOrigin::signed(account),
        storage_amount,
        multiaddresses.clone(),
        value_prop.price_per_unit_of_data_per_block,
        bounded_vec![],
        value_prop.bucket_data_limit,
        account
    ));

    // Check that the request sign up event was emitted
    System::assert_last_event(
        Event::<Test>::MspRequestSignUpSuccess {
            who: account,
            multiaddresses: multiaddresses.clone(),
            capacity: storage_amount,
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

    let value_prop_id = value_prop.derive_id();

    // Check that the confirm MSP sign up event was emitted
    System::assert_last_event(
        Event::<Test>::MspSignUpSuccess {
            who: account,
            msp_id,
            multiaddresses: multiaddresses.clone(),
            capacity: storage_amount,
            value_prop: ValuePropositionWithId {
                id: value_prop_id,
                value_prop,
            },
        }
        .into(),
    );

    // Return the deposit amount that was utilized from the account's balance and the MSP information
    (
        deposit_for_storage_amount,
        MainStorageProvider {
            capacity: storage_amount,
            capacity_used: 0,
            multiaddresses,
            last_capacity_change: frame_system::Pallet::<Test>::block_number(),
            owner_account: account,
            payment_account: account,
            sign_up_block: frame_system::Pallet::<Test>::block_number(),
        },
        value_prop_id,
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
            sign_up_block: frame_system::Pallet::<Test>::block_number(),
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

/// This module is just a test to make sure the MockRandomness trait works.
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

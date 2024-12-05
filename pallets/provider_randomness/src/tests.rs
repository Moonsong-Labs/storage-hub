use crate::{mock::*, types::*, DeadlineBlockToProviders, Event, FirstSubmittersProviders};
use frame_support::{
    assert_ok,
    pallet_prelude::Weight,
    traits::{OnFinalize, OnIdle, OnInitialize, OnPoll},
    weights::WeightMeter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_storage_providers::types::{MaxMultiAddressAmount, MultiAddress};
use sp_core::{blake2_256, Get};
use sp_runtime::{
    testing::H256,
    traits::{BlakeTwo256, Hash},
    BoundedVec,
};

#[test]
fn provider_cycle_is_initialised_correctly() {
    ExtBuilder::build().execute_with(|| {
        let alice: AccountId = 0;
        let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");

        // Register Alice as a Provider
        register_account_as_provider(alice, alice_provider_id);

        // Initialise its randomness cycle
        assert_ok!(CrRandomness::force_initialise_provider_cycle(
            RuntimeOrigin::root(),
            alice_provider_id,
        ));

        // Check that the Provider's cycle has been correctly initialised
        let maybe_first_deadline: Option<BlockNumberFor<Test>> =
            FirstSubmittersProviders::<Test>::get(alice_provider_id);
        assert!(maybe_first_deadline.is_some());
        assert!(
            DeadlineBlockToProviders::<Test>::get(maybe_first_deadline.unwrap())
                .contains(&alice_provider_id)
        );

        // Check that the `ProviderCycleInitialised` event has been emitted
        System::assert_last_event(
            Event::ProviderCycleInitialised {
                provider_id: alice_provider_id,
                first_seed_commitment_deadline_block: maybe_first_deadline.unwrap(),
            }
            .into(),
        );
    });
}

/// Helper function that advances the blockchain until block n, executing the hooks for each block
fn run_to_block(n: u64) {
    assert!(n > System::block_number(), "Cannot go back in time");

    while System::block_number() < n {
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        CrRandomness::on_poll(System::block_number(), &mut WeightMeter::new());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    }
}

/// Function that registers a Provider for the given account with the given Provider ID
fn register_account_as_provider(account: AccountId, provider_id: ProviderIdFor<Test>) {
    // Initialize variables:
    let capacity = 1000;
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
    // The deposit for any amount of storage is be MinDeposit + DepositPerData * (capacity - MinCapacity)
    let deposit_for_capacity: BalanceOf<Test> =
        <<Test as pallet_storage_providers::Config>::SpMinDeposit as Get<u128>>::get()
            .saturating_add(
                <<Test as pallet_storage_providers::Config>::DepositPerData as Get<u128>>::get()
                    .saturating_mul(
                        (capacity
                            - <<Test as pallet_storage_providers::Config>::SpMinCapacity as Get<
                                u64,
                            >>::get())
                        .into(),
                    ),
            );

    // Check the balance of the account to make sure it has more than the deposit amount needed
    assert!(BalancesPalletFor::<Test>::free_balance(&account) >= deposit_for_capacity);

    // Sign up the account as a Backup Storage Provider
    assert_ok!(Providers::force_bsp_sign_up(
        RuntimeOrigin::root(),
        account,
        provider_id,
        capacity,
        multiaddresses.clone(),
        account,
        None
    ));
}

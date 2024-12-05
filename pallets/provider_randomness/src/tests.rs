use crate::{
    mock::*, types::*, BoundedQueue, DeadlineBlockToProviders, Event, FirstSubmittersProviders,
    PendingCommitments, QueueParameters, RandomSeedMixer, RandomnessSeedsQueue,
    ReceivedCommitments, SeedCommitmentToDeadline,
};
use frame_support::{
    assert_ok,
    pallet_prelude::Weight,
    traits::{OnFinalize, OnIdle, OnInitialize, OnPoll},
    weights::WeightMeter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_storage_providers::types::{MaxMultiAddressAmount, MultiAddress};
use shp_traits::ReadChallengeableProvidersInterface;
use sp_core::{blake2_256, Get};
use sp_runtime::{
    testing::H256,
    traits::{BlakeTwo256, Convert, Hash},
    BoundedVec,
};

mod queue {
    use super::*;

    #[test]
    fn circular_queue_of_randomness_gets_initialised() {
        ExtBuilder::build().execute_with(|| {
            // Get the queue from storage
            let queue = RandomnessSeedsQueue::<Test>::get();

            // Get the `MaxSeedTolerance` from the configuration
            let max_seed_tolerance: usize =
                <<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get() as usize;

            // Queue should be the size of `MaxSeedTolerance`
            assert_eq!(queue.len(), max_seed_tolerance);

            // The queue size should be equal to `MaxSeedTolerance`
            assert_eq!(queue.capacity(), max_seed_tolerance);

            // The head of the queue should be the first element and the tail the last
            // since we have not shifted it yet
            let (head, tail) = QueueParameters::<Test>::get();
            assert_eq!(head, 0);
            assert_eq!(tail, max_seed_tolerance as u32 - 1);
        });
    }

    #[test]
    fn circular_queue_correctly_advances_with_block() {
        ExtBuilder::build().execute_with(|| {
            // Advance a block, executing the `on_poll` hook of this pallet
            // which should shift the queue
            run_to_block(System::block_number() + 1);

            // The head of the queue should be the second element and the tail the first,
            // since we have shifted it once
            let (head, tail) = QueueParameters::<Test>::get();
            assert_eq!(head, 1);
            assert_eq!(tail, 0);

            // Advance a few more blocks
            run_to_block(System::block_number() + 4);

            // The head of the queue should be the sixth element and the tail the fifth,
            // since we have shifted it four more times
            let (head, tail) = QueueParameters::<Test>::get();
            assert_eq!(head, 5);
            assert_eq!(tail, 4);

            // Insert an element in the head and another in the tail

            let head_value: SeedFor<Test> = BlakeTwo256::hash(b"head_value");
            let tail_value: SeedFor<Test> = BlakeTwo256::hash(b"tail_value");
            RandomnessSeedsQueue::<Test>::mutate(|queue| {
                queue[head as usize] = head_value;
                queue[tail as usize] = tail_value;
            });

            // Advance a block. The element should be copied to the new tail, overwriting the previous head
            run_to_block(System::block_number() + 1);
            let (new_head, new_tail) = QueueParameters::<Test>::get();
            assert_eq!(new_head, 6);
            assert_eq!(new_tail, 5);
            let queue = RandomnessSeedsQueue::<Test>::get();
            // The new head should have the default value
            assert_eq!(queue[new_head as usize], Default::default());
            // The old head (which is equal to the new tail) should now have the previous tail value
            assert_eq!(head, new_tail);
            assert_eq!(queue[head as usize], tail_value);
        });
    }
}

mod provider_initialisation {
    use super::*;

    #[test]
    fn provider_cycle_is_initialised_correctly() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider
            register_account_as_provider(alice, alice_provider_id, capacity);

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

    #[test]
    fn provider_cycle_initialisation_uses_minimum_period_for_provider_with_high_stake() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 100_000_000_000_000;

            // Register Alice as a Provider
            register_account_as_provider(alice, alice_provider_id, capacity);

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

            // Check that, since the Provider has a high stake, its period is the minimum possible
            let min_seed_period: BlockNumberFor<Test> =
                <Test as crate::Config>::MinSeedPeriod::get();
            let tolerance_period: BlockNumberFor<Test> =
                <<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get().into();
            assert_eq!(
                maybe_first_deadline.unwrap() - System::block_number() - tolerance_period,
                min_seed_period
            );
        });
    }
}

mod add_randomness {
    use super::*;

    #[test]
    fn provider_can_add_randomness_for_the_first_time() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few blocks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline = FirstSubmittersProviders::<Test>::get(alice_provider_id).unwrap();
            run_to_block(first_deadline - 1);

            // Alice submits a commitment
            let seed: SeedFor<Test> = BlakeTwo256::hash(b"seed");
            let commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                commitment
            ));

            // Check that the commitment has been correctly added to the pending commitments to reveal
            let pending_commitments: Option<ProviderIdFor<Test>> =
                PendingCommitments::<Test>::get(commitment);
            assert!(pending_commitments.is_some());
            assert_eq!(pending_commitments.unwrap(), alice_provider_id);

            // Check that the Provider is now in the ReceivedCommitments storage for this deadline,
            // since it has successfully submitted its commitment
            let received_commitments: Vec<ProviderIdFor<Test>> =
                ReceivedCommitments::<Test>::get(first_deadline);
            assert!(received_commitments.contains(&alice_provider_id));

            // Get the next deadline for this Provider:
            // Calculate the Provider's seed period based on their current stake
            let min_seed_period: BlockNumberFor<Test> =
                <Test as crate::Config>::MinSeedPeriod::get();
            let stake =
                <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                    alice_provider_id,
                )
                .unwrap();
            let seed_period =
                match <<Test as crate::Config>::StakeToSeedPeriod as Get<BalanceOf<Test>>>::get()
                    .checked_div(stake)
                {
                    Some(period) => {
                        let seed_period = StakeToBlockNumberFor::<Test>::convert(period);
                        min_seed_period.max(seed_period)
                    }
                    None => min_seed_period,
                };
            // Calculate the deadline block for the commitment Alice just submitted
            let new_deadline = first_deadline.saturating_add(seed_period);

            // The seed commitment should have the same deadline as the one we calculated
            assert_eq!(
                SeedCommitmentToDeadline::<Test>::get(commitment).unwrap(),
                new_deadline
            );

            // And the Provider should have as deadline this new deadline
            assert!(
                DeadlineBlockToProviders::<Test>::get(new_deadline).contains(&alice_provider_id)
            );

            // Finally, the `ProviderInitialisedRandomness` event should have been emitted
            System::assert_last_event(
                Event::ProviderInitialisedRandomness {
                    first_seed_commitment: commitment,
                    next_deadline_block: new_deadline,
                }
                .into(),
            );
        });
    }

    #[test]
    fn revealed_randomness_gets_mixed_with_existent_randomness() {
        ExtBuilder::build().execute_with(|| {
        let alice: AccountId = 0;
        let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
		let capacity = 1000;

        // Register Alice as a Provider and initialise its randomness cycle
        register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

        // Advance a few blocks to simulate a realistic scenario, but not enough to
        // reach the first deadline since that would get Alice slashed
        let first_deadline = FirstSubmittersProviders::<Test>::get(alice_provider_id).unwrap();
        run_to_block(first_deadline - 1);

        // Alice submits a commitment
        let first_seed: SeedFor<Test> = BlakeTwo256::hash(b"seed");
        let first_commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(first_seed.as_bytes());
        assert_ok!(CrRandomness::add_randomness(
            RuntimeOrigin::signed(alice),
            alice_provider_id.clone(),
            None,
            first_commitment
        ));

        // Check that the commitment has been correctly added to the pending commitments to reveal
        let pending_commitments: Option<ProviderIdFor<Test>> =
            PendingCommitments::<Test>::get(first_commitment);
        assert!(pending_commitments.is_some());
        assert_eq!(pending_commitments.unwrap(), alice_provider_id);

        // Get the deadline for this new commitment
        let second_deadline = SeedCommitmentToDeadline::<Test>::get(first_commitment).unwrap();

        // Advance blocks to enter the tolerance period but not enough to get to the deadline
        run_to_block(second_deadline - 1);

        // Alice reveals their commitment and submits a new seed commitment
        let new_seed = BlakeTwo256::hash(b"new_seed");
        let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
        let first_commitment_with_seed: CommitmentWithSeed<Test> = CommitmentWithSeed {
            commitment: first_commitment.clone(),
            seed: first_seed.clone(),
        };
        assert_ok!(CrRandomness::add_randomness(
            RuntimeOrigin::signed(alice),
            alice_provider_id.clone(),
            Some(first_commitment_with_seed),
            new_commitment
        ));

        // Check that the new commitment has been correctly added to the pending commitments to reveal
        let pending_commitments: Option<ProviderIdFor<Test>> =
            PendingCommitments::<Test>::get(new_commitment);
        assert!(pending_commitments.is_some());
        assert_eq!(pending_commitments.unwrap(), alice_provider_id);

        // Check that the Provider is now in the ReceivedCommitments storage for this deadline,
        // since it has successfully submitted its commitment
        let received_commitments: Vec<ProviderIdFor<Test>> =
            ReceivedCommitments::<Test>::get(second_deadline);
        assert!(received_commitments.contains(&alice_provider_id));

        // Get the next deadline for this Provider:
        // Calculate the Provider's seed period based on their current stake
        let min_seed_period: BlockNumberFor<Test> = <Test as crate::Config>::MinSeedPeriod::get();
        let stake = <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
            alice_provider_id,
        )
        .unwrap();
        let seed_period =
            match <<Test as crate::Config>::StakeToSeedPeriod as Get<BalanceOf<Test>>>::get()
                .checked_div(stake)
            {
                Some(period) => {
                    let seed_period = StakeToBlockNumberFor::<Test>::convert(period);
                    min_seed_period.max(seed_period)
                }
                None => min_seed_period,
            };
        // Calculate the deadline block for the commitment Alice just submitted
        let third_deadline = second_deadline.saturating_add(seed_period);

        // The new seed commitment should have the same deadline as the one we calculated
        assert_eq!(
            SeedCommitmentToDeadline::<Test>::get(new_commitment).unwrap(),
            third_deadline
        );

        // And the Provider should have as deadline this new deadline
        assert!(DeadlineBlockToProviders::<Test>::get(third_deadline).contains(&alice_provider_id));

        // The previous commitment should have been removed from the pending commitments storage
        assert!(PendingCommitments::<Test>::get(first_commitment).is_none());

        // Since we got to the last block before the deadline, the randomness queue should have been mixed
        // with all elements except the head
        let head = BoundedQueue::<Test>::head();
        assert_eq!(head.0, Default::default());
        for i in 1..<<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get() {
            let expected_randomness =
                <<Test as crate::Config>::RandomSeedMixer as RandomSeedMixer<SeedFor<Test>>>::mix_randomness_seed(
                    &Default::default(),
                    &first_seed,
                    None::<SeedFor<Test>>,
                );
            let actual_randomness = BoundedQueue::<Test>::element_at_index(i).unwrap().0;
			assert_eq!(expected_randomness, actual_randomness);
        }

		// We make sure that the `RandomnessCommitted` event has been emitted
		System::assert_last_event(
			Event::RandomnessCommitted {
				previous_randomness_revealed: first_seed,
				valid_from_block: second_deadline,
				new_seed_commitment: new_commitment,
				next_deadline_block: third_deadline,
			}.into(),
		);
    });
    }

    #[test]
    fn revealed_randomness_gets_mixed_with_existent_randomness_at_any_index() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            // A low enough capacity is needed to make it so that the seed period is greater than the seed tolerance
            // but we need a capacity at least equal to the minimum capacity. The easiest solution is to just
            // set it to the minimum capacity
            let capacity = <Test as pallet_storage_providers::Config>::SpMinCapacity::get();

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few blocks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline = FirstSubmittersProviders::<Test>::get(alice_provider_id).unwrap();
            run_to_block(first_deadline - 1);

            // Alice submits its first commitment
            let first_seed: SeedFor<Test> = BlakeTwo256::hash(b"seed");
            let first_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(first_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                first_commitment
            ));

            // Check that the commitment has been correctly added to the pending commitments to reveal
            let pending_commitments: Option<ProviderIdFor<Test>> =
                PendingCommitments::<Test>::get(first_commitment);
            assert!(pending_commitments.is_some());
            assert_eq!(pending_commitments.unwrap(), alice_provider_id);

            // Get the deadline for this new commitment
            let mut next_deadline =
                SeedCommitmentToDeadline::<Test>::get(first_commitment).unwrap();

            // To test this easily, we need a Provider that has a seed period greater than the seed tolerance,
            // so when advancing until the deadline all randomness elements of the queue are overwritten with
            // the latest randomness. We check for that:
            assert!(
                next_deadline - first_deadline
                    > <<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get().into()
            );

            // Initialise variables looping through all possible indexes
            let mut previous_seed = first_seed;
            let mut previous_commitment = first_commitment;
            let mut previous_randomness: H256 = Default::default();

            // Try every scenario for submission and reveal of commitments
            for blocks_until_deadline in
                0..<<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get()
            {
                // Advance blocks to enter the tolerance period but not enough to get to the deadline
                run_to_block(
                    next_deadline
                        - <u32 as Into<BlockNumberFor<Test>>>::into(blocks_until_deadline),
                );

                // Alice reveals their commitment and submits a new seed commitment
                let next_seed =
                    BlakeTwo256::hash(format! { "new_seed_{}", blocks_until_deadline }.as_bytes());
                let next_commitment = BlakeTwo256::hash(next_seed.as_bytes());
                let previous_commitment_with_seed: CommitmentWithSeed<Test> = CommitmentWithSeed {
                    commitment: previous_commitment.clone(),
                    seed: previous_seed.clone(),
                };
                assert_ok!(CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id.clone(),
                    Some(previous_commitment_with_seed),
                    next_commitment
                ));

                // Check that the new commitment has been correctly added to the pending commitments to reveal
                let pending_commitments: Option<ProviderIdFor<Test>> =
                    PendingCommitments::<Test>::get(next_commitment);
                assert!(pending_commitments.is_some());
                assert_eq!(pending_commitments.unwrap(), alice_provider_id);

                // Check that the Provider is now in the ReceivedCommitments storage for this deadline,
                // since it has successfully submitted its commitment
                let received_commitments: Vec<ProviderIdFor<Test>> =
                    ReceivedCommitments::<Test>::get(next_deadline);
                assert!(received_commitments.contains(&alice_provider_id));

                // Get the next deadline for this Provider:
                // Calculate the Provider's seed period based on their current stake
                let min_seed_period: BlockNumberFor<Test> =
                    <Test as crate::Config>::MinSeedPeriod::get();
                let stake =
                    <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                        alice_provider_id,
                    )
                    .unwrap();
                let seed_period = match <<Test as crate::Config>::StakeToSeedPeriod as Get<
                    BalanceOf<Test>,
                >>::get()
                .checked_div(stake)
                {
                    Some(period) => {
                        let seed_period = StakeToBlockNumberFor::<Test>::convert(period);
                        min_seed_period.max(seed_period)
                    }
                    None => min_seed_period,
                };
                // Update the previous deadline
                let previous_deadline = next_deadline;
                // Calculate the deadline block for the commitment Alice just submitted
                next_deadline = next_deadline.saturating_add(seed_period);

                // The new seed commitment should have the same deadline as the one we calculated
                assert_eq!(
                    SeedCommitmentToDeadline::<Test>::get(next_commitment).unwrap(),
                    next_deadline
                );

                // And the Provider should have as deadline this new deadline
                assert!(DeadlineBlockToProviders::<Test>::get(next_deadline)
                    .contains(&alice_provider_id));

                // The previous commitment should have been removed from the pending commitments storage
                assert!(PendingCommitments::<Test>::get(previous_commitment).is_none());

                // The randomness queue should have been mixed with all elements since the index `block_until_deadline`
                for i in blocks_until_deadline
                    ..<<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get()
                {
                    let expected_randomness =
                        <<Test as crate::Config>::RandomSeedMixer as RandomSeedMixer<
                            SeedFor<Test>,
                        >>::mix_randomness_seed(
                            &previous_randomness,
                            &previous_seed,
                            None::<SeedFor<Test>>,
                        );
                    let actual_randomness = BoundedQueue::<Test>::element_at_index(i).unwrap().0;
                    assert_eq!(expected_randomness, actual_randomness);
                }
                // But not with the elements before the index `block_until_deadline`
                for i in 0..blocks_until_deadline {
                    let expected_randomness: H256 = previous_randomness;
                    let actual_randomness = BoundedQueue::<Test>::element_at_index(i).unwrap().0;
                    assert_eq!(expected_randomness, actual_randomness);
                }

                // Update the randomness for the next iteration
                previous_randomness = <<Test as crate::Config>::RandomSeedMixer as RandomSeedMixer<
                SeedFor<Test>,
            >>::mix_randomness_seed(
                &previous_randomness, &previous_seed, None::<SeedFor<Test>>
            );

                // Finally, we make sure that the `RandomnessCommitted` event has been emitted
                System::assert_last_event(
                    Event::RandomnessCommitted {
                        previous_randomness_revealed: previous_seed,
                        valid_from_block: previous_deadline,
                        new_seed_commitment: next_commitment,
                        next_deadline_block: next_deadline,
                    }
                    .into(),
                );

                // Update the previous seed and commitment
                previous_seed = next_seed;
                previous_commitment = next_commitment;
            }
        });
    }
}

/// Helper function that advances the blockchain until block n, executing the hooks for each block
fn run_to_block(n: u64) {
    assert!(n > System::block_number(), "Cannot go back in time");

    while System::block_number() < n {
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::reset_events();
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        CrRandomness::on_poll(System::block_number(), &mut WeightMeter::new());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    }
}

/// Function that registers a Provider for the given account with the given Provider ID
fn register_account_as_provider(
    account: AccountId,
    provider_id: ProviderIdFor<Test>,
    capacity: StorageDataUnit,
) {
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

fn register_as_provider_and_initialise_account(
    account: AccountId,
    provider_id: ProviderIdFor<Test>,
    capacity: StorageDataUnit,
) {
    register_account_as_provider(account, provider_id, capacity);
    assert_ok!(CrRandomness::force_initialise_provider_cycle(
        RuntimeOrigin::root(),
        provider_id,
    ));
}

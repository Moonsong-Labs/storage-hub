use crate::{
    mock::*, types::*, ActiveProviders, BoundedQueue, DeadlineTickToProviders, Error, Event,
    PendingCommitments, ProvidersWithoutCommitment, QueueParameters, RandomSeedMixer,
    RandomnessSeedsQueue, ReceivedCommitments, SeedCommitmentToDeadline,
};
use frame_support::{
    assert_noop, assert_ok,
    pallet_prelude::Weight,
    traits::{fungible::Inspect, OnFinalize, OnIdle, OnInitialize, OnPoll},
    weights::WeightMeter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_storage_providers::types::{MaxMultiAddressAmount, MultiAddress};
use shp_traits::ReadChallengeableProvidersInterface;
use sp_core::Get;
use sp_runtime::{
    testing::H256,
    traits::{BlakeTwo256, Convert, Hash},
    BoundedVec,
};

/// The Balances pallet of the runtime.
pub type BalancesPalletFor<T> = <T as pallet_proofs_dealer::Config>::NativeBalance;

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <BalancesPalletFor<T> as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

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
    fn circular_queue_correctly_advances_with_tick() {
        ExtBuilder::build().execute_with(|| {
            // Advance a tick executing the `on_poll` hook of `pallet_proofs_dealer` and of this pallet
            // which should shift the queue
            run_to_block(pallet_proofs_dealer::ChallengesTicker::<Test>::get() + 1);

            // The head of the queue should be the second element and the tail the first,
            // since we have shifted it once
            let (head, tail) = QueueParameters::<Test>::get();
            assert_eq!(head, 1);
            assert_eq!(tail, 0);

            // Advance a few more ticks
            run_to_block(pallet_proofs_dealer::ChallengesTicker::<Test>::get() + 4);

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

            // Advance a tick. The element should be copied to the new tail, overwriting the previous head
            run_to_block(pallet_proofs_dealer::ChallengesTicker::<Test>::get() + 1);
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
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id);
            assert!(maybe_first_deadline.is_some());
            assert!(
                DeadlineTickToProviders::<Test>::get(maybe_first_deadline.unwrap())
                    .contains(&alice_provider_id)
            );

            // Check that Alice was added to the Active Providers storage with None as their next commitment
            assert!(ActiveProviders::<Test>::contains_key(&alice_provider_id));
            assert_eq!(
                ActiveProviders::<Test>::get(&alice_provider_id),
                Some(None::<SeedCommitmentFor<Test>>)
            );

            // Check that the `ProviderCycleInitialised` event has been emitted
            System::assert_last_event(
                Event::ProviderCycleInitialised {
                    provider_id: alice_provider_id,
                    first_seed_commitment_deadline_tick: maybe_first_deadline.unwrap(),
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
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id);
            assert!(maybe_first_deadline.is_some());
            assert!(
                DeadlineTickToProviders::<Test>::get(maybe_first_deadline.unwrap())
                    .contains(&alice_provider_id)
            );

            // Check that, since the Provider has a high stake, its period is the minimum possible
            let min_seed_period: BlockNumberFor<Test> =
                <Test as crate::Config>::MinSeedPeriod::get();
            let tolerance_period: BlockNumberFor<Test> =
                <<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get().into();
            let current_tick = pallet_proofs_dealer::ChallengesTicker::<Test>::get();
            assert_eq!(
                maybe_first_deadline.unwrap() - current_tick - tolerance_period,
                min_seed_period
            );
        });
    }

    #[test]
    fn provider_cycle_initialisation_fails_if_provider_id_not_valid() {
        ExtBuilder::build().execute_with(|| {
            // Alice tries to initialise the cycle of a Provider with an invalid Provider ID
            assert_noop!(
                CrRandomness::force_initialise_provider_cycle(
                    RuntimeOrigin::root(),
                    BlakeTwo256::hash(b"invalid_provider_id")
                ),
                Error::<Test>::ProviderIdNotValid
            );
        });
    }
}

mod provider_cycle_stop {
    use super::*;

    #[test]
    fn provider_cycle_is_stopped_correctly_for_old_provider() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            run_to_block(first_deadline - 1);

            // Alice submits a commitment
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

            // Check that Alice has been removed from the ProvidersWithoutCommitment storage
            assert!(ProvidersWithoutCommitment::<Test>::get(alice_provider_id).is_none());

            // Deactivate Alice as a Provider, stopping its cycle
            assert_ok!(CrRandomness::force_stop_provider_cycle(
                RuntimeOrigin::root(),
                alice_provider_id
            ));

            // Alice should have been removed from the Active Providers storage
            assert!(!ActiveProviders::<Test>::contains_key(&alice_provider_id));

            // Its seed commitment should have been removed from the seed commitment to deadline storage
            assert!(SeedCommitmentToDeadline::<Test>::get(first_commitment).is_none());

            // The `ProviderCycleStopped` event should have been emitted
            System::assert_last_event(
                Event::ProviderCycleStopped {
                    provider_id: alice_provider_id,
                }
                .into(),
            );
        });
    }

    #[test]
    fn provider_cycle_is_stopped_correctly_for_new_provider() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Alice should be in the Providers Without Commitment storage since it has not submitted any commitment yet
            assert!(ProvidersWithoutCommitment::<Test>::contains_key(
                &alice_provider_id
            ));

            // Deactivate Alice as a Provider, stopping its cycle
            assert_ok!(CrRandomness::force_stop_provider_cycle(
                RuntimeOrigin::root(),
                alice_provider_id
            ));

            // Alice should have been removed from the Active Providers storage
            assert!(!ActiveProviders::<Test>::contains_key(&alice_provider_id));

            // And it should have been removed from the ProvidersWithoutCommitment storage
            assert!(ProvidersWithoutCommitment::<Test>::get(alice_provider_id).is_none());

            // The `ProviderCycleStopped` event should have been emitted
            System::assert_last_event(
                Event::ProviderCycleStopped {
                    provider_id: alice_provider_id,
                }
                .into(),
            );
        });
    }

    #[test]
    fn stop_provider_cycle_fails_if_provider_id_not_valid() {
        ExtBuilder::build().execute_with(|| {
            // Alice tries to stop the cycle of a Provider with an invalid Provider ID
            assert_noop!(
                CrRandomness::force_stop_provider_cycle(
                    RuntimeOrigin::root(),
                    BlakeTwo256::hash(b"invalid_provider_id")
                ),
                Error::<Test>::ProviderIdNotValid
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

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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
            // Calculate the deadline tick for the commitment Alice just submitted
            let new_deadline = first_deadline.saturating_add(seed_period);

            // The seed commitment should have the same deadline as the one we calculated
            assert_eq!(
                SeedCommitmentToDeadline::<Test>::get(commitment).unwrap(),
                new_deadline
            );

            // And the Provider should have as deadline this new deadline
            assert!(
                DeadlineTickToProviders::<Test>::get(new_deadline).contains(&alice_provider_id)
            );

            // Finally, the `ProviderInitialisedRandomness` event should have been emitted
            System::assert_last_event(
                Event::ProviderInitialisedRandomness {
                    first_seed_commitment: commitment,
                    next_deadline_tick: new_deadline,
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

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            run_to_block(first_deadline - 1);

            // Alice submits a commitment
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
            let second_deadline = SeedCommitmentToDeadline::<Test>::get(first_commitment).unwrap();

            // Advance ticks to enter the tolerance period but not enough to get to the deadline
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
            // Calculate the deadline tick for the commitment Alice just submitted
            let third_deadline = second_deadline.saturating_add(seed_period);

            // The new seed commitment should have the same deadline as the one we calculated
            assert_eq!(
                SeedCommitmentToDeadline::<Test>::get(new_commitment).unwrap(),
                third_deadline
            );

            // And the Provider should have as deadline this new deadline
            assert!(
                DeadlineTickToProviders::<Test>::get(third_deadline).contains(&alice_provider_id)
            );

            // The previous commitment should have been removed from the pending commitments storage
            assert!(PendingCommitments::<Test>::get(first_commitment).is_none());

            // Since we got to the last tick before the deadline, the randomness queue should have been mixed
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
                    valid_from_tick: second_deadline,
                    new_seed_commitment: new_commitment,
                    next_deadline_tick: third_deadline,
                }
                .into(),
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

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Try every scenario for submission and reveal of commitments. A commitment cannot be revealed in
            // the deadline tick, since that would mean mixing that randomness with the one at the head of the queue
            for ticks_until_deadline in
                1..<<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get()
            {
                // Advance ticks to enter the tolerance period but not enough to get to the deadline
                run_to_block(
                    next_deadline - <u32 as Into<BlockNumberFor<Test>>>::into(ticks_until_deadline),
                );

                // Alice reveals their commitment and submits a new seed commitment
                let next_seed =
                    BlakeTwo256::hash(format! { "new_seed_{}", ticks_until_deadline }.as_bytes());
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
                // Calculate the deadline tick for the commitment Alice just submitted
                next_deadline = next_deadline.saturating_add(seed_period);

                // The new seed commitment should have the same deadline as the one we calculated
                assert_eq!(
                    SeedCommitmentToDeadline::<Test>::get(next_commitment).unwrap(),
                    next_deadline
                );

                // And the Provider should have as deadline this new deadline
                assert!(DeadlineTickToProviders::<Test>::get(next_deadline)
                    .contains(&alice_provider_id));

                // The previous commitment should have been removed from the pending commitments storage
                assert!(PendingCommitments::<Test>::get(previous_commitment).is_none());

                // The randomness queue should have been mixed with all elements since the index `ticks_until_deadline`
                for i in ticks_until_deadline
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
                // But not with the elements before the index `ticks_until_deadline`
                for i in 0..ticks_until_deadline {
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
                        valid_from_tick: previous_deadline,
                        new_seed_commitment: next_commitment,
                        next_deadline_tick: next_deadline,
                    }
                    .into(),
                );

                // Update the previous seed and commitment
                previous_seed = next_seed;
                previous_commitment = next_commitment;
            }
        });
    }

    #[test]
    fn add_randomness_fails_if_provider_id_not_valid() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;

            // Generate a commitment to use for the extrinsic call
            let seed: SeedFor<Test> = BlakeTwo256::hash(b"seed");
            let commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(seed.as_bytes());

            // Alice tries to submit a commitment with an invalid Provider ID
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    BlakeTwo256::hash(b"invalid_provider_id"),
                    None,
                    commitment
                ),
                Error::<Test>::ProviderIdNotValid
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_account_not_owner_of_provider_id() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let bob: AccountId = 1;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Generate a commitment to use for the extrinsic call
            let seed: SeedFor<Test> = BlakeTwo256::hash(b"seed");
            let commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(seed.as_bytes());

            // Bob tries to submit a commitment with Alice's Provider ID
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(bob),
                    alice_provider_id,
                    None,
                    commitment
                ),
                Error::<Test>::CallerNotOwner
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_trying_to_add_already_pending_commitment() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Advance enough ticks to allow Alice to submit a new commitment
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Alice tries to submit the same commitment again
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id,
                    None,
                    commitment
                ),
                Error::<Test>::NewCommitmentAlreadyPending
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_seed_reveal_is_not_sent_for_old_provider() {
        ExtBuilder::build().execute_with(|| {
            // This test checks that a Provider cannot submit a new commitment without revealing the previous
            // seed if it's not a first time submitter
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Advance enough ticks to allow Alice to submit a new commitment
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Alice tries to submit a new commitment without revealing the seed for the previous one
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id,
                    None,
                    new_commitment
                ),
                Error::<Test>::MissingSeedReveal
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_commitment_to_reveal_does_not_exist() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Advance enough ticks to allow Alice to submit a new commitment
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Alice tries to reveal the seed for a commitment different that the one she submitted
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: new_commitment,
                seed: new_seed,
            };
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id,
                    Some(commitment_to_reveal),
                    new_commitment
                ),
                Error::<Test>::NoEndTickForSeedCommitment
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_reveal_is_not_in_the_tolerance_window() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Alice tries to reveal the seed of the commitment before the tolerance window starts
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: commitment.clone(),
                seed: seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id,
                    Some(commitment_to_reveal),
                    new_commitment
                ),
                Error::<Test>::EarlySubmissionOfSeed
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_seed_reveal_is_past_its_deadline() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Advance enough ticks so Alice is late to reveal its previous commitment
            // We don't execute the `on_idle` hook since that would slash Alice and reset its deadline
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(commitment).unwrap();
            run_to_block_without_on_idle(new_deadline + 1);

            // Alice tries to reveal the seed of the commitment after its deadline
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: commitment.clone(),
                seed: seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id,
                    Some(commitment_to_reveal),
                    new_commitment
                ),
                Error::<Test>::LateSubmissionOfSeed
            );
        });
    }

    #[test]
    fn add_randomness_fails_if_seed_revealed_is_not_the_commitment_seed() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let capacity = 1000;

            // Register Alice as a Provider and initialise its randomness cycle
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get Alice slashed
            let first_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
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

            // Advance enough ticks to allow Alice to submit a new commitment
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Alice tries to reveal the seed of the commitment after its deadline
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: commitment.clone(),
                seed: BlakeTwo256::hash(b"another_seed"),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_noop!(
                CrRandomness::add_randomness(
                    RuntimeOrigin::signed(alice),
                    alice_provider_id,
                    Some(commitment_to_reveal),
                    new_commitment
                ),
                Error::<Test>::NotAValidSeed
            );
        });
    }
}

mod slashing {
    use crate::{ProvidersToMarkAsSlashable, TickToCheckForSlashableProviders};

    use super::*;

    #[test]
    fn providers_that_miss_deadlines_get_stored_to_be_marked_as_slashable() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let bob: AccountId = 1;
            let bob_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"bob");
            let charlie: AccountId = 2;
            let charlie_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"charlie");
            // Use the same capacity for all so they have the same seed period
            let capacity = 1000;

            // Register Alice, Bob and Charlie as Providers and initialise their randomness cycles
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);
            register_as_provider_and_initialise_account(bob, bob_provider_id, capacity);
            register_as_provider_and_initialise_account(charlie, charlie_provider_id, capacity);

            // Check that they all have the same initial deadline
            let alice_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_deadline = ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            let charlie_deadline =
                ProvidersWithoutCommitment::<Test>::get(charlie_provider_id).unwrap();
            assert_eq!(alice_deadline, bob_deadline);
            assert_eq!(bob_deadline, charlie_deadline);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get the Providers slashed
            let first_deadline = alice_deadline;
            run_to_block(first_deadline - 1);

            // All Providers submit their initial commitment
            let alice_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_seed");
            let alice_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                alice_commitment
            ));

            let bob_seed: SeedFor<Test> = BlakeTwo256::hash(b"bob_seed");
            let bob_commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(bob_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(bob),
                bob_provider_id.clone(),
                None,
                bob_commitment
            ));

            let charlie_seed: SeedFor<Test> = BlakeTwo256::hash(b"charlie_seed");
            let charlie_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(charlie_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                None,
                charlie_commitment
            ));

            // Advance enough ticks to allow Charlie to submit a new commitment
            // Alice and Bob will miss their deadline and should be stored in order to be marked
            // slashable in the next execution of the `on_idle` hook
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(charlie_commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Charlie submits a new commitment, revealing his previous one
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: charlie_commitment.clone(),
                seed: charlie_seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                Some(commitment_to_reveal),
                new_commitment
            ));

            // Advance enough ticks to make Alice and Bob miss their deadline, executing the `on_poll` hook
            // but not the `on_idle` hook (so we simulate that the Providers are not marked as slashable yet)
            run_to_block_without_on_idle(new_deadline + 1);

            // Now, Alice and Bob should be in the list of Providers to be marked as slashable, while
            // Charlie should not
            let providers_to_mark_as_slashable =
                ProvidersToMarkAsSlashable::<Test>::get(new_deadline).unwrap();
            assert!(providers_to_mark_as_slashable.contains(&alice_provider_id));
            assert!(providers_to_mark_as_slashable.contains(&bob_provider_id));
            assert!(!providers_to_mark_as_slashable.contains(&charlie_provider_id));
        });
    }

    #[test]
    fn providers_stored_to_be_marked_as_slashable_get_marked_as_slashable_on_idle() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let bob: AccountId = 1;
            let bob_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"bob");
            let charlie: AccountId = 2;
            let charlie_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"charlie");
            // Use the same capacity for all so they have the same seed period
            let capacity = 1000;

            // Register Alice, Bob and Charlie as Providers and initialise their randomness cycles
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);
            register_as_provider_and_initialise_account(bob, bob_provider_id, capacity);
            register_as_provider_and_initialise_account(charlie, charlie_provider_id, capacity);

            // Check that they all have the same initial deadline
            let alice_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_deadline = ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            let charlie_deadline =
                ProvidersWithoutCommitment::<Test>::get(charlie_provider_id).unwrap();
            assert_eq!(alice_deadline, bob_deadline);
            assert_eq!(bob_deadline, charlie_deadline);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get the Providers slashed
            let first_deadline = alice_deadline;
            run_to_block(first_deadline - 1);

            // All Providers submit their initial commitment
            let alice_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_seed");
            let alice_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                alice_commitment
            ));

            let bob_seed: SeedFor<Test> = BlakeTwo256::hash(b"bob_seed");
            let bob_commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(bob_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(bob),
                bob_provider_id.clone(),
                None,
                bob_commitment
            ));

            let charlie_seed: SeedFor<Test> = BlakeTwo256::hash(b"charlie_seed");
            let charlie_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(charlie_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                None,
                charlie_commitment
            ));

            // Advance enough ticks to allow Charlie to submit a new commitment
            // Alice and Bob will miss their deadline and should be stored in order to be marked
            // slashable in the next execution of the `on_idle` hook
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(charlie_commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Charlie submits a new commitment, revealing his previous one
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: charlie_commitment.clone(),
                seed: charlie_seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                Some(commitment_to_reveal),
                new_commitment
            ));

            // Advance enough ticks to make Alice and Bob miss their deadline, executing the `on_poll` hook
            // and the `on_idle` hook, which should mean that they are marked as slashable
            run_to_block(new_deadline + 1);

            // Now, Alice and Bob should not be in the list of Providers to be marked as slashable since
            // they have already been marked as such
            let providers_to_mark_as_slashable =
                ProvidersToMarkAsSlashable::<Test>::get(new_deadline);
            assert!(providers_to_mark_as_slashable.is_none());

            // They should have been marked as slashable
            let alice_accrued_slashes =
                pallet_proofs_dealer::SlashableProviders::<Test>::get(alice_provider_id).unwrap();
            let bob_accrued_slashes =
                pallet_proofs_dealer::SlashableProviders::<Test>::get(bob_provider_id).unwrap();
            assert_eq!(alice_accrued_slashes, 1);
            assert_eq!(bob_accrued_slashes, 1);
            assert!(
                pallet_proofs_dealer::SlashableProviders::<Test>::get(charlie_provider_id)
                    .is_none()
            );

            // They also should have their next deadline to be one tolerance period after the previous one
            let tolerance_period: BlockNumberFor<Test> =
                <<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get().into();
            let next_deadline = new_deadline + tolerance_period;
            let providers_of_next_deadline = DeadlineTickToProviders::<Test>::get(next_deadline);
            assert!(providers_of_next_deadline.contains(&alice_provider_id));
            assert!(providers_of_next_deadline.contains(&bob_provider_id));

            // The events should have been emitted
            System::assert_has_event(
                Event::ProviderMarkedAsSlashable {
                    provider_id: alice_provider_id,
                    next_deadline: next_deadline,
                }
                .into(),
            );
            System::assert_has_event(
                Event::ProviderMarkedAsSlashable {
                    provider_id: bob_provider_id,
                    next_deadline: next_deadline,
                }
                .into(),
            );

            // And they should be in the ProvidersWithoutCommitment storage
            let alice_next_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_next_deadline =
                ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            assert_eq!(alice_next_deadline, next_deadline);
            assert_eq!(bob_next_deadline, next_deadline);
        });
    }

    #[test]
    fn providers_dont_get_slashed_if_on_idle_weight_is_not_enough() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let bob: AccountId = 1;
            let bob_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"bob");
            let charlie: AccountId = 2;
            let charlie_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"charlie");
            // Use the same capacity for all so they have the same seed period
            let capacity = 1000;

            // Register Alice, Bob and Charlie as Providers and initialise their randomness cycles
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);
            register_as_provider_and_initialise_account(bob, bob_provider_id, capacity);
            register_as_provider_and_initialise_account(charlie, charlie_provider_id, capacity);

            // Check that they all have the same initial deadline
            let alice_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_deadline = ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            let charlie_deadline =
                ProvidersWithoutCommitment::<Test>::get(charlie_provider_id).unwrap();
            assert_eq!(alice_deadline, bob_deadline);
            assert_eq!(bob_deadline, charlie_deadline);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get the Providers slashed
            let first_deadline = alice_deadline;
            run_to_block(first_deadline - 1);

            // All Providers submit their initial commitment
            let alice_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_seed");
            let alice_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                alice_commitment
            ));

            let bob_seed: SeedFor<Test> = BlakeTwo256::hash(b"bob_seed");
            let bob_commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(bob_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(bob),
                bob_provider_id.clone(),
                None,
                bob_commitment
            ));

            let charlie_seed: SeedFor<Test> = BlakeTwo256::hash(b"charlie_seed");
            let charlie_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(charlie_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                None,
                charlie_commitment
            ));

            // Advance enough ticks to allow Charlie to submit a new commitment
            // Alice and Bob will miss their deadline and should be stored in order to be marked
            // slashable in the next execution of the `on_idle` hook
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(charlie_commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Charlie submits a new commitment, revealing his previous one
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: charlie_commitment.clone(),
                seed: charlie_seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                Some(commitment_to_reveal),
                new_commitment
            ));

            // Advance enough ticks to make Alice and Bob miss their deadline, executing the `on_poll` hook
            // but not the `on_idle` hook (so we simulate that the Providers are not marked as slashable yet)
            run_to_block_without_on_idle(new_deadline + 1);

            // Now, Alice and Bob should be in the list of Providers to be marked as slashable, while
            // Charlie should not
            let providers_to_mark_as_slashable =
                ProvidersToMarkAsSlashable::<Test>::get(new_deadline).unwrap();
            assert!(providers_to_mark_as_slashable.contains(&alice_provider_id));
            assert!(providers_to_mark_as_slashable.contains(&bob_provider_id));
            assert!(!providers_to_mark_as_slashable.contains(&charlie_provider_id));

            // Set the tick to check for slashable Providers to the missed deadline
            TickToCheckForSlashableProviders::<Test>::set(new_deadline);

            // Call the `on_idle` hook without enough weight to execute the slashing
            AllPalletsWithSystem::on_idle(new_deadline, 0.into());

            // Nothing should have happened
            let providers_to_mark_as_slashable =
                ProvidersToMarkAsSlashable::<Test>::get(new_deadline).unwrap();
            assert!(providers_to_mark_as_slashable.contains(&alice_provider_id));
            assert!(providers_to_mark_as_slashable.contains(&bob_provider_id));
            assert!(!providers_to_mark_as_slashable.contains(&charlie_provider_id));
            assert!(
                pallet_proofs_dealer::SlashableProviders::<Test>::get(alice_provider_id).is_none()
            );
            assert!(
                pallet_proofs_dealer::SlashableProviders::<Test>::get(bob_provider_id).is_none()
            );
            assert_eq!(
                TickToCheckForSlashableProviders::<Test>::get(),
                new_deadline
            );
        });
    }

    #[test]
    fn number_of_slashed_providers_depends_on_remaining_weight_for_on_idle() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let bob: AccountId = 1;
            let bob_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"bob");
            let charlie: AccountId = 2;
            let charlie_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"charlie");
            // Use the same capacity for all so they have the same seed period
            let capacity = 1000;

            // Register Alice, Bob and Charlie as Providers and initialise their randomness cycles
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);
            register_as_provider_and_initialise_account(bob, bob_provider_id, capacity);
            register_as_provider_and_initialise_account(charlie, charlie_provider_id, capacity);

            // Check that they all have the same initial deadline
            let alice_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_deadline = ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            let charlie_deadline =
                ProvidersWithoutCommitment::<Test>::get(charlie_provider_id).unwrap();
            assert_eq!(alice_deadline, bob_deadline);
            assert_eq!(bob_deadline, charlie_deadline);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get the Providers slashed
            let first_deadline = alice_deadline;
            run_to_block(first_deadline - 1);

            // All Providers submit their initial commitment
            let alice_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_seed");
            let alice_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                alice_commitment
            ));

            let bob_seed: SeedFor<Test> = BlakeTwo256::hash(b"bob_seed");
            let bob_commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(bob_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(bob),
                bob_provider_id.clone(),
                None,
                bob_commitment
            ));

            let charlie_seed: SeedFor<Test> = BlakeTwo256::hash(b"charlie_seed");
            let charlie_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(charlie_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                None,
                charlie_commitment
            ));

            // Advance enough ticks to allow Charlie to submit a new commitment
            // Alice and Bob will miss their deadline and should be stored in order to be marked
            // slashable in the next execution of the `on_idle` hook
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(charlie_commitment).unwrap();
            println!("new_deadline: {:?}", new_deadline);
            run_to_block(new_deadline - 1);

            // Charlie submits a new commitment, revealing his previous one
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: charlie_commitment.clone(),
                seed: charlie_seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                Some(commitment_to_reveal),
                new_commitment
            ));

            // Advance enough ticks to make Alice and Bob miss their deadline, executing the `on_poll` hook
            // but not the `on_idle` hook (so we simulate that the Providers are not marked as slashable yet)
            run_to_block_without_on_idle(new_deadline + 1);

            // Now, Alice and Bob should be in the list of Providers to be marked as slashable, while
            // Charlie should not
            let providers_to_mark_as_slashable =
                ProvidersToMarkAsSlashable::<Test>::get(new_deadline).unwrap();
            assert!(providers_to_mark_as_slashable.contains(&alice_provider_id));
            assert!(providers_to_mark_as_slashable.contains(&bob_provider_id));
            assert!(!providers_to_mark_as_slashable.contains(&charlie_provider_id));

            // Set the tick to check for slashable Providers to the missed deadline
            TickToCheckForSlashableProviders::<Test>::set(new_deadline);

            // Call the `on_idle` hook with enough weight to only slash one Provider
            // TODO: Update this test after benchmarking
            let weight_required_for_slashing_one_provider =
                <Test as frame_system::Config>::DbWeight::get().reads_writes(3, 1)
                    + <Test as frame_system::Config>::DbWeight::get().reads_writes(1, 5);
            CrRandomness::on_idle(
                new_deadline,
                weight_required_for_slashing_one_provider.into(),
            );

            // One of Alice or Bob should have been marked as slashable, while the other not.
            // Since it depends on the order they were processed by the `on_poll` hook, we check both
            let providers_to_mark_as_slashable =
                ProvidersToMarkAsSlashable::<Test>::get(new_deadline).unwrap();
            let alice_accrued_slashes =
                pallet_proofs_dealer::SlashableProviders::<Test>::get(alice_provider_id).is_some();
            let bob_accrued_slashes =
                pallet_proofs_dealer::SlashableProviders::<Test>::get(bob_provider_id).is_some();
            assert_eq!(alice_accrued_slashes, !bob_accrued_slashes);
            assert!(
                !pallet_proofs_dealer::SlashableProviders::<Test>::get(charlie_provider_id)
                    .is_some()
            );
            if alice_accrued_slashes {
                assert!(!providers_to_mark_as_slashable.contains(&alice_provider_id));
                assert!(providers_to_mark_as_slashable.contains(&bob_provider_id));
            } else {
                assert!(providers_to_mark_as_slashable.contains(&alice_provider_id));
                assert!(!providers_to_mark_as_slashable.contains(&bob_provider_id));
            }

            // Since not all Providers of this tick were processed, the tick to check for slashable Providers
            // should remain the same
            assert_eq!(
                TickToCheckForSlashableProviders::<Test>::get(),
                new_deadline
            );

            // If we call the `on_idle` hook again with enough weight to process the remaining Provider,
            // the remaining Provider should be marked as slashable and the tick to check for slashable Providers
            // should advance
            CrRandomness::on_idle(new_deadline, weight_required_for_slashing_one_provider);
            assert!(ProvidersToMarkAsSlashable::<Test>::get(new_deadline).is_none());
            assert_eq!(
                TickToCheckForSlashableProviders::<Test>::get(),
                new_deadline + 1
            );
            assert_eq!(
                pallet_proofs_dealer::SlashableProviders::<Test>::get(alice_provider_id).unwrap(),
                1
            );
            assert_eq!(
                pallet_proofs_dealer::SlashableProviders::<Test>::get(bob_provider_id).unwrap(),
                1
            );
        });
    }

    #[test]
    fn slashed_providers_can_submit_new_commitment() {
        ExtBuilder::build().execute_with(|| {
            let alice: AccountId = 0;
            let alice_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"alice");
            let bob: AccountId = 1;
            let bob_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"bob");
            let charlie: AccountId = 2;
            let charlie_provider_id: ProviderIdFor<Test> = BlakeTwo256::hash(b"charlie");
            // Use the same capacity for all so they have the same seed period
            let capacity = 1000;

            // Register Alice, Bob and Charlie as Providers and initialise their randomness cycles
            register_as_provider_and_initialise_account(alice, alice_provider_id, capacity);
            register_as_provider_and_initialise_account(bob, bob_provider_id, capacity);
            register_as_provider_and_initialise_account(charlie, charlie_provider_id, capacity);

            // Check that they all have the same initial deadline
            let alice_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_deadline = ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            let charlie_deadline =
                ProvidersWithoutCommitment::<Test>::get(charlie_provider_id).unwrap();
            assert_eq!(alice_deadline, bob_deadline);
            assert_eq!(bob_deadline, charlie_deadline);

            // Advance a few ticks to simulate a realistic scenario, but not enough to
            // reach the first deadline since that would get the Providers slashed
            let first_deadline = alice_deadline;
            run_to_block(first_deadline - 1);

            // All Providers submit their initial commitment
            let alice_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_seed");
            let alice_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                alice_commitment
            ));

            let bob_seed: SeedFor<Test> = BlakeTwo256::hash(b"bob_seed");
            let bob_commitment: SeedCommitmentFor<Test> = BlakeTwo256::hash(bob_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(bob),
                bob_provider_id.clone(),
                None,
                bob_commitment
            ));

            let charlie_seed: SeedFor<Test> = BlakeTwo256::hash(b"charlie_seed");
            let charlie_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(charlie_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                None,
                charlie_commitment
            ));

            // Advance enough ticks to allow Charlie to submit a new commitment
            // Alice and Bob will miss their deadline and should be stored in order to be marked
            // slashable in the next execution of the `on_idle` hook
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(charlie_commitment).unwrap();
            run_to_block(new_deadline - 1);

            // Charlie submits a new commitment, revealing his previous one
            let commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: charlie_commitment.clone(),
                seed: charlie_seed.clone(),
            };
            let new_seed = BlakeTwo256::hash(b"new_seed");
            let new_commitment = BlakeTwo256::hash(new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(charlie),
                charlie_provider_id.clone(),
                Some(commitment_to_reveal),
                new_commitment
            ));

            // Advance enough ticks to make Alice and Bob miss their deadline, executing the `on_poll` hook
            // and the `on_idle` hook, which should mean that they are marked as slashable
            run_to_block(new_deadline + 1);

            // They should have been marked as slashable
            let alice_accrued_slashes =
                pallet_proofs_dealer::SlashableProviders::<Test>::get(alice_provider_id).unwrap();
            let bob_accrued_slashes =
                pallet_proofs_dealer::SlashableProviders::<Test>::get(bob_provider_id).unwrap();
            assert_eq!(alice_accrued_slashes, 1);
            assert_eq!(bob_accrued_slashes, 1);
            assert!(
                pallet_proofs_dealer::SlashableProviders::<Test>::get(charlie_provider_id)
                    .is_none()
            );

            // And they should be in the ProvidersWithoutCommitment storage
            let tolerance_period: BlockNumberFor<Test> =
                <<Test as crate::Config>::MaxSeedTolerance as Get<u32>>::get().into();
            let next_deadline = new_deadline + tolerance_period;
            let alice_next_deadline =
                ProvidersWithoutCommitment::<Test>::get(alice_provider_id).unwrap();
            let bob_next_deadline =
                ProvidersWithoutCommitment::<Test>::get(bob_provider_id).unwrap();
            assert_eq!(alice_next_deadline, next_deadline);
            assert_eq!(bob_next_deadline, next_deadline);

            // Which means they should be able to submit a new commitment to "restart" their challenge cycle
            let alice_new_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_new_seed");
            let alice_new_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                None,
                alice_new_commitment
            ));

            let bob_new_seed: SeedFor<Test> = BlakeTwo256::hash(b"bob_new_seed");
            let bob_new_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(bob_new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(bob),
                bob_provider_id.clone(),
                None,
                bob_new_commitment
            ));

            // And after their deadline, they should be able to submit a new commitment
            let new_deadline = SeedCommitmentToDeadline::<Test>::get(alice_new_commitment).unwrap();
            run_to_block(new_deadline - 1);
            let alice_commitment_to_reveal: CommitmentWithSeed<Test> = CommitmentWithSeed {
                commitment: alice_new_commitment.clone(),
                seed: alice_new_seed.clone(),
            };
            let alice_new_seed: SeedFor<Test> = BlakeTwo256::hash(b"alice_new_seed_2");
            let alice_new_commitment: SeedCommitmentFor<Test> =
                BlakeTwo256::hash(alice_new_seed.as_bytes());
            assert_ok!(CrRandomness::add_randomness(
                RuntimeOrigin::signed(alice),
                alice_provider_id.clone(),
                Some(alice_commitment_to_reveal),
                alice_new_commitment
            ));
        });
    }

    #[test]
    fn tick_to_check_for_slashable_providers_does_not_go_over_the_next_tick_but_keeps_up() {
        ExtBuilder::build().execute_with(|| {
            let current_tick = pallet_proofs_dealer::ChallengesTicker::<Test>::get();
            let next_tick = current_tick + 1;
            let tick_to_check_for_slashable_providers =
                TickToCheckForSlashableProviders::<Test>::get();
            assert_eq!(tick_to_check_for_slashable_providers, next_tick);

            // Execute the `on_idle` hook without advancing the tick
            for i in 0..10 {
                CrRandomness::on_idle(current_tick + i, Weight::MAX);
            }

            // The tick to check should have remained the same
            let tick_to_check_for_slashable_providers =
                TickToCheckForSlashableProviders::<Test>::get();
            assert_eq!(tick_to_check_for_slashable_providers, next_tick);

            // Manually advance the tick
            System::set_block_number(current_tick + 20);
            pallet_proofs_dealer::ChallengesTicker::<Test>::set(current_tick + 20);
            let current_tick = pallet_proofs_dealer::ChallengesTicker::<Test>::get();
            let next_tick = current_tick + 1;

            // Execute the `on_idle` hook. Now the tick to check for slashable Providers should catch up to the next tick
            for i in 0..50 {
                CrRandomness::on_idle(current_tick + i, Weight::MAX);
            }

            let tick_to_check_for_slashable_providers =
                TickToCheckForSlashableProviders::<Test>::get();
            assert_eq!(tick_to_check_for_slashable_providers, next_tick);
        });
    }
}

/// Helper function that advances the blockchain until block n, executing the hooks for each block
fn run_to_block(n: BlockNumberFor<Test>) {
    assert!(n > System::block_number(), "Cannot go back in time");

    while System::block_number() < n {
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        CrRandomness::on_poll(System::block_number(), &mut WeightMeter::new());
        AllPalletsWithSystem::on_idle(System::block_number(), Weight::MAX);
    }
}

/// Helper function that advances the blockchain until block n without executing the `on_idle` hook,
/// simulating a chain with a lot of activity
fn run_to_block_without_on_idle(n: BlockNumberFor<Test>) {
    assert!(n > System::block_number(), "Cannot go back in time");

    while System::block_number() < n {
        AllPalletsWithSystem::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        AllPalletsWithSystem::on_initialize(System::block_number());
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        CrRandomness::on_poll(System::block_number(), &mut WeightMeter::new());
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

use std::{collections::BTreeMap, vec};

use codec::Encode;
use frame_support::{
    assert_err, assert_noop, assert_ok,
    dispatch::DispatchClass,
    pallet_prelude::Weight,
    traits::{
        fungible::{Mutate, MutateHold},
        OnFinalize, OnIdle, OnPoll,
    },
    weights::WeightMeter,
    BoundedBTreeSet,
};
use frame_system::{limits::BlockWeights, BlockWeight, ConsumedWeight};
use pallet_storage_providers::HoldReason;
use shp_traits::{ProofsDealerInterface, ReadChallengeableProvidersInterface, TrieRemoveMutation};
use sp_core::{blake2_256, Get, Hasher, H256};
use sp_runtime::{
    traits::{BlakeTwo256, Zero},
    BoundedVec, DispatchError,
};
use sp_trie::CompactProof;

use crate::{
    mock::*,
    pallet::Event,
    types::{
        BlockFullnessHeadroomFor, BlockFullnessPeriodFor, ChallengeHistoryLengthFor,
        ChallengeTicksToleranceFor, ChallengesQueueLengthFor, CheckpointChallengePeriodFor,
        KeyProof, MaxCustomChallengesPerBlockFor, MaxSubmittersPerTickFor,
        MinNotFullBlocksRatioFor, Proof, ProviderIdFor, ProvidersPalletFor,
        RandomChallengesPerBlockFor, TargetTicksStorageOfSubmittersFor,
    },
    ChallengesTicker, ChallengesTickerPaused, LastCheckpointTick, LastDeletedTick,
    LastTickProviderSubmittedAProofFor, NotFullBlocksCount, SlashableProviders,
    TickToChallengesSeed, TickToCheckpointChallenges, TickToProvidersDeadlines,
    ValidProofSubmittersLastTicks,
};

fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);

        // Trigger on_poll hook execution.
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

        // Set weight used to be zero.
        let zero_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => Zero::zero(),
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<Test>::set(zero_block_weight);

        // Trigger on_finalize hook execution.
        ProofsDealer::on_finalize(System::block_number());
    }
}

fn run_to_block_spammed(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);

        // Trigger on_poll hook execution.
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

        // Fill up block.
        let weights: BlockWeights = <Test as frame_system::Config>::BlockWeights::get();
        let max_weight_normal = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => max_weight_normal,
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<Test>::set(block_weight);

        // Trigger on_finalize hook execution.
        ProofsDealer::on_finalize(System::block_number());
    }
}

#[test]
fn challenge_submit_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::challenge(user, file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn challenge_submit_twice_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create two users and add funds to the accounts.
        let user_1 = RuntimeOrigin::signed(1);
        let user_2 = RuntimeOrigin::signed(2);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &2,
            user_balance
        ));

        // Mock two FileKeys.
        let file_key_1 = BlakeTwo256::hash(b"file_key_1");
        let file_key_2 = BlakeTwo256::hash(b"file_key_2");

        // Dispatch challenge extrinsic twice.
        assert_ok!(ProofsDealer::challenge(user_1, file_key_1));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key_1,
            }
            .into(),
        );

        assert_ok!(ProofsDealer::challenge(user_2, file_key_2));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 2,
                key_challenged: file_key_2,
            }
            .into(),
        );

        // Check users' balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee
        );
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&2),
            user_balance - challenge_fee
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 2);
        assert_eq!(challenges_queue[0], file_key_1);
        assert_eq!(challenges_queue[1], file_key_2);
    });
}

#[test]
fn challenge_submit_existing_challenge_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic twice.
        assert_ok!(ProofsDealer::challenge(user.clone(), file_key));
        assert_ok!(ProofsDealer::challenge(user, file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee * 2
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn challenge_submit_in_two_rounds_succeed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic twice.
        assert_ok!(ProofsDealer::challenge(user.clone(), file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        let challenge_fee: u128 = <Test as crate::Config>::ChallengesFee::get();
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee
        );

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);

        // Advance `CheckpointChallengePeriod` blocks.
        let challenge_period: u64 = <Test as crate::Config>::CheckpointChallengePeriod::get();
        run_to_block(challenge_period as u64 + 1);

        // Dispatch challenge extrinsic twice.
        let file_key = BlakeTwo256::hash(b"file_key_2");
        assert_ok!(ProofsDealer::challenge(user, file_key));

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewChallenge {
                who: 1,
                key_challenged: file_key,
            }
            .into(),
        );

        // Check user's balance after challenge.
        assert_eq!(
            <Test as crate::Config>::NativeBalance::usable_balance(&1),
            user_balance - challenge_fee * 2
        );
    });
}

#[test]
fn challenge_wrong_origin_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic with wrong origin.
        assert_noop!(
            ProofsDealer::challenge(RuntimeOrigin::none(), file_key),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn challenge_submit_by_regular_user_with_no_funds_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user with no funds.
        let user = RuntimeOrigin::signed(1);

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::challenge(user, file_key),
            crate::Error::<Test>::FeeChargeFailed
        );
    });
}

#[test]
fn challenge_overflow_challenges_queue_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Fill the challenges queue.
        let queue_size: u32 = <Test as crate::Config>::ChallengesQueueLength::get();
        for i in 0..queue_size {
            let file_key = BlakeTwo256::hash(&i.to_le_bytes());
            assert_ok!(ProofsDealer::challenge(user.clone(), file_key));
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::challenge(user, file_key),
            crate::Error::<Test>::ChallengesQueueOverflow
        );
    });
}

#[test]
fn proofs_dealer_trait_challenge_succeed() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Challenge using trait.
        <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge(&file_key).unwrap();

        // Check that the challenge is in the queue.
        let challenges_queue = crate::ChallengesQueue::<Test>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);
    });
}

#[test]
fn proofs_dealer_trait_challenge_overflow_challenges_queue_fail() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Fill the challenges queue.
        let queue_size: u32 = <Test as crate::Config>::ChallengesQueueLength::get();
        for i in 0..queue_size {
            let file_key = BlakeTwo256::hash(&i.to_le_bytes());
            assert_ok!(<ProofsDealer as shp_traits::ProofsDealerInterface>::challenge(&file_key));
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge(&file_key),
            crate::Error::<Test>::ChallengesQueueOverflow
        );
    });
}

#[test]
fn proofs_dealer_trait_challenge_with_priority_succeed() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Challenge using trait.
        <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
            &file_key, None,
        )
        .unwrap();

        // Check that the challenge is in the queue.
        let priority_challenges_queue = crate::PriorityChallengesQueue::<Test>::get();
        assert_eq!(priority_challenges_queue.len(), 1);
        assert_eq!(priority_challenges_queue[0], (file_key, None));
    });
}

#[test]
fn proofs_dealer_trait_challenge_with_priority_overflow_challenges_queue_fail() {
    new_test_ext().execute_with(|| {
        // Mock a FileKey.
        let file_key = BlakeTwo256::hash(b"file_key");

        // Fill the challenges queue.
        let queue_size: u32 = <Test as crate::Config>::ChallengesQueueLength::get();
        for i in 0..queue_size {
            let file_key = BlakeTwo256::hash(&i.to_le_bytes());
            assert_ok!(
                <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                    &file_key, None
                )
            );
        }

        // Dispatch challenge extrinsic.
        assert_noop!(
            <ProofsDealer as shp_traits::ProofsDealerInterface>::challenge_with_priority(
                &file_key, None
            ),
            crate::Error::<Test>::PriorityChallengesQueueOverflow
        );
    });
}

#[test]
fn proofs_dealer_trait_initialise_challenge_cycle_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Mock a Provider ID.
        let provider_id = BlakeTwo256::hash(b"provider_id");

        // Register user as a Provider in Providers pallet.
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to that Provider and hold some so it has a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));

        // Dispatch initialise provider extrinsic.
        assert_ok!(ProofsDealer::force_initialise_challenge_cycle(
            RuntimeOrigin::root(),
            provider_id
        ));

        // Check that the Provider's last tick was set to 1.
        let last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(&provider_id).unwrap();
        assert_eq!(last_tick_provider_submitted_proof, 1);

        // Check that the Provider's deadline was set to `challenge_period + challenge_ticks_tolerance`
        // after the initialisation.
        let stake = <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
            provider_id,
        )
        .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let expected_deadline =
            last_tick_provider_submitted_proof + challenge_period_plus_tolerance;
        let deadline = TickToProvidersDeadlines::<Test>::get(expected_deadline, provider_id);
        assert_eq!(deadline, Some(()));

        // Check that the last event emitted is the correct one.
        System::assert_last_event(
            Event::NewChallengeCycleInitialised {
                current_tick: 1,
                next_challenge_deadline: expected_deadline,
                provider: provider_id,
                maybe_provider_account: Some(1u64),
            }
            .into(),
        );
    });
}

#[test]
fn proofs_dealer_trait_initialise_challenge_cycle_already_initialised_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Mock a Provider ID.
        let provider_id = BlakeTwo256::hash(b"provider_id");

        // Register user as a Provider in Providers pallet.
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to that Provider and hold some so it has a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));

        // Dispatch initialise provider extrinsic.
        assert_ok!(ProofsDealer::force_initialise_challenge_cycle(
            RuntimeOrigin::root(),
            provider_id
        ));

        // Check that the Provider's last tick was set to 1.
        let last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(&provider_id).unwrap();
        assert_eq!(last_tick_provider_submitted_proof, 1);

        // Check that the Provider's deadline was set to `challenge_period + challenge_ticks_tolerance`
        // after the initialisation.
        let stake = <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
            provider_id,
        )
        .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = last_tick_provider_submitted_proof + challenge_period_plus_tolerance;
        let deadline = TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id);
        assert_eq!(deadline, Some(()));

        // Let some blocks pass (less than `ChallengeTicksTolerance` blocks).
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Re-initialise the provider.
        assert_ok!(ProofsDealer::force_initialise_challenge_cycle(
            RuntimeOrigin::root(),
            provider_id
        ));

        // Check that the Provider's last tick is the current now.
        let last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(&provider_id).unwrap();
        let current_tick = ChallengesTicker::<Test>::get();
        assert_eq!(last_tick_provider_submitted_proof, current_tick);

        // Check that the Provider's deadline was set to `challenge_period + challenge_ticks_tolerance`
        // after the initialisation.
        let stake = <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
            provider_id,
        )
        .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let expected_deadline =
            last_tick_provider_submitted_proof + challenge_period_plus_tolerance;
        let deadline = TickToProvidersDeadlines::<Test>::get(expected_deadline, provider_id);
        assert_eq!(deadline, Some(()));

        // Check that the Provider no longer has the previous deadline.
        let deadline = TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id);
        assert_eq!(deadline, None);

        // Advance beyond the previous deadline block and check that the Provider is not marked as slashable.
        run_to_block(current_block + challenge_ticks_tolerance + 1);

        assert!(!SlashableProviders::<Test>::contains_key(&provider_id));
    });
}

#[test]
fn proofs_dealer_trait_initialise_challenge_cycle_already_initialised_and_new_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Mock two Provider IDs.
        let provider_id_1 = BlakeTwo256::hash(b"provider_id_1");
        let provider_id_2 = BlakeTwo256::hash(b"provider_id_2");

        // Register users as a Provider in Providers pallet.
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id_1,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id_1,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &2,
            provider_id_2,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id_2,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 2u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to those Providers and hold some so they have a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &2,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &2,
            provider_balance / 100
        ));

        // Initialise providers
        assert_ok!(ProofsDealer::initialise_challenge_cycle(&provider_id_1));
        assert_ok!(ProofsDealer::initialise_challenge_cycle(&provider_id_2));

        // Check that the Providers' last tick was set to 1.
        let last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(&provider_id_1).unwrap();
        assert_eq!(last_tick_provider_submitted_proof, 1);
        let last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(&provider_id_2).unwrap();
        assert_eq!(last_tick_provider_submitted_proof, 1);

        // Check that Provider 1's deadline was set to `challenge_period + challenge_ticks_tolerance`
        // after the initialisation.
        let stake = <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
            provider_id_1,
        )
        .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = last_tick_provider_submitted_proof + challenge_period_plus_tolerance;
        let deadline = TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id_1);
        assert_eq!(deadline, Some(()));

        // Let some blocks pass (less than `ChallengeTicksTolerance` blocks).
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Re-initialise the provider.
        assert_ok!(ProofsDealer::initialise_challenge_cycle(&provider_id_1));

        // Check that the Provider's last tick is the current now.
        let last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(&provider_id_1).unwrap();
        let current_tick = ChallengesTicker::<Test>::get();
        assert_eq!(last_tick_provider_submitted_proof, current_tick);

        // Check that the Provider's deadline was set to `challenge_period + challenge_ticks_tolerance`
        // after the initialisation.
        let stake = <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
            provider_id_1,
        )
        .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let expected_deadline =
            last_tick_provider_submitted_proof + challenge_period_plus_tolerance;
        let deadline = TickToProvidersDeadlines::<Test>::get(expected_deadline, provider_id_1);
        assert_eq!(deadline, Some(()));

        // Check that the Provider no longer has the previous deadline.
        let deadline = TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id_1);
        assert_eq!(deadline, None);

        // Advance beyond the previous deadline block and check that the Provider is not marked as slashable.
        run_to_block(current_block + challenge_ticks_tolerance + 1);
        assert!(!SlashableProviders::<Test>::contains_key(&provider_id_1));
    });
}

#[test]
fn proofs_dealer_trait_initialise_challenge_cycle_not_provider_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Mock a Provider ID.
        let provider_id = BlakeTwo256::hash(b"provider_id");

        // Expect failure since the user is not a provider.
        assert_noop!(
            ProofsDealer::initialise_challenge_cycle(&provider_id),
            crate::Error::<Test>::NotProvider
        );
    });
}

#[test]
fn submit_proof_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        let current_tick = ChallengesTicker::<Test>::get();
        let last_tick_provider_submitted_proof = current_tick;
        LastTickProviderSubmittedAProofFor::<Test>::insert(
            &provider_id,
            last_tick_provider_submitted_proof,
        );

        // Set Provider's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, provider_id, ());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(user, proof.clone(), None));

        // Check for event submitted.
        System::assert_last_event(
            Event::ProofAccepted {
                provider: provider_id,
                proof,
            }
            .into(),
        );

        // Check the new last time this provider submitted a proof.
        let expected_new_tick = last_tick_provider_submitted_proof + challenge_period;
        let new_last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(provider_id).unwrap();
        assert_eq!(expected_new_tick, new_last_tick_provider_submitted_proof);

        // Check that the Provider's deadline was pushed forward.
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id),
            None
        );
        let new_deadline = expected_new_tick + challenge_period + challenge_ticks_tolerance;
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(new_deadline, provider_id),
            Some(()),
        );
    });
}

#[test]
fn submit_proof_adds_provider_to_valid_submitters_set() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        let current_tick = ChallengesTicker::<Test>::get();
        let last_tick_provider_submitted_proof = current_tick;
        LastTickProviderSubmittedAProofFor::<Test>::insert(
            &provider_id,
            last_tick_provider_submitted_proof,
        );

        // Set Provider's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, provider_id, ());

        // Advance to the next challenge the Provider should listen to.
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(user, proof.clone(), None));

        // Check for event submitted.
        System::assert_last_event(
            Event::ProofAccepted {
                provider: provider_id,
                proof,
            }
            .into(),
        );

        // Check that the Provider is in the valid submitters set.
        assert!(
            ValidProofSubmittersLastTicks::<Test>::get(ChallengesTicker::<Test>::get())
                .unwrap()
                .contains(&provider_id)
        );
    });
}

#[test]
fn submit_proof_submitted_by_not_a_provider_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Register user as a Provider in Providers pallet.
        // The registered Provider ID will be different from the one that will be used in the proof.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to that Provider and hold some so it has a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(
            RuntimeOrigin::signed(2),
            proof,
            Some(provider_id)
        ));
    });
}

#[test]
fn submit_proof_with_checkpoint_challenges_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof tick.
        let last_tick_provider_submitted_proof = System::block_number();
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let mut challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Set last checkpoint challenge block to be equal to the last tick this provider has submitted
        // a proof for, so that custom challenges will be taken into account in proof verification.
        let checkpoint_challenge_block = last_tick_provider_submitted_proof;
        LastCheckpointTick::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (BlakeTwo256::hash(b"custom_challenge_1"), None),
            (BlakeTwo256::hash(b"custom_challenge_2"), None),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        TickToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Add custom challenges to the challenges vector.
        challenges.extend(custom_challenges.iter().map(|(challenge, _)| *challenge));

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in &challenges {
            key_proofs.insert(
                *challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(user, proof, None));
    });
}

#[test]
fn submit_proof_with_checkpoint_challenges_mutations_success() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: 1000,
                capacity_used: 100,
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

		// Increment the used capacity of BSPs in the Providers pallet.
		pallet_storage_providers::UsedBspsCapacity::<Test>::set(100);

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

		// Create a dynamic-rate payment stream between the user and the Provider.
		pallet_payment_streams::DynamicRatePaymentStreams::<Test>::insert(
            &provider_id,
			&1,
            pallet_payment_streams::types::DynamicRatePaymentStream {
                amount_provided: 10,
				price_index_when_last_charged: pallet_payment_streams::AccumulatedPriceIndex::<Test>::get(),
				user_deposit: 10 * <<Test as pallet_payment_streams::Config>::NewStreamDeposit as Get<u64>>::get() as u128 * pallet_payment_streams::CurrentPricePerUnitPerTick::<Test>::get(),
				out_of_funds_tick: None,
            },
        );

        // Set Provider's last submitted proof tick.
        let last_tick_provider_submitted_proof = System::block_number();
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let mut challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Set last checkpoint challenge block to be equal to the last tick this provider has submitted
        // a proof for, so that custom challenges will be taken into account in proof verification.
        let checkpoint_challenge_block = last_tick_provider_submitted_proof;
        LastCheckpointTick::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (
                BlakeTwo256::hash(b"custom_challenge_1"),
                Some(TrieRemoveMutation::default()),
            ),
            (
                BlakeTwo256::hash(b"custom_challenge_2"),
                Some(TrieRemoveMutation::default()),
            ),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        TickToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Add custom challenges to the challenges vector.
        challenges.extend(custom_challenges.iter().map(|(challenge, _)| *challenge));

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in &challenges {
            key_proofs.insert(
                *challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic.
        assert_ok!(ProofsDealer::submit_proof(user, proof, None));

        // Check that the event for mutations applied is emitted.
        System::assert_has_event(
            Event::MutationsApplied {
                provider: provider_id,
                mutations: custom_challenges
                    .iter()
                    .map(|(key, mutation)| (*key, mutation.clone().unwrap()))
                    .collect(),
                new_root: challenges.last().unwrap().clone(),
            }
            .into(),
        );

        // Check if root of the provider was updated the last challenge key
        // Note: The apply_delta method is applying the mutation the root of the provider for every challenge key.
        // This is to avoid having to construct valid tries and proofs.
        let root =
            <<Test as crate::Config>::ProvidersPallet as ReadChallengeableProvidersInterface>::get_root(provider_id)
                .unwrap();
        assert_eq!(root.as_ref(), challenges.last().unwrap().as_ref());
    });
}

#[test]
fn submit_proof_with_checkpoint_challenges_mutations_fails_if_decoded_metadata_is_invalid() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: 1000,
                capacity_used: 100,
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Increment the used capacity of BSPs in the Providers pallet.
        pallet_storage_providers::UsedBspsCapacity::<Test>::set(100);

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Create a dynamic-rate payment stream between the user and the Provider.
        pallet_payment_streams::DynamicRatePaymentStreams::<Test>::insert(
            &provider_id,
            &1,
            pallet_payment_streams::types::DynamicRatePaymentStream {
                amount_provided: 10,
                price_index_when_last_charged:
                    pallet_payment_streams::AccumulatedPriceIndex::<Test>::get(),
                user_deposit: 10
                    * <<Test as pallet_payment_streams::Config>::NewStreamDeposit as Get<u64>>::get(
                    ) as u128
                    * pallet_payment_streams::CurrentPricePerUnitPerTick::<Test>::get(),
                out_of_funds_tick: None,
            },
        );

        // Set Provider's last submitted proof tick.
        let last_tick_provider_submitted_proof = System::block_number();
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let mut challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Set last checkpoint challenge block to be equal to the last tick this provider has submitted
        // a proof for, so that custom challenges will be taken into account in proof verification.
        let checkpoint_challenge_block = last_tick_provider_submitted_proof;
        LastCheckpointTick::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (
                [0; BlakeTwo256::LENGTH].into(), // Challenge that will return invalid metadata
                Some(TrieRemoveMutation::default()),
            ),
            (
                BlakeTwo256::hash(b"custom_challenge_2"),
                Some(TrieRemoveMutation::default()),
            ),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        TickToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Add custom challenges to the challenges vector.
        challenges.extend(custom_challenges.iter().map(|(challenge, _)| *challenge));

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in &challenges {
            key_proofs.insert(
                *challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic and it fails because of the invalid metadata.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::FailedToApplyDelta
        );
    });
}

#[test]
fn submit_proof_caller_not_a_provider_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs: Default::default(),
        };

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::NotProvider
        );
    });
}

#[test]
fn submit_proof_provider_passed_not_registered_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs: Default::default(),
        };

        // Creating a Provider ID but not registering it.
        let provider_id = BlakeTwo256::hash(b"provider_id");

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, Some(provider_id)),
            crate::Error::<Test>::NotProvider
        );
    });
}

#[test]
fn submit_proof_empty_key_proofs_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs: Default::default(),
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::EmptyKeyProofs
        );
    });
}

#[test]
fn submit_proof_no_record_of_last_proof_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::NoRecordOfLastSubmittedProof
        );
    });
}

#[test]
fn submit_proof_challenges_block_not_reached_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, 1);

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::ChallengesTickNotReached
        );
    });
}

#[test]
#[should_panic(
    expected = "internal error: entered unreachable code: Challenges tick is too old, beyond the history this pallet keeps track of. This should not be possible."
)]
fn submit_proof_challenges_block_too_old_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );

        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, 1);

        // Advance more than `ChallengeHistoryLength` blocks.
        let challenge_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_to_block(challenge_history_length * 2);

        // Dispatch challenge extrinsic.
        let _ = ProofsDealer::submit_proof(user, proof, None);
    });
}

#[test]
#[should_panic(
    expected = "internal error: entered unreachable code: Seed for challenges tick not found, when checked it should be within history."
)]
fn submit_proof_seed_not_found_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, 1);

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Remove challenge seed for challenge block.
        TickToChallengesSeed::<Test>::remove(challenge_block);

        // Dispatch challenge extrinsic.
        let _ = ProofsDealer::submit_proof(user, proof, None);
    });
}

#[test]
#[should_panic(
    expected = "internal error: entered unreachable code: Checkpoint challenges not found, when dereferencing in last registered checkpoint challenge block."
)]
fn submit_proof_checkpoint_challenge_not_found_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: (2 * 100) as u64,
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        TickToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Set last checkpoint challenge block to something before the challenge tick
        // that is being submitted.
        let checkpoint_challenge_block = 1;
        LastCheckpointTick::<Test>::set(checkpoint_challenge_block);

        // Dispatch challenge extrinsic.
        let _ = ProofsDealer::submit_proof(user, proof, None);
    });
}

#[test]
fn submit_proof_forest_proof_verification_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Mock key proofs.
        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![vec![0]],
                },
                challenge_count: Default::default(),
            },
        );

        // Create an empty forest proof to fail verification.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );

        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Dispatch challenge extrinsic.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::ForestProofVerificationFailed
        );
    });
}

#[test]
fn submit_proof_no_key_proofs_for_keys_verified_in_forest_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Creating empty key proof to fail verification.
        let mut key_proofs = BTreeMap::new();
        key_proofs.insert(
            BlakeTwo256::hash(b"key"),
            KeyProof::<Test> {
                proof: CompactProof {
                    encoded_nodes: vec![],
                },
                challenge_count: Default::default(),
            },
        );

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Dispatch challenge extrinsic.
        // The forest proof will pass because it's not empty, so the MockVerifier will accept it,
        // and it will return the generated challenges as keys proven. The key proofs are an empty
        // vector, so it will fail saying that there are no key proofs for the keys proven.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::KeyProofNotFound
        );
    });
}

#[test]
fn submit_proof_out_checkpoint_challenges_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Set random seed for this block challenges.
        let seed = BlakeTwo256::hash(b"seed");
        TickToChallengesSeed::<Test>::insert(System::block_number(), seed);

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Set last checkpoint challenge block.
        let checkpoint_challenge_block = System::block_number() + 1;
        LastCheckpointTick::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (BlakeTwo256::hash(b"custom_challenge_1"), None),
            (BlakeTwo256::hash(b"custom_challenge_2"), None),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        TickToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Dispatch challenge extrinsic.
        // The forest proof will pass because it's not empty, so the MockVerifier will accept it,
        // and it will return the generated challenges as keys proven. The key proofs only contain
        // proofs for the regular challenges, not the checkpoint challenges, so it will fail saying
        // that there are no key proofs for the keys proven.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::KeyProofNotFound
        );
    });
}

#[test]
fn submit_proof_key_proof_verification_fail() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Create user and add funds to the account.
        let user = RuntimeOrigin::signed(1);
        let user_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            user_balance
        ));

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Hold some of the Provider's balance so it simulates it having a stake.
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            user_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        LastTickProviderSubmittedAProofFor::<Test>::insert(&provider_id, System::block_number());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for challenge block.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of empty key proofs for each challenge, to fail verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Dispatch challenge extrinsic.
        // The forest proof will pass because it's not empty, so the MockVerifier will accept it,
        // and it will return the generated challenges as keys proven. There will be key proofs
        // for each key proven, but they are empty, so it will fail saying that the verification
        // failed.
        assert_noop!(
            ProofsDealer::submit_proof(user, proof, None),
            crate::Error::<Test>::KeyProofVerificationFailed
        );
    });
}

#[test]
fn new_challenges_round_random_and_checkpoint_challenges() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Run a block and check that the random challenge was emitted.
        run_to_block(2);

        // Build the expected random seed.
        let challenges_ticker = ChallengesTicker::<Test>::get().encode();
        let challenges_ticker: &[u8] = challenges_ticker.as_ref();
        let subject = [challenges_ticker, &System::block_number().to_le_bytes()].concat();
        let hashed_subject = blake2_256(&subject);
        let expected_seed = H256::from_slice(&hashed_subject);

        // Check that the event is emitted.
        // This would be the first time the random seed is emitted.
        System::assert_last_event(
            Event::NewChallengeSeed {
                challenges_ticker: 2,
                seed: expected_seed,
            }
            .into(),
        );

        // Run another block and check that the random challenge was emitted.
        run_to_block(3);

        // Build the expected random seed.
        let challenges_ticker = ChallengesTicker::<Test>::get().encode();
        let challenges_ticker: &[u8] = challenges_ticker.as_ref();
        let subject: Vec<u8> = [
            challenges_ticker,
            &frame_system::Pallet::<Test>::block_number().to_le_bytes(),
        ]
        .concat();
        let hashed_subject = blake2_256(&subject);
        let expected_seed = H256::from_slice(&hashed_subject);

        // Check that the event is emitted.
        // This would be the second time the random seed is emitted.
        System::assert_last_event(
            Event::NewChallengeSeed {
                challenges_ticker: 3,
                seed: expected_seed,
            }
            .into(),
        );

        // Run until the next checkpoint challenge block.
        let checkpoint_challenge_period: u64 = CheckpointChallengePeriodFor::<Test>::get();
        run_to_block(checkpoint_challenge_period);

        // Expect an empty set of checkpoint challenges.
        let challenges_ticker = ChallengesTicker::<Test>::get();
        let checkpoint_challenges =
            TickToCheckpointChallenges::<Test>::get(challenges_ticker).unwrap();
        assert_eq!(checkpoint_challenges.len(), 0);

        // Check that the event is emitted.
        System::assert_last_event(
            Event::NewCheckpointChallenge {
                challenges_ticker,
                challenges: Default::default(),
            }
            .into(),
        );
    });
}

#[test]
fn new_challenges_round_random_challenges_cleanup() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Run until block number `ChallengesHistoryLength` + 1.
        let challenges_history_length: u64 = ChallengeHistoryLengthFor::<Test>::get();
        run_to_block(challenges_history_length + 1u64);

        // Check that the challenge seed for block 1 is not found.
        assert_eq!(
            TickToChallengesSeed::<Test>::get(1),
            None,
            "Challenge seed for block 1 should not be found."
        );

        // Check that the challenge seed exists for block 2.
        let challenges_ticker = 2u64.encode();
        let challenges_ticker: &[u8] = challenges_ticker.as_ref();
        let subject: Vec<u8> = [challenges_ticker, &2u64.to_le_bytes()].concat();
        let hashed_subject = blake2_256(&subject);
        let expected_seed = H256::from_slice(&hashed_subject);
        assert_eq!(
            TickToChallengesSeed::<Test>::get(2),
            Some(expected_seed),
            "Challenge seed for block 2 should be found."
        );
    });
}

#[test]
fn new_challenges_round_checkpoint_challenges_cleanup() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Run until block number 2 * `CheckpointChallengePeriod`.
        let checkpoint_challenge_period: u64 = CheckpointChallengePeriodFor::<Test>::get();
        run_to_block(checkpoint_challenge_period * 2);

        // Check that the checkpoint challenge for block `checkpoint_challenge_period` is not found.
        assert_eq!(
            TickToCheckpointChallenges::<Test>::get(checkpoint_challenge_period),
            None,
            "Checkpoint challenge for block `CheckpointChallengePeriod` should not be found."
        );

        // Check that the checkpoint challenge exists for block `checkpoint_challenge_period * 2`.
        assert_eq!(
            TickToCheckpointChallenges::<Test>::get(checkpoint_challenge_period * 2),
            Some(Default::default()),
            "Checkpoint challenge for block `CheckpointChallengePeriod * 2` should be found."
        )
    });
}

#[test]
fn new_challenges_round_checkpoint_challenges_with_custom_challenges() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Add custom challenges to the challenges vector.
        let key_challenged = BlakeTwo256::hash(b"key_challenged");
        assert_ok!(<crate::Pallet<Test> as ProofsDealerInterface>::challenge(
            &key_challenged
        ));

        // Add priority challenge to the challenges vector.
        let priority_key_challenged = BlakeTwo256::hash(b"priority_key_challenged");
        assert_ok!(
            <crate::Pallet<Test> as ProofsDealerInterface>::challenge_with_priority(
                &priority_key_challenged,
                Some(TrieRemoveMutation::default())
            )
        );

        // Run until the next checkpoint challenge block.
        let checkpoint_challenge_period: u64 = CheckpointChallengePeriodFor::<Test>::get();
        run_to_block(checkpoint_challenge_period);

        // Expect checkpoint challenges to be emitted, with the priority first.
        let challenges_ticker = ChallengesTicker::<Test>::get();
        let checkpoint_challenges =
            TickToCheckpointChallenges::<Test>::get(challenges_ticker).unwrap();
        assert_eq!(checkpoint_challenges.len(), 2);
        assert_eq!(
            checkpoint_challenges[0],
            (priority_key_challenged, Some(TrieRemoveMutation::default()))
        );
        assert_eq!(checkpoint_challenges[1], (key_challenged, None));
    });
}

#[test]
fn new_challenges_round_max_custom_challenges() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Add max amount of custom challenges to the challenges vector.
        let max_custom_challenges = ChallengesQueueLengthFor::<Test>::get();
        for i in 0..max_custom_challenges {
            let key_challenged = BlakeTwo256::hash(&(i as usize).to_le_bytes());
            assert_ok!(<crate::Pallet<Test> as ProofsDealerInterface>::challenge(
                &key_challenged
            ));
        }

        // Add another custom challenge. It should fail.
        assert_err!(
            <crate::Pallet<Test> as ProofsDealerInterface>::challenge(&BlakeTwo256::hash(
                b"key_challenged"
            )),
            crate::Error::<Test>::ChallengesQueueOverflow
        );

        // Add max amount of priority challenges to the challenges vector.
        let max_priority_challenges = ChallengesQueueLengthFor::<Test>::get();
        for i in 0..max_priority_challenges {
            let key_challenged = BlakeTwo256::hash(&(i as usize).to_le_bytes());
            assert_ok!(
                <crate::Pallet<Test> as ProofsDealerInterface>::challenge_with_priority(
                    &key_challenged,
                    Some(TrieRemoveMutation::default())
                )
            );
        }

        // Add another priority challenge. It should fail.
        assert_err!(
            <crate::Pallet<Test> as ProofsDealerInterface>::challenge_with_priority(
                &BlakeTwo256::hash(b"key_challenged"),
                Some(TrieRemoveMutation::default())
            ),
            crate::Error::<Test>::PriorityChallengesQueueOverflow
        );

        // Check how many checkpoint challenges round are needed to evacuate all the queue.
        let queue_length: u32 = ChallengesQueueLengthFor::<Test>::get();
        let custom_challenges_per_round: u32 = MaxCustomChallengesPerBlockFor::<Test>::get();
        let mut checkpoint_challenge_rounds_needed = queue_length / custom_challenges_per_round;
        if queue_length % custom_challenges_per_round != 0 {
            checkpoint_challenge_rounds_needed += 1;
        }

        // Run until the next checkpoint challenge round.
        let checkpoint_challenge_period: u64 = CheckpointChallengePeriodFor::<Test>::get();
        run_to_block(checkpoint_challenge_period);

        // Expect checkpoint challenges to be emitted, with the priority first.
        let challenges_ticker = ChallengesTicker::<Test>::get();
        let checkpoint_challenges =
            TickToCheckpointChallenges::<Test>::get(challenges_ticker).unwrap();
        assert_eq!(
            checkpoint_challenges.len(),
            custom_challenges_per_round as usize
        );
        for i in 0..checkpoint_challenges.len() {
            let key_challenged = BlakeTwo256::hash(&(i as usize).to_le_bytes());
            assert_eq!(
                checkpoint_challenges[i],
                (key_challenged, Some(TrieRemoveMutation::default()))
            );
        }

        // Run until the needed checkpoint challenge block.
        let checkpoint_challenge_period: u64 = CheckpointChallengePeriodFor::<Test>::get();
        run_to_block(checkpoint_challenge_period * checkpoint_challenge_rounds_needed as u64);

        // The length of the checkpoint challenges should be max, because even if the priority
        // challenges don't fill the queue, the custom challenges will.
        let challenges_ticker = ChallengesTicker::<Test>::get();
        let checkpoint_challenges =
            TickToCheckpointChallenges::<Test>::get(challenges_ticker).unwrap();
        assert_eq!(
            checkpoint_challenges.len(),
            custom_challenges_per_round as usize
        );

        // Expect the last priority challenges in the priority queue to be emitted first.
        let last_priority_challenges_amount = if queue_length % custom_challenges_per_round == 0 {
            custom_challenges_per_round
        } else {
            queue_length % custom_challenges_per_round
        };
        let last_priority_challenges_start_index =
            (checkpoint_challenge_rounds_needed - 1) * custom_challenges_per_round;
        for i in 0..last_priority_challenges_amount {
            let key_challenged = BlakeTwo256::hash(
                &((last_priority_challenges_start_index + i) as usize).to_le_bytes(),
            );
            assert_eq!(
                checkpoint_challenges[i as usize],
                (key_challenged, Some(TrieRemoveMutation::default()))
            );
        }

        // Check that the last checkpoint challenges contain the custom challenges, if there was
        // enough space in this challenge round.
        let checkpoint_challenges_start_index = if queue_length % custom_challenges_per_round == 0 {
            custom_challenges_per_round
        } else {
            queue_length % custom_challenges_per_round
        };
        let checkpoint_challenges_amount =
            custom_challenges_per_round - checkpoint_challenges_start_index;
        for i in 0..checkpoint_challenges_amount {
            let key_challenged = BlakeTwo256::hash(&(i as usize).to_le_bytes());
            assert_eq!(
                checkpoint_challenges[(checkpoint_challenges_start_index + i) as usize],
                (key_challenged, None)
            );
        }

        // Run until the custom challenges are all evacuated.
        let mut checkpoint_challenge_rounds_needed = queue_length / custom_challenges_per_round * 2;
        if queue_length % custom_challenges_per_round != 0 {
            checkpoint_challenge_rounds_needed += 1;
        }
        run_to_block(checkpoint_challenge_period * checkpoint_challenge_rounds_needed as u64);

        // The last checkpoint challenge should be the last custom challenge.
        let challenges_ticker = ChallengesTicker::<Test>::get();
        let checkpoint_challenges =
            TickToCheckpointChallenges::<Test>::get(challenges_ticker).unwrap();
        let last_checkpoint_challenge = &checkpoint_challenges[checkpoint_challenges.len() - 1];
        assert_eq!(
            last_checkpoint_challenge,
            &(
                BlakeTwo256::hash(&((queue_length - 1) as usize).to_le_bytes()),
                None
            )
        )
    });
}

#[test]
fn new_challenges_round_provider_marked_as_slashable() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to that Provider and hold some so it has a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        let current_tick = ChallengesTicker::<Test>::get();
        let prev_tick_provider_submitted_proof = current_tick;
        LastTickProviderSubmittedAProofFor::<Test>::insert(
            &provider_id,
            prev_tick_provider_submitted_proof,
        );

        // Set Provider's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, provider_id, ());

        // Check that Provider is not in the SlashableProviders storage map.
        assert!(!SlashableProviders::<Test>::contains_key(&provider_id));

        // Advance to the deadline block for this Provider.
        run_to_block(prev_deadline);

        // Check event of provider being marked as slashable.
        System::assert_has_event(
            Event::SlashableProvider {
                provider: provider_id,
                next_challenge_deadline: prev_deadline + challenge_period,
            }
            .into(),
        );

        // Check that Provider is in the SlashableProviders storage map.
        assert!(SlashableProviders::<Test>::contains_key(&provider_id));
        assert_eq!(
            SlashableProviders::<Test>::get(&provider_id),
            Some(<Test as crate::Config>::RandomChallengesPerBlock::get())
        );

        // Check the new last time this provider submitted a proof.
        let current_tick_provider_submitted_proof =
            prev_tick_provider_submitted_proof + challenge_period;
        let new_last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(provider_id).unwrap();
        assert_eq!(
            current_tick_provider_submitted_proof,
            new_last_tick_provider_submitted_proof
        );

        // Check that the Provider's deadline was pushed forward.
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id),
            None
        );
        let new_deadline =
            new_last_tick_provider_submitted_proof + challenge_period + challenge_ticks_tolerance;
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(new_deadline, provider_id),
            Some(()),
        );
    });
}

#[test]
fn multiple_new_challenges_round_provider_accrued_many_failed_proof_submissions() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to that Provider and hold some so it has a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different from the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        let prev_tick_provider_submitted_proof = ChallengesTicker::<Test>::get();
        LastTickProviderSubmittedAProofFor::<Test>::insert(
            &provider_id,
            prev_tick_provider_submitted_proof,
        );

        // New challenges round
        let current_tick = ChallengesTicker::<Test>::get();
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;

        TickToProvidersDeadlines::<Test>::insert(prev_deadline, provider_id, ());

        // Check that Provider is not in the SlashableProviders storage map.
        assert!(!SlashableProviders::<Test>::contains_key(&provider_id));

        // Set last checkpoint challenge block.
        let checkpoint_challenge_block = 1;
        LastCheckpointTick::<Test>::set(checkpoint_challenge_block);

        // Make up custom challenges.
        let custom_challenges = BoundedVec::try_from(vec![
            (BlakeTwo256::hash(b"custom_challenge_1"), None),
            (BlakeTwo256::hash(b"custom_challenge_2"), None),
        ])
        .unwrap();

        // Set custom challenges in checkpoint block.
        TickToCheckpointChallenges::<Test>::insert(
            checkpoint_challenge_block,
            custom_challenges.clone(),
        );

        // Advance to the deadline block for this Provider.
        run_to_block(prev_deadline);

        let next_challenge_deadline = prev_deadline + challenge_period;
        // Check event of provider being marked as slashable.
        System::assert_has_event(
            Event::SlashableProvider {
                provider: provider_id,
                next_challenge_deadline,
            }
            .into(),
        );

        // Check that Provider is in the SlashableProviders storage map.
        assert!(SlashableProviders::<Test>::contains_key(&provider_id));

        let random_challenges_per_block: u32 =
            <Test as crate::Config>::RandomChallengesPerBlock::get();

        let missed_proof_submissions = random_challenges_per_block.saturating_add(2);

        assert_eq!(
            SlashableProviders::<Test>::get(&provider_id),
            Some(missed_proof_submissions)
        );

        // New challenges round
        let current_tick = ChallengesTicker::<Test>::get();
        let prev_deadline = current_tick + challenge_period;
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, provider_id, ());

        // Advance to the deadline block for this Provider.
        run_to_block(next_challenge_deadline);

        // Check event of provider being marked as slashable.
        System::assert_has_event(
            Event::SlashableProvider {
                provider: provider_id,
                next_challenge_deadline: prev_deadline + challenge_period,
            }
            .into(),
        );

        // Check that Provider is in the SlashableProviders storage map.
        assert!(SlashableProviders::<Test>::contains_key(&provider_id));
        assert_eq!(
            SlashableProviders::<Test>::get(&provider_id),
            Some(random_challenges_per_block.saturating_add(missed_proof_submissions))
        );
    });
}

#[test]
fn new_challenges_round_bad_provider_marked_as_slashable_but_good_no() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Register Alice as a Provider in Providers pallet.
        let alice_provider_id = BlakeTwo256::hash(b"alice_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            alice_provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &alice_provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to Alice and hold some so it has a stake.
        let alice_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            alice_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            alice_balance / 100
        ));

        // Register Bob as a Provider in Providers pallet.
        let bob_provider_id = BlakeTwo256::hash(b"bob_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &2,
            bob_provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &bob_provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 2u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to Bob and hold some so it has a stake.
        let bob_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &2,
            bob_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &2,
            bob_balance / 100
        ));

        // Set Alice and Bob's root to be an arbitrary value, different than the default root,
        // to simulate that they are actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &alice_provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &bob_provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Alice and Bob's last submitted proof block.
        let current_tick = ChallengesTicker::<Test>::get();
        let last_interval_tick = current_tick;
        LastTickProviderSubmittedAProofFor::<Test>::insert(&alice_provider_id, last_interval_tick);
        LastTickProviderSubmittedAProofFor::<Test>::insert(&bob_provider_id, last_interval_tick);

        // Set Alice and Bob's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                alice_provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, alice_provider_id, ());
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, bob_provider_id, ());

        // Check that Alice and Bob are not in the SlashableProviders storage map.
        assert!(!SlashableProviders::<Test>::contains_key(
            &alice_provider_id
        ));
        assert!(!SlashableProviders::<Test>::contains_key(&bob_provider_id));

        // Advance to the next challenge Alice and Bob should listen to.
        let current_block = System::block_number();
        let challenge_block = current_block + challenge_period;
        run_to_block(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let current_block = System::block_number();
        run_to_block(current_block + challenge_ticks_tolerance - 1);

        // Get the seed for block 2.
        let seed = TickToChallengesSeed::<Test>::get(challenge_block).unwrap();

        // Calculate challenges from seed, so that we can mock a key proof for each.
        let challenges = crate::Pallet::<Test>::generate_challenges_from_seed(
            seed,
            &alice_provider_id,
            RandomChallengesPerBlockFor::<Test>::get(),
        );

        // Creating a vec of proofs with some content to pass verification.
        let mut key_proofs = BTreeMap::new();
        for challenge in challenges {
            key_proofs.insert(
                challenge,
                KeyProof::<Test> {
                    proof: CompactProof {
                        encoded_nodes: vec![vec![0]],
                    },
                    challenge_count: Default::default(),
                },
            );
        }

        // Mock a proof.
        let proof = Proof::<Test> {
            forest_proof: CompactProof {
                encoded_nodes: vec![vec![0]],
            },
            key_proofs,
        };

        // Have Alice submit a proof.
        assert_ok!(ProofsDealer::submit_proof(
            RuntimeOrigin::signed(1),
            proof.clone(),
            None
        ));

        // Check for event submitted.
        System::assert_last_event(
            Event::ProofAccepted {
                provider: alice_provider_id,
                proof,
            }
            .into(),
        );

        // Advance to the deadline block for this Provider.
        run_to_block(prev_deadline);

        System::assert_has_event(
            Event::SlashableProvider {
                provider: bob_provider_id,
                next_challenge_deadline: prev_deadline + challenge_period,
            }
            .into(),
        );

        // Check that Bob is in the SlashableProviders storage map and that Alice is not.
        assert!(!SlashableProviders::<Test>::contains_key(
            &alice_provider_id
        ));
        assert!(SlashableProviders::<Test>::contains_key(&bob_provider_id));
        assert_eq!(
            SlashableProviders::<Test>::get(&bob_provider_id),
            Some(<Test as crate::Config>::RandomChallengesPerBlock::get())
        );

        // Check the new last tick interval for Alice and Bob.
        let expected_new_tick = last_interval_tick + challenge_period;
        let new_last_interval_tick_alice =
            LastTickProviderSubmittedAProofFor::<Test>::get(alice_provider_id).unwrap();
        assert_eq!(expected_new_tick, new_last_interval_tick_alice);
        let new_last_interval_tick_bob =
            LastTickProviderSubmittedAProofFor::<Test>::get(bob_provider_id).unwrap();
        assert_eq!(expected_new_tick, new_last_interval_tick_bob);

        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(prev_deadline, alice_provider_id),
            None
        );
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(prev_deadline, bob_provider_id),
            None
        );

        // Check that the both Alice and Bob's deadlines were pushed forward.
        let new_deadline = expected_new_tick + challenge_period_plus_tolerance;
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(new_deadline, alice_provider_id),
            Some(()),
        );
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(new_deadline, bob_provider_id),
            Some(()),
        );
    });
}

#[test]
fn challenges_ticker_paused_works() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Get the current tick.
        let current_tick = ChallengesTicker::<Test>::get();

        // Set the challenges ticker to paused.
        assert_ok!(ProofsDealer::set_paused(RuntimeOrigin::root(), true));

        // Assert event emitted.
        System::assert_last_event(Event::<Test>::ChallengesTickerSet { paused: true }.into());

        // Advance a number of blocks.
        let current_block = System::block_number();
        run_to_block(current_block + 10);

        // Check that the challenges ticker is still the same.
        assert_eq!(ChallengesTicker::<Test>::get(), current_tick);

        // Unpause the challenges ticker.
        assert_ok!(ProofsDealer::set_paused(RuntimeOrigin::root(), false));

        // Assert event emitted.
        System::assert_last_event(Event::<Test>::ChallengesTickerSet { paused: false }.into());

        // Advance a number of blocks.
        let current_block = System::block_number();
        run_to_block(current_block + 10);

        // Check that the challenges ticker is now incremented.
        assert_eq!(ChallengesTicker::<Test>::get(), current_tick + 10);
    });
}

#[test]
fn challenges_ticker_block_considered_full_with_max_normal_weight() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Simulate a full block.
        System::set_block_number(System::block_number() + 1);

        // Starting with `on_poll` hook.
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

        // Set normal weight used to be the maximum.
        let weights: BlockWeights = <Test as frame_system::Config>::BlockWeights::get();
        let max_weight_normal = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let max_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => max_weight_normal,
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<Test>::set(max_block_weight);

        // Trigger on_finalize hook execution.
        ProofsDealer::on_finalize(System::block_number());

        // Get the current count of non-full blocks.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();

        // In the next block, after executing `on_poll`, `NonFullBlocksCount` should NOT be incremented.
        System::set_block_number(System::block_number() + 1);
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        assert_eq!(NotFullBlocksCount::<Test>::get(), blocks_not_full);
    });
}

#[test]
fn challenges_ticker_block_considered_full_with_weight_left_smaller_than_headroom() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Simulate an almost full block.
        System::set_block_number(System::block_number() + 1);

        // Starting with `on_poll` hook.
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

        // Set weight used to NOT leave headroom.
        let headroom_weight = BlockFullnessHeadroomFor::<Test>::get();
        let weights: BlockWeights = <Test as frame_system::Config>::BlockWeights::get();
        let max_weight_normal = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let max_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => max_weight_normal - headroom_weight + Weight::from_parts(1, 0),
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<Test>::set(max_block_weight);

        // Trigger on_finalize hook execution.
        ProofsDealer::on_finalize(System::block_number());

        // Get the current count of non-full blocks.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();

        // In the next block, after executing `on_poll`, `NonFullBlocksCount` should NOT be incremented.
        System::set_block_number(System::block_number() + 1);
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        assert_eq!(NotFullBlocksCount::<Test>::get(), blocks_not_full);
    });
}

#[test]
fn challenges_ticker_block_considered_not_full_with_weight_left_equal_to_headroom() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Simulate a block with exactly headroom weight left.
        System::set_block_number(System::block_number() + 1);

        // Starting with `on_poll` hook.
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

        // Set weight used to leave headroom.
        let headroom_weight = BlockFullnessHeadroomFor::<Test>::get();
        let weights: BlockWeights = <Test as frame_system::Config>::BlockWeights::get();
        let max_weight_normal = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let max_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => max_weight_normal.saturating_sub(headroom_weight),
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<Test>::set(max_block_weight);

        // Trigger on_finalize hook execution.
        ProofsDealer::on_finalize(System::block_number());

        // Get the current count of non-full blocks.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();

        // In the next block, after executing `on_poll`, `NonFullBlocksCount` should be incremented.
        System::set_block_number(System::block_number() + 1);
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        assert_eq!(NotFullBlocksCount::<Test>::get(), blocks_not_full + 1);
    });
}

#[test]
fn challenges_ticker_block_considered_not_full_with_weight_left_greater_than_headroom() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Simulate a not-full block.
        System::set_block_number(System::block_number() + 1);

        // Starting with `on_poll` hook.
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

        // Set weight used to leave headroom.
        let headroom_weight = BlockFullnessHeadroomFor::<Test>::get();
        let weights: BlockWeights = <Test as frame_system::Config>::BlockWeights::get();
        let max_weight_normal = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let max_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => max_weight_normal.saturating_sub(headroom_weight.mul(2)),
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<Test>::set(max_block_weight);

        // Trigger on_finalize hook execution.
        ProofsDealer::on_finalize(System::block_number());

        // Get the current count of non-full blocks.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();

        // In the next block, after executing `on_poll`, `NonFullBlocksCount` should be incremented.
        System::set_block_number(System::block_number() + 1);
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        assert_eq!(NotFullBlocksCount::<Test>::get(), blocks_not_full + 1);
    });
}

#[test]
fn challenges_ticker_paused_only_after_tolerance_blocks() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Tick counter should be 1 while in block 1.
        assert_eq!(ChallengesTicker::<Test>::get(), 1);

        // Go until `BlockFullnessPeriodFor` blocks, with spammed blocks.
        let block_fullness_period = BlockFullnessPeriodFor::<Test>::get();
        run_to_block_spammed(block_fullness_period);

        // Assert that the challenges ticker is NOT paused, and the tick counter advanced `BlockFullnessPeriodFor`.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period);

        // Go one more block beyond `BlockFullnessPeriodFor`.
        // Ticker should stop at this tick.
        run_to_block_spammed(block_fullness_period + 1);

        // Assert that now the challenges ticker is paused, and the tick counter stopped at `BlockFullnessPeriodFor` + 1.
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);

        // Going one block beyond, shouldn't increment the ticker.
        run_to_block(block_fullness_period + 2);
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);
    });
}

#[test]
fn challenges_ticker_paused_when_less_than_min_not_full_blocks_ratio_are_not_full() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Tick counter should be 1 while in block 1.
        assert_eq!(ChallengesTicker::<Test>::get(), 1);

        // Make sure there are `BlockFullnessPeriod * MinNotFullBlocksRatio` (floor)
        // not-spammed blocks. Consider that the first block was not spammed.
        let block_fullness_period: u64 = BlockFullnessPeriodFor::<Test>::get();
        let min_not_full_blocks_ratio = MinNotFullBlocksRatioFor::<Test>::get();
        let blocks_to_spam = min_not_full_blocks_ratio
            .mul_floor(block_fullness_period)
            .saturating_sub(1);
        let current_block = System::block_number();
        run_to_block(current_block + blocks_to_spam);

        // Go until `BlockFullnessPeriodFor` blocks, with spammed blocks.
        run_to_block_spammed(block_fullness_period);

        // Assert that the challenges ticker is NOT paused, and the tick counter advanced `BlockFullnessPeriod`.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period);

        // Go one more block beyond `BlockFullnessPeriod`.
        // Ticker should stop at this tick.
        run_to_block(block_fullness_period + 1);

        // Assert that now the challenges ticker is paused, and the tick counter stopped at `BlockFullnessPeriod` + 1.
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);

        // Going one block beyond, shouldn't increment the ticker.
        run_to_block(block_fullness_period + 2);
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);
    });
}

#[test]
fn challenges_ticker_not_paused_when_more_than_min_not_full_blocks_ratio_are_not_full() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Tick counter should be 1 while in block 1.
        assert_eq!(ChallengesTicker::<Test>::get(), 1);

        // Make sure there are more than `BlockFullnessPeriod * MinNotFullBlocksRatio` (floor)
        // not-spammed blocks. Consider that the first block was not spammed.
        let block_fullness_period: u64 = BlockFullnessPeriodFor::<Test>::get();
        let min_not_full_blocks_ratio = MinNotFullBlocksRatioFor::<Test>::get();
        let blocks_to_spam = min_not_full_blocks_ratio
            .mul_floor(block_fullness_period)
            .saturating_sub(1);
        let current_block = System::block_number();
        run_to_block(current_block + blocks_to_spam + 1);

        // Go until `BlockFullnessPeriodFor` blocks, with spammed blocks.
        run_to_block_spammed(block_fullness_period);

        // Assert that the challenges ticker is NOT paused, and the tick counter advanced `BlockFullnessPeriod`.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period);

        // Go one more block beyond `BlockFullnessPeriod`.
        // Ticker should NOT stop at this tick.
        run_to_block(block_fullness_period + 1);

        // Assert that the challenges ticker is still NOT paused, and the tick counter continues.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);

        // Going one block beyond, should increment the ticker.
        run_to_block(block_fullness_period + 2);
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 2);
    });
}

#[test]
fn challenges_ticker_not_paused_when_blocks_dont_run_on_poll() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Simulate multiple non-spammed blocks that don't run on `on_poll`.
        // Like multi block migrations.
        let block_fullness_period = BlockFullnessPeriodFor::<Test>::get();
        for _ in 1..block_fullness_period {
            System::set_block_number(System::block_number() + 1);

            // Set weight used to zero (not-spammed).
            let max_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
                DispatchClass::Normal => Zero::zero(),
                DispatchClass::Operational => Zero::zero(),
                DispatchClass::Mandatory => Zero::zero(),
            });
            BlockWeight::<Test>::set(max_block_weight);

            // Trigger on_finalize hook execution.
            ProofsDealer::on_finalize(System::block_number());
        }

        // Get the current count of non-full blocks. Should be zero as `on_poll` was only run
        // once in `run_to_block(1)`, taking into account the genesis block. In other words, not
        // adding anything.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();
        assert_eq!(blocks_not_full, 0);

        // Current ticker should be 1.
        assert_eq!(ChallengesTicker::<Test>::get(), 1);

        // In the next block, after executing `on_poll`, `NonFullBlocksCount` should be incremented.
        System::set_block_number(System::block_number() + 1);
        ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
        assert_eq!(NotFullBlocksCount::<Test>::get(), blocks_not_full + 1);
    });
}

#[test]
fn challenges_ticker_unpaused_after_spam_finishes() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Tick counter should be 1 while in block 1.
        assert_eq!(ChallengesTicker::<Test>::get(), 1);

        // Go until `BlockFullnessPeriod` blocks, with spammed blocks.
        let block_fullness_period = BlockFullnessPeriodFor::<Test>::get();
        run_to_block_spammed(block_fullness_period);

        // Go one more block beyond `BlockFullnessPeriod`.
        // Ticker should stop at this tick.
        run_to_block_spammed(block_fullness_period + 1);

        // Going one block beyond, shouldn't increment the ticker.
        run_to_block(block_fullness_period + 2);
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);

        // Getting how many blocks have been considered NOT full from the last `BlockFullnessPeriod`.
        // We need to increase that number so that it is greater than `BlockFullnessPeriod * MinNotFullBlocksRatio`.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();
        let min_non_full_blocks_ratio = MinNotFullBlocksRatioFor::<Test>::get();
        let min_non_full_blocks: u64 =
            min_non_full_blocks_ratio.mul_floor(BlockFullnessPeriodFor::<Test>::get());
        let empty_blocks_to_advance = min_non_full_blocks + 1 - blocks_not_full;

        // Advance `empty_blocks_to_advance` blocks.
        let current_ticker = ChallengesTicker::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + empty_blocks_to_advance);

        // Assert that the challenges ticker is NOT paused, but that the `ChallengesTicker` is still the same.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), current_ticker);

        // Advance one more block and assert that the challenges ticker increments.
        run_to_block(current_block + empty_blocks_to_advance + 1);
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), current_ticker + 1);
    });
}

#[test]
fn challenges_ticker_paused_twice() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Tick counter should be 1 while in block 1.
        assert_eq!(ChallengesTicker::<Test>::get(), 1);

        // Go until `BlockFullnessPeriod` blocks, with spammed blocks.
        let block_fullness_period = BlockFullnessPeriodFor::<Test>::get();
        run_to_block_spammed(block_fullness_period);

        // Go one more block beyond `BlockFullnessPeriod`.
        // Ticker should stop at this tick.
        run_to_block_spammed(block_fullness_period + 1);

        // Going one block beyond, shouldn't increment the ticker.
        run_to_block(block_fullness_period + 2);
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(ChallengesTicker::<Test>::get(), block_fullness_period + 1);

        // Getting how many blocks have been considered NOT full from the last `BlockFullnessPeriod`.
        // We need to increase that number so that it is greater than `BlockFullnessPeriod * MinNotFullBlocksRatio`.
        let blocks_not_full = NotFullBlocksCount::<Test>::get();
        let min_non_full_blocks_ratio = MinNotFullBlocksRatioFor::<Test>::get();
        let min_non_full_blocks: u64 =
            min_non_full_blocks_ratio.mul_floor(BlockFullnessPeriodFor::<Test>::get());
        let empty_blocks_to_advance = min_non_full_blocks + 1 - blocks_not_full;

        // Advance `empty_blocks_to_advance` blocks.
        let current_ticker = ChallengesTicker::<Test>::get();
        let current_block = System::block_number();
        run_to_block(current_block + empty_blocks_to_advance);

        // Assert that the challenges ticker is NOT paused, but that the `ChallengesTicker` is still the same.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), current_ticker);

        // Advance one more block and assert that the challenges ticker increments.
        run_to_block(current_block + empty_blocks_to_advance + 1);
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), current_ticker + 1);

        // Getting how many blocks have been considered NOT full from the last `BlockFullnessPeriod`.
        // We need to decrease that number so that it is smaller or equal to`BlockFullnessPeriod * MinNotFullBlocksRatio`.
        let mut blocks_not_full = NotFullBlocksCount::<Test>::get();
        let min_non_full_blocks_ratio = MinNotFullBlocksRatioFor::<Test>::get();
        let min_non_full_blocks: u64 =
            min_non_full_blocks_ratio.mul_floor(BlockFullnessPeriodFor::<Test>::get());

        // We cannot just spam however many blocks of difference are in `blocks_not_full` - `min_non_full_blocks`
        // because the oldest blocks being considered were also spammed. We would be adding new spammed blocks
        // in the newest blocks, and removing them from the oldest ones. So we need to spam blocks until non-spammed
        // blocks are old enough and start getting discarded.
        let mut blocks_advanced = 0;
        let current_ticker = ChallengesTicker::<Test>::get();
        while blocks_not_full > min_non_full_blocks {
            let current_block = System::block_number();
            run_to_block_spammed(current_block + 1);
            blocks_not_full = NotFullBlocksCount::<Test>::get();
            blocks_advanced += 1;
        }

        // Assert that the challenges ticker IS paused, but that the `ChallengesTicker` has advanced `not_empty_blocks_to_advance`.
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(
            ChallengesTicker::<Test>::get(),
            current_ticker + blocks_advanced
        );

        // Advance one more block and assert that the challenges ticker doesn't increment.
        let current_block = System::block_number();
        run_to_block(current_block + 1);
        assert!(ChallengesTickerPaused::<Test>::get().is_some());
        assert_eq!(
            ChallengesTicker::<Test>::get(),
            current_ticker + blocks_advanced
        );
    });
}

#[test]
fn challenges_ticker_provider_not_slashed_if_network_spammed() {
    new_test_ext().execute_with(|| {
        // Go past genesis block so events get deposited.
        run_to_block(1);

        // Go beyond `BlockFullnessPeriod` blocks, with not spammed blocks, to simulate
        // an operational scenario already.
        let block_fullness_period: u64 = BlockFullnessPeriodFor::<Test>::get();
        run_to_block(block_fullness_period + 1);

        // Register user as a Provider in Providers pallet.
        let provider_id = BlakeTwo256::hash(b"provider_id");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
            &1,
            provider_id,
        );
        pallet_storage_providers::BackupStorageProviders::<Test>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: Default::default(),
                capacity_used: Default::default(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: 1u64,
                payment_account: Default::default(),
                reputation_weight:
                    <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
            },
        );

        // Add balance to that Provider and hold some so it has a stake.
        let provider_balance = 1_000_000_000_000_000;
        assert_ok!(<Test as crate::Config>::NativeBalance::mint_into(
            &1,
            provider_balance
        ));
        assert_ok!(<Test as crate::Config>::NativeBalance::hold(
            &HoldReason::StorageProviderDeposit.into(),
            &1,
            provider_balance / 100
        ));

        // Set Provider's root to be an arbitrary value, different than the default root,
        // to simulate that it is actually providing a service.
        let root = BlakeTwo256::hash(b"1234");
        pallet_storage_providers::BackupStorageProviders::<Test>::mutate(
            &provider_id,
            |provider| {
                provider.as_mut().expect("Provider should exist").root = root;
            },
        );

        // Set Provider's last submitted proof block.
        let current_tick = ChallengesTicker::<Test>::get();
        let prev_tick_provider_submitted_proof = current_tick;
        LastTickProviderSubmittedAProofFor::<Test>::insert(
            &provider_id,
            prev_tick_provider_submitted_proof,
        );

        // Set Provider's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<Test> as ReadChallengeableProvidersInterface>::get_stake(
                provider_id,
            )
            .unwrap();
        let challenge_period = crate::Pallet::<Test>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: u64 = ChallengeTicksToleranceFor::<Test>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<Test>::insert(prev_deadline, provider_id, ());

        // Check that Provider is not in the SlashableProviders storage map.
        assert!(!SlashableProviders::<Test>::contains_key(&provider_id));

        // Up until this point, all blocks have been not-spammed, so the `NotFullBlocksCount`
        // should be equal to `BlockFullnessPeriod`.
        let current_not_full_blocks_count = NotFullBlocksCount::<Test>::get();
        assert_eq!(current_not_full_blocks_count, block_fullness_period);

        // Advance until the next challenge period block without spammed blocks.
        let current_block = System::block_number();
        run_to_block(current_block + challenge_period);

        // Advance to the deadline block for this Provider, but with spammed blocks.
        run_to_block_spammed(prev_deadline);

        // Check that Provider is NOT in the SlashableProviders storage map.
        assert!(!SlashableProviders::<Test>::contains_key(&provider_id));

        // Getting how many blocks have been considered NOT full from the last `BlockFullnessPeriod`.
        // We need to increase that number so that it is greater than `BlockFullnessPeriod * MinNotFullBlocksRatio`.
        let mut blocks_not_full = NotFullBlocksCount::<Test>::get();
        let min_non_full_blocks_ratio = MinNotFullBlocksRatioFor::<Test>::get();
        let min_non_full_blocks: u64 =
            min_non_full_blocks_ratio.mul_floor(BlockFullnessPeriodFor::<Test>::get());

        let current_ticker = ChallengesTicker::<Test>::get();
        while blocks_not_full <= min_non_full_blocks {
            let current_block = System::block_number();
            run_to_block(current_block + 1);
            blocks_not_full = NotFullBlocksCount::<Test>::get();
        }

        // Now the `ChallengesTicker` shouldn't be paused. But current ticker should be the same.
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), current_ticker);

        // Advancing one more block should increase the `ChallengesTicker` by one.
        let current_block = System::block_number();
        run_to_block(current_block + 1);
        assert!(ChallengesTickerPaused::<Test>::get().is_none());
        assert_eq!(ChallengesTicker::<Test>::get(), current_ticker + 1);

        // Get how many blocks until the deadline tick.
        let current_ticker = ChallengesTicker::<Test>::get();
        let blocks_to_advance = prev_deadline - current_ticker;
        let current_block = System::block_number();
        run_to_block(current_block + blocks_to_advance);

        // Check event of provider being marked as slashable.
        System::assert_has_event(
            Event::SlashableProvider {
                provider: provider_id,
                next_challenge_deadline: prev_deadline + challenge_period,
            }
            .into(),
        );

        // Check that Provider is in the SlashableProviders storage map.
        assert!(SlashableProviders::<Test>::contains_key(&provider_id));
        assert_eq!(
            SlashableProviders::<Test>::get(&provider_id),
            Some(<Test as crate::Config>::RandomChallengesPerBlock::get())
        );

        // Check the new last time this provider submitted a proof.
        let current_tick_provider_submitted_proof =
            prev_tick_provider_submitted_proof + challenge_period;
        let new_last_tick_provider_submitted_proof =
            LastTickProviderSubmittedAProofFor::<Test>::get(provider_id).unwrap();
        assert_eq!(
            current_tick_provider_submitted_proof,
            new_last_tick_provider_submitted_proof
        );

        // Check that the Provider's deadline was pushed forward.
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(prev_deadline, provider_id),
            None
        );
        let new_deadline =
            new_last_tick_provider_submitted_proof + challenge_period + challenge_ticks_tolerance;
        assert_eq!(
            TickToProvidersDeadlines::<Test>::get(new_deadline, provider_id),
            Some(()),
        );
    });
}

mod on_idle_hook_tests {
    use super::*;

    #[test]
    fn on_idle_hook_works() {
        new_test_ext().execute_with(|| {
            // Go past genesis block so events get deposited.
            run_to_block(1);

            // Register user as a Provider in Providers pallet.
            let provider_id = BlakeTwo256::hash(b"provider_id");
            pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
                &1,
                provider_id,
            );
            pallet_storage_providers::BackupStorageProviders::<Test>::insert(
                &provider_id,
                pallet_storage_providers::types::BackupStorageProvider {
                    capacity: Default::default(),
                    capacity_used: Default::default(),
                    multiaddresses: Default::default(),
                    root: Default::default(),
                    last_capacity_change: Default::default(),
                    owner_account: 1u64,
                    payment_account: Default::default(),
                    reputation_weight:
                        <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                },
            );

            // Add the Provider to the `ValidProofSubmittersLastTicks` storage map for the current tick.
            let tick_when_proof_provided = ChallengesTicker::<Test>::get();
            let mut new_valid_submitters =
                BoundedBTreeSet::<ProviderIdFor<Test>, MaxSubmittersPerTickFor<Test>>::new();
            new_valid_submitters.try_insert(provider_id).unwrap();
            ValidProofSubmittersLastTicks::<Test>::insert(
                tick_when_proof_provided,
                new_valid_submitters,
            );

            // Check that the Provider was successfully added to the set.
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Advance a tick, executing `on_idle`, to check if the Provider is removed from the set.
            run_to_block(System::block_number() + 1);
            ProofsDealer::on_idle(System::block_number(), Weight::MAX);

            // Check that the set which had the Provider that submitted a valid proof has not been deleted.
            assert!(ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided).is_some());
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Check that the last deleted tick is still 0
            assert_eq!(LastDeletedTick::<Test>::get(), 0);

            // Advance enough ticks so that the Provider list which has the `provider_id` is set to be deleted.
            run_to_block(
                System::block_number()
                    + <TargetTicksStorageOfSubmittersFor<Test> as Get<u32>>::get() as u64,
            );

            // Call the `on_idle` hook.
            ProofsDealer::on_idle(System::block_number(), Weight::MAX);

            // Check that the set which had the Provider that submitted a valid proof has been correctly deleted.
            assert!(ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided).is_none());

            // Check that the last deleted tick is the one that was just deleted.
            assert_eq!(
                LastDeletedTick::<Test>::get(),
                System::block_number()
                    - <TargetTicksStorageOfSubmittersFor<Test> as Get<u32>>::get() as u64
            );
        });
    }

    #[test]
    fn on_idle_hook_does_not_delete_with_not_enough_weight() {
        new_test_ext().execute_with(|| {
            // Go past genesis block so events get deposited.
            run_to_block(1);

            // Register user as a Provider in Providers pallet.
            let provider_id = BlakeTwo256::hash(b"provider_id");
            pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
                &1,
                provider_id,
            );
            pallet_storage_providers::BackupStorageProviders::<Test>::insert(
                &provider_id,
                pallet_storage_providers::types::BackupStorageProvider {
                    capacity: Default::default(),
                    capacity_used: Default::default(),
                    multiaddresses: Default::default(),
                    root: Default::default(),
                    last_capacity_change: Default::default(),
                    owner_account: 1u64,
                    payment_account: Default::default(),
                    reputation_weight:
                        <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                },
            );

            // Add the Provider to the `ValidProofSubmittersLastTicks` storage map for the current tick.
            let tick_when_proof_provided = ChallengesTicker::<Test>::get();
            let mut new_valid_submitters =
                BoundedBTreeSet::<ProviderIdFor<Test>, MaxSubmittersPerTickFor<Test>>::new();
            new_valid_submitters.try_insert(provider_id).unwrap();
            ValidProofSubmittersLastTicks::<Test>::insert(
                tick_when_proof_provided,
                new_valid_submitters,
            );

            // Check that the Provider was successfully added to the set.
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Advance a tick, executing `on_idle`, to check if the Provider is removed from the set.
            run_to_block(System::block_number() + 1);
            ProofsDealer::on_idle(System::block_number(), Weight::MAX);

            // Check that the set which had the Provider that submitted a valid proof has not been deleted.
            assert!(ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided).is_some());
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Check that the last deleted tick is still 0
            assert_eq!(LastDeletedTick::<Test>::get(), 0);

            // Advance enough ticks so that the Provider list which has the `provider_id` is set to be deleted.
            run_to_block(
                System::block_number()
                    + <TargetTicksStorageOfSubmittersFor<Test> as Get<u32>>::get() as u64,
            );

            // Call the `on_idle` hook, but without enough weight to delete the set.
            ProofsDealer::on_idle(System::block_number(), Weight::zero());

            // Check that the set which had the Provider that submitted a valid proof still exists and has the Provider
            assert!(ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided).is_some());
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Check that the last deleted tick is still zero.
            assert_eq!(LastDeletedTick::<Test>::get(), 0);
        });
    }

    #[test]
    fn on_idle_hook_deletes_multiple_old_ticks_if_enough_weight_is_remaining() {
        new_test_ext().execute_with(|| {
            // Go past genesis block so events get deposited.
            run_to_block(1); // Block number = 1

            // Register user as a Provider in Providers pallet.
            let provider_id = BlakeTwo256::hash(b"provider_id");
            pallet_storage_providers::AccountIdToBackupStorageProviderId::<Test>::insert(
                &1,
                provider_id,
            );
            pallet_storage_providers::BackupStorageProviders::<Test>::insert(
                &provider_id,
                pallet_storage_providers::types::BackupStorageProvider {
                    capacity: Default::default(),
                    capacity_used: Default::default(),
                    multiaddresses: Default::default(),
                    root: Default::default(),
                    last_capacity_change: Default::default(),
                    owner_account: 1u64,
                    payment_account: Default::default(),
                    reputation_weight:
                        <Test as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                },
            );

            // Add the Provider to the `ValidProofSubmittersLastTicks` storage map for the current tick and two ticks after that.
            let tick_when_first_proof_provided = ChallengesTicker::<Test>::get(); // Block number = 1
            let mut new_valid_submitters =
                BoundedBTreeSet::<ProviderIdFor<Test>, MaxSubmittersPerTickFor<Test>>::new();
            new_valid_submitters.try_insert(provider_id).unwrap();
            ValidProofSubmittersLastTicks::<Test>::insert(
                tick_when_first_proof_provided,
                new_valid_submitters.clone(),
            );
            let tick_when_second_proof_provided = tick_when_first_proof_provided + 2; // Block number = 1 + 2 = 3
            ValidProofSubmittersLastTicks::<Test>::insert(
                tick_when_second_proof_provided,
                new_valid_submitters,
            );

            // Check that the Provider was successfully added to the set in both ticks
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_first_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_second_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Advance a tick, executing `on_idle`, to check if the Provider is removed from any of the two sets.
            run_to_block(System::block_number() + 1); // Block number = 2
            ProofsDealer::on_idle(System::block_number(), Weight::MAX);

            // Check that the sets which had the Provider that submitted a valid proof have not been deleted.
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_first_proof_provided)
                    .is_some()
            );
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_first_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_second_proof_provided)
                    .is_some()
            );
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_second_proof_provided)
                    .unwrap()
                    .contains(&provider_id)
            );

            // Check that the last deleted tick is still 0
            assert_eq!(LastDeletedTick::<Test>::get(), 0);

            // Advance enough ticks so that both the Provider lists which have the `provider_id` are set to be deleted.
            run_to_block(
                System::block_number()
                    + <TargetTicksStorageOfSubmittersFor<Test> as Get<u32>>::get() as u64
                    + 3,
            ); // Block number = 2 + 3 + 3 = 8, 8 - 3 - 0 > 3 so both sets should be deleted.

            // Call the `on_idle` hook with enough weight to delete both sets.
            ProofsDealer::on_idle(System::block_number(), Weight::MAX);

            // Check that the sets which had the Provider that submitted a valid proof have been correctly deleted.
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_first_proof_provided)
                    .is_none()
            );
            assert!(
                ValidProofSubmittersLastTicks::<Test>::get(tick_when_second_proof_provided)
                    .is_none()
            );

            // Check that the last deleted tick is the one that was just deleted.
            assert_eq!(
                LastDeletedTick::<Test>::get(),
                System::block_number()
                    - <TargetTicksStorageOfSubmittersFor<Test> as Get<u32>>::get() as u64
            );
        });
    }
}

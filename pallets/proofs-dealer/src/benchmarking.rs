//! Benchmarking setup for pallet-proofs-dealer

use frame_benchmarking::v2::*;

#[benchmarks(
    where
        // Runtime `T` implements, `pallet_balances::Config` `pallet_storage_providers::Config` and this pallet's `Config`.
        T: pallet_balances::Config + pallet_storage_providers::Config + crate::Config,
        // The Storage Providers pallet is the `Providers` pallet that this pallet requires.
        T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
        // The `Balances` pallet is the `NativeBalance` pallet that this pallet requires.
        T: crate::Config<NativeBalance = pallet_balances::Pallet<T>>,
        // The `Balances` pallet is the `NativeBalance` pallet that `pallet_storage_providers::Config` requires.
        T: pallet_storage_providers::Config<NativeBalance = pallet_balances::Pallet<T>>,
        // The `Proof` inner type of the `ForestVerifier` trait is `CompactProof`.
        <T as crate::Config>::ForestVerifier: shp_traits::CommitmentVerifier<Proof = sp_trie::CompactProof>,
        // The `Proof` inner type of the `KeyVerifier` trait is `CompactProof`.
        <<T as crate::Config>::KeyVerifier as shp_traits::CommitmentVerifier>::Proof: From<sp_trie::CompactProof>,
        // The Storage Providers pallet's `HoldReason` type can be converted into the Native Balance's `Reason`.
        pallet_storage_providers::HoldReason: Into<<<T as pallet::Config>::NativeBalance as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
        // The Storage Providers `MerklePatriciaRoot` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<MerklePatriciaRoot = <T as frame_system::Config>::Hash>,
        // The Storage Providers `ProviderId` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
        // The events from this pallet can be converted into events of the Runtime.
        <T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>
)]
mod benchmarks {
    use alloc::{vec, vec::Vec};
    use codec::Decode;
    use frame_support::{
        assert_ok,
        dispatch::DispatchClass,
        traits::{
            fungible::{Mutate, MutateHold},
            Get, Hooks,
        },
    };
    use frame_system::{pallet_prelude::BlockNumberFor, BlockWeight, ConsumedWeight, RawOrigin};
    use pallet_storage_providers::types::ProviderIdFor;
    use scale_info::prelude::format;
    use shp_traits::{
        PaymentStreamsInterface, ProofsDealerInterface, ReadChallengeableProvidersInterface,
    };
    use sp_core::U256;
    use sp_runtime::{
        traits::{Hash, One, Zero},
        BoundedBTreeSet, BoundedVec,
    };
    use sp_weights::{Weight, WeightMeter};

    use super::*;
    use crate::{
        benchmark_proofs::{
            fetch_challenges, fetch_proof, get_provider_id, get_root, get_seed, get_user_account,
        },
        pallet,
        types::{
            ChallengeTicksToleranceFor, CheckpointChallengePeriodFor, CustomChallenge,
            MaxCustomChallengesPerBlockFor, MerkleTrieHashingFor, Proof, ProofSubmissionRecord,
            ProvidersPalletFor,
        },
        Call, ChallengesQueue, ChallengesTicker, ChallengesTickerPaused, Config, Event,
        LastCheckpointTick, LastDeletedTick, Pallet, PastBlocksStatus, PastBlocksWeight,
        ProviderToProofSubmissionRecord, SlashableProviders, TickToChallengesSeed,
        TickToCheckForSlashableProviders, TickToCheckpointChallenges, TickToProvidersDeadlines,
        ValidProofSubmittersLastTicks,
    };

    #[benchmark]
    fn challenge() -> Result<(), BenchmarkError> {
        // Setup initial conditions.
        let caller: T::AccountId = whitelisted_caller();
        let file_key = MerkleTrieHashingFor::<T>::hash(b"file_key");
        let user_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &caller,
            user_balance,
        ));

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::challenge(RawOrigin::Signed(caller.clone()), file_key);

        // Verify the challenge event was emitted.
        let expected_event = <T as pallet::Config>::RuntimeEvent::from(Event::NewChallenge {
            who: Some(caller),
            key_challenged: file_key,
        });
        frame_system::Pallet::<T>::assert_last_event(expected_event.into());

        // Verify that the challenge is in the queue.
        let challenges_queue = ChallengesQueue::<T>::get();
        assert_eq!(challenges_queue.len(), 1);
        assert_eq!(challenges_queue[0], file_key);

        Ok(())
    }

    /// > Assumptions:
    /// > - In the runtime configuration, [`T::MaxCustomChallengesPerBlock`] = [`T::RandomChallengesPerBlock`].
    /// > - For the purpose of this benchmark, [`T::MaxCustomChallengesPerBlock`] = 2 * [`T::RandomChallengesPerBlock`].
    /// > - This allows to "simulate" random challenges with checkpoint challenges, crafting them carefully to
    /// >   fall exactly where we need them, to benchmark a specific scenario.
    ///
    /// * Case: Up to {[`T::MaxCustomChallengesPerBlock`] * 2} file key proofs in proof.
    ///
    /// There are [`T::MaxCustomChallengesPerBlock`] random challenges, which can be responded with 1 to
    /// [`T::MaxCustomChallengesPerBlock`] * 2 file key proofs, depending on the Forest of the BSP and
    /// where the challenges fall within it. Additionally, in the worst case scenario for this amount
    /// of file key proofs, there can be [`T::MaxCustomChallengesPerBlock`] more file keys proven in the
    /// forest proof, that correspond to an exact match of a challenge with [`TrieRemoveMutation`].
    /// File keys that would be removed from the Forest, are not meant to also send a file key proof, and
    /// that is the case for an exact match of a custom challenge with [`TrieRemoveMutation`].
    #[benchmark]
    fn submit_proof_no_checkpoint_challenges_key_proofs(
        n: Linear<1, { T::MaxCustomChallengesPerBlock::get() }>,
    ) -> Result<(), BenchmarkError> {
        let file_key_proofs_count: u32 = n.into();
        let (caller, user, provider_id, challenged_tick, proof) =
            setup_submit_proof::<T>(file_key_proofs_count)?;

        // Payment stream should exist before calling the extrinsic, to account for the worst-case scenario
        // of having to update it.
        let maybe_payment_stream_amount_provided = <<T as pallet_storage_providers::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(
			&provider_id,
			&user,
		);
        assert!(maybe_payment_stream_amount_provided.is_some());
        let amount_provided_before = maybe_payment_stream_amount_provided.unwrap();

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::submit_proof(RawOrigin::Signed(caller.clone()), proof.clone(), None);

        // Check that the payment stream still exists but the amount provided has decreased.
        let maybe_payment_stream_amount_provided = <<T as pallet_storage_providers::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(
			&provider_id,
			&user,
		);
        assert!(maybe_payment_stream_amount_provided.is_some());
        let amount_provided_after = maybe_payment_stream_amount_provided.unwrap();
        assert!(amount_provided_after < amount_provided_before);

        // Check that the proof submission was successful.
        frame_system::Pallet::<T>::assert_last_event(
            Event::ProofAccepted {
                provider_id,
                proof,
                last_tick_proven: challenged_tick,
            }
            .into(),
        );

        Ok(())
    }

    /// > Assumptions:
    /// > - In the runtime configuration, [`T::MaxCustomChallengesPerBlock`] = [`T::RandomChallengesPerBlock`].
    /// > - For the purpose of this benchmark, [`T::MaxCustomChallengesPerBlock`] = 2 * [`T::RandomChallengesPerBlock`].
    /// > - This allows to "simulate" random challenges with checkpoint challenges, crafting them carefully to
    /// >   fall exactly where we need them, to benchmark a specific scenario.
    ///
    /// * Case: {[`T::MaxCustomChallengesPerBlock`] * 2 + 1} to {[`T::MaxCustomChallengesPerBlock`] * 4} file key proofs in proof.
    ///
    /// If there are more than {[`T::MaxCustomChallengesPerBlock`] * 2} file key proofs, then it means that
    /// some of those file key proofs are a response to checkpoint challenges, so it is now impossible to
    /// have [`T::MaxCustomChallengesPerBlock`] file keys proven to be removed from the Forest. For example,
    /// if {[`T::MaxCustomChallengesPerBlock`] = 10} and there are 21 file key proofs, then at least one of those
    /// file keys proven is a consequence of a checkpoint challenge either not falling exactly in an existing
    /// leaf, or not having a [`TrieRemoveMutation`]. So the worst case scenario for 21 file keys proven is
    /// another 9 file keys proven with a [`TrieRemoveMutation`]. For 22 file keys proven, the worst case scenario
    /// is also 9 file keys proven with a [`TrieRemoveMutation`]. For 23, 8 file keys proven with a [`TrieRemoveMutation`].
    /// For 24, also 8 file keys proven with a [`TrieRemoveMutation`]. It continues like this until with 40 file keys
    /// proven, the worst case scenario is 0 file keys proven with a [`TrieRemoveMutation`]. Basically, with 40 file
    /// keys proven, it means that there are 2 file keys proven for every random and checkpoint challenge, so no
    /// checkpoint challenge fell exactly in an existing leaf.
    #[benchmark]
    fn submit_proof_with_checkpoint_challenges_key_proofs(
        n: Linear<
            { T::MaxCustomChallengesPerBlock::get() + 1 },
            { T::MaxCustomChallengesPerBlock::get() * 2 },
        >,
    ) -> Result<(), BenchmarkError> {
        let file_key_proofs_count: u32 = n.into();
        let (caller, user, provider_id, challenged_tick, proof) =
            setup_submit_proof::<T>(file_key_proofs_count)?;

        // Payment stream should exist before calling the extrinsic, to account for the worst-case scenario
        // of having to update it.
        let maybe_payment_stream_amount_provided = <<T as pallet_storage_providers::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(
			&provider_id,
			&user,
		);
        assert!(maybe_payment_stream_amount_provided.is_some());
        let amount_provided_before = maybe_payment_stream_amount_provided.unwrap();

        // Call some extrinsic.
        #[extrinsic_call]
        Pallet::submit_proof(RawOrigin::Signed(caller.clone()), proof.clone(), None);

        // Check that the payment stream still exists but the amount provided has decreased if there was a trie remove mutation.
        let maybe_payment_stream_amount_provided = <<T as pallet_storage_providers::Config>::PaymentStreams as PaymentStreamsInterface>::get_dynamic_rate_payment_stream_amount_provided(
			&provider_id,
			&user,
		);
        assert!(maybe_payment_stream_amount_provided.is_some());
        let amount_provided_after = maybe_payment_stream_amount_provided.unwrap();
        if file_key_proofs_count < T::MaxCustomChallengesPerBlock::get() * 2 - 1 {
            assert!(amount_provided_after < amount_provided_before);
        } else {
            assert!(amount_provided_after == amount_provided_before);
        }

        // Check that the proof submission was successful.
        frame_system::Pallet::<T>::assert_last_event(
            Event::ProofAccepted {
                provider_id,
                proof,
                last_tick_proven: challenged_tick,
            }
            .into(),
        );

        Ok(())
    }

    /// > Assumptions:
    /// > - This is not a checkpoint challenge round. That function is benchmarked separately.
    ///
    /// * Case: There are [`T::MaxCustomChallengesPerBlock`] checkpoint challenges in the last checkpoint tick.
    #[benchmark]
    fn new_challenges_round(
        n: Linear<0, { T::MaxSlashableProvidersPerTick::get() }>,
    ) -> Result<(), BenchmarkError> {
        let slashable_providers_count: u32 = n.into();
        register_providers::<T>(slashable_providers_count)?;

        // Add the maximum number of checkpoint challenges to the last checkpoint tick.
        let last_checkpoint_tick = LastCheckpointTick::<T>::get();
        let checkpoint_challenges: BoundedVec<_, T::MaxCustomChallengesPerBlock> = vec![
            CustomChallenge {
                key: MerkleTrieHashingFor::<T>::hash(b"checkpoint_challenge"),
                should_remove_key: false,
            };
            T::MaxCustomChallengesPerBlock::get() as usize
        ]
        .try_into()
        .expect("Failed to convert checkpoint challenges to BoundedVec, when the size is known.");
        TickToCheckpointChallenges::<T>::insert(last_checkpoint_tick, checkpoint_challenges);

        // Check that there are no slashable Providers before the execution.
        let slashable_providers_count_before = SlashableProviders::<T>::iter().count();
        assert_eq!(slashable_providers_count_before, 0);

        // Get current tick before the execution.
        let current_tick_before = ChallengesTicker::<T>::get();

        let mut meter: WeightMeter = WeightMeter::new();
        #[block]
        {
            Pallet::<T>::do_new_challenges_round(&mut meter);
        }

        // Check that the slashable Providers are updated to be `n` after the execution.
        let slashable_providers_count_after = SlashableProviders::<T>::iter().count();
        assert_eq!(
            slashable_providers_count_after,
            slashable_providers_count as usize
        );

        // Check that the current tick is incremented by 1 after the execution.
        let current_tick_after = ChallengesTicker::<T>::get();
        assert_eq!(current_tick_after, current_tick_before + One::one());

        Ok(())
    }

    /// > Assumptions:
    /// > - Filling up the checkpoint challenges vector with custom or priority challenges is the same.
    /// >   Both represent the same execution complexity.
    /// > - We call the function with `current_tick` being the first checkpoint challenge period (could be any really).
    ///
    /// * Case: Considering up to the maximum number of checkpoint challenges per block.
    #[benchmark]
    fn new_checkpoint_challenge_round(
        n: Linear<0, { T::MaxCustomChallengesPerBlock::get() }>,
    ) -> Result<(), BenchmarkError> {
        let custom_challenges_in_queue: u32 = n.into();

        // Add the custom challenges to the queue.
        let custom_challenges: BoundedVec<_, T::ChallengesQueueLength> =
            vec![
                MerkleTrieHashingFor::<T>::hash(b"custom_challenge");
                custom_challenges_in_queue as usize
            ]
            .try_into()
            .expect("Failed to convert custom challenges to BoundedVec, when the size is known.");
        ChallengesQueue::<T>::set(custom_challenges);

        // Before executions there shouldn't be any checkpoint challenges in the checkpoint challenge tick.
        let checkpoint_challenge_tick = CheckpointChallengePeriodFor::<T>::get();
        let checkpoint_challenges_before =
            TickToCheckpointChallenges::<T>::get(checkpoint_challenge_tick);
        assert!(checkpoint_challenges_before.is_none());

        let mut meter: WeightMeter = WeightMeter::new();
        #[block]
        {
            Pallet::<T>::do_new_checkpoint_challenge_round(checkpoint_challenge_tick, &mut meter);
        }

        // Check that the challenges queue is empty after the execution.
        let challenges_queue_after = ChallengesQueue::<T>::get();
        assert!(challenges_queue_after.is_empty());

        // Check that now the checkpoint challenges are in the checkpoint challenge tick.
        let checkpoint_challenges_after =
            TickToCheckpointChallenges::<T>::get(checkpoint_challenge_tick)
                .expect("Checkpoint challenges should be Some() after the execution.");
        assert!(checkpoint_challenges_after.len() == custom_challenges_in_queue as usize);

        // Check that the checkpoint challenge tick was registered.
        let checkpoint_challenge_tick_after = LastCheckpointTick::<T>::get();
        assert_eq!(checkpoint_challenge_tick_after, checkpoint_challenge_tick);

        Ok(())
    }

    /// * Case:
    /// - The previous block is considered _not_ full, so it increments the count.
    /// - We're already past the first {[`T::BlockFullnessPeriod`] + 1} blocks, so that the part of this
    ///   function that clears old blocks from the `NotFullBlocksCount` is executed.
    /// - The oldest block to clear was a full block, so it doesn't decrement the count. Although the
    ///   intuition would be that in the worst case scenario, it should execute the code to decrement, if
    ///   it did decrement, there would be no chance in the number of blocks considered not full, so...
    /// - There is a change in the number of blocks considered not full, so there is a write to `NotFullBlocksCount`.
    /// - It is irrelevant if the chain is considered to be spammed or not, as both executions are of the same
    ///   complexity in terms of computation and storage read/writes. We're going to consider it going from spammed
    ///   to not spammed.
    #[benchmark]
    fn check_spamming_condition() -> Result<(), BenchmarkError> {
        // Set the block number to be the first block after going beyond `T::BlockFullnessPeriod` blocks.
        let block_fullness_period = T::BlockFullnessPeriod::get();
        frame_system::Pallet::<T>::set_block_number((block_fullness_period + 1).into());

        // Set tick number to be the same as the block number.
        ChallengesTicker::<T>::set(frame_system::Pallet::<T>::block_number());

        // Set the previous block weight to something below the threshold, so that it is considered not full.
        let weights = T::BlockWeights::get();
        let max_weight_for_class = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let prev_block_weight = max_weight_for_class
            - T::BlockFullnessHeadroom::get()
            - Weight::from_parts(One::one(), One::one());
        let prev_block = frame_system::Pallet::<T>::block_number() - One::one();
        PastBlocksWeight::<T>::insert(prev_block, prev_block_weight);

        // Set the weight of a block `BlockFullnessPeriod + 1` blocks before, to one such that it is considered full.
        let old_block = frame_system::Pallet::<T>::block_number()
            - T::BlockFullnessPeriod::get().into()
            - One::one();
        PastBlocksWeight::<T>::insert(old_block, max_weight_for_class);

        // Setting the `PastBlocksStatus` bounded vector to contain, as the first element, a block considered full, followed
        // by exactly the minimum required non-full blocks, and then all full blocks, so that when adding the new non-full block
        // the chain transitions from being considered spammed to not spammed.
        let mut past_blocks_status: BoundedVec<bool, T::BlockFullnessPeriod> = Default::default();
        past_blocks_status
            .try_push(true)
            .expect("Failed to push the initial block to past blocks status, it should fit.");
        let min_not_full_blocks_to_spam_block_type: U256 =
            Pallet::<T>::calculate_min_non_full_blocks_to_spam().into();
        let min_not_full_blocks_to_spam: u32 =
            min_not_full_blocks_to_spam_block_type.try_into().unwrap();
        for _ in 0..min_not_full_blocks_to_spam {
            past_blocks_status.try_push(false).expect(
                "Failed to push non full blocks to past blocks status when the size is known.",
            );
        }
        for _ in min_not_full_blocks_to_spam.saturating_add(1)..T::BlockFullnessPeriod::get() {
            past_blocks_status
                .try_push(true)
                .expect("Failed to push full blocks to past blocks status when the size is known.");
        }
        PastBlocksStatus::<T>::set(past_blocks_status);

        // Set the chain to be considered spammed.
        ChallengesTickerPaused::<T>::set(Some(()));

        let mut meter: WeightMeter = WeightMeter::new();
        #[block]
        {
            Pallet::<T>::do_check_spamming_condition(&mut meter);
        }

        // Check that blocks considered NOT full is incremented by 1.
        let past_blocks_status = PastBlocksStatus::<T>::get();
        let not_full_blocks_count = past_blocks_status
            .iter()
            .filter(|&&is_full| !is_full)
            .count() as u32;
        assert_eq!(
            not_full_blocks_count,
            min_not_full_blocks_to_spam.saturating_add(1)
        );

        // Check that chain is considered to be not spammed.
        assert!(ChallengesTickerPaused::<T>::get().is_none());

        Ok(())
    }

    /// * Case:
    /// - The number of ticks to remove is 0. This means that the loop never executes, but that loop execution
    ///   is benchmarked in the `trim_valid_proof_submitters_last_ticks_loop` benchmark.
    #[benchmark]
    fn trim_valid_proof_submitters_last_ticks_constant_execution() -> Result<(), BenchmarkError> {
        // Set the current tick to be `TargetTicksStorageOfSubmitters`.
        // `LastDeletedTick` would be 0 by default, so this way, when `target_ticks_storage_of_submitters`
        // subtracts from `current_tick`, it will be 0, and the loop will never execute.
        let target_ticks_storage_of_submitters: u32 = T::TargetTicksStorageOfSubmitters::get();
        frame_system::Pallet::<T>::set_block_number(target_ticks_storage_of_submitters.into());

        // Pass the whole available weight for the Normal dispatch class.
        let weights = T::BlockWeights::get();
        let max_weight_for_class = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);

        #[block]
        {
            Pallet::<T>::do_trim_valid_proof_submitters_last_ticks(
                frame_system::Pallet::<T>::block_number(),
                max_weight_for_class,
            );
        }

        // Check that `LastDeletedTick` is still 0.
        assert_eq!(
            LastDeletedTick::<T>::get(),
            0u32.into(),
            "LastDeletedTick should still be 0."
        );

        Ok(())
    }

    /// * Case:
    /// - The tick to remove has a maximum size [`BoundedBTreeSet`], where the maximum size is
    ///   [`T::MaxSubmittersPerTick`].
    #[benchmark]
    fn trim_valid_proof_submitters_last_ticks_loop() -> Result<(), BenchmarkError> {
        // Generate a vector of proof submitters of max size.
        let mut proof_submitters =
            BoundedBTreeSet::<ProviderIdFor<T>, T::MaxSubmittersPerTick>::new();
        let max_submitters_per_tick: u32 = T::MaxSubmittersPerTick::get();
        for i in 0..max_submitters_per_tick {
            let provider_id = <T as frame_system::Config>::Hashing::hash(
                format!("provider_id_{:?}", i).as_bytes(),
            );
            proof_submitters.try_insert(provider_id).expect("Failed to insert provider ID. This shouldn't happen because we're iterating until the maximum size.");
        }

        // Add the vector of proof submitters to the `ValidProofSubmittersLastTicks` storage element.
        let tick: BlockNumberFor<T> = 181222u32.into();
        ValidProofSubmittersLastTicks::<T>::insert(tick, proof_submitters);

        #[block]
        {
            Pallet::<T>::remove_proof_submitters_for_tick(tick);
        }

        // Check that the `ValidProofSubmittersLastTicks` storage element has been updated.
        assert_eq!(
            ValidProofSubmittersLastTicks::<T>::get(tick),
            None,
            "The `ValidProofSubmittersLastTicks` storage element should have been removed."
        );

        // Check that the `LastDeletedTick` storage element has been updated.
        assert_eq!(
            LastDeletedTick::<T>::get(),
            tick,
            "The `LastDeletedTick` storage element should have been updated."
        );

        Ok(())
    }

    /// * Case:
    /// - Current block is greater than `BlockFullnessPeriod`, so the part of this function that removes
    ///   old blocks from the `NotFullBlocksCount` is executed.
    #[benchmark]
    fn on_finalize() -> Result<(), BenchmarkError> {
        // Set current block to be one block beyond `BlockFullnessPeriod`, so that the removal of old
        // blocks' weight is executed.
        let current_block: BlockNumberFor<T> = (T::BlockFullnessPeriod::get() + 1).into();
        frame_system::Pallet::<T>::set_block_number(current_block);

        // Set the weight used in block `current_block` - (`BlockFullnessPeriod` + 1).
        let weights = T::BlockWeights::get();
        let max_weight_for_class = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);
        let block_to_remove_weight =
            current_block - T::BlockFullnessPeriod::get().into() - One::one();
        PastBlocksWeight::<T>::insert(block_to_remove_weight, max_weight_for_class);

        // Set the current block's weight.
        let current_block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
            DispatchClass::Normal => max_weight_for_class,
            DispatchClass::Operational => Zero::zero(),
            DispatchClass::Mandatory => Zero::zero(),
        });
        BlockWeight::<T>::set(current_block_weight);

        // Check that there is no value in the `PastBlocksWeight` StorageMap for block `current_block`.
        assert!(PastBlocksWeight::<T>::get(current_block).is_none());

        #[block]
        {
            Pallet::<T>::on_finalize(current_block);
        }

        // Check that the current block's weight is registered in the `PastBlocksWeight` StorageMap.
        assert!(PastBlocksWeight::<T>::get(current_block).is_some());
        assert_eq!(
            PastBlocksWeight::<T>::get(current_block).unwrap(),
            max_weight_for_class
        );

        // Check that block `current_block` - (`BlockFullnessPeriod` + 1) is removed from the `PastBlocksWeight` StorageMap.
        assert!(PastBlocksWeight::<T>::get(block_to_remove_weight).is_none());

        Ok(())
    }

    /// * Case:
    /// - Provider is already initialised, so its current deadline has to be removed first.
    #[benchmark]
    fn force_initialise_challenge_cycle() -> Result<(), BenchmarkError> {
        // Setup initial conditions.
        let provider_id = <T as frame_system::Config>::Hashing::hash(
            format!("provider_id_{:?}", 0u32).as_bytes(),
        );
        register_providers::<T>(1u32)?;

        // Force initialise challenge cycle once, so that it is already initialised.
        <Pallet<T> as ProofsDealerInterface>::initialise_challenge_cycle(&provider_id)?;

        // Check that the last tick the Provider submitted a proof for so far is the current block.
        let ProofSubmissionRecord {
            last_tick_proven, ..
        } = ProviderToProofSubmissionRecord::<T>::get(provider_id)
            .expect("Provider should have a last tick it submitted a proof for.");
        assert_eq!(last_tick_proven, frame_system::Pallet::<T>::block_number());

        // Advance a block so that we initialise it in a different block.
        let current_block = frame_system::Pallet::<T>::block_number();
        frame_system::Pallet::<T>::set_block_number(current_block + One::one());

        #[extrinsic_call]
        Pallet::<T>::force_initialise_challenge_cycle(RawOrigin::Root, provider_id);

        // Check that the last tick the Provider submitted a proof for, is the new current block.
        let ProofSubmissionRecord {
            last_tick_proven, ..
        } = ProviderToProofSubmissionRecord::<T>::get(provider_id)
            .expect("Provider should have a last tick it submitted a proof for.");
        assert_eq!(last_tick_proven, current_block);

        Ok(())
    }

    /// * Case:
    /// - Any case has the same complexity, so we'll just go from unpaused to paused.
    #[benchmark]
    fn set_paused() -> Result<(), BenchmarkError> {
        // Setup initial conditions.
        let is_ticker_paused = ChallengesTickerPaused::<T>::get();

        // Ticker should be unpaused.
        assert!(is_ticker_paused.is_none());

        #[extrinsic_call]
        Pallet::<T>::set_paused(RawOrigin::Root, true);

        // Ticker should be paused.
        let is_ticker_paused = ChallengesTickerPaused::<T>::get();
        assert!(is_ticker_paused.is_some());

        Ok(())
    }

    impl_benchmark_test_suite! {
            Pallet,
            crate::mock::new_test_ext(),
            crate::mock::Test,
    }

    fn setup_submit_proof<T>(n: u32) -> Result<(T::AccountId, T::AccountId, ProviderIdFor<T>, BlockNumberFor<T>, Proof<T>), BenchmarkError>
    where
    // Runtime `T` implements, `pallet_balances::Config` `pallet_storage_providers::Config` and this pallet's `Config`.
        T: pallet_balances::Config + pallet_storage_providers::Config + crate::Config,
    // The Storage Providers pallet is the `Providers` pallet that this pallet requires.
        T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that this pallet requires.
        T: crate::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that `pallet_storage_providers::Config` requires.
        T: pallet_storage_providers::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Proof` inner type of the `ForestVerifier` trait is `CompactProof`.
        <T as crate::Config>::ForestVerifier: shp_traits::CommitmentVerifier<Proof = sp_trie::CompactProof>,
    // The `Proof` inner type of the `KeyVerifier` trait is `CompactProof`.
        <<T as crate::Config>::KeyVerifier as shp_traits::CommitmentVerifier>::Proof: From<sp_trie::CompactProof>,
    // The Storage Providers pallet's `HoldReason` type can be converted into the Native Balance's `Reason`.
        pallet_storage_providers::HoldReason: Into<<<T as pallet::Config>::NativeBalance as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
    // The Storage Providers `MerklePatriciaRoot` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<MerklePatriciaRoot = <T as frame_system::Config>::Hash>,
    // The Storage Providers `ProviderId` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
    {
        // Setup initial conditions.
        let caller: T::AccountId = whitelisted_caller();
        let provider_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &caller,
            provider_balance,
        ));

        // Set up an account with some balance.
        let user_as_bytes: [u8; 32] = get_user_account().clone().try_into().unwrap();
        let user_account: T::AccountId = T::AccountId::decode(&mut &user_as_bytes[..]).unwrap();
        let user_balance = match 1_000_000_000_000_000u128.try_into() {
            Ok(balance) => balance,
            Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
        };
        assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
            &user_account,
            user_balance,
        ));

        // Register caller as a Provider in Providers pallet.
        let encoded_provider_id = get_provider_id();
        let provider_id =
            <T as frame_system::Config>::Hash::decode(&mut encoded_provider_id.as_ref())
                .expect("Failed to decode provider ID from bytes.");
        pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::insert(
            &caller,
            provider_id,
        );
        let used_capacity: u32 = 1024 * 1024 * 1024; // One gigabyte
        let total_capacity: u32 = used_capacity * 2; // Two gigabytes
        pallet_storage_providers::BackupStorageProviders::<T>::insert(
            &provider_id,
            pallet_storage_providers::types::BackupStorageProvider {
                capacity: total_capacity.into(),
                capacity_used: used_capacity.into(),
                multiaddresses: Default::default(),
                root: Default::default(),
                last_capacity_change: Default::default(),
                owner_account: caller.clone(),
                payment_account: caller.clone(),
                reputation_weight:
                    <T as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                sign_up_block: Default::default(),
            },
        );
        pallet_storage_providers::UsedBspsCapacity::<T>::set(used_capacity.into());

        // Hold some of the Provider's balance so it simulates it having a stake.
        let provider_stake = provider_balance / 100u32.into();
        assert_ok!(<T as crate::Config>::NativeBalance::hold(
            &pallet_storage_providers::HoldReason::StorageProviderDeposit.into(),
            &caller,
            provider_stake,
        ));

        // Set Provider's root to be the one that matches the proofs that will be submitted.
        let encoded_root = get_root();
        let root = <T as frame_system::Config>::Hash::decode(&mut encoded_root.as_ref())
            .expect("Root should be decodable as it is a hash");
        pallet_storage_providers::BackupStorageProviders::<T>::mutate(&provider_id, |provider| {
            provider.as_mut().expect("Provider should exist").root = root;
        });

        // Set Provider's last submitted proof block.
        let current_tick = ChallengesTicker::<T>::get();
        let last_tick_provider_submitted_proof = current_tick;
        let challenge_period = crate::Pallet::<T>::stake_to_challenge_period(provider_stake);
        let proof_record = ProofSubmissionRecord {
            last_tick_proven: last_tick_provider_submitted_proof,
            next_tick_to_submit_proof_for: last_tick_provider_submitted_proof + challenge_period,
        };
        ProviderToProofSubmissionRecord::<T>::insert(&provider_id, proof_record);

        // Set Provider's deadline for submitting a proof.
        // It is the sum of this Provider's challenge period and the `ChallengesTicksTolerance`.
        let providers_stake =
            <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_stake(provider_id)
                .unwrap();
        let challenge_period = crate::Pallet::<T>::stake_to_challenge_period(providers_stake);
        let challenge_ticks_tolerance: BlockNumberFor<T> = ChallengeTicksToleranceFor::<T>::get();
        let challenge_period_plus_tolerance = challenge_period + challenge_ticks_tolerance;
        let prev_deadline = current_tick + challenge_period_plus_tolerance;
        TickToProvidersDeadlines::<T>::insert(prev_deadline, provider_id, ());

        // Advance to the next challenge the Provider should listen to.
        let providers_stake =
            <ProvidersPalletFor<T> as ReadChallengeableProvidersInterface>::get_stake(provider_id)
                .unwrap();
        let challenge_period = crate::Pallet::<T>::stake_to_challenge_period(providers_stake);
        let current_block = frame_system::Pallet::<T>::block_number();
        let challenge_block = current_block + challenge_period;
        frame_system::Pallet::<T>::set_block_number(challenge_block);
        // Advance less than `ChallengeTicksTolerance` blocks.
        let challenge_ticks_tolerance: BlockNumberFor<T> = ChallengeTicksToleranceFor::<T>::get();
        let current_block = frame_system::Pallet::<T>::block_number();
        frame_system::Pallet::<T>::set_block_number(
            current_block + challenge_ticks_tolerance - 1u32.into(),
        );

        // Manually set the current tick.
        ChallengesTicker::<T>::set(frame_system::Pallet::<T>::block_number());

        // Set the seed for the challenge block.
        let encoded_seed = get_seed();
        let seed = <T as frame_system::Config>::Hash::decode(&mut encoded_seed.as_ref())
            .expect("Seed should be decodable as it is a hash");
        TickToChallengesSeed::<T>::insert(challenge_block, seed);

        // Calculate the custom challenges to respond to, so that we can generate a proof for each.
        let custom_challenges = generate_challenges::<T>(n);

        // Set the custom challenges in the last checkpoint challenge tick,
        // which in this case is going to be 1.
        let last_checkpoint_tick = 1u32.into();
        LastCheckpointTick::<T>::set(last_checkpoint_tick);
        TickToCheckpointChallenges::<T>::insert(last_checkpoint_tick, custom_challenges.clone());

        // Fetch proof for the challenged keys.
        let encoded_proof = fetch_proof(n);
        let proof =
            <Proof<T>>::decode(&mut encoded_proof.as_ref()).expect("Proof should be decodable");

        // Check that the proof has the expected number of file key proofs.
        assert_eq!(proof.key_proofs.len() as u32, n);

        // Create a dynamic-rate payment stream between the user and the provider.
        let amount_provided: u32 = 1024 * 1024 * 1024; // One gigabyte
        let payment_stream_creation_result = <<T as pallet_storage_providers::Config>::PaymentStreams as PaymentStreamsInterface>::create_dynamic_rate_payment_stream(&provider_id,
			&user_account,
			&amount_provided.into());
        assert_ok!(payment_stream_creation_result);

        Ok((caller, user_account, provider_id, challenge_block, proof))
    }

    fn generate_challenges<T: Config>(
        n: u32,
    ) -> BoundedVec<CustomChallenge<T>, MaxCustomChallengesPerBlockFor<T>> {
        let encoded_challenges = fetch_challenges(n);
        let mut custom_challenges = Vec::new();
        for encoded_challenge in encoded_challenges {
            let typed_challenge =
                <T as crate::Config>::MerkleTrieHash::decode(&mut encoded_challenge.as_ref())
                    .expect("Challenge key should be decodable as it is a hash");

            let custom_challenge = CustomChallenge {
                key: typed_challenge,
                should_remove_key: true,
            };
            custom_challenges.push(custom_challenge);
        }
        BoundedVec::try_from(custom_challenges).expect("Length of custom challenges should be less than or equal to MaxCustomChallengesPerBlockFor")
    }

    fn register_providers<T>(n: u32) -> Result<(), BenchmarkError>
    where
    // Runtime `T` implements, `pallet_balances::Config` `pallet_storage_providers::Config` and this pallet's `Config`.
        T: pallet_balances::Config + pallet_storage_providers::Config + crate::Config,
    // The Storage Providers pallet is the `Providers` pallet that this pallet requires.
        T: crate::Config<ProvidersPallet = pallet_storage_providers::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that this pallet requires.
        T: crate::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Balances` pallet is the `NativeBalance` pallet that `pallet_storage_providers::Config` requires.
        T: pallet_storage_providers::Config<NativeBalance = pallet_balances::Pallet<T>>,
    // The `Proof` inner type of the `ForestVerifier` trait is `CompactProof`.
        <T as crate::Config>::ForestVerifier: shp_traits::CommitmentVerifier<Proof = sp_trie::CompactProof>,
    // The `Proof` inner type of the `KeyVerifier` trait is `CompactProof`.
        <<T as crate::Config>::KeyVerifier as shp_traits::CommitmentVerifier>::Proof: From<sp_trie::CompactProof>,
    // The Storage Providers pallet's `HoldReason` type can be converted into the Native Balance's `Reason`.
        pallet_storage_providers::HoldReason: Into<<<T as pallet::Config>::NativeBalance as frame_support::traits::fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason>,
    // The Storage Providers `MerklePatriciaRoot` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<MerklePatriciaRoot = <T as frame_system::Config>::Hash>,
    // The Storage Providers `ProviderId` type is the same as `frame_system::Hash`.
        T: pallet_storage_providers::Config<ProviderId = <T as frame_system::Config>::Hash>,
    {
        let tick_to_check_for_slashable_providers = TickToCheckForSlashableProviders::<T>::get();
        for i in 0..n {
            // Setup initial conditions.
            let provider_account: T::AccountId = account("provider_account", i as u32, i);
            let provider_balance = match 1_000_000_000_000_000u128.try_into() {
                Ok(balance) => balance,
                Err(_) => return Err(BenchmarkError::Stop("Balance conversion failed.")),
            };
            assert_ok!(<T as crate::Config>::NativeBalance::mint_into(
                &provider_account,
                provider_balance,
            ));

            // Register caller as a Provider in Providers pallet.
            let provider_id = <T as frame_system::Config>::Hashing::hash(
                format!("provider_id_{:?}", i).as_bytes(),
            );
            pallet_storage_providers::AccountIdToBackupStorageProviderId::<T>::insert(
                &provider_account,
                provider_id,
            );
            pallet_storage_providers::BackupStorageProviders::<T>::insert(
                &provider_id,
                pallet_storage_providers::types::BackupStorageProvider {
                    capacity: Default::default(),
                    capacity_used: Default::default(),
                    multiaddresses: Default::default(),
                    root: Default::default(),
                    last_capacity_change: Default::default(),
                    owner_account: provider_account.clone(),
                    payment_account: provider_account.clone(),
                    reputation_weight:
                        <T as pallet_storage_providers::Config>::StartingReputationWeight::get(),
                    sign_up_block: Default::default(),
                },
            );

            // Hold some of the Provider's balance so it simulates it having a stake.
            assert_ok!(<T as crate::Config>::NativeBalance::hold(
                &pallet_storage_providers::HoldReason::StorageProviderDeposit.into(),
                &provider_account,
                provider_balance / 100u32.into(),
            ));

            // Add Provider to the next deadline to check.
            TickToProvidersDeadlines::<T>::insert(
                tick_to_check_for_slashable_providers,
                provider_id,
                (),
            );
        }

        Ok(())
    }
}

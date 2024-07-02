use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{fungible::Mutate, tokens::Preservation, Get, Randomness},
    weights::WeightMeter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_proofs_dealer_runtime_api::{
    GetCheckpointChallengesError, GetLastTickProviderSubmittedProofError,
};
use shp_traits::{
    CommitmentVerifier, ProofsDealerInterface, ProvidersInterface, TrieMutation,
    TrieProofDeltaApplier, TrieRemoveMutation,
};
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedSub, Convert, Hash, Zero},
    ArithmeticError, BoundedVec, DispatchError, SaturatedConversion, Saturating,
};
use sp_std::{
    collections::{btree_set::BTreeSet, vec_deque::VecDeque},
    vec::Vec,
};

use crate::{
    pallet,
    types::{
        AccountIdFor, BalanceFor, BalancePalletFor, ChallengeHistoryLengthFor, ChallengesFeeFor,
        ChallengesQueueLengthFor, CheckpointChallengePeriodFor, ForestRootFor, ForestVerifierFor,
        ForestVerifierProofFor, KeyFor, KeyVerifierFor, KeyVerifierProofFor,
        MaxCustomChallengesPerBlockFor, Proof, ProviderIdFor, ProvidersPalletFor,
        RandomChallengesPerBlockFor, RandomnessOutputFor, RandomnessProviderFor,
        StakeToChallengePeriodFor, TreasuryAccountFor,
    },
    ChallengeTickToChallengedProviders, ChallengesQueue, ChallengesTicker, Error, Event,
    LastCheckpointTick, LastTickProviderSubmittedProofFor, Pallet, PriorityChallengesQueue,
    SlashableProviders, TickToChallengesSeed, TickToCheckpointChallenges,
};

macro_rules! expect_or_err {
    // Handle Option type
    ($optional:expr, $error_msg:expr, $error_type:path) => {{
        match $optional {
            Some(value) => value,
            None => {
                #[cfg(test)]
                unreachable!($error_msg);

                #[allow(unreachable_code)]
                {
                    Err($error_type)?
                }
            }
        }
    }};
    // Handle boolean type
    ($condition:expr, $error_msg:expr, $error_type:path, bool) => {{
        if !$condition {
            #[cfg(test)]
            unreachable!($error_msg);

            #[allow(unreachable_code)]
            {
                Err($error_type)?
            }
        }
    }};
}

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    /// Add custom challenge to ChallengesQueue.
    ///
    /// Check if sender is a registered Provider. If it is not, charge a fee for the challenge.
    /// This is to prevent spamming the network with challenges. If the challenge is already queued,
    /// just return. Otherwise, add the challenge to the queue.
    ///
    /// Failures:
    /// - `FeeChargeFailed`: If the fee transfer to the treasury account fails.
    /// - `ChallengesQueueOverflow`: If the challenges queue is full.
    pub fn do_challenge(who: &AccountIdFor<T>, key: &KeyFor<T>) -> DispatchResult {
        // Check if sender is a registered Provider.
        if ProvidersPalletFor::<T>::get_provider_id(who.clone()).is_none() {
            // Charge a fee for the challenge if it is not.
            BalancePalletFor::<T>::transfer(
                &who,
                &TreasuryAccountFor::<T>::get(),
                ChallengesFeeFor::<T>::get(),
                Preservation::Expendable,
            )
            .map_err(|_| Error::<T>::FeeChargeFailed)?;
        }

        // Enqueue challenge.
        Self::enqueue_challenge(key)
    }

    /// Submit proof.
    ///
    /// For a given `submitter`, verify the `proof` submitted. The proof is verified by checking
    /// the forest proof and each key proof.
    /// Relies on the `ProvidersPallet` to get the root for the submitter, the last tick for which
    /// the submitter submitted a proof and the stake for the submitter. With that information, it
    /// computes the next tick for which the submitter should be submitting a proof. It then gets
    /// the seed for that tick and generates the challenges from the seed. It also checks if there
    /// has been a Checkpoint Challenge block in between the last tick proven and the current tick.
    /// If there has been, the Provider should have included proofs for the challenges in that block.
    /// It then verifies the forest proof and each key proof, using the `ForestVerifier` and `KeyVerifier`.
    pub fn do_submit_proof(submitter: &ProviderIdFor<T>, proof: &Proof<T>) -> DispatchResult {
        let forest_proof = &proof.forest_proof;
        let key_proofs = &proof.key_proofs;

        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(*submitter),
            Error::<T>::NotProvider
        );

        // Check that key_proofs is not empty.
        ensure!(!key_proofs.is_empty(), Error::<T>::EmptyKeyProofs);

        // The check for whether forest_proof and each key_proof is not empty is handled by the corresponding
        // verifiers for each. We do not preemptively check for this here, since the `CommitmentVerifier::Proof`
        // type is not required to have an `is_empty` method.

        // Get root for submitter.
        // If a submitter is a registered Provider, it must have a root, so this shouldn't happen.
        // However, since the implementation of that is not up to this pallet, we need to check.
        let root = ProvidersPalletFor::<T>::get_root(*submitter)
            .ok_or(Error::<T>::ProviderRootNotFound)?;

        // Check that the root is not the default root.
        // A default root means that the Provider is not providing any service yet, so he shouldn't be
        // submitting any proofs.
        ensure!(root != Self::default_forest_root(), Error::<T>::ZeroRoot);

        // Get last tick for which the submitter submitted a proof.
        let last_tick_proven = match LastTickProviderSubmittedProofFor::<T>::get(submitter.clone())
        {
            Some(tick) => tick,
            None => return Err(Error::<T>::NoRecordOfLastSubmittedProof.into()),
        };

        // Get stake for submitter.
        // If a submitter is a registered Provider, it must have a stake, so this shouldn't happen.
        // However, since the implementation of that is not up to this pallet, we need to check.
        let stake = ProvidersPalletFor::<T>::get_stake(*submitter)
            .ok_or(Error::<T>::ProviderStakeNotFound)?;

        // Check that the stake is non-zero.
        ensure!(stake > BalanceFor::<T>::zero(), Error::<T>::ZeroStake);

        // Compute the next tick for which the submitter should be submitting a proof.
        let challenges_tick = last_tick_proven
            .checked_add(&Self::stake_to_challenge_period(stake))
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Check that the challenges tick is lower than the current tick.
        ensure!(
            challenges_tick < ChallengesTicker::<T>::get(),
            Error::<T>::ChallengesTickNotReached
        );

        // Check that the challenges tick is greater than current tick minus `ChallengeHistoryLength`,
        // i.e. that the challenges tick is within the ticks this pallet keeps track of.
        expect_or_err!(
            challenges_tick
                > ChallengesTicker::<T>::get()
                    .saturating_sub(ChallengeHistoryLengthFor::<T>::get()),
            "Challenges tick is too old, beyond the history this pallet keeps track of. This should not be possible.",
            Error::<T>::ChallengesTickTooOld,
            bool
        );

        // Check that the submitter is not submitting the proof to late, i.e. that the challenges tick
        // is not greater or equal than `challenges_tick` + `T::ChallengeTicksTolerance::get()`.
        // This should never happen, as the `ChallengeTickToChallengedProviders` StorageMap is
        // cleaned up every block. Therefore if a Provider reached this deadline, it should have been
        // slashed, and its next challenge tick pushed forwards.
        let challenges_tick_deadline = challenges_tick
            .checked_add(&T::ChallengeTicksTolerance::get())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        expect_or_err!(
            challenges_tick_deadline > <frame_system::Pallet<T>>::block_number(),
            "Challenges tick is too late, the proof should be submitted at most `T::ChallengeTicksTolerance::get()` ticks after the challenges tick.",
            Error::<T>::ChallengesTickTooLate,
            bool
        );

        // Get seed for challenges tick.
        let seed = expect_or_err!(
            TickToChallengesSeed::<T>::get(challenges_tick),
            "Seed for challenges tick not found, when checked it should be within history.",
            Error::<T>::SeedNotFound
        );

        // Generate forest challenges from seed.
        let mut challenges = Self::generate_challenges_from_seed(
            seed,
            submitter,
            RandomChallengesPerBlockFor::<T>::get(),
        );

        // Check if there's been a Checkpoint Challenge tick in between the last tick proven and
        // the tick for which the proof is being submitted. If there has been, the Provider should
        // have included proofs for those checkpoint challenges.
        let last_checkpoint_tick = LastCheckpointTick::<T>::get();
        let mut checkpoint_challenges = None;
        if last_tick_proven <= last_checkpoint_tick && last_checkpoint_tick < challenges_tick {
            // Add challenges from the Checkpoint Challenge block.
            checkpoint_challenges =
                Some(expect_or_err!(
                    TickToCheckpointChallenges::<T>::get(last_checkpoint_tick),
                    "Checkpoint challenges not found, when dereferencing in last registered checkpoint challenge block.",
                    Error::<T>::CheckpointChallengesNotFound
                ));

            if let Some(ref checkpoint_challenges) = checkpoint_challenges {
                challenges.extend(checkpoint_challenges.iter().map(|(key, _)| key));
            }
        }

        // Verify forest proof.
        let mut forest_keys_proven =
            ForestVerifierFor::<T>::verify_proof(&root, &challenges, forest_proof)
                .map_err(|_| Error::<T>::ForestProofVerificationFailed)?;

        // Apply the delta to the Forest root for all mutations that are in checkpoint challenges.
        if let Some(challenges) = checkpoint_challenges {
            // Aggregate all mutations to apply to the Forest root.
            let mutations: Vec<_> = challenges
                .iter()
                .filter_map(|(key, mutation)| match mutation {
                    Some(mutation) if forest_keys_proven.contains(key) => Some((*key, mutation)),
                    Some(_) | None => None,
                })
                .collect();

            if !mutations.is_empty() {
                let new_root = mutations.iter().try_fold(root, |acc_root, mutation| {
                    // Remove the key from the list of `forest_keys_proven` to avoid having to verify the key proof.
                    forest_keys_proven.remove(&mutation.0);

                    // TODO: Reduce the storage used by the Provider with some interface exposed by the Providers pallet.

                    <T::ForestVerifier as TrieProofDeltaApplier<T::MerkleTrieHashing>>::apply_delta(
                        &acc_root,
                        &[(mutation.0, mutation.1.clone().into())],
                        forest_proof,
                    )
                    .map(|(_, new_root)| new_root)
                    .map_err(|_| Error::<T>::FailedToApplyDelta)
                })?;

                // Update root of Provider after all mutations have been applied to the Forest.
                <T::ProvidersPallet as shp_traits::ProvidersInterface>::update_root(
                    *submitter, new_root,
                )?;
            }
        };

        // Verify each key proof.
        for key_proven in forest_keys_proven {
            // Check that there is a key proof for each key proven.
            let key_proof = key_proofs
                .get(&key_proven)
                .ok_or(Error::<T>::KeyProofNotFound)?;

            // Generate the challenges for the key.
            let challenges =
                Self::generate_challenges_from_seed(seed, submitter, key_proof.challenge_count);

            // Verify key proof.
            KeyVerifierFor::<T>::verify_proof(&key_proven, &challenges, &key_proof.proof)
                .map_err(|_| Error::<T>::KeyProofVerificationFailed)?;
        }

        // Update `LastTickProviderSubmittedProofFor` to the challenge tick the provider has just
        // submitted a proof for.
        LastTickProviderSubmittedProofFor::<T>::set(submitter.clone(), Some(challenges_tick));

        // Remove the submitter from its current deadline registered in `ChallengeTickToChallengedProviders`.
        ChallengeTickToChallengedProviders::<T>::remove(challenges_tick_deadline, submitter);

        // Calculate the next tick for which the submitter should be submitting a proof.
        let next_challenges_tick = challenges_tick
            .checked_add(&Self::stake_to_challenge_period(stake))
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Add tolerance to `next_challenges_tick` to know when is the next deadline for submitting a
        // proof, for this provider.
        let next_challenges_tick_deadline = next_challenges_tick
            .checked_add(&T::ChallengeTicksTolerance::get())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Add this Provider to the `ChallengeTickToChallengedProviders` StorageMap, with its new deadline.
        ChallengeTickToChallengedProviders::<T>::set(
            next_challenges_tick_deadline,
            submitter,
            Some(()),
        );

        // TODO: Register this block as the last block that this provider can charge for in the payment stream.

        Ok(())
    }

    /// Generate a new round of challenges, be it random or checkpoint.
    ///
    /// Random challenges are automatically generated based on some external source of
    /// randomness, and are added to `TickToChallengesSeed`, for this block's number.
    ///
    /// It also takes care of including the challenges from the `ChallengesQueue` and
    /// `PriorityChallengesQueue`. These custom challenges are only included in "checkpoint"
    /// blocks
    ///
    /// Additionally, it takes care of checking if there are Providers that have
    /// failed to submit a proof, and should have submitted one by this block. It does so
    /// by checking the `ChallengeTickToChallengedProviders` StorageMap. If a Provider is found
    /// to have failed to submit a proof, it is subject to slashing.
    ///
    /// Finally, it cleans up:
    /// - The `TickToChallengesSeed` StorageMap, removing entries older than `ChallengeHistoryLength`.
    /// - The `TickToCheckpointChallenges` StorageMap, removing the previous checkpoint challenge block.
    /// - The `ChallengeTickToChallengedProviders` StorageMap, removing entries for the current challenges tick.
    pub fn do_new_challenges_round(n: BlockNumberFor<T>, weight: &mut WeightMeter) {
        // Increment the challenges ticker.
        let mut challenges_ticker = ChallengesTicker::<T>::get();
        challenges_ticker.saturating_inc();
        ChallengesTicker::<T>::set(challenges_ticker);
        weight.consume(T::DbWeight::get().reads_writes(1, 1));

        // Store random seed for this tick.
        let (seed, _) = RandomnessProviderFor::<T>::random(challenges_ticker.encode().as_ref());
        TickToChallengesSeed::<T>::set(challenges_ticker, Some(seed));
        weight.consume(T::DbWeight::get().reads_writes(0, 1));

        // Remove the oldest challenge seed stored, to clean up the storage.
        let tick_to_remove = challenges_ticker.checked_sub(&ChallengeHistoryLengthFor::<T>::get());
        if let Some(tick_to_remove) = tick_to_remove {
            TickToChallengesSeed::<T>::remove(tick_to_remove);
            weight.consume(T::DbWeight::get().reads_writes(0, 1));
        }

        // Emit new challenge seed event.
        Self::deposit_event(Event::NewChallengeSeed {
            challenges_ticker,
            seed,
        });

        // Calculate if this is a checkpoint challenge round.
        let last_checkpoint_tick = LastCheckpointTick::<T>::get();
        // This hook does not return an error, and it cannot fail, that's why we use `saturating_add`.
        let next_checkpoint_tick =
            last_checkpoint_tick.saturating_add(T::CheckpointChallengePeriod::get());
        if challenges_ticker == next_checkpoint_tick {
            // This is a checkpoint challenge round, so we also generate new checkpoint challenges.
            Self::do_new_checkpoint_challenge_round(challenges_ticker, weight);
        }
        weight.consume(T::DbWeight::get().reads_writes(2, 0));

        // If there are providers left in `ChallengeTickToChallengedProviders` for this tick,
        // they are marked as slashable.
        let mut slashable_providers =
            ChallengeTickToChallengedProviders::<T>::drain_prefix(challenges_ticker);
        while let Some((provider, _)) = slashable_providers.next() {
            // One read for every provider in the prefix, and one write as we're consuming and deleting the entry.
            weight.consume(T::DbWeight::get().reads_writes(1, 1));

            // Mark this provider as slashable.
            SlashableProviders::<T>::set(provider, Some(()));
            weight.consume(T::DbWeight::get().reads_writes(0, 1));

            // Get the stake for this Provider, to know its challenge period.
            // If a submitter is a registered Provider, it must have a stake, so there shouldn't be an error.
            let stake = match ProvidersPalletFor::<T>::get_stake(provider) {
                Some(stake) => stake,
                // But to avoid panics, in the odd case of a Provider not being registered, we
                // arbitrarily set the stake to be that which would result in `CheckpointChallengePeriod` ticks of challenge period.
                None => {
                    weight.consume(T::DbWeight::get().reads_writes(1, 0));
                    let checkpoint_challenge_period =
                        CheckpointChallengePeriodFor::<T>::get().saturated_into::<u32>();
                    StakeToChallengePeriodFor::<T>::get() * checkpoint_challenge_period.into()
                }
            };
            weight.consume(T::DbWeight::get().reads_writes(1, 0));

            // Calculate the next challenge deadline for this Provider.
            let next_challenge_deadline =
                challenges_ticker.saturating_add(Self::stake_to_challenge_period(stake));

            // Calculate the tick for which the Provider should have submitted a proof.
            let last_tick_proven =
                challenges_ticker.saturating_sub(T::ChallengeTicksTolerance::get());
            weight.consume(T::DbWeight::get().reads_writes(1, 0));

            // Update this Provider's next challenge deadline.
            ChallengeTickToChallengedProviders::<T>::set(
                next_challenge_deadline,
                provider,
                Some(()),
            );
            weight.consume(T::DbWeight::get().reads_writes(0, 1));

            // Update this Provider's last tick it submitted a proof for.
            // It didn't actually submit a proof for this tick, but we need it to properly calculate next time
            // it should submit a proof.
            LastTickProviderSubmittedProofFor::<T>::set(provider, Some(last_tick_proven));
            weight.consume(T::DbWeight::get().reads_writes(0, 1));

            // Emit slashable provider event.
            Self::deposit_event(Event::SlashableProvider { provider });
        }
    }

    /// Generate new checkpoint challenges for a given block.
    ///
    /// Fills up a new vector of checkpoint challenges with challenges in the `PriorityChallengesQueue`,
    /// and the `ChallengesQueue` if there is space left.
    ///
    /// Cleans up the `TickToCheckpointChallenges` StorageMap, removing the previous checkpoint challenge block.
    fn do_new_checkpoint_challenge_round(
        current_tick: BlockNumberFor<T>,
        weight: &mut WeightMeter,
    ) {
        let mut new_checkpoint_challenges: BoundedVec<
            (KeyFor<T>, Option<TrieRemoveMutation>),
            MaxCustomChallengesPerBlockFor<T>,
        > = BoundedVec::new();

        // Fill up this round's checkpoint challenges with challenges in the `PriorityChallengesQueue`.
        // It gets filled up until the max number of custom challenges for a block is reached, or until
        // there are no more challenges in the `PriorityChallengesQueue`.
        let original_priority_challenges_queue = PriorityChallengesQueue::<T>::get();
        let mut priority_challenges_queue =
            VecDeque::from(original_priority_challenges_queue.to_vec());
        weight.consume(T::DbWeight::get().reads_writes(1, 0));

        while !new_checkpoint_challenges.is_full() && !priority_challenges_queue.is_empty() {
            let challenge = match priority_challenges_queue.pop_front() {
                Some(challenge) => challenge,
                // This should not happen, as we check that priority_challenges_queue is not empty
                // in the while condition above, but we add this to be safe.
                None => break,
            };

            if new_checkpoint_challenges.try_push(challenge).is_err() {
                // This should not happen, as we check that new_checkpoint_challenges is not full
                // in the while condition above, but we add this to be safe.
                break;
            }
        }

        // Convert priority_challenges_queue back to a bounded vector.
        let new_priority_challenges_queue: BoundedVec<
            (KeyFor<T>, Option<TrieRemoveMutation>),
            ChallengesQueueLengthFor<T>,
        > = match Vec::from(priority_challenges_queue).try_into() {
            Ok(new_priority_challenges_queue) => new_priority_challenges_queue,
            // This should not happen, as `priority_challenges_queue` would now have equal or less elements
            // than what was originally in `PriorityChallengesQueue`, but we add this to be safe.
            // In here we care that no priority challenges are ever lost.
            Err(_) => original_priority_challenges_queue,
        };

        // Reset the priority challenges queue with the leftovers.
        PriorityChallengesQueue::<T>::set(new_priority_challenges_queue);
        weight.consume(T::DbWeight::get().reads_writes(0, 1));

        // Fill up this round's checkpoint challenges with challenges in the `ChallengesQueue`.
        // It gets filled up until the max number of custom challenges for a block is reached, or until
        // there are no more challenges in the `ChallengesQueue`.
        let mut challenges_queue = VecDeque::from(ChallengesQueue::<T>::get().to_vec());
        weight.consume(T::DbWeight::get().reads_writes(1, 0));

        while !new_checkpoint_challenges.is_full() && !challenges_queue.is_empty() {
            let challenge = match challenges_queue.pop_front() {
                Some(challenge) => challenge,
                // This should not happen, as we check that challenges_queue is not empty
                // in the while condition above, but we add this to be safe.
                None => break,
            };

            if new_checkpoint_challenges
                .try_push((challenge, None))
                .is_err()
            {
                // This should not happen, as we check that new_checkpoint_challenges is not full
                // in the while condition above, but we add this to be safe.
                break;
            }
        }

        // Convert challenges_queue back to a bounded vector.
        let new_challenges_queue: BoundedVec<KeyFor<T>, ChallengesQueueLengthFor<T>> =
            match Vec::from(challenges_queue).try_into() {
                Ok(new_challenges_queue) => new_challenges_queue,
                // This should not happen, as `challenges_queue` would now have equal or less elements
                // than what was originally in `ChallengesQueue`, but we add this to be safe.
                // Here we accept if some challenges are lost, since they're not priority challenges.
                Err(_) => BoundedVec::new(),
            };

        // Reset the challenges queue with the leftovers.
        ChallengesQueue::<T>::set(new_challenges_queue);
        weight.consume(T::DbWeight::get().reads_writes(0, 1));

        // Store the new checkpoint challenges.
        TickToCheckpointChallenges::<T>::set(current_tick, Some(new_checkpoint_challenges.clone()));
        weight.consume(T::DbWeight::get().reads_writes(0, 1));

        // Remove the last checkpoint challenge from storage to clean up.
        let last_checkpoint_tick = LastCheckpointTick::<T>::get();
        TickToCheckpointChallenges::<T>::remove(last_checkpoint_tick);
        weight.consume(T::DbWeight::get().reads_writes(1, 1));

        // Set this tick as the last checkpoint tick.
        LastCheckpointTick::<T>::set(current_tick);
        weight.consume(T::DbWeight::get().reads_writes(0, 1));

        // Emit new checkpoint challenge event.
        Self::deposit_event(Event::NewCheckpointChallenge {
            challenges_ticker: current_tick,
            challenges: new_checkpoint_challenges,
        });
    }

    /// Convert stake to challenge period.
    ///
    /// Stake is divided by `StakeToChallengePeriod` to get the number of blocks in between challenges
    /// for a Provider. The result is then converted to `BlockNumber` type.
    pub(crate) fn stake_to_challenge_period(stake: BalanceFor<T>) -> BlockNumberFor<T> {
        let block_period = stake
            .checked_div(&StakeToChallengePeriodFor::<T>::get())
            .unwrap_or(1u32.into());

        T::StakeToBlockNumber::convert(block_period)
    }

    /// Add challenge to ChallengesQueue.
    ///
    /// Check if challenge is already queued. If it is, just return. Otherwise, add the challenge
    /// to the queue.
    fn enqueue_challenge(key: &KeyFor<T>) -> DispatchResult {
        // Get challenges queue from storage.
        let mut challenges_queue = ChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        if challenges_queue.contains(&key) {
            return Ok(());
        }

        // Add challenge to queue.
        challenges_queue
            .try_push(*key)
            .map_err(|_| Error::<T>::ChallengesQueueOverflow)?;

        // Set challenges queue in storage.
        ChallengesQueue::<T>::put(challenges_queue);

        Ok(())
    }

    /// Add challenge to `PriorityChallengesQueue`.
    ///
    /// Check if challenge is already queued. If it is, just return. Otherwise, add the challenge
    /// to the queue.
    fn enqueue_challenge_with_priority(
        key: &KeyFor<T>,
        mutation: Option<TrieRemoveMutation>,
    ) -> DispatchResult {
        // Get priority challenges queue from storage.
        let mut priority_challenges_queue = PriorityChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        if priority_challenges_queue.contains(&(*key, mutation.clone())) {
            return Ok(());
        }

        // Add challenge to queue.
        priority_challenges_queue
            .try_push((*key, mutation))
            .map_err(|_| Error::<T>::PriorityChallengesQueueOverflow)?;

        // Set priority challenges queue in storage.
        PriorityChallengesQueue::<T>::put(priority_challenges_queue);

        Ok(())
    }

    /// Generate challenges from seed.
    ///
    /// Generate a number of challenges from a seed and a Provider's ID.
    /// Challenges are generated by hashing the seed, the Provider's ID and an index.
    pub(crate) fn generate_challenges_from_seed(
        seed: RandomnessOutputFor<T>,
        provider_id: &ProviderIdFor<T>,
        count: u32,
    ) -> Vec<T::MerkleTrieHash> {
        let mut challenges = Vec::new();

        for i in 0..count {
            // Each challenge is generated by hashing the seed, the provider's ID and the index.
            let challenge = T::MerkleTrieHashing::hash(
                &[
                    seed.as_ref(),
                    provider_id.encode().as_ref(),
                    i.encode().as_ref(),
                ]
                .concat(),
            );

            challenges.push(challenge.into());
        }

        challenges
    }

    /// Returns the default forest root.
    fn default_forest_root() -> ForestRootFor<T> {
        // TODO: Check that this returns the root for an empty forest and change if necessary.
        ForestRootFor::<T>::default()
    }
}

impl<T: pallet::Config> ProofsDealerInterface for Pallet<T> {
    type ProviderId = ProviderIdFor<T>;
    type ForestProof = ForestVerifierProofFor<T>;
    type KeyProof = KeyVerifierProofFor<T>;
    type MerkleHash = T::MerkleTrieHash;
    type MerkleHashing = T::MerkleTrieHashing;

    fn verify_forest_proof(
        who: &Self::ProviderId,
        challenges: &[Self::MerkleHash],
        proof: &Self::ForestProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError> {
        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(*who),
            Error::<T>::NotProvider
        );

        // Get root for submitter.
        // If a submitter is a registered Provider, it must have a root.
        let root =
            ProvidersPalletFor::<T>::get_root(*who).ok_or(Error::<T>::ProviderRootNotFound)?;

        // Verify forest proof.
        ForestVerifierFor::<T>::verify_proof(&root, challenges, proof)
            .map_err(|_| Error::<T>::ForestProofVerificationFailed.into())
    }

    fn verify_key_proof(
        key: &Self::MerkleHash,
        challenges: &[Self::MerkleHash],
        proof: &Self::KeyProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError> {
        // Verify key proof.
        KeyVerifierFor::<T>::verify_proof(key, &challenges, proof)
            .map_err(|_| Error::<T>::KeyProofVerificationFailed.into())
    }

    fn challenge(key_challenged: &Self::MerkleHash) -> DispatchResult {
        Self::enqueue_challenge(key_challenged)
    }

    fn challenge_with_priority(
        key_challenged: &Self::MerkleHash,
        mutation: Option<TrieRemoveMutation>,
    ) -> DispatchResult {
        Self::enqueue_challenge_with_priority(key_challenged, mutation)
    }

    fn apply_delta(
        commitment: &Self::MerkleHash,
        mutations: &[(Self::MerkleHash, TrieMutation)],
        proof: &Self::ForestProof,
    ) -> Result<Self::MerkleHash, DispatchError> {
        Ok(
            <T::ForestVerifier as TrieProofDeltaApplier<T::MerkleTrieHashing>>::apply_delta(
                commitment, mutations, proof,
            )
            .map_err(|_| Error::<T>::FailedToApplyDelta)?
            .1,
        )
    }

    // TODO: Add `initialise_provider` method to be called by the FileSystem pallet.
    // TODO: when a file is first uploaded to a BSP or bucket.
    // TODO: It would set `LastTickProviderSubmittedProofFor` to the current tick and
    // TODO: the deadline for submitting a proof in `ChallengeTickToChallengedProviders`.
}

/// Runtime API implementation for the ProofsDealer pallet.
impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn get_last_tick_provider_submitted_proof(
        who: &ProviderIdFor<T>,
    ) -> Result<BlockNumberFor<T>, GetLastTickProviderSubmittedProofError> {
        // Check if submitter is a registered Provider.
        if !ProvidersPalletFor::<T>::is_provider(*who) {
            return Err(GetLastTickProviderSubmittedProofError::ProviderNotRegistered);
        }

        LastTickProviderSubmittedProofFor::<T>::get(who)
            .ok_or(GetLastTickProviderSubmittedProofError::ProviderNeverSubmittedProof)
    }

    pub fn get_last_checkpoint_challenge_tick() -> BlockNumberFor<T> {
        LastCheckpointTick::<T>::get()
    }

    pub fn get_checkpoint_challenges(
        tick: BlockNumberFor<T>,
    ) -> Result<Vec<(KeyFor<T>, Option<TrieRemoveMutation>)>, GetCheckpointChallengesError> {
        // Check that the tick is smaller than the last checkpoint tick.
        if LastCheckpointTick::<T>::get() < tick {
            return Err(GetCheckpointChallengesError::TickGreaterThanLastCheckpointTick);
        }

        let checkpoint_challenges = TickToCheckpointChallenges::<T>::get(tick)
            .ok_or(GetCheckpointChallengesError::NoCheckpointChallengesInTick)?;

        Ok(checkpoint_challenges.into())
    }
}

// TODO: Remove this attribute once the file is implemented.
#![allow(dead_code)]
#![allow(unused_variables)]

use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{fungible::Mutate, tokens::Preservation, Get, Randomness},
    weights::WeightMeter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use shp_traits::{CommitmentVerifier, ProofsDealerInterface, ProvidersInterface};
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedSub, Convert, Hash, Zero},
    ArithmeticError, BoundedVec, DispatchError, Saturating,
};
use sp_std::{collections::vec_deque::VecDeque, vec::Vec};

use crate::{
    pallet,
    types::{
        AccountIdFor, BalanceFor, BalancePalletFor, ChallengeHistoryLengthFor, ChallengesFeeFor,
        ChallengesQueueLengthFor, ForestRootFor, ForestVerifierFor, ForestVerifierProofFor, KeyFor,
        KeyVerifierFor, KeyVerifierProofFor, MaxCustomChallengesPerBlockFor, Proof, ProviderFor,
        ProvidersPalletFor, RandomChallengesPerBlockFor, RandomnessOutputFor,
        RandomnessProviderFor, StakeToChallengePeriodFor, TreasuryAccountFor,
    },
    ChallengesQueue, ChallengesTicker, Error, Event, LastCheckpointTick,
    LastTickProviderSubmittedProofFor, Pallet, PriorityChallengesQueue, TickToChallengesSeed,
    TickToCheckpointChallenges,
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
    pub fn do_submit_proof(submitter: &ProviderFor<T>, proof: &Proof<T>) -> DispatchResult {
        let forest_proof = &proof.forest_proof;
        let key_proofs = &proof.key_proofs;

        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(submitter.clone()),
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
        let root = ProvidersPalletFor::<T>::get_root(submitter.clone())
            .ok_or(Error::<T>::ProviderRootNotFound)?;

        // Check if root is non-zero.
        // A zero root means that the Provider is not providing any service yet, so he shouldn't be
        // submitting any proofs.
        ensure!(root == Self::default_forest_root(), Error::<T>::ZeroRoot);

        // Get last tick for which the submitter submitted a proof.
        let last_tick_proven = match LastTickProviderSubmittedProofFor::<T>::get(submitter.clone())
        {
            Some(tick) => tick,
            None => return Err(Error::<T>::NoRecordOfLastSubmittedProof.into()),
        };

        // Get stake for submitter.
        // If a submitter is a registered Provider, it must have a stake, so this shouldn't happen.
        // However, since the implementation of that is not up to this pallet, we need to check.
        let stake = ProvidersPalletFor::<T>::get_stake(submitter.clone())
            .ok_or(Error::<T>::ProviderStakeNotFound)?;

        // Check that the stake is non-zero.
        ensure!(stake > BalanceFor::<T>::zero(), Error::<T>::ZeroStake);

        // Compute the next tick for which the submitter should be submitting a proof.
        let challenges_tick = last_tick_proven
            .checked_add(&Self::stake_to_challenge_period(stake)?)
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
        if last_tick_proven <= last_checkpoint_tick && last_checkpoint_tick < challenges_tick {
            // Add challenges from the Checkpoint Challenge block.
            let checkpoint_challenges =
                expect_or_err!(
                    TickToCheckpointChallenges::<T>::get(last_checkpoint_tick),
                    "Checkpoint challenges not found, when dereferencing in last registered checkpoint challenge block.",
                    Error::<T>::CheckpointChallengesNotFound
                );
            challenges.extend(checkpoint_challenges);
        }

        // Verify forest proof.
        let forest_keys_proven =
            ForestVerifierFor::<T>::verify_proof(&root, &challenges, forest_proof)
                .map_err(|_| Error::<T>::ForestProofVerificationFailed)?;

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

        // TODO: Update LastTickProviderSubmittedProofFor.
        // TODO: Update ChallengeTickToChallengedProviders.

        Ok(())
    }

    // TODO: Document.
    pub fn do_new_challenges_round(n: BlockNumberFor<T>, weight: &mut WeightMeter) {
        // TODO: Benchmark computational weight cost of this hook.
        // TODO: Specify read-write weight of this hook.

        // TODO: Consider checkpoint challenge rounds.
        // TODO: Check if providers failed to submit a proof.
        // TODO: Clean up `TickToChallengesSeed`, `TickToCheckpointChallenges` and `ChallengeTickToChallengedProviders` storage.

        // Increment the challenges ticker.
        let mut challenges_ticker = ChallengesTicker::<T>::get();
        challenges_ticker.saturating_inc();
        ChallengesTicker::<T>::set(challenges_ticker);

        // Store random seed for this tick.
        let (seed, _) = RandomnessProviderFor::<T>::random(challenges_ticker.encode().as_ref());
        TickToChallengesSeed::<T>::insert(challenges_ticker, seed);

        // Remove the oldest challenge seed stored, to clean up the storage.
        let tick_to_remove = challenges_ticker.checked_sub(&ChallengeHistoryLengthFor::<T>::get());
        if let Some(tick_to_remove) = tick_to_remove {
            TickToChallengesSeed::<T>::remove(tick_to_remove);
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
            last_checkpoint_tick.saturating_add(T::CheckpointChallengePeriod::get().into());
        if challenges_ticker == next_checkpoint_tick {
            // This is a checkpoint challenge round, so we also generate new checkpoint challenges.
            Self::do_new_checkpoint_challenge_round(challenges_ticker, weight);
        }
    }

    // TODO: Document.
    fn do_new_checkpoint_challenge_round(
        current_tick: BlockNumberFor<T>,
        weight: &mut WeightMeter,
    ) {
        // TODO: Specify read-write weight of this hook.

        let mut new_checkpoint_challenges: BoundedVec<
            KeyFor<T>,
            MaxCustomChallengesPerBlockFor<T>,
        > = BoundedVec::new();

        // Fill up this round's checkpoint challenges with challenges in the `PriorityChallengesQueue`.
        // It gets filled up until the max number of custom challenges for a block is reached, or until
        // there are no more challenges in the `PriorityChallengesQueue`.
        let original_priority_challenges_queue = PriorityChallengesQueue::<T>::get();
        let mut priority_challenges_queue =
            VecDeque::from(original_priority_challenges_queue.to_vec());

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
        let new_priority_challenges_queue: BoundedVec<KeyFor<T>, ChallengesQueueLengthFor<T>> =
            match Vec::from(priority_challenges_queue).try_into() {
                Ok(new_priority_challenges_queue) => new_priority_challenges_queue,
                // This should not happen, as `priority_challenges_queue` would now have equal or less elements
                // than what was originally in `PriorityChallengesQueue`, but we add this to be safe.
                // In here we care that no priority challenges are ever lost.
                Err(_) => original_priority_challenges_queue,
            };

        // Reset the priority challenges queue with the leftovers.
        PriorityChallengesQueue::<T>::set(new_priority_challenges_queue);

        // Fill up this round's checkpoint challenges with challenges in the `ChallengesQueue`.
        // It gets filled up until the max number of custom challenges for a block is reached, or until
        // there are no more challenges in the `ChallengesQueue`.
        let mut challenges_queue = VecDeque::from(ChallengesQueue::<T>::get().to_vec());

        while !new_checkpoint_challenges.is_full() && !challenges_queue.is_empty() {
            let challenge = match challenges_queue.pop_front() {
                Some(challenge) => challenge,
                // This should not happen, as we check that challenges_queue is not empty
                // in the while condition above, but we add this to be safe.
                None => break,
            };

            if new_checkpoint_challenges.try_push(challenge).is_err() {
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

        // Store the new checkpoint challenges.
        TickToCheckpointChallenges::<T>::set(current_tick, Some(new_checkpoint_challenges.clone()));

        // Remove the last checkpoint challenge from storage to clean up.
        let last_checkpoint_tick = LastCheckpointTick::<T>::get();
        TickToCheckpointChallenges::<T>::remove(last_checkpoint_tick);

        // Set this tick as the last checkpoint tick.
        LastCheckpointTick::<T>::set(current_tick);

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
    fn stake_to_challenge_period(stake: BalanceFor<T>) -> Result<BlockNumberFor<T>, DispatchError> {
        let block_period = stake
            .checked_div(&StakeToChallengePeriodFor::<T>::get())
            .unwrap_or(1u32.into());

        Ok(T::StakeToBlockNumber::convert(block_period))
    }

    /// Add challenge to ChallengesQueue.
    ///
    /// Check if challenge is already queued. If it is, just return. Otherwise, add the challenge
    /// to the queue.
    fn enqueue_challenge(key: &KeyFor<T>) -> DispatchResult {
        // Get challenges queue from storage.
        let mut challenges_queue = ChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        if challenges_queue.contains(key) {
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

    /// Add challenge to PriorityChallengesQueue.
    ///
    /// Check if challenge is already queued. If it is, just return. Otherwise, add the challenge
    /// to the queue.
    fn enqueue_challenge_with_priority(key: &KeyFor<T>) -> DispatchResult {
        // Get priority challenges queue from storage.
        let mut priority_challenges_queue = PriorityChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        if priority_challenges_queue.contains(key) {
            return Ok(());
        }

        // Add challenge to queue.
        priority_challenges_queue
            .try_push(*key)
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
        provider_id: &ProviderFor<T>,
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
    type ProviderId = ProviderFor<T>;
    type ForestProof = ForestVerifierProofFor<T>;
    type KeyProof = KeyVerifierProofFor<T>;
    type MerkleHash = T::MerkleTrieHash;
    type MerkleHashing = T::MerkleTrieHashing;

    fn verify_forest_proof(
        who: &Self::ProviderId,
        challenges: &[Self::MerkleHash],
        proof: &Self::ForestProof,
    ) -> Result<Vec<Self::MerkleHash>, DispatchError> {
        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(who.clone()),
            Error::<T>::NotProvider
        );

        // Get root for submitter.
        // If a submitter is a registered Provider, it must have a root.
        let root = ProvidersPalletFor::<T>::get_root(who.clone())
            .ok_or(Error::<T>::ProviderRootNotFound)?;

        // Verify forest proof.
        ForestVerifierFor::<T>::verify_proof(&root, challenges, proof)
            .map_err(|_| Error::<T>::ForestProofVerificationFailed.into())
    }

    fn verify_key_proof(
        key: &Self::MerkleHash,
        challenges: &[Self::MerkleHash],
        proof: &Self::KeyProof,
    ) -> Result<Vec<Self::MerkleHash>, DispatchError> {
        // Verify key proof.
        KeyVerifierFor::<T>::verify_proof(key, &challenges, proof)
            .map_err(|_| Error::<T>::KeyProofVerificationFailed.into())
    }

    fn challenge(key_challenged: &Self::MerkleHash) -> DispatchResult {
        Self::enqueue_challenge(key_challenged)
    }

    fn challenge_with_priority(key_challenged: &Self::MerkleHash) -> DispatchResult {
        Self::enqueue_challenge_with_priority(key_challenged)
    }
}

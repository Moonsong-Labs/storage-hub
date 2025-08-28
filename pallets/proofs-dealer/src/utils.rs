use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::{DispatchClass, DispatchResult},
    traits::{fungible::Mutate, tokens::Preservation, Get, Randomness},
    weights::{Weight, WeightMeter},
    BoundedBTreeSet,
};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, GetCheckpointChallengesError,
    GetNextDeadlineTickError, GetProofSubmissionRecordError,
};
use shp_traits::{
    CommitmentVerifier, MutateChallengeableProvidersInterface, ProofSubmittersInterface,
    ProofsDealerInterface, ReadChallengeableProvidersInterface, StorageHubTickGetter, TrieMutation,
    TrieProofDeltaApplier,
};
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, CheckedSub, Convert, Hash, One, Zero},
    ArithmeticError, BoundedVec, DispatchError, SaturatedConversion, Saturating,
};
use sp_std::{
    collections::{btree_set::BTreeSet, vec_deque::VecDeque},
    vec::Vec,
};

use crate::{
    pallet,
    types::{
        AccountIdFor, BalanceFor, BalancePalletFor, ChallengeHistoryLengthFor,
        ChallengeTicksToleranceFor, ChallengesFeeFor, ChallengesQueueLengthFor,
        CheckpointChallengePeriodFor, CustomChallenge, ForestVerifierFor, ForestVerifierProofFor,
        KeyFor, KeyVerifierFor, KeyVerifierProofFor, MaxCustomChallengesPerBlockFor,
        MaxSlashableProvidersPerTickFor, MaxSubmittersPerTickFor, MinChallengePeriodFor,
        PriorityChallengesFeeFor, Proof, ProofSubmissionRecord, ProviderIdFor, ProvidersPalletFor,
        RandomChallengesPerBlockFor, RandomnessOutputFor, RandomnessProviderFor,
        StakeToChallengePeriodFor, TargetTicksStorageOfSubmittersFor, TreasuryAccountFor,
    },
    weights::WeightInfo,
    ChallengesQueue, ChallengesTicker, ChallengesTickerPaused, Error, Event, LastCheckpointTick,
    LastDeletedTick, Pallet, PastBlocksStatus, PastBlocksWeight, PriorityChallengesQueue,
    ProviderToProofSubmissionRecord, SlashableProviders, TickToChallengesSeed,
    TickToCheckForSlashableProviders, TickToCheckpointChallenges, TickToProvidersDeadlines,
    ValidProofSubmittersLastTicks,
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
    // Handle Result type
    ($result:expr, $error_msg:expr, $error_type:path, result) => {{
        match $result {
            Ok(value) => value,
            Err(_) => {
                #[cfg(test)]
                unreachable!($error_msg);

                #[allow(unreachable_code)]
                {
                    Err($error_type)?
                }
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
    /// Charges a fee for the challenge.
    /// This is to prevent spamming the network with challenges. If the challenge is already queued,
    /// just return. Otherwise, add the challenge to the queue.
    ///
    /// Arguments:
    /// - `who`: Origin of the challenge request. If `Some(AccountId)`, represents a signed origin that will be charged the fee.
    ///   If `None`, represents a Root or None origin that is exempt from fees (None only allowed if the CustomOrigin permits it).
    ///
    /// Failures:
    /// - `FeeChargeFailed`: If the fee transfer to the treasury account fails.
    /// - `ChallengesQueueOverflow`: If the challenges queue is full.
    pub fn do_challenge(who: &Option<AccountIdFor<T>>, key: &KeyFor<T>) -> DispatchResult {
        // Charge a fee for the challenge only if origin is not root and fee is > 0
        if let Some(who) = who {
            let fee = ChallengesFeeFor::<T>::get();
            if !fee.is_zero() {
                BalancePalletFor::<T>::transfer(
                    &who,
                    &TreasuryAccountFor::<T>::get(),
                    fee,
                    Preservation::Expendable,
                )
                .map_err(|_| Error::<T>::FeeChargeFailed)?;
            }
        };
        // Enqueue challenge.
        Self::enqueue_challenge(key)
    }

    /// Add priority challenge to PriorityChallengesQueue.
    ///
    /// Charges a fee for the priority challenge.
    /// This is to prevent spamming the network with priority challenges. If the challenge is already queued,
    /// just return. Otherwise, add the challenge to the queue.
    ///
    /// Arguments:
    /// - `who`: Origin of the priority challenge request. If `Some(AccountId)`, represents a signed origin that will be charged the fee.
    ///   If `None`, represents a Root or None origin that is exempt from fees (None only allowed if the CustomOrigin permits it).
    ///
    /// Failures:
    /// - `FeeChargeFailed`: If the fee transfer to the treasury account fails.
    /// - `PriorityChallengesQueueOverflow`: If the priority challenges queue is full.
    pub fn do_priority_challenge(
        who: &Option<AccountIdFor<T>>,
        key: &KeyFor<T>,
        should_remove_key: bool,
    ) -> DispatchResult {
        // Charge a fee for the priority challenge only if origin is not root and fee is > 0
        if let Some(who) = who {
            let fee = PriorityChallengesFeeFor::<T>::get();
            if !fee.is_zero() {
                BalancePalletFor::<T>::transfer(
                    &who,
                    &TreasuryAccountFor::<T>::get(),
                    fee,
                    Preservation::Expendable,
                )
                .map_err(|_| Error::<T>::FeeChargeFailed)?;
            }
        };
        // Enqueue priority challenge.
        Self::enqueue_challenge_with_priority(key, should_remove_key)
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
    pub fn do_submit_proof(
        submitter: &ProviderIdFor<T>,
        proof: &Proof<T>,
    ) -> Result<BlockNumberFor<T>, DispatchError> {
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
        ensure!(
            root != ProvidersPalletFor::<T>::get_default_root(),
            Error::<T>::ZeroRoot
        );

        // Get last tick for which the submitter submitted a proof, as well as the tick for which
        // now it should be submitting the proof.
        let ProofSubmissionRecord {
            last_tick_proven,
            next_tick_to_submit_proof_for: challenges_tick,
        } = match ProviderToProofSubmissionRecord::<T>::get(*submitter) {
            Some(record) => record,
            None => return Err(Error::<T>::NoRecordOfLastSubmittedProof.into()),
        };

        // Get stake for submitter.
        // If a submitter is a registered Provider, it must have a stake, so this shouldn't happen.
        // However, since the implementation of that is not up to this pallet, we need to check.
        let stake = ProvidersPalletFor::<T>::get_stake(*submitter)
            .ok_or(Error::<T>::ProviderStakeNotFound)?;

        // Check that the stake is non-zero.
        ensure!(stake > BalanceFor::<T>::zero(), Error::<T>::ZeroStake);

        // Check that the challenges tick is lower than the current tick.
        let current_tick = ChallengesTicker::<T>::get();
        ensure!(
            challenges_tick < current_tick,
            Error::<T>::ChallengesTickNotReached
        );

        // Check that the challenges tick is greater than current tick minus `ChallengeHistoryLength`,
        // i.e. that the challenges tick is within the ticks this pallet keeps track of.
        expect_or_err!(
            challenges_tick
                > current_tick
                    .saturating_sub(ChallengeHistoryLengthFor::<T>::get()),
            "Challenges tick is too old, beyond the history this pallet keeps track of. This should not be possible.",
            Error::<T>::ChallengesTickTooOld,
            bool
        );

        // Check that the submitter is not submitting the proof too late, i.e. that the challenges tick
        // is not greater or equal than `challenges_tick` + `T::ChallengeTicksTolerance::get()`.
        // This should never happen, as the `TickToProvidersDeadlines` StorageMap is
        // cleaned up every block. Therefore, if a Provider reached this deadline, it should have been
        // slashed, and its next challenge tick pushed forwards.
        let challenges_tick_deadline = challenges_tick
            .checked_add(&T::ChallengeTicksTolerance::get())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        expect_or_err!(
            challenges_tick_deadline > current_tick,
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

        if last_tick_proven < last_checkpoint_tick && last_checkpoint_tick <= challenges_tick {
            // Add challenges from the Checkpoint Challenge block.
            checkpoint_challenges =
                Some(expect_or_err!(
                    TickToCheckpointChallenges::<T>::get(last_checkpoint_tick),
                    "Checkpoint challenges not found, when dereferencing in last registered checkpoint challenge block.",
                    Error::<T>::CheckpointChallengesNotFound
                ));

            if let Some(ref checkpoint_challenges) = checkpoint_challenges {
                challenges.extend(
                    checkpoint_challenges
                        .iter()
                        .map(|custom_challenge| custom_challenge.key),
                );
            }
        }

        // Verify forest proof.
        let mut forest_keys_proven =
            ForestVerifierFor::<T>::verify_proof(&root, &challenges, forest_proof)
                .map_err(|_| Error::<T>::ForestProofVerificationFailed)?;

        // Apply the delta to the Forest root for all mutations that are in checkpoint challenges.
        if let Some(challenges) = checkpoint_challenges {
            // Aggregate all mutations to apply to the Forest root.
            let mutations = challenges
                .iter()
                .filter_map(|custom_challenge| {
                    // The key should be removed if `should_remove_key` is `true` and if when the Provider responds to this challenge with a proof,
                    // in that proof there is an inclusion proof for that key (i.e. the key is in the Merkle Patricia Forest).
                    if custom_challenge.should_remove_key
                        && forest_keys_proven.contains(&custom_challenge.key)
                    {
                        Some((
                            custom_challenge.key,
                            TrieMutation::Remove(Default::default()),
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if !mutations.is_empty() {
                // Apply the mutations to the Forest.
                let (_, new_root, mutated_keys_and_values) = <T::ForestVerifier as TrieProofDeltaApplier<
                    T::MerkleTrieHashing,
                >>::apply_delta(
                    &root, mutations.as_slice(), forest_proof
                )
                .map_err(|_| Error::<T>::FailedToApplyDelta)?;

                // Check that the number of mutated keys is the same as the mutations expected.
                ensure!(
                    mutated_keys_and_values.len() == mutations.len(),
                    Error::<T>::UnexpectedNumberOfRemoveMutations
                );

                for (key, maybe_value) in mutated_keys_and_values.iter() {
                    // Remove the mutated key from the list of `forest_keys_proven` to avoid having to verify the key proof.
                    forest_keys_proven.remove(key);

                    // Use the interface exposed by the Providers pallet to update the submitting Provider
                    // after the key removal if the key had a value.
                    if let Some(trie_value) = maybe_value {
                        ProvidersPalletFor::<T>::update_provider_after_key_removal(
                            submitter, trie_value,
                        )
                        .map_err(|_| Error::<T>::FailedToApplyDelta)?;
                    }
                }

                // Update root of Provider after all mutations have been applied to the Forest.
                <T::ProvidersPallet as MutateChallengeableProvidersInterface>::update_root(
                    *submitter, new_root,
                )?;

                if new_root == ProvidersPalletFor::<T>::get_default_root() {
                    // We should remove the BSP from the dealer proof
                    Self::stop_challenge_cycle(submitter)?;
                };

                // Emit event of mutation applied.
                Self::deposit_event(Event::MutationsAppliedForProvider {
                    provider_id: *submitter,
                    mutations: mutations.to_vec(),
                    old_root: root,
                    new_root,
                });
            }
        };

        // Check that the correct number of key proofs were submitted.
        ensure!(
            key_proofs.len() == forest_keys_proven.len(),
            Error::<T>::IncorrectNumberOfKeyProofs
        );

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

        // Calculate the next tick for which the submitter should submit a proof.
        let next_challenges_tick = challenges_tick
            .checked_add(&Self::stake_to_challenge_period(stake))
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Update `ProviderToProofSubmissionRecord` to the challenge tick the Provider has just
        // submitted a proof for, and the next tick for which the Provider should submit a proof for.
        let proof_submission_record = ProofSubmissionRecord {
            last_tick_proven: challenges_tick,
            next_tick_to_submit_proof_for: next_challenges_tick,
        };
        ProviderToProofSubmissionRecord::<T>::set(*submitter, Some(proof_submission_record));

        // Remove the submitter from its current deadline registered in `TickToProvidersDeadlines`.
        TickToProvidersDeadlines::<T>::remove(challenges_tick_deadline, submitter);

        // Add tolerance to `next_challenges_tick` to know when is the next deadline for submitting a
        // proof, for this Provider.
        let next_challenges_tick_deadline = next_challenges_tick
            .checked_add(&T::ChallengeTicksTolerance::get())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Add this Provider to the `TickToProvidersDeadlines` StorageMap, with its new deadline.
        TickToProvidersDeadlines::<T>::set(next_challenges_tick_deadline, submitter, Some(()));

        // Add this Provider to the `ValidProofSubmittersLastTicks` StorageMap, with the current tick number.
        let current_tick_valid_submitters = ValidProofSubmittersLastTicks::<T>::get(current_tick);
        match current_tick_valid_submitters {
            // If the set already exists and has valid submitters, we just insert the new submitter.
            Some(mut valid_submitters) => {
                let did_not_already_exist = expect_or_err!(valid_submitters.try_insert(*submitter), "The set should never be full as the limit we set should be greater than the implicit limit given by max block weight.", Error::<T>::TooManyValidProofSubmitters, result);
                // We only update storage if the Provider ID wasn't yet in the set to avoid unnecessary writes.
                if did_not_already_exist {
                    ValidProofSubmittersLastTicks::<T>::insert(current_tick, valid_submitters);
                }
            }
            // If the set doesn't exist, we create it and insert the submitter.
            None => {
                let mut new_valid_submitters =
                    BoundedBTreeSet::<ProviderIdFor<T>, MaxSubmittersPerTickFor<T>>::new();
                expect_or_err!(
                    new_valid_submitters.try_insert(*submitter),
                    "The set has just been created, it's empty and as such won't be full. qed",
                    Error::<T>::TooManyValidProofSubmitters,
                    result
                );
                ValidProofSubmittersLastTicks::<T>::insert(current_tick, new_valid_submitters);
            }
        }

        Ok(challenges_tick)
    }

    /// Generate a new round of challenges, both random and checkpoint if corresponding.
    ///
    /// Random challenges are automatically generated based on some external source of
    /// randomness. To be more precise, a random seed is generated and added to
    /// [`TickToChallengesSeed`], for this tick's number.
    ///
    /// It also takes care of including the challenges from the `ChallengesQueue` and
    /// `PriorityChallengesQueue`. These custom challenges are only included in "checkpoint"
    /// ticks.
    ///
    /// Additionally, it takes care of checking if there are Providers that have
    /// failed to submit a proof, and should have submitted one by this tick. It does so
    /// by checking the [`TickToProvidersDeadlines`] StorageMap. If a Provider is found
    /// to have failed to submit a proof, it is subject to slashing.
    ///
    /// Finally, it cleans up:
    /// - The [`TickToChallengesSeed`] StorageMap, removing entries older than `ChallengeHistoryLength`.
    /// - The [`TickToCheckpointChallenges`] StorageMap, removing the previous checkpoint challenge block.
    /// - The [`TickToProvidersDeadlines`] StorageMap, removing entries for the current challenges tick.
    pub fn do_new_challenges_round(weight: &mut WeightMeter) {
        // Increment the challenges' ticker.
        let mut challenges_ticker = ChallengesTicker::<T>::get();
        challenges_ticker.saturating_inc();
        ChallengesTicker::<T>::set(challenges_ticker);

        // Store random seed for this tick.
        let (seed, _) = RandomnessProviderFor::<T>::random(challenges_ticker.encode().as_ref());
        TickToChallengesSeed::<T>::set(challenges_ticker, Some(seed));

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

        let last_checkpoint_tick = LastCheckpointTick::<T>::get();

        // Count last checkpoint challenges tick's challenges. This is to consider if slashable Providers should
        // have responses to checkpoint challenges, and slash them for the corresponding number of missed challenges.
        let checkpoint_challenges_count =
            TickToCheckpointChallenges::<T>::get(last_checkpoint_tick)
                .unwrap_or_else(||
                    // Returning an empty list so slashable providers will not accrue any failed proof submissions for checkpoint challenges.
                    BoundedVec::new())
                .len();

        // This hook does not return an error, and it cannot fail, that's why we use `saturating_add`.
        let next_checkpoint_tick =
            last_checkpoint_tick.saturating_add(T::CheckpointChallengePeriod::get());
        if challenges_ticker == next_checkpoint_tick {
            // This is a checkpoint challenge round, so we also generate new checkpoint challenges.
            Self::do_new_checkpoint_challenge_round(challenges_ticker, weight);
        }

        // If there are Providers left in `TickToProvidersDeadlines` for `TickToCheckedForSlashableProviders`,
        // they will be marked as slashable.
        let mut tick_to_check_for_slashable_providers =
            TickToCheckForSlashableProviders::<T>::get();
        let mut slashable_providers =
            TickToProvidersDeadlines::<T>::drain_prefix(tick_to_check_for_slashable_providers);

        // This loop is expected to run for a low number of iterations, given that normally, there should
        // be little to no Providers in the `TickToProvidersDeadlines` StorageMap for the `TickToCheckedForSlashableProviders`.
        // However, in the extreme scenario where a large number of Providers are missing the proof submissions,
        // this is bounded by the `MaxSlashableProvidersPerTick` configuration.
        let max_slashable_providers = MaxSlashableProvidersPerTickFor::<T>::get();
        let challenge_ticks_tolerance = T::ChallengeTicksTolerance::get();
        let mut slashable_providers_count = 0;
        while tick_to_check_for_slashable_providers <= challenges_ticker
            && slashable_providers_count < max_slashable_providers
        {
            // If there are Providers left in `TickToProvidersDeadlines` for `TickToCheckedForSlashableProviders`,
            // they are marked as slashable.
            if let Some((provider, _)) = slashable_providers.next() {
                let mut proof_submission_record =
                    match ProviderToProofSubmissionRecord::<T>::get(provider) {
                        Some(record) => record,
                        None => {
                            Self::deposit_event(Event::NoRecordOfLastSubmittedProof { provider });

                            #[cfg(test)]
                            unreachable!(
                                "Provider should have a last tick it submitted a proof for."
                            );

                            #[allow(unreachable_code)]
                            {
                                // If the Provider has no record of the last tick it submitted a proof for,
                                // we set it to the current challenges ticker, so checkpoint challenges will
                                // not be considered in slashing it.
                                ProofSubmissionRecord {
                                    last_tick_proven: challenges_ticker,
                                    next_tick_to_submit_proof_for: challenges_ticker,
                                }
                            }
                        }
                    };
                let last_tick_proven = proof_submission_record.last_tick_proven;

                // Accrue number of failed proof submission for this slashable provider.
                // Add custom checkpoint challenges if the provider needed to respond to them.
                SlashableProviders::<T>::mutate(provider, |slashable| {
                    let mut accrued = slashable.unwrap_or(0);

                    let challenge_ticker_provider_should_have_responded_to =
                        challenges_ticker.saturating_sub(challenge_ticks_tolerance);

                    if checkpoint_challenges_count != 0
                        && last_tick_proven < last_checkpoint_tick
                        && last_checkpoint_tick
                            <= challenge_ticker_provider_should_have_responded_to
                    {
                        accrued = accrued.saturating_add(checkpoint_challenges_count as u32);
                    }

                    accrued = accrued.saturating_add(RandomChallengesPerBlockFor::<T>::get());

                    *slashable = Some(accrued);
                });

                // Get the stake for this Provider, to know its challenge period.
                // If a submitter is a registered Provider, it must have a stake, so there shouldn't be an error.
                let stake = match ProvidersPalletFor::<T>::get_stake(provider) {
                    Some(stake) => stake,
                    // But to avoid panics, in the odd case of a Provider not being registered, we
                    // arbitrarily set the stake to be that which would result in `CheckpointChallengePeriod` ticks of challenge period.
                    None => {
                        let checkpoint_challenge_period =
                            CheckpointChallengePeriodFor::<T>::get().saturated_into::<u32>();
                        StakeToChallengePeriodFor::<T>::get() * checkpoint_challenge_period.into()
                    }
                };

                // Calculate the next challenge deadline for this Provider.
                // At this point, we are processing all providers who have reached their deadline (i.e. tolerance
                // ticks after the tick they should provide a proof for):
                // challenge_ticker = last_tick_provider_should_have_submitted_a_proof_for + ChallengeTicksTolerance
                //
                // By definition, the next deadline should be tolerance ticks after the next tick they should submit
                // proof for (i.e. one period after the last tick they should have submitted a proof for):
                // next_challenge_deadline = last_tick_provider_should_have_submitted_a_proof_for + provider_period + ChallengeTicksTolerance
                //
                // Therefore, the next deadline is one period from now:
                // next_challenge_deadline = challenge_ticker + provider_period
                let next_challenge_deadline =
                    challenges_ticker.saturating_add(Self::stake_to_challenge_period(stake));

                // Update this Provider's next challenge deadline.
                TickToProvidersDeadlines::<T>::set(next_challenge_deadline, provider, Some(()));

                // Calculate the next tick for which this Provider should submit a proof for, which is equal to:
                // next_tick_to_submit_proof_for = next_challenge_deadline - ChallengeTicksTolerance
                let next_tick_to_submit_proof_for =
                    next_challenge_deadline.saturating_sub(challenge_ticks_tolerance);

                // Update this Provider's proof submission record.
                proof_submission_record.next_tick_to_submit_proof_for =
                    next_tick_to_submit_proof_for;
                ProviderToProofSubmissionRecord::<T>::set(provider, Some(proof_submission_record));

                // Emit slashable provider event.
                Self::deposit_event(Event::SlashableProvider {
                    provider,
                    next_challenge_deadline,
                });

                // Increment the number of slashable providers.
                slashable_providers_count += 1;
            } else {
                // If there are no more Providers left in `TickToProvidersDeadlines` for `TickToCheckedForSlashableProviders`,
                // we increment `TickToCheckedForSlashableProviders` to the next tick. If in doing so, `TickToCheckedForSlashableProviders`
                // goes beyond `ChallengesTicker`, this loop will exit, leaving everything ready for the next tick.
                tick_to_check_for_slashable_providers =
                    tick_to_check_for_slashable_providers.saturating_add(One::one());
                slashable_providers = TickToProvidersDeadlines::<T>::drain_prefix(
                    tick_to_check_for_slashable_providers,
                );
            }
        }

        // Update `TickToCheckedForSlashableProviders` to the value resulting from the last iteration of the loop.
        TickToCheckForSlashableProviders::<T>::set(tick_to_check_for_slashable_providers);

        // Consume weight.
        weight.consume(T::WeightInfo::new_challenges_round(
            slashable_providers_count,
        ));
    }

    /// Check if the network is presumably under a spam attack.
    ///
    /// The function looks at the weight used in the past block, comparing it
    /// with the maximum allowed weight (`max_weight_for_class`) for the dispatch class of `submit_proof` extrinsics.
    /// The idea is to track blocks that have not been filled to capacity within a
    /// specific period (`BlockFullnessPeriod`) and determine if there is enough "headroom"
    /// (unused block capacity) to consider the network not under spam.
    pub fn do_check_spamming_condition(weight: &mut WeightMeter) {
        // Get the maximum weight for the dispatch class of `submit_proof` extrinsics.
        let weights = T::BlockWeights::get();
        let max_weight_for_class = weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block);

        let current_block = frame_system::Pallet::<T>::block_number();

        // Get the past `BlockFullnessPeriod` blocks and whether they were considered full or not.
        let mut past_blocks_status = PastBlocksStatus::<T>::get();

        // Remove the oldest block from the list of past blocks statuses if the list is full.
        if past_blocks_status.len() == T::BlockFullnessPeriod::get() as usize {
            past_blocks_status.remove(0);
        }

        // This would only be `None` if the block number is 0, so this should be safe.
        if let Some(prev_block) = current_block.checked_sub(&1u32.into()) {
            // Get the weight usage in the previous block.
            if let Some(weight_used_in_prev_block) = PastBlocksWeight::<T>::get(prev_block) {
                // Check how much weight was left in the previous block, compared to the maximum weight.
                // This is computed both for proof size and ref time.
                let weight_left_in_prev_block =
                    max_weight_for_class.saturating_sub(weight_used_in_prev_block);

                // If the weight left in the previous block is greater or equal than the headroom, for both proof size or ref time,
                // we consider the previous block to be NOT full and count it as such.
                if weight_left_in_prev_block.ref_time()
                    >= T::BlockFullnessHeadroom::get().ref_time()
                    && weight_left_in_prev_block.proof_size()
                        >= T::BlockFullnessHeadroom::get().proof_size()
                {
                    // Push the status of the previous block as NOT full to the list of past blocks statuses.
                    past_blocks_status
                        .try_push(false)
                        .expect("If the bounded vector was full, the oldest block was removed so there should be space. qed");
                } else {
                    // Push the status of the previous block as full to the list of past blocks statuses.
                    past_blocks_status
                        .try_push(true)
                        .expect("If the bounded vector was full, the oldest block was removed so there should be space. qed");
                }
            }
        }

        // At this point, we have an updated count of blocks that were not full in the past `BlockFullnessPeriod`.
        // Running this check only makes sense after `ChallengesTicker` has advanced past `BlockFullnessPeriod`.
        if ChallengesTicker::<T>::get() > T::BlockFullnessPeriod::get().into() {
            // To consider the network NOT to be under spam, we need more than `min_non_full_blocks` blocks to be not full.
            let min_non_full_blocks = Self::calculate_min_non_full_blocks_to_spam();

            // Get the number of blocks that were not full in the past `BlockFullnessPeriod`.
            // Since the bound on the number of blocks in the vector is a u32, we can safely cast it to a `BlockNumberFor<T>`.
            let not_full_blocks_count: BlockNumberFor<T> = (past_blocks_status
                .iter()
                .filter(|&&is_full| !is_full)
                .count() as u32)
                .into();

            // If `not_full_blocks_count` is greater than `min_non_full_blocks`, we consider the network NOT to be under spam.
            if not_full_blocks_count > min_non_full_blocks {
                // The network is NOT considered to be under a spam attack, so we resume the `ChallengesTicker`.
                ChallengesTickerPaused::<T>::set(None);
            } else {
                // At this point, the network is presumably under a spam attack, so we pause the `ChallengesTicker`.
                ChallengesTickerPaused::<T>::set(Some(()));
            }
        }

        // Update the PastBlocksStatus storage.
        PastBlocksStatus::<T>::set(past_blocks_status);

        // Consume weight.
        weight.consume(T::WeightInfo::check_spamming_condition());
    }

    /// Generate new checkpoint challenges for a given block.
    ///
    /// Fills up a new vector of checkpoint challenges with challenges in the `PriorityChallengesQueue`,
    /// and the `ChallengesQueue` if there is space left.
    ///
    /// Cleans up the `TickToCheckpointChallenges` StorageMap, removing the previous checkpoint challenge block.
    pub(crate) fn do_new_checkpoint_challenge_round(
        current_tick: BlockNumberFor<T>,
        weight: &mut WeightMeter,
    ) {
        let mut new_checkpoint_challenges: BoundedVec<
            CustomChallenge<T>,
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
        // The conversion shouldn't fail because we now have a vector that has even less elements than the original.
        // Anyway, in case it fails, we just use the original priority challenges queue.
        let new_priority_challenges_queue: BoundedVec<
            CustomChallenge<T>,
            ChallengesQueueLengthFor<T>,
        > = Vec::from(priority_challenges_queue)
            .try_into()
            .unwrap_or_else(|_| original_priority_challenges_queue);

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

            if new_checkpoint_challenges
                .try_push(CustomChallenge {
                    key: challenge,
                    should_remove_key: false,
                })
                .is_err()
            {
                // This should not happen, as we check that new_checkpoint_challenges is not full
                // in the while condition above, but we add this to be safe.
                break;
            }
        }

        // Convert challenges_queue back to a bounded vector.
        let new_challenges_queue: BoundedVec<KeyFor<T>, ChallengesQueueLengthFor<T>> =
            Vec::from(challenges_queue)
                .try_into()
                .unwrap_or_else(|_| BoundedVec::new());

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
            challenges: new_checkpoint_challenges.clone(),
        });

        // Consume weight.
        weight.consume(T::WeightInfo::new_checkpoint_challenge_round(
            new_checkpoint_challenges.len() as u32,
        ));
    }

    /// Trim the storage that holds the Providers that submitted valid proofs in the last ticks until there's
    /// `TargetTicksOfProofsStorage` ticks left (or until the remaining weight allows it).
    ///
    /// This function is called in the `on_idle` hook, which means it's only called when the block has
    /// unused weight.
    ///
    /// It removes the oldest tick from the storage that holds the providers that submitted valid proofs
    /// in the last ticks as many times as the remaining weight allows it, but at most until the storage
    /// has `TargetTicksOfProofsStorage` ticks left.
    pub fn do_trim_valid_proof_submitters_last_ticks(
        _n: BlockNumberFor<T>,
        usable_weight: Weight,
    ) -> Weight {
        // Check how many ticks should be removed to keep the storage at the target amount.
        let last_deleted_tick = LastDeletedTick::<T>::get();
        let target_ticks_to_keep = TargetTicksStorageOfSubmittersFor::<T>::get();
        let current_tick = ChallengesTicker::<T>::get();
        let ticks_to_remove: BlockNumberFor<T> = current_tick
            .saturating_sub(last_deleted_tick)
            .saturating_sub(target_ticks_to_keep.into());

        // Check how much ticks can be removed considering weight limitations
        let weight_for_one_iteration = T::WeightInfo::trim_valid_proof_submitters_last_ticks_loop();
        let removable_ticks = usable_weight.checked_div_per_component(&weight_for_one_iteration);

        // If there is enough weight to remove ticks, try to remove as many ticks as possible until the target is reached.
        let ticks_removed: u64 = if let Some(removable_ticks) = removable_ticks {
            // Take the minimum between all the ticks that we want to remove, and the ones we can.
            let removable_ticks: BlockNumberFor<T> = removable_ticks.saturated_into();
            let removable_ticks = removable_ticks.min(ticks_to_remove);

            // Remove all the ticks that we can, until we reach the target amount.
            let start_tick = last_deleted_tick.saturating_add(One::one());
            let end_tick = last_deleted_tick.saturating_add(removable_ticks);
            let mut tick_to_remove = start_tick;
            while tick_to_remove <= end_tick {
                tick_to_remove = Self::remove_proof_submitters_for_tick(tick_to_remove);
            }

            // Return the number of ticks removed.
            removable_ticks.saturated_into()
        } else {
            // No ticks can be removed.
            Zero::zero()
        };

        // Return the weight used by this function.
        T::WeightInfo::trim_valid_proof_submitters_last_ticks_constant_execution()
            .saturating_add(weight_for_one_iteration.saturating_mul(ticks_removed))
    }

    /// Convert stake to challenge period.
    ///
    /// [`StakeToChallengePeriodFor`] is divided by `stake` to get the number of blocks in between challenges
    /// for a Provider. The result is then converted to `BlockNumber` type. The division saturates at [`MinChallengePeriodFor`].
    pub(crate) fn stake_to_challenge_period(stake: BalanceFor<T>) -> BlockNumberFor<T> {
        let min_challenge_period = MinChallengePeriodFor::<T>::get();
        let challenge_period = match StakeToChallengePeriodFor::<T>::get().checked_div(&stake) {
            Some(block_period) => T::StakeToBlockNumber::convert(block_period),
            None => min_challenge_period,
        };

        // Return the maximum between the calculated challenge period and the minimum challenge period.
        min_challenge_period.max(challenge_period)
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
    fn enqueue_challenge_with_priority(key: &KeyFor<T>, should_remove_key: bool) -> DispatchResult {
        // Get priority challenges queue from storage.
        let mut priority_challenges_queue = PriorityChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        let new_priority_challenge = CustomChallenge {
            key: *key,
            should_remove_key,
        };
        if priority_challenges_queue.contains(&new_priority_challenge) {
            return Ok(());
        }

        // Add challenge to queue.
        priority_challenges_queue
            .try_push(new_priority_challenge)
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

    /// Calculate the minimum number of blocks that should be not full to consider the network
    /// to be presumably under spam attack.
    ///
    /// To be precise, the number of non full blocks should be greater than what this function
    /// returns.
    pub(crate) fn calculate_min_non_full_blocks_to_spam() -> BlockNumberFor<T> {
        let min_non_full_blocks_ratio = T::MinNotFullBlocksRatio::get();
        min_non_full_blocks_ratio.mul_floor(T::BlockFullnessPeriod::get().into())
    }

    /// Remove all the proof submitters for a given tick and update the `LastDeletedTick` storage element
    /// to the given tick.
    ///
    /// Returns the next tick to delete.
    pub(crate) fn remove_proof_submitters_for_tick(tick: BlockNumberFor<T>) -> BlockNumberFor<T> {
        // Remove it from storage
        ValidProofSubmittersLastTicks::<T>::remove(tick);

        // Update the last removed tick
        LastDeletedTick::<T>::set(tick);

        // Return the next tick to delete
        tick.saturating_add(One::one())
    }
}

impl<T: pallet::Config> StorageHubTickGetter for Pallet<T> {
    type TickNumber = <Pallet<T> as ProofsDealerInterface>::TickNumber;

    fn get_current_tick() -> Self::TickNumber {
        <Pallet<T> as ProofsDealerInterface>::get_current_tick()
    }
}

impl<T: pallet::Config> ProofsDealerInterface for Pallet<T> {
    type ProviderId = ProviderIdFor<T>;
    type ForestProof = ForestVerifierProofFor<T>;
    type KeyProof = KeyVerifierProofFor<T>;
    type MerkleHash = T::MerkleTrieHash;
    type MerkleHashing = T::MerkleTrieHashing;
    type RandomnessOutput = RandomnessOutputFor<T>;
    type TickNumber = BlockNumberFor<T>;

    fn verify_forest_proof(
        provider_id: &Self::ProviderId,
        challenges: &[Self::MerkleHash],
        proof: &Self::ForestProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError> {
        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(*provider_id),
            Error::<T>::NotProvider
        );

        // Get root for submitter.
        // If a submitter is a registered Provider, it must have a root.
        let root = ProvidersPalletFor::<T>::get_root(*provider_id)
            .ok_or(Error::<T>::ProviderRootNotFound)?;

        // Verify forest proof.
        ForestVerifierFor::<T>::verify_proof(&root, challenges, proof)
            .map_err(|_| Error::<T>::ForestProofVerificationFailed.into())
    }

    fn verify_generic_forest_proof(
        root: &Self::MerkleHash,
        challenges: &[Self::MerkleHash],
        proof: &Self::ForestProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, DispatchError> {
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
        should_remove_key: bool,
    ) -> DispatchResult {
        Self::enqueue_challenge_with_priority(key_challenged, should_remove_key)
    }

    fn generate_challenges_from_seed(
        seed: Self::RandomnessOutput,
        provider_id: &Self::ProviderId,
        count: u32,
    ) -> Vec<Self::MerkleHash> {
        Self::generate_challenges_from_seed(seed, provider_id, count)
    }

    fn apply_delta(
        provider_id: &Self::ProviderId,
        mutations: &[(Self::MerkleHash, TrieMutation)],
        proof: &Self::ForestProof,
    ) -> Result<Self::MerkleHash, DispatchError> {
        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(*provider_id),
            Error::<T>::NotProvider
        );

        // Get root for submitter.
        // If a submitter is a registered Provider, it must have a root.
        let root = ProvidersPalletFor::<T>::get_root(*provider_id)
            .ok_or(Error::<T>::ProviderRootNotFound)?;

        let (_, new_root, _) =
            <T::ForestVerifier as TrieProofDeltaApplier<T::MerkleTrieHashing>>::apply_delta(
                &root, mutations, proof,
            )
            .map_err(|_| Error::<T>::FailedToApplyDelta)?;

        // Emit event of mutation applied.
        Self::deposit_event(Event::MutationsAppliedForProvider {
            provider_id: *provider_id,
            mutations: mutations.to_vec(),
            old_root: root,
            new_root,
        });

        Ok(new_root)
    }

    fn generic_apply_delta(
        root: &Self::MerkleHash,
        mutations: &[(Self::MerkleHash, TrieMutation)],
        proof: &Self::ForestProof,
        event_info: Option<Vec<u8>>,
    ) -> Result<Self::MerkleHash, DispatchError> {
        let (_, new_root, _) =
            <T::ForestVerifier as TrieProofDeltaApplier<T::MerkleTrieHashing>>::apply_delta(
                &root, mutations, proof,
            )
            .map_err(|_| Error::<T>::FailedToApplyDelta)?;

        // Emit event of mutation applied.
        Self::deposit_event(Event::MutationsApplied {
            mutations: mutations.to_vec(),
            old_root: *root,
            new_root,
            event_info,
        });

        Ok(new_root)
    }

    // Remove a provider from challenge cycle.
    fn stop_challenge_cycle(provider_id: &Self::ProviderId) -> DispatchResult {
        // Check that `provider_id` is a registered Provider.
        if !ProvidersPalletFor::<T>::is_provider(*provider_id) {
            return Err(Error::<T>::NotProvider.into());
        }

        // Check if this Provider previously had a challenge cycle initialised so we can delete it.
        if let Some(record) = ProviderToProofSubmissionRecord::<T>::get(*provider_id) {
            // Compute the next tick for which the Provider should have been submitting a proof.
            let old_next_challenge_tick = record.next_tick_to_submit_proof_for;

            // Calculate the deadline for submitting a proof. Should be the next challenge tick + the challenges tick tolerance.
            let old_next_challenge_deadline = old_next_challenge_tick
                .checked_add(&ChallengeTicksToleranceFor::<T>::get())
                .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

            // Remove the provider from the deadlines storage
            TickToProvidersDeadlines::<T>::remove(old_next_challenge_deadline, *provider_id);

            // Remove the provider from the submitted proof storage.
            ProviderToProofSubmissionRecord::<T>::remove(*provider_id);
        }

        Ok(())
    }

    fn initialise_challenge_cycle(provider_id: &Self::ProviderId) -> DispatchResult {
        // Check that `provider_id` is a registered Provider.
        if !ProvidersPalletFor::<T>::is_provider(*provider_id) {
            return Err(Error::<T>::NotProvider.into());
        }

        // Get stake for submitter.
        // If a submitter is a registered Provider, it must have a stake, so this shouldn't happen.
        // However, since the implementation of that is not up to this pallet, we need to check.
        let stake = ProvidersPalletFor::<T>::get_stake(*provider_id)
            .ok_or(Error::<T>::ProviderStakeNotFound)?;

        // Check that the stake is non-zero.
        ensure!(stake > BalanceFor::<T>::zero(), Error::<T>::ZeroStake);

        // Check if this Provider previously had a challenge cycle initialised.
        if let Some(record) = ProviderToProofSubmissionRecord::<T>::get(*provider_id) {
            let old_next_challenge_tick = record.next_tick_to_submit_proof_for;

            // Calculate this Provider's deadline for submitting a proof.
            // Should be the next challenge tick + the challenges tick tolerance.
            let old_next_challenge_deadline = old_next_challenge_tick
                .checked_add(&ChallengeTicksToleranceFor::<T>::get())
                .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

            // Remove the old deadline.
            TickToProvidersDeadlines::<T>::remove(old_next_challenge_deadline, *provider_id);
        }

        // Initialise the Provider's proof submission record.
        // The last tick the Provider submitted a proof for will be the current tick, and
        // the next tick the Provider should submit a proof for will be:
        // next_tick_to_submit_proof_for = current_tick + provider_challenge_period
        let current_tick = ChallengesTicker::<T>::get();
        let next_challenge_tick = current_tick
            .checked_add(&Self::stake_to_challenge_period(stake))
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        let proof_submission_record = ProofSubmissionRecord {
            last_tick_proven: current_tick,
            next_tick_to_submit_proof_for: next_challenge_tick,
        };
        ProviderToProofSubmissionRecord::<T>::set(*provider_id, Some(proof_submission_record));

        // Calculate the deadline for submitting a proof. Should be the next challenge tick + the challenges tick tolerance.
        let next_challenge_deadline = next_challenge_tick
            .checked_add(&ChallengeTicksToleranceFor::<T>::get())
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Set the deadline for submitting a proof.
        TickToProvidersDeadlines::<T>::set(next_challenge_deadline, *provider_id, Some(()));

        // Emit event.
        Self::deposit_event(Event::NewChallengeCycleInitialised {
            current_tick,
            next_challenge_deadline,
            provider: *provider_id,
            maybe_provider_account: ProvidersPalletFor::<T>::get_owner_account(*provider_id),
        });

        Ok(())
    }

    fn get_current_tick() -> Self::TickNumber {
        ChallengesTicker::<T>::get()
    }

    fn get_checkpoint_challenge_period() -> Self::TickNumber {
        T::CheckpointChallengePeriod::get()
    }
}

impl<T: pallet::Config> ProofSubmittersInterface for Pallet<T> {
    type ProviderId = ProviderIdFor<T>;
    type TickNumber = BlockNumberFor<T>;
    type MaxProofSubmitters = MaxSubmittersPerTickFor<T>;

    fn get_proof_submitters_for_tick(
        tick_number: &Self::TickNumber,
    ) -> Option<BoundedBTreeSet<Self::ProviderId, Self::MaxProofSubmitters>> {
        ValidProofSubmittersLastTicks::<T>::get(tick_number)
    }

    fn get_current_tick() -> Self::TickNumber {
        ChallengesTicker::<T>::get()
    }

    fn get_accrued_failed_proof_submissions(provider_id: &Self::ProviderId) -> Option<u32> {
        SlashableProviders::<T>::get(provider_id)
    }

    fn clear_accrued_failed_proof_submissions(provider_id: &Self::ProviderId) {
        SlashableProviders::<T>::remove(provider_id);
    }
}

/// Runtime API implementation for the ProofsDealer pallet.
impl<T> Pallet<T>
where
    T: pallet::Config,
{
    pub fn get_last_tick_provider_submitted_proof(
        provider_id: &ProviderIdFor<T>,
    ) -> Result<BlockNumberFor<T>, GetProofSubmissionRecordError> {
        // Check if submitter is a registered Provider.
        if !ProvidersPalletFor::<T>::is_provider(*provider_id) {
            return Err(GetProofSubmissionRecordError::ProviderNotRegistered);
        }

        let record = ProviderToProofSubmissionRecord::<T>::get(provider_id)
            .ok_or(GetProofSubmissionRecordError::ProviderNeverSubmittedProof)?;

        Ok(record.last_tick_proven)
    }

    pub fn get_next_tick_to_submit_proof_for(
        provider_id: &ProviderIdFor<T>,
    ) -> Result<BlockNumberFor<T>, GetProofSubmissionRecordError> {
        // Check if submitter is a registered Provider.
        if !ProvidersPalletFor::<T>::is_provider(*provider_id) {
            return Err(GetProofSubmissionRecordError::ProviderNotRegistered);
        }

        let record = ProviderToProofSubmissionRecord::<T>::get(provider_id)
            .ok_or(GetProofSubmissionRecordError::ProviderNeverSubmittedProof)?;

        Ok(record.next_tick_to_submit_proof_for)
    }

    pub fn get_last_checkpoint_challenge_tick() -> BlockNumberFor<T> {
        LastCheckpointTick::<T>::get()
    }

    pub fn get_checkpoint_challenges(
        tick: BlockNumberFor<T>,
    ) -> Result<Vec<CustomChallenge<T>>, GetCheckpointChallengesError> {
        // Check that the tick is smaller than the last checkpoint tick.
        if LastCheckpointTick::<T>::get() < tick {
            return Err(GetCheckpointChallengesError::TickGreaterThanLastCheckpointTick);
        }

        let checkpoint_challenges = TickToCheckpointChallenges::<T>::get(tick)
            .ok_or(GetCheckpointChallengesError::NoCheckpointChallengesInTick)?;

        Ok(checkpoint_challenges.into())
    }

    pub fn get_challenge_seed(
        tick: BlockNumberFor<T>,
    ) -> Result<RandomnessOutputFor<T>, GetChallengeSeedError> {
        let current_tick = ChallengesTicker::<T>::get();
        if tick > current_tick {
            return Err(GetChallengeSeedError::TickIsInTheFuture);
        }

        let seed = TickToChallengesSeed::<T>::get(tick)
            .ok_or(GetChallengeSeedError::TickBeyondLastSeedStored)?;

        Ok(seed)
    }

    pub fn get_challenge_period(
        provider_id: &ProviderIdFor<T>,
    ) -> Result<BlockNumberFor<T>, GetChallengePeriodError> {
        let stake = ProvidersPalletFor::<T>::get_stake(*provider_id)
            .ok_or(GetChallengePeriodError::ProviderNotRegistered)?;

        Ok(Self::stake_to_challenge_period(stake))
    }

    pub fn get_checkpoint_challenge_period() -> BlockNumberFor<T> {
        CheckpointChallengePeriodFor::<T>::get()
    }

    pub fn get_challenges_from_seed(
        seed: &RandomnessOutputFor<T>,
        provider_id: &ProviderIdFor<T>,
        count: u32,
    ) -> Vec<KeyFor<T>> {
        Self::generate_challenges_from_seed(*seed, provider_id, count)
    }

    pub fn get_forest_challenges_from_seed(
        seed: &RandomnessOutputFor<T>,
        provider_id: &ProviderIdFor<T>,
    ) -> Vec<KeyFor<T>> {
        Self::generate_challenges_from_seed(
            *seed,
            provider_id,
            RandomChallengesPerBlockFor::<T>::get(),
        )
    }

    pub fn get_current_tick() -> BlockNumberFor<T> {
        ChallengesTicker::<T>::get()
    }

    pub fn get_next_deadline_tick(
        provider_id: &ProviderIdFor<T>,
    ) -> Result<BlockNumberFor<T>, GetNextDeadlineTickError> {
        // Check if the provider is indeed a registered Provider.
        if !ProvidersPalletFor::<T>::is_provider(*provider_id) {
            return Err(GetNextDeadlineTickError::ProviderNotRegistered);
        }

        // Get this Provider's proof submission record.
        let record = ProviderToProofSubmissionRecord::<T>::get(provider_id)
            .ok_or(GetNextDeadlineTickError::ProviderNotInitialised)?;

        let next_deadline_tick = record
            .next_tick_to_submit_proof_for
            .checked_add(&ChallengeTicksToleranceFor::<T>::get())
            .ok_or(GetNextDeadlineTickError::ArithmeticOverflow)?;

        Ok(next_deadline_tick)
    }
}

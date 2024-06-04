// TODO: Remove this attribute once the file is implemented.
#![allow(dead_code)]
#![allow(unused_variables)]

use codec::Encode;
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{fungible::Mutate, tokens::Preservation, Get},
};
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::prelude::vec::Vec;
use shp_traits::{
    ChallengeKeyInclusion, CommitmentVerifier, ProofDeltaApplier, ProofsDealerInterface,
    ProvidersInterface,
};
use sp_runtime::{
    traits::{CheckedAdd, CheckedDiv, Convert, Hash, Zero},
    ArithmeticError, DispatchError, Saturating,
};

use crate::{
    pallet,
    types::{
        AccountIdFor, BalanceFor, BalancePalletFor, ChallengeHistoryLengthFor, ChallengesFeeFor,
        ForestRootFor, ForestVerifierFor, ForestVerifierProofFor, KeyFor, KeyVerifierFor,
        KeyVerifierProofFor, Proof, ProviderFor, ProvidersPalletFor, RandomChallengesPerBlockFor,
        StakeToChallengePeriodFor, TreasuryAccountFor,
    },
    BlockToChallengesSeed, BlockToCheckpointChallenges, ChallengesQueue, Error,
    LastBlockProviderSubmittedProofFor, LastCheckpointBlock, Pallet, PriorityChallengesQueue,
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
    /// Relies on the `ProvidersPallet` to get the root for the submitter, the last block for which
    /// the submitter submitted a proof and the stake for the submitter. With that information, it
    /// computes the next block for which the submitter should be submitting a proof. It then gets
    /// the seed for that block and generates the challenges from the seed. It also checks if there
    /// has been a Checkpoint Challenge block in between the last block proven and the current block.
    /// If there has been, the Provider should have included proofs for the challenges in that block.
    /// It then verifies the forest proof and each key proof, using the `ForestVerifier` and `KeyVerifier`.
    pub fn do_submit_proof(submitter: &ProviderFor<T>, proof: &Proof<T>) -> DispatchResult {
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

        // Check if root is non-zero.
        // A zero root means that the Provider is not providing any service yet, so he shouldn't be
        // submitting any proofs.
        ensure!(root == Self::default_forest_root(), Error::<T>::ZeroRoot);

        // Get last block for which the submitter submitted a proof.
        let last_block_proven = match LastBlockProviderSubmittedProofFor::<T>::get(*submitter) {
            Some(block) => block,
            None => return Err(Error::<T>::NoRecordOfLastSubmittedProof.into()),
        };

        // Get stake for submitter.
        // If a submitter is a registered Provider, it must have a stake, so this shouldn't happen.
        // However, since the implementation of that is not up to this pallet, we need to check.
        let stake = ProvidersPalletFor::<T>::get_stake(*submitter)
            .ok_or(Error::<T>::ProviderStakeNotFound)?;

        // Check that the stake is non-zero.
        ensure!(stake > BalanceFor::<T>::zero(), Error::<T>::ZeroStake);

        // Compute the next block for which the submitter should be submitting a proof.
        let challenges_block = last_block_proven
            .checked_add(&Self::stake_to_challenge_period(stake)?)
            .ok_or(DispatchError::Arithmetic(ArithmeticError::Overflow))?;

        // Check that the challenges block is lower than the current block.
        ensure!(
            challenges_block < frame_system::Pallet::<T>::block_number(),
            Error::<T>::ChallengesBlockNotReached
        );

        // Check that the challenges block is greater than current block minus `ChallengeHistoryLength`,
        // i.e. that the challenges block is within the blocks this pallet keeps track of.
        expect_or_err!(
            challenges_block
                > frame_system::Pallet::<T>::block_number()
                    .saturating_sub(ChallengeHistoryLengthFor::<T>::get()),
            "Challenges block is too old, beyond the history this pallet keeps track of. This should not be possible.",
            Error::<T>::ChallengesBlockTooOld,
            bool
        );

        // Get seed for challenges block.
        let seed = expect_or_err!(
            BlockToChallengesSeed::<T>::get(challenges_block),
            "Seed for challenges block not found, when checked it should be within history.",
            Error::<T>::SeedNotFound
        );

        // Generate forest challenges from seed.
        let mut challenges = Self::generate_challenges_from_seed(
            seed,
            submitter,
            RandomChallengesPerBlockFor::<T>::get(),
        );

        // Check if there's been a Checkpoint Challenge block in between the last block proven and
        // the current block. If there has been, the Provider should have included proofs for the
        // challenges in that block.
        let last_checkpoint_block = LastCheckpointBlock::<T>::get();
        if last_block_proven < last_checkpoint_block {
            // Add challenges from the Checkpoint Challenge block.
            let checkpoint_challenges =
                expect_or_err!(
                    BlockToCheckpointChallenges::<T>::get(last_checkpoint_block),
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

        Ok(())
    }

    // TODO: Document and add proper parameters.
    pub fn do_new_challenges_round() -> DispatchResult {
        // TODO
        unimplemented!()
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
        seed: T::MerkleTrieHash,
        provider_id: &ProviderFor<T>,
        count: u32,
    ) -> Vec<(T::MerkleTrieHash, Option<ChallengeKeyInclusion>)> {
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

            // The challenge inclusion type is None since we are generating random challenges and don't expect proofs of inclusion or non-inclusion.
            challenges.push((challenge.into(), None));
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
        challenges: &[(Self::MerkleHash, Option<ChallengeKeyInclusion>)],
        proof: &Self::ForestProof,
    ) -> Result<Vec<Self::MerkleHash>, DispatchError> {
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
        challenges: &[(Self::MerkleHash, Option<ChallengeKeyInclusion>)],
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

    fn apply_delta(
        commitment: &Self::MerkleHash,
        mutations: &[shp_traits::Mutation<Self::MerkleHash>],
        proof: &Self::ForestProof,
    ) -> Result<Self::MerkleHash, DispatchError> {
        Ok(
            <T::ForestVerifier as ProofDeltaApplier<T::MerkleTrieHashing>>::apply_delta(
                commitment, mutations, proof,
            )
            .map_err(|_| Error::<T>::DeltaApplicationFailed)?
            .1,
        )
    }
}

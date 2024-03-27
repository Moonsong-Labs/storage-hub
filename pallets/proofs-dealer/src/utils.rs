// TODO: Remove this attribute once the file is implemented.
#![allow(dead_code)]
#![allow(unused_variables)]
use frame_support::{
    ensure,
    pallet_prelude::DispatchResult,
    traits::{fungible::Mutate, tokens::Preservation, Get},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_trie::CompactProof;
use storage_hub_traits::{ProofsDealerInterface, ProvidersInterface};

use crate::{
    pallet,
    types::{
        AccountIdFor, BalanceFor, BalancePalletFor, ChallengesFeeFor, KeyFor, ProviderFor,
        ProvidersPalletFor, TreasuryAccountFor,
    },
    ChallengesQueue, Error, Pallet, PriorityChallengesQueue,
};

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
        if ProvidersPalletFor::<T>::get_provider(who.clone()).is_none() {
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

    // TODO: Document and add proper parameters.
    // TODO: Remove unused variable allow attribute.
    #[allow(unused_variables)]
    pub fn do_submit_proof(submitter: &ProviderFor<T>, proof: &CompactProof) -> DispatchResult {
        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_provider(submitter.clone()),
            Error::<T>::NotProvider
        );

        // TODO: Check that the proof corresponds to a correct challenge block.

        // TODO: Verify proof.

        // TODO
        unimplemented!()
    }

    // TODO: Document and add proper parameters.
    pub fn do_new_challenges_round() -> DispatchResult {
        // TODO
        unimplemented!()
    }

    // TODO: Document and add proper parameters.
    #[allow(unused_variables)]
    fn stake_to_challenge_period(stake: BalanceFor<T>) -> BlockNumberFor<T> {
        // TODO
        unimplemented!()
    }

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

    // TODO: Document.
    fn verify_proof(
        who: &ProviderFor<T>,
        root: &T::MerkleHash,
        proof: &CompactProof,
    ) -> DispatchResult {
        // TODO
        unimplemented!()
    }
}

impl<T: pallet::Config> ProofsDealerInterface for Pallet<T> {
    type Provider = ProviderFor<T>;
    type Proof = CompactProof;
    type MerkleHash = T::MerkleHash;

    fn verify_proof(
        who: &Self::Provider,
        root: &Self::MerkleHash,
        proof: &Self::Proof,
    ) -> DispatchResult {
        Self::verify_proof(who, root, proof)
    }

    fn challenge(key_challenged: &Self::MerkleHash) -> DispatchResult {
        Self::enqueue_challenge(key_challenged)
    }

    fn challenge_with_priority(key_challenged: &Self::MerkleHash) -> DispatchResult {
        Self::enqueue_challenge_with_priority(key_challenged)
    }
}

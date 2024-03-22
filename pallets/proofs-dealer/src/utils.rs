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
use storage_hub_traits::ProvidersInterface;

use crate::{
    pallet,
    types::{
        AccountIdFor, BalanceFor, BalancePalletFor, ChallengesFeeFor, FileKeyFor, ProviderFor,
        ProvidersPalletFor, TreasuryAccountFor,
    },
    ChallengesQueue, Error, Pallet,
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
    pub fn do_challenge(who: &AccountIdFor<T>, file_key: &FileKeyFor<T>) -> DispatchResult {
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

        // Get challenges queue from storage.
        let mut challenges_queue = ChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        if challenges_queue.contains(file_key) {
            return Ok(());
        }

        // Add challenge to queue.
        challenges_queue
            .try_push(*file_key)
            .map_err(|_| Error::<T>::ChallengesQueueOverflow)?;

        // Set challenges queue in storage.
        ChallengesQueue::<T>::put(challenges_queue);

        Ok(())
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
}

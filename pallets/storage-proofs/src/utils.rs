#![allow(dead_code)]
#![allow(unused_variables)]
use frame_support::{ensure, pallet_prelude::DispatchResult};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_trie::CompactProof;

use crate::{
    pallet,
    types::{AccountIdFor, BalanceFor, FileKeyFor, ProviderFor, ProvidersPalletFor},
    ChallengesQueue, Error, Pallet, ProvidersInterface,
};

impl<T> Pallet<T>
where
    T: pallet::Config,
{
    // TODO: Document.
    pub fn do_challenge(who: &AccountIdFor<T>, file_key: &FileKeyFor<T>) -> DispatchResult {
        // TODO: Add user payment for challenging.

        // Get challenge to queue from storage.
        let mut challenges_queue = ChallengesQueue::<T>::get();

        // Check if challenge is already queued. If it is, just return.
        if challenges_queue.contains(file_key) {
            return Ok(());
        }

        // Add challenge to queue.
        challenges_queue
            .try_push(*file_key)
            .map_err(|_| Error::<T>::ChallengesQueueOverflow)?;

        Ok(())
    }

    // TODO: Document and add proper parameters.
    #[allow(unused_variables)]
    pub fn do_submit_proof(submitter: &ProviderFor<T>, proof: &CompactProof) -> DispatchResult {
        // Check if submitter is a registered Provider.
        ensure!(
            ProvidersPalletFor::<T>::is_sp(submitter.clone()),
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

use frame_support::{ensure, pallet_prelude::DispatchResult};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_trie::CompactProof;

use crate::{
    pallet,
    types::{AccountIdFor, BalanceFor, FileKeyFor, StorageProvidersFor},
    ChallengesQueue, Error, Pallet, StorageProvidersInterface,
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
            .try_push(file_key.clone())
            .map_err(|_| Error::<T>::ChallengesQueueOverflow)?;

        Ok(())
    }

    // TODO: Document and add proper parameters.
    pub fn do_submit_proof(submitter: &AccountIdFor<T>, proof: &CompactProof) -> DispatchResult {
        // Check if submitter is a registered Storage Provider.
        ensure!(
            StorageProvidersFor::<T>::is_sp(submitter.clone()),
            Error::<T>::NotStorageProvider
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
    fn stake_to_challenge_period(stake: BalanceFor<T>) -> BlockNumberFor<T> {
        // TODO
        unimplemented!()
    }
}

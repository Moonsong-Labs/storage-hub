use anyhow::anyhow;
use sc_tracing::tracing::*;
use sp_trie::TrieLayout;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::NewChallengeSeed};
use shc_common::types::{HasherOutT, Proven};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-submit-proof-task";

/// TODO: Document this task.
pub struct BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    storage_hub_handler: StorageHubHandler<T, FL, FS>,
}

impl<T, FL, FS> Clone for BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    fn clone(&self) -> BspSubmitProofTask<T, FL, FS> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
        }
    }
}

impl<T, FL, FS> BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout,
    FL: Send + Sync + FileStorage<T>,
    FS: Send + Sync + ForestStorage<T>,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    pub fn new(storage_hub_handler: StorageHubHandler<T, FL, FS>) -> Self {
        Self {
            storage_hub_handler,
        }
    }
}

/// Handles the `NewChallengeSeed` event.
///
/// This event is triggered by an on-chain event of a new challenge seed being generated.
/// TODO: Complete this docs.
impl<T, FL, FS> EventHandler<NewChallengeSeed> for BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn handle_event(&mut self, event: NewChallengeSeed) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP proof submission for BSP ID: {:?}, at tick: {:?}, with seed: {:?}",
            event.provider_id,
            event.tick,
            event.seed
        );
        let seed = event.seed;
        let provider_id = event.provider_id;

        // Derive forest challenges from seed.
        let forest_challenges = self
            .storage_hub_handler
            .blockchain
            .query_forest_challenges_from_seed(seed, provider_id)
            .await?;
        let mut converted_forest_challenges: Vec<HasherOutT<T>> = Vec::new();
        for challenge in forest_challenges {
            let raw_key: [u8; 32] = challenge.into();
            match raw_key.try_into() {
                Ok(key) => converted_forest_challenges.push(key),
                Err(_) => {
                    error!(target: LOG_TARGET, "Failed to challenge key to hasher output. This should not be possible, as the challenge keys are hasher outputs.");
                    return Err(anyhow!("Failed to challenge key to hasher output. This should not be possible, as the challenge keys are hasher outputs."));
                }
            }
        }
        let mut forest_challenges = converted_forest_challenges;

        // Check if there are checkpoint challenges since last tick this provider submitted a proof for.
        let last_tick_provided_submitted_proof = self
            .storage_hub_handler
            .blockchain
            .query_last_tick_provider_submitted_proof(provider_id)
            .await
            .map_err(|e| {
                anyhow!(
                    "Failed to query last tick provider submitted proof: {:?}",
                    e
                )
            })?;
        let last_checkpoint_tick = self
            .storage_hub_handler
            .blockchain
            .query_last_checkpoint_challenge_tick()
            .await?;
        // This variable is going to be used to apply mutations down the line if there are checkpoint challenges,
        // and if the proof is successfully verified.
        let mut checkpoint_challenges = None;
        if last_tick_provided_submitted_proof <= last_checkpoint_tick {
            // If so, get the last checkpoint challenges.
            checkpoint_challenges = Some(
                self.storage_hub_handler
                    .blockchain
                    .query_last_checkpoint_challenges(last_checkpoint_tick)
                    .await
                    .map_err(|e| anyhow!("Failed to query last checkpoint challenges: {:?}", e))?,
            );

            let checkpoint_challenges = checkpoint_challenges
                .expect("`checkpoint_challenges` was just instantiated with Some()")
                .clone();
            let mut converted_checkpoint_challenges: Vec<HasherOutT<T>> = Vec::new();
            for challenge in checkpoint_challenges {
                let raw_key: [u8; 32] = challenge.0.into();
                match raw_key.try_into() {
                    Ok(key) => converted_checkpoint_challenges.push(key),
                    Err(_) => {
                        error!(target: LOG_TARGET, "Failed to challenge key to hasher output. This should not be possible, as the challenge keys are hasher outputs.");
                        return Err(anyhow!("Failed to challenge key to hasher output. This should not be possible, as the challenge keys are hasher outputs."));
                    }
                }
            }

            // Add the checkpoint challenges to the forest challenges.
            forest_challenges.extend(converted_checkpoint_challenges);
        }

        // Get a read lock on the forest storage to generate a proof for the file.
        let read_forest_storage = self.storage_hub_handler.forest_storage.read().await;
        let proven_file_keys = read_forest_storage
            .generate_proof(forest_challenges)
            .expect("Failed to generate forest proof.");
        // Release the forest storage read lock.
        drop(read_forest_storage);

        // Get the keys that were proven.
        let mut proven_keys: Vec<HasherOutT<T>> = Vec::new();
        for key in proven_file_keys.proven {
            match key {
                Proven::ExactKey(leaf) => proven_keys.push(leaf.key),
                Proven::NeighbourKeys((left, right)) => match (left, right) {
                    (Some(left), Some(right)) => {
                        proven_keys.push(left.key);
                        proven_keys.push(right.key);
                    }
                    (Some(left), None) => proven_keys.push(left.key),
                    (None, Some(right)) => proven_keys.push(right.key),
                    (None, None) => {
                        error!(target: LOG_TARGET, "Both left and right leaves in forest proof are None. This should not be possible.");
                    }
                },
                Proven::Empty => {
                    error!(target: LOG_TARGET, "Forest proof generated with empty forest. This should not be possible, as this provider shouldn't have been challenged with an empty forest.");
                }
            }
        }

        // TODO: Build key challenges from seed for every key proven.

        // TODO: Construct key proofs.

        // TODO: Submit proofs to the runtime.

        // TODO: Handle extrinsic submission result.

        // TODO: Attempt to submit again if there is a failure.

        // TODO: Apply mutations if extrinsic was successful, if any, update the Forest storage and file storage.

        Ok(())
    }
}

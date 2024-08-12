use std::time::Duration;

use anyhow::anyhow;
use sc_tracing::tracing::*;
use shp_file_metadata::ChunkId;
use sp_core::H256;
use sp_trie::TrieLayout;

use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::{commands::BlockchainServiceInterface, events::NewChallengeSeed};
use shc_common::types::{
    HasherOutT, KeyProof, KeyProofs, Proven, ProviderId, RandomnessOutput, StorageProof,
    TrieRemoveMutation,
};
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;

use crate::services::handler::StorageHubHandler;

const LOG_TARGET: &str = "bsp-submit-proof-task";
const MAX_PROOF_SUBMISSION_ATTEMPTS: u32 = 3;

/// BSP Submit Proof Task: Handles the submission of proof for BSP (Block Storage Provider) to the runtime.
///
/// The flow includes the following steps:
/// - Reacting to `NewChallengeSeed` event from the runtime:
///     - Derive forest challenges from the seed.
///     - Check and add checkpoint challenges.
///     - Generate proof for the file from the forest storage.
///     - Generate key proofs for each file key.
///     - Submit the proof to the runtime.
///     - Apply mutations if necessary and ensure the new Forest root matches the one on-chain.
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
/// This event is triggered by an on-chain event of a new challenge seed being generated. The task performs the following actions:
/// - Derives forest challenges from the seed.
/// - Checks for checkpoint challenges and adds them to the forest challenges.
/// - Generates proofs for the challenges.
/// - Constructs key proofs and submits the proof to the runtime.
/// - Applies any necessary mutations.
/// - Ensures the new Forest root matches the one on-chain.
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
        let mut forest_challenges = self
            .derive_forest_challenges_from_seed(seed, provider_id)
            .await?;
        trace!(target: LOG_TARGET, "Forest challenges to respond to: {:?}", forest_challenges);

        // Check if there are checkpoint challenges since last tick this provider submitted a proof for.
        // If so, this will add them to the forest challenges.
        let checkpoint_challenges = self
            .add_checkpoint_challenges_to_forest_challenges(provider_id, &mut forest_challenges)
            .await?;
        trace!(target: LOG_TARGET, "Checkpoint challenges to respond to: {:?}", checkpoint_challenges);

        // TODO: In the near future, from here onwards we should be using the locking mechanism so that only
        // TODO: one task at a time can be sending Forest-related transactions to the runtime.
        // Get a read lock on the forest storage to generate a proof for the file.
        let read_forest_storage = self.storage_hub_handler.forest_storage.read().await;
        let proven_file_keys = read_forest_storage
            .generate_proof(forest_challenges)
            .map_err(|e| anyhow!("Failed to generate forest proof: {:?}", e))?;
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
        trace!(target: LOG_TARGET, "Proven file keys: {:?}", proven_keys);

        // Construct key challenges and generate key proofs for them.
        let mut key_proofs = KeyProofs::new();
        for file_key in &proven_keys {
            // Generate the key proof for each file key.
            let key_proof = self
                .generate_key_proof(*file_key, seed, provider_id)
                .await?;

            // Convert the file key to the runtime's hasher output type.
            // Although redundant in reality, this is done because technically the type of `file_key` is
            // a `HasherOutT<T>` and not a `H256`, which is what the runtime expects.
            let file_key = H256::from_slice(file_key.as_ref());
            key_proofs.insert(file_key, key_proof);
        }

        // Construct full proof.
        let proof = StorageProof {
            forest_proof: proven_file_keys.proof,
            key_proofs,
        };

        // Submit proof to the runtime.
        // Provider is `None` since we're submitting with the account linked to the BSP.
        let call = storage_hub_runtime::RuntimeCall::ProofsDealer(
            pallet_proofs_dealer::Call::submit_proof {
                proof,
                provider: None,
            },
        );

        // Attempt three times to submit extrinsic if it fails.
        let mut extrinsic_submitted = false;
        for attempt in 0..MAX_PROOF_SUBMISSION_ATTEMPTS {
            let mut transaction = self
                .storage_hub_handler
                .blockchain
                .send_extrinsic(call.clone())
                .await?
                .with_timeout(Duration::from_secs(60));

            if transaction
                .watch_for_success(&self.storage_hub_handler.blockchain)
                .await
                .is_ok()
            {
                // TODO: Wait for finality of the extrinsic.

                extrinsic_submitted = true;
                break;
            }

            warn!(target: LOG_TARGET, "Failed to submit proof, attempt #{}", attempt + 1);
        }

        // Exit with error if extrinsic was not submitted.
        if !extrinsic_submitted {
            error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to submit proof after {} attempts", MAX_PROOF_SUBMISSION_ATTEMPTS);
            return Err(anyhow!(
                "Failed to submit proof after {} attempts",
                MAX_PROOF_SUBMISSION_ATTEMPTS
            ));
        }

        trace!(target: LOG_TARGET, "Proof submitted successfully");

        // Apply mutations, if any.
        let mut mutations_applied = false;
        for (file_key, maybe_mutation) in &checkpoint_challenges {
            if proven_keys.contains(file_key) {
                // If the file key is proven, it means that this provider had an exact match for a checkpoint challenge.
                trace!(target: LOG_TARGET, "Checkpoint challenge proven with exact match for file key: {:?}", file_key);

                if let Some(mutation) = maybe_mutation {
                    // If the mutation (which is a remove mutation) is Some and the file key was proven exactly,
                    // then the mutation needs to be applied (i.e. the file key is removed from the Forest).
                    trace!(target: LOG_TARGET, "Applying mutation: {:?}", mutation);

                    self.remove_file(file_key).await?;
                    mutations_applied = true;
                }
            }
        }

        if mutations_applied {
            trace!(target: LOG_TARGET, "Mutations applied successfully");

            // Check that the new Forest root matches the one on-chain.
            self.check_provider_root(provider_id).await?;
        }

        Ok(())
    }
}

impl<T, FL, FS> BspSubmitProofTask<T, FL, FS>
where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    async fn derive_forest_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProviderId,
    ) -> anyhow::Result<Vec<HasherOutT<T>>> {
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

        Ok(converted_forest_challenges)
    }

    async fn add_checkpoint_challenges_to_forest_challenges(
        &self,
        provider_id: ProviderId,
        forest_challenges: &mut Vec<HasherOutT<T>>,
    ) -> anyhow::Result<Vec<(HasherOutT<T>, Option<TrieRemoveMutation>)>> {
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

        // If there were checkpoint challenges since the last tick this provider submitted a proof for,
        // get the checkpoint challenges.
        if last_tick_provided_submitted_proof <= last_checkpoint_tick {
            let checkpoint_challenges = self
                .storage_hub_handler
                .blockchain
                .query_last_checkpoint_challenges(last_checkpoint_tick)
                .await
                .map_err(|e| anyhow!("Failed to query last checkpoint challenges: {:?}", e))?;

            let mut converted_checkpoint_challenges: Vec<(
                HasherOutT<T>,
                Option<TrieRemoveMutation>,
            )> = Vec::new();
            for challenge in checkpoint_challenges {
                let raw_key: [u8; 32] = challenge.0.into();
                match raw_key.try_into() {
                    Ok(key) => converted_checkpoint_challenges.push((key, challenge.1)),
                    Err(_) => {
                        let error_msg = "Failed to challenge key to hasher output. This should not be possible, as the challenge keys are hasher outputs.";
                        error!(target: LOG_TARGET, error_msg);
                        return Err(anyhow!(error_msg));
                    }
                }
            }

            // Add the checkpoint challenges to the forest challenges.
            forest_challenges.extend(converted_checkpoint_challenges.iter().map(|(key, _)| *key));

            // Return the checkpoint challenges.
            return Ok(converted_checkpoint_challenges);
        } else {
            // Else, return an empty checkpoint challenges vector.
            return Ok(Vec::new());
        }
    }

    async fn generate_key_proof(
        &self,
        file_key: HasherOutT<T>,
        seed: RandomnessOutput,
        provider_id: ProviderId,
    ) -> anyhow::Result<KeyProof> {
        // Get the metadata for the file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let metadata = read_file_storage
            .get_metadata(&file_key)
            .map_err(|e| anyhow!("File metadata not found: {:?}", e))?;
        // Release the file storage read lock as soon as possible.
        drop(read_file_storage);

        // Calculate the number of challenges for this file.
        let challenge_count = metadata.chunks_to_check();

        // Generate the challenges for this file.
        let file_key_challenges = self
            .storage_hub_handler
            .blockchain
            .query_challenges_from_seed(seed, provider_id, challenge_count)
            .await?;

        // Convert the challenges to chunk IDs.
        let chunks_count = metadata.chunks_count();
        let chunks_to_prove = file_key_challenges
            .iter()
            .map(|challenge| ChunkId::from_challenge(challenge.as_ref(), chunks_count))
            .collect::<Vec<_>>();

        // Construct file key proofs for the challenges.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let file_key_proof = read_file_storage
            .generate_proof(&file_key, &chunks_to_prove)
            .map_err(|e| anyhow!("File is not in storage, or proof does not exist: {:?}", e))?;
        // Release the file storage read lock as soon as possible.
        drop(read_file_storage);

        // Return the key proof.
        Ok(KeyProof {
            proof: file_key_proof,
            challenge_count,
        })
    }

    async fn remove_file(&self, file_key: &HasherOutT<T>) -> anyhow::Result<()> {
        // Remove the file key from the Forest.
        let mut write_forest_storage = self.storage_hub_handler.forest_storage.write().await;
        write_forest_storage.delete_file_key(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to apply mutation to Forest storage. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
            anyhow!(
                "Failed to remove file key from Forest storage: {:?}",
                e
            )
        })?;
        // Release the forest storage write lock.
        drop(write_forest_storage);

        // TODO: This should actually be done after waiting for finality of the extrinsic.
        // Remove the file from the File Storage.
        let mut write_file_storage = self.storage_hub_handler.file_storage.write().await;
        write_file_storage.delete_file(file_key).map_err(|e| {
            error!(target: LOG_TARGET, "Failed to remove file from File Storage after it was removed from the Forest. \nError: {:?}", e);
            anyhow!(
                "Failed to delete file from File Storage after it was removed from the Forest: {:?}",
                e
            )
        })?;
        // Release the file storage write lock.
        drop(write_file_storage);

        Ok(())
    }

    async fn check_provider_root(&self, provider_id: ProviderId) -> anyhow::Result<()> {
        // Get root for this provider according to the runtime.
        let onchain_root = self
            .storage_hub_handler
            .blockchain
            .query_provider_forest_root(provider_id)
            .await
            .map_err(|e| {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to query provider root from runtime after successfully submitting proof. This may result in a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team. \nError: {:?}", e);
                anyhow!(
                    "Failed to query provider root from runtime after successfully submitting proof: {:?}",
                    e
                )
            })?;

        trace!(target: LOG_TARGET, "Provider root according to runtime: {:?}", onchain_root);

        // Check that the new Forest root matches the one on-chain.
        let read_forest_storage = self.storage_hub_handler.forest_storage.read().await;
        let root = read_forest_storage.root();
        // Release the forest storage read lock.
        drop(read_forest_storage);

        // Convert the root to H256 for comparison.
        let root = H256::from_slice(root.as_ref());

        trace!(target: LOG_TARGET, "Provider root according to Forest Storage: {:?}", root);

        if root != onchain_root {
            error!(target: LOG_TARGET, "CRITICAL❗️❗️ Applying mutations yielded different root than the one on-chain. This means that there is a mismatch between the Forest root on-chain and in this node. \nThis is a critical bug. Please report it to the StorageHub team.");
            return Err(anyhow!(
                "Applying mutations yielded different root than the one on-chain."
            ));
        }

        Ok(())
    }
}

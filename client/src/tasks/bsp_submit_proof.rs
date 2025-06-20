use std::{collections::HashSet, future::Future, pin::Pin, sync::Arc, time::Duration};

use anyhow::anyhow;
use sc_tracing::tracing::*;
use shc_file_manager::traits::FileStorage;
use shp_file_metadata::ChunkId;
use sp_core::H256;

use shc_actors_framework::{actor::ActorHandle, event_bus::EventHandler};
use shc_blockchain_service::{
    commands::{BlockchainServiceCommandInterface, BlockchainServiceCommandInterfaceExt},
    events::{
        FinalisedTrieRemoveMutationsApplied, MultipleNewChallengeSeeds, ProcessSubmitProofRequest,
    },
    types::{RetryStrategy, SendExtrinsicOptions, SubmitProofRequest, WatchTransactionError},
    BlockchainService,
};
use shc_common::traits::{StorageEnableApiCollection, StorageEnableRuntimeApi};
use shc_common::{
    consts::CURRENT_FOREST_KEY,
    types::{
        BlockNumber, CustomChallenge, FileKey, ForestRoot, KeyProof, KeyProofs,
        ProofsDealerProviderId, Proven, RandomnessOutput, StorageProof,
    },
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};

use crate::{
    handler::StorageHubHandler,
    types::{BspForestStorageHandlerT, ShNodeType},
};

const LOG_TARGET: &str = "bsp-submit-proof-task";

/// Configuration for the BspSubmitProofTask
#[derive(Debug, Clone)]
pub struct BspSubmitProofConfig {
    /// Maximum number of attempts to submit a proof
    pub max_submission_attempts: u32,
}

impl Default for BspSubmitProofConfig {
    fn default() -> Self {
        Self {
            max_submission_attempts: 5, // Default value that was in command.rs
        }
    }
}

/// BSP Submit Proof Task: Handles the submission of proof for BSP (Backup Storage Provider) to the runtime.
///
/// The flow includes the following steps:
/// - **[`MultipleNewChallengeSeeds`] Event:**
///   - Triggered by the on-chain generation of a new challenge seed.
///   - For each seed:
///     - Derives forest challenges from the seed.
///     - Checks for any checkpoint challenges and adds them to the forest challenges.
///     - Queues the challenges for submission to the runtime, to be processed when the Forest write lock is released.
///
/// - **[`ProcessSubmitProofRequest`] Event:**
///   - Triggered when the Blockchain Service detects that the Forest write lock has been released.
///   - Generates proofs for the queued challenges derived from the seed in the [`MultipleNewChallengeSeeds`] event.
///   - Constructs key proofs for each file key involved in the challenges.
///   - Submits the proofs to the runtime, with up to [`MAX_PROOF_SUBMISSION_ATTEMPTS`] retries on failure.
///   - Applies any necessary mutations to the Forest Storage (but not the File Storage).
///   - Verifies that the new Forest root matches the one recorded on-chain to ensure consistency.
///
/// - **[`FinalisedTrieRemoveMutationsApplied`] Event:**
///   - Triggered when mutations applied to the Merkle Trie have been finalized, indicating that certain keys should be removed.
///   - Iterates over each file key that was part of the finalised mutations.
///   - Checks if the file key is still present in the Forest Storage:
///     - If the key is still present, logs a warning, as this may indicate that the key was re-added after deletion.
///     - If the key is absent from the Forest Storage, safely removes the corresponding file from the File Storage.
///   - Ensures that no residual file keys remain in the File Storage when they should have been deleted.
pub struct BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    storage_hub_handler: StorageHubHandler<NT, RuntimeApi>,
    /// Configuration for this task
    config: BspSubmitProofConfig,
}

impl<NT, RuntimeApi> Clone for BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    fn clone(&self) -> BspSubmitProofTask<NT, RuntimeApi> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            config: self.config.clone(),
        }
    }
}

impl<NT, RuntimeApi> BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, RuntimeApi>) -> Self {
        Self {
            storage_hub_handler: storage_hub_handler.clone(),
            config: storage_hub_handler.provider_config.bsp_submit_proof.clone(),
        }
    }
}

/// Handles the [`MultipleNewChallengeSeeds`] event.
///
/// This event is triggered when catching up to proof submissions, and there are multiple new challenge seeds
/// that have to be responded in order. It queues the proof submissions for the given seeds.
/// The task performs the following actions for each seed:
/// - Derives forest challenges from the seed.
/// - Checks for checkpoint challenges and adds them to the forest challenges.
/// - Queues the challenges for submission to the runtime, for when the Forest write lock is released.
impl<NT, RuntimeApi> EventHandler<MultipleNewChallengeSeeds> for BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_event(&mut self, event: MultipleNewChallengeSeeds) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Initiating BSP multiple proof submissions for BSP ID: {:?}, with seeds: {:?}",
            event.provider_id,
            event.seeds
        );

        for seed in event.seeds {
            let provider_id = event.provider_id;
            let tick = seed.0;
            let seed = seed.1;
            self.queue_submit_proof_request(provider_id, tick, seed)
                .await?;
        }

        Ok(())
    }
}

/// Handles the [`ProcessSubmitProofRequest`] event.
///
/// This event is triggered when the Blockchain Service realises that the Forest write lock has been released,
/// giving this task the opportunity to generate proofs and submit them to the runtime.
///
/// This task performs the following actions:
/// - Generates proofs for the challenges.
/// - Constructs key proofs and submits the proof to the runtime.
///   - Retries up to [`MAX_PROOF_SUBMISSION_ATTEMPTS`] times if the submission fails.
/// - Applies any necessary mutations to the Forest Storage (not the File Storage).
/// - Ensures the new Forest root matches the one on-chain.
impl<NT, RuntimeApi> EventHandler<ProcessSubmitProofRequest> for BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_event(&mut self, event: ProcessSubmitProofRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing SubmitProofRequest {:?}",
            event.data
        );

        if event.data.forest_challenges.is_empty() && event.data.checkpoint_challenges.is_empty() {
            warn!(target: LOG_TARGET, "No challenges to respond to. Skipping proof submission.");
            return Ok(());
        }

        // Acquire Forest root write lock. This prevents other Forest-root-writing tasks from starting while we are processing this task.
        // That is until we release the lock gracefully with the `release_forest_root_write_lock` method, or `forest_root_write_lock` is dropped.
        let forest_root_write_tx = match event.forest_root_write_tx.lock().await.take() {
            Some(tx) => tx,
            None => {
                error!(target: LOG_TARGET, "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken. This is a critical bug. Please report it to the StorageHub team.");
                return Err(anyhow!(
                    "CRITICAL❗️❗️ This is a bug! Forest root write tx already taken!"
                ));
            }
        };

        // Check if this proof is the next one to be submitted.
        // This is, for example, in case that this provider is trying to submit a proof for a tick that is not the next one to be submitted.
        // Exiting early in this case is important so that the provider doesn't get stuck trying to submit an outdated proof.
        Self::check_if_proof_is_outdated(&self.storage_hub_handler.blockchain, &event).await?;

        // Get the current Forest key of the Provider running this node.
        let current_forest_key = CURRENT_FOREST_KEY.to_vec();

        // Generate the Forest proof, i.e. the proof that some file keys belong to this Provider's Forest.
        let proven_file_keys = {
            let fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&current_forest_key)
                .await
                .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage."))?;

            let p = fs
                .read()
                .await
                .generate_proof(event.data.forest_challenges.clone())
                .map_err(|e| anyhow!("Failed to generate forest proof: {:?}", e))?;

            p
        };

        // Get the keys that were proven.
        let mut proven_keys = Vec::new();
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

        // Construct key challenges and generate key proofs for them.
        let mut key_proofs = KeyProofs::new();
        for file_key in &proven_keys {
            // If the file key is a checkpoint challenge for a file deletion, we should NOT generate a key proof for it.
            let should_generate_key_proof =
                !event.data.checkpoint_challenges.contains(&CustomChallenge {
                    key: *file_key,
                    should_remove_key: true,
                });

            if should_generate_key_proof {
                // Generate the key proof for each file key.
                let key_proof = self
                    .generate_key_proof(*file_key, event.data.seed, event.data.provider_id)
                    .await?;

                key_proofs.insert(*file_key, key_proof);
            };
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

        // We consider that the maximum tip we're willing to pay for the submission of the proof is
        // equal to the amount that this BSP would be slashed for, if the proof cannot be submitted.
        let max_tip = self
            .storage_hub_handler
            .blockchain
            .query_slash_amount_per_max_file_size()
            .await?
            .saturating_mul(event.data.forest_challenges.len() as u128)
            .saturating_mul(2u32.into());

        // Get necessary data for the retry check.
        let cloned_sh_handler = Arc::new(self.storage_hub_handler.clone());
        let cloned_event = Arc::new(event.clone());
        let cloned_forest_root = {
            let fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&current_forest_key)
                .await
                .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage."))?;
            let root = fs.read().await.root();
            root
        };

        // This function is a check to see if we should continue to retry the submission of the proof.
        // If this proof submission is invalid, we should not retry it, and release the forest write lock.
        let should_retry = move |error| {
            let cloned_sh_handler = Arc::clone(&cloned_sh_handler);
            let cloned_event = Arc::clone(&cloned_event);
            let cloned_forest_root = Arc::new(cloned_forest_root);

            // Check:
            // - If the proof is outdated.
            // - If the Forest root of the BSP has changed.
            Box::pin(Self::should_retry_submit_proof(
                cloned_sh_handler.clone(),
                cloned_event.clone(),
                cloned_forest_root.clone(),
                error,
            )) as Pin<Box<dyn Future<Output = bool> + Send>>
        };

        // Attempt to submit the extrinsic with retries and tip increase.
        self.storage_hub_handler
            .blockchain
            .submit_extrinsic_with_retry(
                call,
                SendExtrinsicOptions::new(Duration::from_secs(
                    self.storage_hub_handler
                        .provider_config
                        .blockchain_service
                        .extrinsic_retry_timeout,
                )),
                RetryStrategy::default()
                    .with_max_retries(self.config.max_submission_attempts)
                    .with_max_tip(max_tip as f64)
                    .with_should_retry(Some(Box::new(should_retry))),
                false,
            )
            .await
            .map_err(|e| {
                error!(target: LOG_TARGET, "❌ Failed to submit proof due to: {}", e);
                anyhow!("Failed to submit proof due to: {}", e)
            })?;

        trace!(target: LOG_TARGET, "Proof submitted successfully");

        // Release the forest root write "lock" and finish the task.
        self.storage_hub_handler
            .blockchain
            .release_forest_root_write_lock(forest_root_write_tx)
            .await
    }
}

/// Handles the [`FinalisedTrieRemoveMutationsApplied`] event.
///
/// This event is triggered when mutations applied to the Forest of this BSP have been finalised,
/// signalling that certain keys (representing files) should be removed from the File Storage if they are
/// not present in the Forest Storage. If the key is still present in the Forest Storage, it sends out
/// a warning, since it could indicate that the key has been re-added after being deleted.
///
/// This task performs the following actions:
/// - Iterates over each removed file key.
/// - Checks if the file key is present in the Forest Storage.
///   - If the key is still present, it logs a warning,
///     since this could indicate that the key has been re-added after being deleted.
///   - If the key is not present in the Forest Storage, it safely removes the key from the File Storage.
impl<NT, RuntimeApi> EventHandler<FinalisedTrieRemoveMutationsApplied>
    for BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType + 'static,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn handle_event(
        &mut self,
        event: FinalisedTrieRemoveMutationsApplied,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing finalised mutations applied for provider [{:?}] with mutations: {:?}",
            event.provider_id,
            event.mutations
        );

        // For each mutation...
        for mutation in event.mutations {
            let file_key = FileKey::from(mutation.0);

            // Check that the file_key is not in the Forest.
            let current_forest_key = CURRENT_FOREST_KEY.to_vec();
            let read_fs = self
                .storage_hub_handler
                .forest_storage_handler
                .get(&current_forest_key)
                .await
                .ok_or_else(|| anyhow!("CRITICAL❗️❗️ Failed to get forest storage."))?;
            if read_fs.read().await.contains_file_key(&file_key.into())? {
                warn!(
                    target: LOG_TARGET,
                    "TrieRemoveMutation applied and finalised for file key {:?}, but file key is still in Forest. This can only happen if the same file key was added again after deleted by the user.\n Mutation: {:?}",
                    file_key,
                    mutation
                );
            } else {
                // If file key is not in Forest, we can now safely remove it from the File Storage.
                self.remove_file_from_file_storage(&file_key.into()).await?;
            }
        }

        Ok(())
    }
}

impl<NT, RuntimeApi> BspSubmitProofTask<NT, RuntimeApi>
where
    NT: ShNodeType,
    NT::FSH: BspForestStorageHandlerT,
    RuntimeApi: StorageEnableRuntimeApi,
    RuntimeApi::RuntimeApi: StorageEnableApiCollection,
{
    async fn queue_submit_proof_request(
        &self,
        provider_id: ProofsDealerProviderId,
        tick: BlockNumber,
        seed: RandomnessOutput,
    ) -> anyhow::Result<()> {
        trace!(target: LOG_TARGET, "Queueing submit proof request for provider [{:?}] with tick [{:?}] and seed [{:?}]", provider_id, tick, seed);

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

        self.storage_hub_handler
            .blockchain
            .queue_submit_proof_request(SubmitProofRequest::new(
                provider_id,
                tick,
                seed,
                forest_challenges,
                checkpoint_challenges,
            ))
            .await?;

        Ok(())
    }

    async fn derive_forest_challenges_from_seed(
        &self,
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
    ) -> anyhow::Result<Vec<H256>> {
        Ok(self
            .storage_hub_handler
            .blockchain
            .query_forest_challenges_from_seed(seed, provider_id)
            .await?)
    }

    async fn add_checkpoint_challenges_to_forest_challenges(
        &self,
        provider_id: ProofsDealerProviderId,
        forest_challenges: &mut Vec<H256>,
    ) -> anyhow::Result<Vec<CustomChallenge>> {
        let last_tick_provider_submitted_proof_for = self
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

        let challenges_tick = self
            .storage_hub_handler
            .blockchain
            .query_next_challenge_tick_for_provider(provider_id)
            .await
            .map_err(|e| anyhow!("Failed to get next challenge tick for provider: {:?}", e))?;

        // If there were checkpoint challenges since the last tick this provider submitted a proof for,
        // get the checkpoint challenges.
        if last_tick_provider_submitted_proof_for < last_checkpoint_tick
            && last_checkpoint_tick <= challenges_tick
        {
            let checkpoint_challenges = self
                .storage_hub_handler
                .blockchain
                .query_last_checkpoint_challenges(last_checkpoint_tick)
                .await
                .map_err(|e| anyhow!("Failed to query last checkpoint challenges: {:?}", e))?;

            // Add the checkpoint challenges to the forest challenges.
            forest_challenges.extend(
                checkpoint_challenges
                    .iter()
                    .map(|custom_challenge| custom_challenge.key),
            );

            // Return the checkpoint challenges.
            Ok(checkpoint_challenges)
        } else {
            // Else, return an empty checkpoint challenges vector.
            Ok(Vec::new())
        }
    }

    async fn check_if_proof_is_outdated(
        blockchain: &ActorHandle<BlockchainService<NT::FSH, RuntimeApi>>,
        event: &ProcessSubmitProofRequest,
    ) -> anyhow::Result<()> {
        // Get the next challenge tick for this provider.
        let next_challenge_tick = blockchain
            .query_next_challenge_tick_for_provider(event.data.provider_id)
            .await
            .map_err(|e| anyhow!("Failed to get next challenge tick for provider, to see if the proof is outdated: {:?}", e))?;

        if next_challenge_tick != event.data.tick {
            warn!(target: LOG_TARGET, "The proof for tick [{:?}] is not the next one to be submitted. Next challenge tick is [{:?}]", event.data.tick, next_challenge_tick);
            return Err(anyhow!(
                "The proof for tick [{:?}] is not the next one to be submitted.",
                event.data.tick,
            ));
        }

        Ok(())
    }

    async fn generate_key_proof(
        &self,
        file_key: H256,
        seed: RandomnessOutput,
        provider_id: ProofsDealerProviderId,
    ) -> anyhow::Result<KeyProof> {
        // Get the metadata for the file.
        let read_file_storage = self.storage_hub_handler.file_storage.read().await;
        let metadata = read_file_storage
            .get_metadata(&file_key)
            .map_err(|e| anyhow!("Error retrieving file metadata: {:?}", e))?
            .ok_or(anyhow!("File metadata not found!"))?;
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
            .generate_proof(&file_key, &HashSet::from_iter(chunks_to_prove))
            .map_err(|e| anyhow!("File is not in storage, or proof does not exist: {:?}", e))?;
        // Release the file storage read lock as soon as possible.
        drop(read_file_storage);

        // Return the key proof.
        Ok(KeyProof {
            proof: file_key_proof,
            challenge_count,
        })
    }

    async fn remove_file_from_file_storage(&self, file_key: &H256) -> anyhow::Result<()> {
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

    /// Function to determine if a proof submission should be retried,
    /// sending the same proof again.
    ///
    /// This function will return `true` if and only if the following conditions are met:
    /// 1. The error is a timeout. Otherwise, it means that the transaction was not successful,
    ///    in which case it is safer to let the BlockchainService eventually schedule a new
    ///    proof submission from scratch.
    /// 2. The proof is up to date, i.e. the Forest root has not changed, and the tick for
    ///    which the proof was generated is still the tick this Provider should submit a proof for.
    async fn should_retry_submit_proof(
        sh_handler: Arc<StorageHubHandler<NT, RuntimeApi>>,
        event: Arc<ProcessSubmitProofRequest>,
        forest_root: Arc<ForestRoot>,
        error: WatchTransactionError,
    ) -> bool {
        // We only retry sending THE SAME proof, if the error is a timeout.
        match error {
            WatchTransactionError::Timeout => {}
            _ => return false,
        }

        let current_forest_key = CURRENT_FOREST_KEY.to_vec();
        let is_proof_outdated = Self::check_if_proof_is_outdated(&sh_handler.blockchain, &event)
            .await
            .is_err();
        let has_forest_root_changed = {
            let fs = sh_handler
                .forest_storage_handler
                .get(&current_forest_key)
                .await;

            match fs {
                Some(fs) => fs.read().await.root() != *forest_root,
                None => {
                    error!(target: LOG_TARGET, "CRITICAL❗️❗️ Failed to get forest storage.");
                    true
                }
            }
        };

        // If the proof is outdated, or the Forest root has changed, we should not retry.
        if is_proof_outdated {
            warn!(target: LOG_TARGET, "❌ Proof to submit is outdated. Stop retrying.");
            return false;
        };
        if has_forest_root_changed {
            warn!(target: LOG_TARGET, "❌ Forest root has changed. Stop retrying.");
            return false;
        };

        true
    }
}

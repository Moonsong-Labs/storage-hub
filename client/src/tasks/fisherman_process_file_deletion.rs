use anyhow::anyhow;
use futures::future::{join_all, BoxFuture};
use hex;
use sc_tracing::tracing::*;
use shc_actors_framework::actor::ActorHandle;
use shc_actors_framework::event_bus::EventHandler;
use shc_blockchain_service::commands::BlockchainServiceCommandInterface;
use shc_blockchain_service::types::SendExtrinsicOptions;
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{
    Fingerprint, ForestProof as CommonForestProof, OffchainSignature,
    StorageProofsMerkleTrieLayout, StorageProviderId,
};
use shc_fisherman_service::events::{ProcessFileDeletionRequest, ProcessIncompleteStorageRequest};
use shc_fisherman_service::{FileKeyOperation, FishermanService, FishermanServiceCommand};
use shc_forest_manager::in_memory::InMemoryForestStorage;
use shc_forest_manager::traits::ForestStorage;
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;
use std::time::Duration;

use crate::{
    handler::StorageHubHandler,
    types::{FishermanForestStorageHandlerT, ShNodeType},
};

/// Data structure holding common file deletion information retrieved from database
struct FileDeletionData<Runtime: StorageEnableRuntime> {
    file_metadata: shc_common::types::FileMetadata,
    bsp_ids: Vec<shc_indexer_db::OnchainBspId>,
    bucket_target: shc_fisherman_service::events::FileDeletionTarget<Runtime>,
    file_account: Vec<u8>,
}

/// Fetch common file deletion data from indexer database
async fn fetch_file_deletion_data<NT, Runtime>(
    storage_hub_handler: &StorageHubHandler<NT, Runtime>,
    file_key: &shp_types::Hash,
) -> anyhow::Result<FileDeletionData<Runtime>>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    // Get indexer connection
    let indexer_db_pool = storage_hub_handler
        .indexer_db_pool
        .as_ref()
        .ok_or_else(|| anyhow!("Indexer is disabled but a file deletion event was received"))?;

    let mut conn = indexer_db_pool
        .get()
        .await
        .map_err(|e| anyhow!("Failed to get indexer connection: {:?}", e))?;

    // Get file and bucket info
    let file = shc_indexer_db::models::File::get_by_file_key(&mut conn, &file_key)
        .await
        .map_err(|e| anyhow!("Failed to get file from indexer: {:?}", e))?;

    let bucket = shc_indexer_db::models::Bucket::get_by_id(&mut conn, file.bucket_id)
        .await
        .map_err(|e| anyhow!("Failed to get bucket from indexer: {:?}", e))?;

    let file_metadata = file
        .to_file_metadata(bucket.onchain_bucket_id.clone())
        .map_err(|e| anyhow!("Failed to convert file to metadata: {:?}", e))?;

    // Query for BSPs storing this file
    let bsp_ids =
        shc_indexer_db::models::bsp::BspFile::get_bsps_for_file_key(&mut conn, file_key.as_ref())
            .await
            .map_err(|e| anyhow!("Failed to query BSPs for file: {:?}", e))?;

    drop(conn);

    let bucket_id_array: [u8; 32] = bucket
        .onchain_bucket_id
        .clone()
        .try_into()
        .map_err(|_| anyhow!("Invalid bucket ID length"))?;
    let bucket_target =
        shc_fisherman_service::events::FileDeletionTarget::BucketId(H256::from(bucket_id_array));

    Ok(FileDeletionData {
        file_metadata,
        bsp_ids,
        bucket_target,
        file_account: file.account,
    })
}

const LOG_TARGET: &str = "fisherman-process-file-deletion-task";
const LOG_TARGET_INCOMPLETE: &str = "fisherman-process-incomplete-storage-task";

pub struct FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    fisherman_service: ActorHandle<FishermanService<Runtime>>,
}

impl<NT, Runtime> Clone for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> FishermanProcessFileDeletionTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            fisherman_service: self.fisherman_service.clone(),
        }
    }
}

impl<NT, Runtime> FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        let fisherman_service = storage_hub_handler
            .fisherman
            .clone()
            .expect("FishermanProcessFileDeletionTask requires fisherman service handle");

        Self {
            storage_hub_handler,
            fisherman_service,
        }
    }
}

pub struct FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    storage_hub_handler: StorageHubHandler<NT, Runtime>,
    fisherman_service: ActorHandle<FishermanService<Runtime>>,
}

impl<NT, Runtime> Clone for FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> FishermanProcessIncompleteStorageTask<NT, Runtime> {
        Self {
            storage_hub_handler: self.storage_hub_handler.clone(),
            fisherman_service: self.fisherman_service.clone(),
        }
    }
}

impl<NT, Runtime> FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    pub fn new(storage_hub_handler: StorageHubHandler<NT, Runtime>) -> Self {
        let fisherman_service = storage_hub_handler
            .fisherman
            .clone()
            .expect("FishermanProcessIncompleteStorageTask requires fisherman service handle");

        Self {
            storage_hub_handler,
            fisherman_service,
        }
    }
}

impl<NT, Runtime> EventHandler<ProcessFileDeletionRequest<Runtime>>
    for FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(
        &mut self,
        event: ProcessFileDeletionRequest<Runtime>,
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing file deletion request for signed intention file key: {:?}",
            event.signed_file_operation_intention.file_key
        );

        let file_key = &event.signed_file_operation_intention.file_key;
        let signature = &event.signature;

        // Fetch common file deletion data
        let deletion_data = fetch_file_deletion_data(&self.storage_hub_handler, file_key).await?;

        // Create a vector of futures for parallel processing
        let mut deletion_futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = Vec::new();

        let event_ref = &event;
        let file_metadata_ref = &deletion_data.file_metadata;
        let file_account_ref = &deletion_data.file_account;

        // Clone self before moving into async blocks
        let self_clone = self.clone();

        deletion_futures.push(Box::pin(async move {
            self_clone
                .process_deletion_for_target(
                    event_ref,
                    file_key,
                    signature,
                    deletion_data.bucket_target,
                    file_metadata_ref,
                    file_account_ref,
                )
                .await
        }));

        // Process BSP targets in parallel
        for onchain_bsp_id in deletion_data.bsp_ids {
            // Convert OnchainBspId to H256 for the target
            let bsp_target =
                shc_fisherman_service::events::FileDeletionTarget::BspId(onchain_bsp_id.into());

            let self_clone = self.clone();

            deletion_futures.push(Box::pin(async move {
                self_clone
                    .process_deletion_for_target(
                        event_ref,
                        file_key,
                        signature,
                        bsp_target,
                        file_metadata_ref,
                        file_account_ref,
                    )
                    .await
            }));
        }

        // Execute all deletions in parallel and collect results
        let results = join_all(deletion_futures).await;

        // Check for any failures
        for result in results {
            result?;
        }

        Ok(())
    }
}

impl<NT, Runtime> EventHandler<ProcessIncompleteStorageRequest>
    for FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime> + 'static,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn handle_event(&mut self, event: ProcessIncompleteStorageRequest) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET_INCOMPLETE,
            "Processing incomplete storage request for file key: {:?}",
            event.file_key
        );

        let file_key = &event.file_key;

        // Fetch common file deletion data
        let deletion_data = fetch_file_deletion_data(&self.storage_hub_handler, file_key).await?;

        // Create a vector of futures for parallel processing
        let mut deletion_futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = Vec::new();

        let file_metadata_ref = &deletion_data.file_metadata;
        let file_account_ref = &deletion_data.file_account;

        // Clone self before moving into async blocks
        let self_clone = self.clone();

        deletion_futures.push(Box::pin(async move {
            self_clone
                .process_deletion_for_target_incomplete(
                    file_key,
                    deletion_data.bucket_target,
                    file_metadata_ref,
                    file_account_ref,
                )
                .await
        }));

        // Process BSP targets in parallel
        for onchain_bsp_id in deletion_data.bsp_ids {
            // Convert OnchainBspId to H256 for the target
            let bsp_target =
                shc_fisherman_service::events::FileDeletionTarget::BspId(onchain_bsp_id.into());

            let self_clone = self.clone();

            deletion_futures.push(Box::pin(async move {
                self_clone
                    .process_deletion_for_target_incomplete(
                        file_key,
                        bsp_target,
                        file_metadata_ref,
                        file_account_ref,
                    )
                    .await
            }));
        }

        // Execute all deletions in parallel and collect results
        let results = join_all(deletion_futures).await;

        // Check for any failures
        for result in results {
            result?;
        }

        Ok(())
    }
}

impl<NT, Runtime> FishermanProcessIncompleteStorageTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn process_deletion_for_target_incomplete(
        &self,
        file_key: &shp_types::Hash,
        deletion_target: shc_fisherman_service::events::FileDeletionTarget<Runtime>,
        file_metadata: &shc_common::types::FileMetadata,
        file_account: &[u8],
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET_INCOMPLETE,
            "Processing deletion for file key {:?} and target {:?} (incomplete storage)",
            file_key,
            deletion_target
        );

        // Use the common logic function to get parameters
        let (_file_owner, _bucket_id, _location, _size, _fingerprint, _provider_id, _forest_proof) =
            process_deletion_common(
                &self.storage_hub_handler,
                &self.fisherman_service,
                file_key,
                deletion_target,
                file_metadata,
                file_account,
            )
            .await?;

        info!(
            target: LOG_TARGET_INCOMPLETE,
            "All parameters ready for delete_file extrinsic (TODO: submit when PR #444 is merged)"
        );

        // TODO: When PR #444 is merged, submit the extrinsic here without requiring user signature
        // This will use a different extrinsic that doesn't require user signature for incomplete storage requests
        // The extrinsic parameters are already prepared above and can be used when the PR is merged

        Ok(())
    }
}

impl<NT, Runtime> FishermanProcessFileDeletionTask<NT, Runtime>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    async fn process_deletion_for_target(
        &self,
        event: &ProcessFileDeletionRequest<Runtime>,
        file_key: &shp_types::Hash,
        signature: &OffchainSignature<Runtime>,
        deletion_target: shc_fisherman_service::events::FileDeletionTarget<Runtime>,
        file_metadata: &shc_common::types::FileMetadata,
        file_account: &[u8],
    ) -> anyhow::Result<()> {
        info!(
            target: LOG_TARGET,
            "Processing deletion for file key {:?} and target {:?}",
            file_key,
            deletion_target
        );

        // Use the common logic function to get parameters
        let (file_owner, bucket_id, location, size, fingerprint, provider_id, forest_proof) =
            process_deletion_common(
                &self.storage_hub_handler,
                &self.fisherman_service,
                file_key,
                deletion_target,
                file_metadata,
                file_account,
            )
            .await?;

        // Log the signed intention file key for signed deletions
        info!(
            target: LOG_TARGET,
            "File key from signed intention: 0x{}",
            hex::encode(event.signed_file_operation_intention.file_key.as_ref() as &[u8])
        );

        info!(
            target: LOG_TARGET,
            "Submitting delete_file extrinsic"
        );

        // Build the delete_file extrinsic call
        let call = pallet_file_system::Call::<Runtime>::delete_file {
            file_owner: file_owner.clone(),
            signed_intention: event.signed_file_operation_intention.clone(),
            signature: signature.clone(),
            bucket_id,
            location: location
                .try_into()
                .map_err(|_| anyhow!("Location too long"))?,
            size,
            fingerprint: H256::from_slice(fingerprint.as_ref()),
            provider_id: match provider_id {
                StorageProviderId::BackupStorageProvider(id) => id,
                StorageProviderId::MainStorageProvider(id) => id,
            },
            forest_proof: forest_proof.proof,
        };

        // Submit the extrinsic
        self.storage_hub_handler
            .blockchain
            .send_extrinsic(
                call.into(),
                SendExtrinsicOptions::new(Duration::from_secs(60)),
            )
            .await
            .map_err(|e| {
                error!(
                    target: LOG_TARGET,
                    "Failed to submit delete_file extrinsic: {:?}", e
                );
                anyhow!("Failed to submit delete_file extrinsic: {:?}", e)
            })?;

        info!(
            target: LOG_TARGET,
            "Successfully submitted delete_file extrinsic for file key {:?}",
            file_key
        );

        Ok(())
    }
}

/// Common deletion processing logic shared between both task implementations
async fn process_deletion_common<NT, Runtime>(
    storage_hub_handler: &StorageHubHandler<NT, Runtime>,
    fisherman_service: &ActorHandle<FishermanService<Runtime>>,
    file_key: &shp_types::Hash,
    deletion_target: shc_fisherman_service::events::FileDeletionTarget<Runtime>,
    file_metadata: &shc_common::types::FileMetadata,
    file_account: &[u8],
) -> anyhow::Result<(
    <Runtime as frame_system::Config>::AccountId,
    H256,
    Vec<u8>,
    <Runtime as pallet_storage_providers::Config>::StorageDataUnit,
    Fingerprint,
    StorageProviderId<Runtime>,
    CommonForestProof<StorageProofsMerkleTrieLayout>,
)>
where
    NT: ShNodeType<Runtime>,
    NT::FSH: FishermanForestStorageHandlerT<Runtime>,
    Runtime: StorageEnableRuntime,
{
    // Extract file details
    let bucket_id = H256::from_slice(file_metadata.bucket_id());
    let location = file_metadata.location().to_vec();
    let size = file_metadata.file_size().saturated_into();
    let fingerprint = file_metadata.fingerprint();

    // Determine provider ID from deletion target
    let provider_id = match &deletion_target {
        shc_fisherman_service::events::FileDeletionTarget::BspId(bsp_id) => {
            StorageProviderId::BackupStorageProvider(*bsp_id)
        }
        shc_fisherman_service::events::FileDeletionTarget::BucketId(target_bucket_id) => {
            let msp_id = storage_hub_handler
                .blockchain
                .query_msp_id_of_bucket_id(*target_bucket_id)
                .await
                .map_err(|e| anyhow!("Failed to query MSP ID for bucket: {:?}", e))?
                .ok_or_else(|| anyhow!("No MSP found for bucket {:?}", target_bucket_id))?;
            StorageProviderId::MainStorageProvider(msp_id)
        }
    };

    // Generate forest proof using two-phase ephemeral trie construction
    let forest_proof = {
        let finalized_block = storage_hub_handler
            .blockchain
            .get_best_block_info()
            .await
            .map_err(|e| anyhow!("Failed to get finalized block info: {:?}", e))?
            .number;

        info!(
            target: LOG_TARGET,
            "Building ephemeral trie from finalized data at block {}",
            finalized_block
        );

        let indexer_db_pool = storage_hub_handler
            .indexer_db_pool
            .as_ref()
            .ok_or_else(|| anyhow!("Indexer is disabled but a file deletion event was received"))?;

        let mut conn = indexer_db_pool
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get indexer connection: {:?}", e))?;

        // Fetch all file keys for the deletion target from finalized data
        let all_file_keys = match &deletion_target {
            shc_fisherman_service::events::FileDeletionTarget::BspId(bsp_id) => {
                // Convert H256 to OnchainBspId for database query
                let onchain_bsp_id = shc_indexer_db::OnchainBspId::from(*bsp_id);
                shc_indexer_db::models::bsp::BspFile::get_all_file_keys_for_bsp(
                    &mut conn,
                    onchain_bsp_id,
                )
                .await
                .map_err(|e| anyhow!("Failed to get all file keys for BSP: {:?}", e))?
            }
            shc_fisherman_service::events::FileDeletionTarget::BucketId(bucket_id) => {
                shc_indexer_db::models::file::File::get_all_file_keys_for_bucket(
                    &mut conn,
                    bucket_id.as_ref(),
                )
                .await
                .map_err(|e| anyhow!("Failed to get all file keys for bucket: {:?}", e))?
            }
        };

        // Fetch all files and convert to FileMetadata
        let mut file_metadatas = Vec::new();
        for key in &all_file_keys {
            let file = shc_indexer_db::models::File::get_by_file_key(&mut conn, key)
                .await
                .map_err(|e| anyhow!("Failed to get file: {:?}", e))?;

            let bucket = shc_indexer_db::models::Bucket::get_by_id(&mut conn, file.bucket_id)
                .await
                .map_err(|e| anyhow!("Failed to get bucket: {:?}", e))?;

            let metadata = file
                .to_file_metadata(bucket.onchain_bucket_id)
                .map_err(|e| anyhow!("Failed to convert file to metadata: {:?}", e))?;

            file_metadatas.push(metadata);
        }

        drop(conn);

        info!(
            target: LOG_TARGET,
            "Building ephemeral trie with {} file keys from finalized data",
            all_file_keys.len(),
        );

        // Create ephemeral in-memory forest storage
        let mut ephemeral_forest = InMemoryForestStorage::<StorageProofsMerkleTrieLayout>::new();

        // Insert all file keys from finalized data
        <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
            StorageProofsMerkleTrieLayout,
            Runtime,
        >>::insert_files_metadata(&mut ephemeral_forest, &file_metadatas)
        .map_err(|e| anyhow!("Failed to insert file keys into ephemeral trie: {:?}", e))?;

        info!(
            target: LOG_TARGET,
            "Ephemeral trie built with root: {:?}",
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<StorageProofsMerkleTrieLayout, Runtime>>::root(&ephemeral_forest)
        );

        info!(
            target: LOG_TARGET,
            "Applying catch-up from block {} to best block",
            finalized_block
        );

        // Create oneshot channel for response
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Get file key changes since finalized block
        fisherman_service
            .send(FishermanServiceCommand::GetFileKeyChangesSinceBlock {
                from_block: finalized_block,
                provider: deletion_target.clone(),
                response_tx: tx,
            })
            .await;

        // Wait for response
        let file_key_changes = rx
            .await
            .map_err(|e| anyhow!("Failed to receive file key changes: {:?}", e))?
            .map_err(|e| anyhow!("Failed to get file key changes: {:?}", e))?;

        info!(
            target: LOG_TARGET,
            "Applying {} file key changes from catch-up",
            file_key_changes.len()
        );

        // Apply changes to the ephemeral trie
        for change in file_key_changes {
            match change.operation {
                FileKeyOperation::Add(metadata) => {
                    <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                        StorageProofsMerkleTrieLayout,
                        Runtime,
                    >>::insert_files_metadata(
                        &mut ephemeral_forest, &[metadata]
                    )
                    .map_err(|e| anyhow!("Failed to insert file key during catch-up: {:?}", e))?;
                }
                FileKeyOperation::Remove => {
                    // Remove the file key from the trie
                    let file_key = H256::from_slice(&change.file_key);
                    <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                        StorageProofsMerkleTrieLayout,
                        Runtime,
                    >>::delete_file_key(&mut ephemeral_forest, &file_key.into())
                    .map_err(|e| anyhow!("Failed to remove file key during catch-up: {:?}", e))?;
                }
            }
        }

        info!(
            target: LOG_TARGET,
            "Updated ephemeral trie root: {:?}",
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<StorageProofsMerkleTrieLayout, Runtime>>::root(&ephemeral_forest)
        );

        // Generate proof for the specific file key being deleted
        let forest_proof_result =
            <InMemoryForestStorage<StorageProofsMerkleTrieLayout> as ForestStorage<
                StorageProofsMerkleTrieLayout,
                Runtime,
            >>::generate_proof(&ephemeral_forest, vec![(*file_key).into()])
            .map_err(|e| anyhow!("Failed to generate forest proof: {:?}", e))?;

        forest_proof_result
    };

    // Validate and convert file owner account
    if file_account.len() != 32 {
        return Err(anyhow!(
            "Invalid file owner account ID length: expected 32 bytes, got {}",
            file_account.len()
        ));
    }
    let file_owner = <Runtime as frame_system::Config>::AccountId::try_from(file_account)
        .map_err(|_| anyhow!("Failed to convert file account to AccountId"))?;

    // Log all parameters
    info!(
        target: LOG_TARGET,
        "File deletion parameters prepared:
        - File owner: {:?}
        - File key: 0x{}
        - Bucket ID: {:?}
        - Location: 0x{}
        - Size: {} bytes
        - Fingerprint: {:?}
        - Provider ID: {:?}
        - Forest proof keys: {} items",
        file_owner,
        hex::encode(file_key.as_ref()),
        bucket_id,
        hex::encode(&location),
        size,
        fingerprint,
        provider_id,
        forest_proof.proof.encoded_nodes.len()
    );

    Ok((
        file_owner,
        bucket_id,
        location,
        size,
        fingerprint.clone(),
        provider_id,
        forest_proof,
    ))
}

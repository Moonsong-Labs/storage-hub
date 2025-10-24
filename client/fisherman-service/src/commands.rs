use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

use shc_actors_derive::actor_command;
use shc_actors_framework::actor::ActorHandle;
use shc_common::traits::StorageEnableRuntime;
use shc_common::types::{
    BackupStorageProviderId, BlockNumber, BucketId, FileMetadata, FileOperationIntention,
    OffchainSignature,
};
use shc_indexer_db::models::FileDeletionType;
use sp_core::H256;

use crate::{events::FileDeletionTarget, handler::FishermanService, FileKeyChange};

/// Contains all metadata required to process file deletion operations in batch mode.
/// Used by both user-initiated deletions (with signatures) and incomplete storage
/// requests (without signatures).
#[derive(Debug, Clone)]
pub struct FileDeletionData<Runtime: StorageEnableRuntime> {
    /// The file key (Merkle hash) uniquely identifying the file
    pub file_key: Runtime::Hash,
    /// File metadata (owner, bucket, location, size, fingerprint)
    pub file_metadata: FileMetadata,
    /// Decoded signature for user deletions, [`None`] for incomplete deletions
    pub signature: Option<OffchainSignature<Runtime>>,
    /// Reconstructed signed file operation intention (only for user deletions)
    pub signed_intention: Option<FileOperationIntention<Runtime>>,
}

/// Grouped pending deletions ready for batch processing.
///
/// Files are grouped by their deletion target (BSP or Bucket) to enable efficient
/// parallel processing of deletions. Each target can be processed independently
/// with its own forest proof.
#[derive(Debug, Clone)]
pub struct PendingDeletionsGrouped<Runtime: StorageEnableRuntime> {
    /// Files to delete from BSP forests, grouped by BSP ID
    pub bsp_deletions: HashMap<BackupStorageProviderId<Runtime>, Vec<FileDeletionData<Runtime>>>,
    /// Files to delete from bucket forests, grouped by bucket ID
    pub bucket_deletions: HashMap<BucketId<Runtime>, Vec<FileDeletionData<Runtime>>>,
}

/// Errors that can occur in the fisherman service
#[derive(Error, Debug)]
pub enum FishermanServiceError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    #[error("Blockchain client error: {0}")]
    Client(String),
    #[error("Events retrieval error: {0}")]
    EventsRetrieval(#[from] shc_common::blockchain_utils::EventsRetrievalError),
    #[error("Storage not found")]
    StorageNotFound,
    #[error("Decoding error: {0}")]
    DecodingError(String),
}

/// Commands that can be sent to the FishermanService actor
#[actor_command(
    service = FishermanService<Runtime: StorageEnableRuntime>,
    default_mode = "ImmediateResponse",
    default_error_type = FishermanServiceError,
)]
pub enum FishermanServiceCommand<Runtime: StorageEnableRuntime> {
    /// Get file key changes since a specific block for a given provider
    #[command(success_type = Vec<FileKeyChange>)]
    GetFileKeyChangesSinceBlock {
        /// The starting block (exclusive) - changes will be tracked from this block + 1
        from_block: BlockNumber<Runtime>,
        /// The provider to track changes for (BSP ID or Bucket ID)
        provider: FileDeletionTarget<Runtime>,
    },
    /// Query incomplete storage request metadata for a file key
    #[command(success_type = pallet_file_system_runtime_api::IncompleteStorageRequestMetadataResponse<
        Runtime::AccountId,
        shc_common::types::BucketId<Runtime>,
        shc_common::types::StorageDataUnit<Runtime>,
        Runtime::Hash,
        shc_common::types::BackupStorageProviderId<Runtime>,
    >)]
    QueryIncompleteStorageRequest {
        /// The file key to query
        file_key: H256,
    },
    /// Get all files pending deletion, grouped by target (BSP/Bucket).
    ///
    /// Queries the indexer database for files marked with `deletion_status = InProgress`,
    /// filtered by the specified deletion type:
    /// - [`FileDeletionType::User`]: Files with signatures (user-initiated deletions)
    /// - [`FileDeletionType::Incomplete`]: Files without signatures (system cleanup)
    ///
    /// Returns files grouped by their BSP and bucket targets to enable efficient
    /// batch processing.
    ///
    /// # Parameters
    /// * `deletion_type` - Type of deletion to query ([`FileDeletionType::User`] or [`FileDeletionType::Incomplete`])
    /// * `bucket_id` - Optional filter to only return files from a specific bucket
    /// * `bsp_id` - Optional filter to only return files from a specific BSP
    /// * `limit` - Maximum number of files to return (default: 1000)
    /// * `offset` - Number of files to skip for pagination (default: 0)
    #[command(success_type = PendingDeletionsGrouped<Runtime>)]
    GetPendingDeletions {
        /// Type of deletion to query
        deletion_type: FileDeletionType,
        /// Optional bucket ID to filter results
        bucket_id: Option<BucketId<Runtime>>,
        /// Optional BSP ID to filter results
        bsp_id: Option<BackupStorageProviderId<Runtime>>,
        /// Maximum number of files to return across all groups
        limit: Option<i64>,
        /// Number of files to skip for pagination
        offset: Option<i64>,
    },
}

/// Interface trait for interacting with the FishermanService actor.
///
/// This trait is automatically generated by the `actor_command` macro and provides
/// async methods corresponding to each command variant.
#[async_trait]
pub trait FishermanServiceCommandInterfaceExt<Runtime: StorageEnableRuntime>:
    FishermanServiceCommandInterface<Runtime>
{
    // Extension methods can be added here in the future if needed
}

/// Default implementation of the extension trait
#[async_trait]
impl<Runtime: StorageEnableRuntime> FishermanServiceCommandInterfaceExt<Runtime>
    for ActorHandle<FishermanService<Runtime>>
{
    // Extension method implementations would go here
}

use std::{
    collections::HashSet, fmt::Debug, marker::PhantomData, path::PathBuf, str::FromStr, sync::Arc,
};

use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    types::error::{ErrorObjectOwned as JsonRpseeError, INTERNAL_ERROR_CODE, INTERNAL_ERROR_MSG},
    Extensions,
};
use log::{debug, error, info};
use tokio::{
    fs,
    io::AsyncReadExt,
    sync::{mpsc, RwLock},
};
use tokio_stream::wrappers::ReceiverStream;

use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi as StorageProvidersRuntimeApi;
use sc_rpc_api::check_if_safe;
use shc_actors_framework::actor::ActorHandle;
use shc_common::{
    blockchain_utils::get_provider_id_from_keystore,
    consts::CURRENT_FOREST_KEY,
    traits::StorageEnableRuntime,
    types::{
        BlockHash, ChunkId, FileKey, FileKeyProof, FileMetadata, HashT, KeyProof, KeyProofs,
        OpaqueBlock, ProofsDealerProviderId, Proven, RandomnessOutput, StorageHubClient,
        StorageProof, StorageProofsMerkleTrieLayout, StorageProviderId, BCSV_KEY_TYPE,
    },
};
use shc_file_manager::traits::{ExcludeType, FileDataTrie, FileStorage, FileStorageError};
use shc_file_transfer_service::{
    commands::FileTransferServiceCommandInterface, FileTransferService,
};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use shp_constants::FILE_CHUNK_SIZE;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{sr25519::Pair as Sr25519Pair, Encode, Pair};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{Deserialize, KeyTypeId, Serialize};
use sp_runtime_interface::pass_by::PassByInner;

pub mod remote_file;
use remote_file::{RemoteFileConfig, RemoteFileHandlerFactory};

const LOG_TARGET: &str = "storage-hub-client-rpc";

/// RPC configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Remote file configuration options
    pub remote_file: RemoteFileConfig,
}

pub struct StorageHubClientRpcConfig<FL, FSH, Runtime>
where
    Runtime: StorageEnableRuntime,
{
    pub file_storage: Arc<RwLock<FL>>,
    pub forest_storage_handler: FSH,
    pub keystore: KeystorePtr,
    pub config: RpcConfig,
    pub file_transfer: ActorHandle<FileTransferService<Runtime>>,
    _runtime: PhantomData<Runtime>,
}

impl<FL, FSH: Clone, Runtime> Clone for StorageHubClientRpcConfig<FL, FSH, Runtime>
where
    Runtime: StorageEnableRuntime,
{
    fn clone(&self) -> Self {
        Self {
            file_storage: self.file_storage.clone(),
            forest_storage_handler: self.forest_storage_handler.clone(),
            keystore: self.keystore.clone(),
            config: self.config.clone(),
            file_transfer: self.file_transfer.clone(),
            _runtime: PhantomData,
        }
    }
}

impl<FL, FSH, Runtime> StorageHubClientRpcConfig<FL, FSH, Runtime>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler<Runtime> + Send + Sync,
    Runtime: StorageEnableRuntime,
{
    pub fn new(
        file_storage: Arc<RwLock<FL>>,
        forest_storage_handler: FSH,
        keystore: KeystorePtr,
        config: RpcConfig,
        file_transfer: ActorHandle<FileTransferService<Runtime>>,
    ) -> Self {
        Self {
            file_storage,
            forest_storage_handler,
            keystore,
            config,
            file_transfer,
            _runtime: PhantomData,
        }
    }

    pub fn with_remote_file_config(mut self, config: RemoteFileConfig) -> Self {
        self.config.remote_file = config;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointChallenge {
    pub file_key: shp_types::Hash,
    pub should_remove_file: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadFileInStorageResult {
    pub file_key: shp_types::Hash,
    pub file_metadata: FileMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncompleteFileStatus {
    pub file_metadata: FileMetadata,
    pub stored_chunks: u64,
    pub total_chunks: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SaveFileToDisk {
    FileNotFound,
    Success(FileMetadata),
    IncompleteFile(IncompleteFileStatus),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GetFileFromFileStorageResult {
    FileNotFound,
    IncompleteFile(IncompleteFileStatus),
    FileFound(FileMetadata),
    FileFoundWithInconsistency(FileMetadata),
}

/// Result of adding files to the forest storage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AddFilesToForestStorageResult {
    ForestNotFound,
    Success,
}

/// Result of removing files from the forest storage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RemoveFilesFromForestStorageResult {
    ForestNotFound,
    Success,
}

/// Result of getting the provider ID of the node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RpcProviderId {
    NotAProvider,
    Bsp(shp_types::Hash),
    Msp(shp_types::Hash),
}

/// Result of getting the value propositions of the node.
/// It returns a vector of the SCALE-encoded `ValuePropositionWithId`s.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GetValuePropositionsResult {
    Success(Vec<Vec<u8>>),
    NotAnMsp,
}

/// Provides an interface with the desired RPC method.
/// Used by the `rpc` macro from `jsonrpsee`
/// to generate the trait that is actually going to be implemented.
///
/// TODO: After adding maintenance mode, make some RPC calls (such as `remove_files_from_file_storage`)
/// only available in maintenance mode.
#[rpc(server, namespace = "storagehubclient")]
pub trait StorageHubClientApi {
    #[method(name = "loadFileInStorage", with_extensions)]
    async fn load_file_in_storage(
        &self,
        file_path: String,
        location: String,
        owner_account_id_hex: String,
        bucket_id: shp_types::Hash,
    ) -> RpcResult<LoadFileInStorageResult>;

    /// Remove a list of files from the file storage.
    ///
    /// This is useful to allow BSPs and MSPs to manually adjust their file storage to match
    /// the state of the network if any inconsistencies are found.
    #[method(name = "removeFilesFromFileStorage", with_extensions)]
    async fn remove_files_from_file_storage(&self, file_key: Vec<shp_types::Hash>)
        -> RpcResult<()>;

    /// Remove all files under a certain prefix from the file storage.
    ///
    /// This is useful to allow MSPs to manually adjust their file storage to match
    /// the state of the network if any inconsistencies are found, allowing them
    /// to remove all files that belong to a bucket without having to call `removeFileFromFileStorage`
    /// for each file.
    #[method(name = "removeFilesWithPrefixFromFileStorage", with_extensions)]
    async fn remove_files_with_prefix_from_file_storage(
        &self,
        prefix: shp_types::Hash,
    ) -> RpcResult<()>;

    #[method(name = "saveFileToDisk", with_extensions)]
    async fn save_file_to_disk(
        &self,
        file_key: shp_types::Hash,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk>;

    /// Add files to the forest storage under the given forest key.
    ///
    /// This allows BSPs and MSPs to add files manually to their forest storage to solve inconsistencies
    /// between their local state and their on-chain state (as represented by their root).
    ///
    /// In the case of an BSP node, the forest key is empty since it only maintains a single forest.
    /// In the case of an MSP node, the forest key is a bucket ID.
    #[method(name = "addFilesToForestStorage", with_extensions)]
    async fn add_files_to_forest_storage(
        &self,
        forest_key: Option<shp_types::Hash>,
        metadata_of_files_to_add: Vec<FileMetadata>,
    ) -> RpcResult<AddFilesToForestStorageResult>;

    /// Remove files from the forest storage under the given forest key.
    ///
    /// This allows BSPs and MSPs to remove files manually from their forest storage to solve inconsistencies
    /// between their local state and their on-chain state (as represented by their root).
    ///
    /// In the case of an BSP node, the forest key is empty since it only maintains a single forest.
    /// In the case of an MSP node, the forest key is a bucket ID.
    #[method(name = "removeFilesFromForestStorage", with_extensions)]
    async fn remove_files_from_forest_storage(
        &self,
        forest_key: Option<shp_types::Hash>,
        file_keys: Vec<shp_types::Hash>,
    ) -> RpcResult<RemoveFilesFromForestStorageResult>;

    /// Get the root hash of a forest.
    ///
    /// In the case of an BSP node, the forest key is empty since it only maintains a single forest.
    /// In the case of an MSP node, the forest key is a bucket id.
    #[method(name = "getForestRoot")]
    async fn get_forest_root(
        &self,
        forest_key: Option<shp_types::Hash>,
    ) -> RpcResult<Option<shp_types::Hash>>;

    /// Check if a forest storage is present for the given forest key.
    ///
    /// For RocksDB-backed storage, this checks if the directory exists on disk.
    /// For in-memory storage, this checks if the forest is registered.
    ///
    /// In the case of a BSP node, the forest key is empty since it only maintains a single forest.
    /// In the case of an MSP node, the forest key is a bucket id.
    #[method(name = "isForestStoragePresent")]
    async fn is_forest_storage_present(
        &self,
        forest_key: Option<shp_types::Hash>,
    ) -> RpcResult<bool>;

    #[method(name = "isFileInForest")]
    async fn is_file_in_forest(
        &self,
        forest_key: Option<shp_types::Hash>,
        file_key: shp_types::Hash,
    ) -> RpcResult<bool>;

    #[method(name = "isFileInFileStorage")]
    async fn is_file_in_file_storage(
        &self,
        file_key: shp_types::Hash,
    ) -> RpcResult<GetFileFromFileStorageResult>;

    #[method(name = "getFileMetadata")]
    async fn get_file_metadata(
        &self,
        forest_key: Option<shp_types::Hash>,
        file_key: shp_types::Hash,
    ) -> RpcResult<Option<FileMetadata>>;

    /// Check if this node is currently expecting to receive the given file key (i.e., it has been registered)
    #[method(name = "isFileKeyExpected")]
    async fn is_file_key_expected(&self, file_key: shp_types::Hash) -> RpcResult<bool>;

    // Note: this RPC method returns a Vec<u8> because the `ForestProof` struct is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    #[method(name = "generateForestProof")]
    async fn generate_forest_proof(
        &self,
        forest_key: Option<shp_types::Hash>,
        challenged_file_keys: Vec<shp_types::Hash>,
    ) -> RpcResult<Vec<u8>>;

    // Note: this RPC method returns a Vec<u8> because the `StorageProof` struct is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    // Note: This RPC method is only meant for nodes running a BSP.
    #[method(name = "generateProof")]
    async fn generate_proof(
        &self,
        provider_id: shp_types::Hash,
        seed: shp_types::Hash,
        checkpoint_challenges: Option<Vec<CheckpointChallenge>>,
    ) -> RpcResult<Vec<u8>>;

    // Note: this RPC method returns a Vec<u8> because the KeyVerifier Proof type is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    #[method(name = "generateFileKeyProofBspConfirm")]
    async fn generate_file_key_proof_bsp_confirm(
        &self,
        bsp_id: shp_types::Hash,
        file_key: shp_types::Hash,
    ) -> RpcResult<Vec<u8>>;

    // Note: this RPC method returns a Vec<u8> because the KeyVerifier Proof type is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    #[method(name = "generateFileKeyProofMspAccept")]
    async fn generate_file_key_proof_msp_accept(
        &self,
        msp_id: shp_types::Hash,
        file_key: shp_types::Hash,
    ) -> RpcResult<Vec<u8>>;

    #[method(name = "insertBcsvKeys", with_extensions)]
    async fn insert_bcsv_keys(&self, seed: Option<String>) -> RpcResult<String>;

    #[method(name = "removeBcsvKeys", with_extensions)]
    async fn remove_bcsv_keys(&self, keystore_path: String) -> RpcResult<()>;

    // Note: This RPC method allow BSP administrator to add a file to the exclude list (and later
    // buckets, users or file fingerprint). This method is required to call before deleting a file to
    // avoid re-uploading a file that has just been deleted.
    #[method(name = "addToExcludeList", with_extensions)]
    async fn add_to_exclude_list(
        &self,
        file_key: shp_types::Hash,
        exclude_type: String,
    ) -> RpcResult<()>;

    // Note: This RPC method allow BSP administrator to remove a file from the exclude list (allowing
    // the BSP to volunteer for this specific file key again). Later it will allow to remove from the exclude
    // list ban users, bucket or even file fingerprint.
    #[method(name = "removeFromExcludeList", with_extensions)]
    async fn remove_from_exclude_list(
        &self,
        file_key: shp_types::Hash,
        exclude_type: String,
    ) -> RpcResult<()>;

    // TODO: Remove this RPC method once legacy upload is deprecated
    /// Send a RemoteUploadDataRequest via the node's FileTransferService
    #[method(name = "receiveBackendFileChunks", with_extensions)]
    async fn receive_backend_file_chunks(
        &self,
        file_key: shp_types::Hash,
        file_key_proof: Vec<u8>,
    ) -> RpcResult<Vec<u8>>;

    /// Get the provider ID of the current node, if any
    #[method(name = "getProviderId", with_extensions)]
    async fn get_provider_id(&self) -> RpcResult<RpcProviderId>;

    /// Get the value propositions of the node if it's an MSP, or None if it's a BSP
    #[method(name = "getValuePropositions", with_extensions)]
    async fn get_value_propositions(&self) -> RpcResult<GetValuePropositionsResult>;
}

/// Stores the required objects to be used in our RPC method.
pub struct StorageHubClientRpc<FL, FSH, Runtime, Block>
where
    Runtime: StorageEnableRuntime,
{
    client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
    file_storage: Arc<RwLock<FL>>,
    forest_storage_handler: FSH,
    keystore: KeystorePtr,
    config: RpcConfig,
    file_transfer: ActorHandle<FileTransferService<Runtime>>,
    _block_marker: std::marker::PhantomData<Block>,
}

impl<FL, FSH, Runtime, Block> StorageHubClientRpc<FL, FSH, Runtime, Block>
where
    Runtime: StorageEnableRuntime,
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler<Runtime> + Send + Sync,
{
    pub fn new(
        client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
        storage_hub_client_rpc_config: StorageHubClientRpcConfig<FL, FSH, Runtime>,
    ) -> Self {
        Self {
            client,
            file_storage: storage_hub_client_rpc_config.file_storage,
            forest_storage_handler: storage_hub_client_rpc_config.forest_storage_handler,
            keystore: storage_hub_client_rpc_config.keystore,
            config: storage_hub_client_rpc_config.config,
            file_transfer: storage_hub_client_rpc_config.file_transfer,
            _block_marker: Default::default(),
        }
    }
}

/// Interface generated by the `rpc` macro from our `StorageHubClientApi` trait.
// TODO: Currently the UserSendsFile task will react to all runtime events triggered by
// file uploads, even if the file is not in its storage. So we need a way to inform the task
// to only react to its file.
#[async_trait]
impl<FL, FSH, Runtime> StorageHubClientApiServer
    for StorageHubClientRpc<FL, FSH, Runtime, OpaqueBlock>
where
    Runtime: StorageEnableRuntime,
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler<Runtime> + Send + Sync + 'static,
{
    async fn load_file_in_storage(
        &self,
        ext: &Extensions,
        file_path: String,
        location: String,
        owner_account_id_hex: String,
        bucket_id: shp_types::Hash,
    ) -> RpcResult<LoadFileInStorageResult> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        let owner_account_id_bytes = hex::decode(owner_account_id_hex).map_err(into_rpc_error)?;
        let owner =
            Runtime::AccountId::try_from(owner_account_id_bytes.as_slice()).map_err(|_| {
                into_rpc_error("Failed to convert owner account id bytes to Runtime's AccountId")
            })?;

        // Create file handler
        let remote_file_config = self.config.remote_file.clone();
        let (handler, url) =
            RemoteFileHandlerFactory::create_from_string(&file_path, remote_file_config)
                .map_err(|e| into_rpc_error(format!("Failed to create file handler: {:?}", e)))?;

        let mut stream = handler
            .download_file()
            .await
            .map_err(remote_file_error_to_rpc_error)?;

        // Instantiate an "empty" [`FileDataTrie`] so we can write the file chunks into it.
        let mut file_data_trie = self.file_storage.write().await.new_file_data_trie();
        // A chunk id is simply an integer index.
        let mut chunk_id: u64 = 0;

        // Read file in chunks of [`FILE_CHUNK_SIZE`] into buffer then push buffer into a vector.
        // Loops until EOF or until some error that is NOT `ErrorKind::Interrupted` is found.
        // If `ErrorKind::Interrupted` is found, the operation is simply retried, as per
        // https://doc.rust-lang.org/std/io/trait.Read.html#errors-1
        // Build the actual [`FileDataTrie`] by inserting each chunk into it.
        //
        // We need to ensure we read exactly FILE_CHUNK_SIZE bytes per chunk (except the last one)
        // to ensure consistent fingerprints regardless of how the underlying stream returns data.
        'read: loop {
            let mut chunk = vec![0u8; FILE_CHUNK_SIZE as usize];
            let mut offset = 0;

            // Keep reading until we fill the chunk or hit EOF
            while offset < FILE_CHUNK_SIZE as usize {
                match stream.read(&mut chunk[offset..]).await {
                    Ok(0) => {
                        // EOF reached
                        if offset > 0 {
                            // We have a partial chunk
                            chunk.truncate(offset);
                            debug!(target: LOG_TARGET, "Read final partial chunk of {} bytes", offset);

                            file_data_trie
                                .write_chunk(&ChunkId::new(chunk_id), &chunk)
                                .map_err(into_rpc_error)?;
                        }
                        debug!(target: LOG_TARGET, "Finished reading file");
                        break 'read;
                    }
                    Ok(bytes_read) => {
                        offset += bytes_read;
                        if offset == FILE_CHUNK_SIZE as usize {
                            // Full chunk
                            debug!(target: LOG_TARGET, "Read full chunk {} of {} bytes", chunk_id, FILE_CHUNK_SIZE);

                            file_data_trie
                                .write_chunk(&ChunkId::new(chunk_id), &chunk)
                                .map_err(into_rpc_error)?;
                            chunk_id += 1;
                            break; // Move to next chunk
                        }
                        // Continue reading to fill the chunk
                    }
                    Err(e) => {
                        error!(target: LOG_TARGET, "Error when trying to read file: {:?}", e);
                        return Err(into_rpc_error(format!(
                            "Error reading file stream: {:?}",
                            e
                        )));
                    }
                }
            }
        }

        // Generate the necessary metadata so we can insert file into the File Storage.
        let root = file_data_trie.get_root();

        // For local files, check if the file is empty
        // Remote files we allow the server to give us "0" as file size to support dynamic content
        let file_size = handler
            .get_file_size()
            .await
            .map_err(remote_file_error_to_rpc_error)?;

        if (url.scheme() == "" || url.scheme() == "file") && file_size == 0 {
            return Err(into_rpc_error(FileStorageError::FileIsEmpty));
        }

        // Build StorageHub's [`FileMetadata`]
        let file_metadata = FileMetadata::new(
            <Runtime::AccountId as AsRef<[u8]>>::as_ref(&owner).to_vec(),
            bucket_id.as_ref().to_vec(),
            location.clone().into(),
            file_size,
            root.as_ref().into(),
        )
        .map_err(into_rpc_error)?;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        // Acquire FileStorage write lock.
        let mut file_storage_write_lock = self.file_storage.write().await;

        // Finally store file in File Storage.
        file_storage_write_lock
            .insert_file_with_data(file_key, file_metadata.clone(), file_data_trie)
            .map_err(into_rpc_error)?;

        let result = LoadFileInStorageResult {
            file_key,
            file_metadata,
        };

        info!(target: LOG_TARGET, "File loaded successfully: {:?}", result);

        Ok(result)
    }

    async fn remove_files_from_file_storage(
        &self,
        ext: &Extensions,
        file_keys: Vec<shp_types::Hash>,
    ) -> RpcResult<()> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Acquire a write lock for the file storage.
        let mut write_file_storage = self.file_storage.write().await;

        // Remove the files from the file storage.
        for file_key in &file_keys {
            write_file_storage
                .delete_file(file_key)
                .map_err(into_rpc_error)?;

            info!(
                target: LOG_TARGET,
                "remove_files_from_file_storage successfully removed file with key=[{:x}] from file storage.",
                file_key
            );
        }

        info!(
            target: LOG_TARGET,
            "remove_files_from_file_storage finished. Successfully removed {} files from file storage.",
            file_keys.len()
        );

        Ok(())
    }

    async fn remove_files_with_prefix_from_file_storage(
        &self,
        ext: &Extensions,
        prefix: shp_types::Hash,
    ) -> RpcResult<()> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Acquire a write lock for the file storage.
        let mut write_file_storage = self.file_storage.write().await;

        // Remove all files with the given prefix from the file storage.
        write_file_storage
            .delete_files_with_prefix(&prefix.inner())
            .map_err(into_rpc_error)?;

        info!(
            target: LOG_TARGET,
            "remove_files_with_prefix_from_file_storage finished for prefix=[{:x}]. Successfully removed files with prefix from file storage.",
            prefix
        );

        Ok(())
    }

    async fn save_file_to_disk(
        &self,
        ext: &Extensions,
        file_key: shp_types::Hash,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Acquire FileStorage read lock to validate metadata and completeness.
        let read_file_storage = self.file_storage.read().await;

        // Retrieve file metadata from File Storage.
        let file_metadata = match read_file_storage
            .get_metadata(&file_key)
            .map_err(into_rpc_error)?
        {
            None => return Ok(SaveFileToDisk::FileNotFound),
            Some(metadata) => metadata,
        };

        // Check if file is incomplete.
        let stored_chunks = read_file_storage
            .stored_chunks_count(&file_key)
            .map_err(into_rpc_error)?;
        let total_chunks = file_metadata.chunks_count();

        if stored_chunks < total_chunks {
            return Ok(SaveFileToDisk::IncompleteFile(IncompleteFileStatus {
                file_metadata,
                stored_chunks,
                total_chunks,
            }));
        }

        // Release the read lock before performing the potentially long-running streaming operation.
        drop(read_file_storage);

        // Create file handler for writing to local or remote destination.
        let remote_file_config = self.config.remote_file.clone();
        let (handler, _url) =
            RemoteFileHandlerFactory::create_from_string(&file_path, remote_file_config)
                .map_err(|e| into_rpc_error(format!("Failed to create file handler: {:?}", e)))?;

        // Stream file chunks from FileStorage to the destination using a bounded channel.
        //
        // This avoids loading the entire file into memory:
        // - A small, bounded channel limits the number of in-flight chunks.
        // - Chunks are read from storage in small batches under a short-lived read lock.
        // - Backpressure from the upload side naturally slows down chunk production.
        //
        // The maximum default buffered size will be internal_buffer_size (default 1024) * FILE_CHUNK_SIZE (1Kb) = 1 Mb
        let queue_buffered_size = self.config.remote_file.internal_buffer_size;
        let (tx, rx) = mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(queue_buffered_size);

        let file_storage = Arc::clone(&self.file_storage);
        let file_key_clone = file_key.clone();

        // We read chunks in batches to amortize the cost of acquiring the read lock.
        // Note: we don't leave it locked as the download process velocity depends on the client receiving the file.
        let batch_size = queue_buffered_size as u64;

        // Channel Sender: reads chunks from FileStorage in batches and sends them through the channel.
        tokio::spawn(async move {
            let mut current_chunk: u64 = 0;

            while current_chunk < total_chunks {
                let batch_end =
                    std::cmp::min(total_chunks, current_chunk.saturating_add(batch_size));

                // Read a batch of chunks under a single read lock.
                let mut batch = Vec::with_capacity((batch_end - current_chunk) as usize);
                {
                    let read_storage = file_storage.read().await;
                    for idx in current_chunk..batch_end {
                        let chunk_id = ChunkId::new(idx);
                        match read_storage.get_chunk(&file_key_clone, &chunk_id) {
                            Ok(chunk) => batch.push(chunk),
                            Err(e) => {
                                // Propagate the error to the consumer and stop producing.
                                let _ = tx
                                    .send(Err(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        format!("Error reading chunk {idx}: {:?}", e),
                                    )))
                                    .await;
                                return;
                            }
                        }
                    }
                }

                // Send the batch to the consumer, backpressure ensured by the bounded channel.
                for chunk in batch {
                    if tx.send(Ok(bytes::Bytes::from(chunk))).await.is_err() {
                        // Consumer dropped (e.g., RPC cancelled or upload failed); stop producing.
                        return;
                    }
                }

                current_chunk = batch_end;
            }
        });

        let stream = ReceiverStream::new(rx);
        let reader = tokio_util::io::StreamReader::new(stream);
        let boxed_reader = Box::new(reader) as _;

        let file_size = file_metadata.file_size();
        // Write file data to destination (local or remote).
        handler
            .upload_file(boxed_reader, file_size, None)
            .await
            .map_err(|e| remote_file_error_to_rpc_error(e))?;

        info!(
            target: LOG_TARGET,
            "save_file_to_disk finished for file_key=[{:x}]. File saved to destination at path={} successfully.",
            file_key,
            file_path
        );

        Ok(SaveFileToDisk::Success(file_metadata))
    }

    async fn add_files_to_forest_storage(
        &self,
        ext: &Extensions,
        forest_key: Option<shp_types::Hash>,
        metadata_of_files_to_add: Vec<FileMetadata>,
    ) -> RpcResult<AddFilesToForestStorageResult> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        // Get the forest storage.
        let fs = match self.forest_storage_handler.get(&forest_key).await {
            Some(fs) => fs,
            None => return Ok(AddFilesToForestStorageResult::ForestNotFound),
        };

        // Acquire a write lock for the forest storage.
        let mut write_fs = fs.write().await;

        // Add the file keys to the forest storage.
        write_fs
            .insert_files_metadata(&metadata_of_files_to_add)
            .map_err(into_rpc_error)?;

        info!(
            target: LOG_TARGET,
            "add_files_to_forest_storage finished for forest_key=[{}]. Successfully added {} files.",
            hex::encode(forest_key),
            metadata_of_files_to_add.len()
        );

        Ok(AddFilesToForestStorageResult::Success)
    }

    async fn remove_files_from_forest_storage(
        &self,
        ext: &Extensions,
        forest_key: Option<shp_types::Hash>,
        file_keys: Vec<shp_types::Hash>,
    ) -> RpcResult<RemoveFilesFromForestStorageResult> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        // Get the forest storage.
        let fs = match self.forest_storage_handler.get(&forest_key).await {
            Some(fs) => fs,
            None => return Ok(RemoveFilesFromForestStorageResult::ForestNotFound),
        };

        // Acquire a write lock for the forest storage.
        let mut write_fs = fs.write().await;

        // Remove the file keys from the forest storage.
        for file_key in &file_keys {
            write_fs.delete_file_key(file_key).map_err(into_rpc_error)?;
        }

        info!(
            target: LOG_TARGET,
            "remove_files_from_forest_storage finished for forest_key=[{}]. Successfully removed {} files.",
            hex::encode(forest_key),
            file_keys.len()
        );

        Ok(RemoveFilesFromForestStorageResult::Success)
    }

    async fn get_forest_root(
        &self,
        forest_key: Option<shp_types::Hash>,
    ) -> RpcResult<Option<shp_types::Hash>> {
        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        // Return None if not found
        let maybe_root = match self.forest_storage_handler.get(&forest_key).await {
            Some(fs) => {
                let read_fs = fs.read().await;
                Some(read_fs.root())
            }
            None => None,
        };

        match maybe_root {
            Some(root) => {
                info!(
                    target: LOG_TARGET,
                    "get_forest_root successfully retrieved forest root for forest_key=[{}]. Root: [{:x}]",
                    hex::encode(forest_key),
                    root
                );
            }
            None => {
                info!(
                    target: LOG_TARGET,
                    "get_forest_root didn't find a forest root for forest_key=[{}]. Returning None.",
                    hex::encode(forest_key)
                );
            }
        }

        Ok(maybe_root)
    }

    async fn is_forest_storage_present(
        &self,
        forest_key: Option<shp_types::Hash>,
    ) -> RpcResult<bool> {
        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        let result = self
            .forest_storage_handler
            .is_forest_storage_present(&forest_key)
            .await;

        info!(
            target: LOG_TARGET,
            "is_forest_storage_present for forest_key=[{}]. Result: {}",
            hex::encode(forest_key),
            result
        );

        Ok(result)
    }

    async fn is_file_in_forest(
        &self,
        forest_key: Option<shp_types::Hash>,
        file_key: shp_types::Hash,
    ) -> RpcResult<bool> {
        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        let fs = self
            .forest_storage_handler
            .get(&forest_key)
            .await
            .ok_or_else(|| {
                into_rpc_error(format!("Forest storage not found for key {:?}", forest_key))
            })?;

        let read_fs = fs.read().await;
        let result = read_fs
            .contains_file_key(&file_key)
            .map_err(into_rpc_error)?;

        info!(
            target: LOG_TARGET,
            "is_file_in_forest finished for forest_key=[{}], file_key=[{:x}]. Result: {}",
            hex::encode(forest_key),
            file_key,
            result
        );

        Ok(result)
    }

    async fn is_file_in_file_storage(
        &self,
        file_key: shp_types::Hash,
    ) -> RpcResult<GetFileFromFileStorageResult> {
        // Acquire FileStorage read lock.
        let read_file_storage = self.file_storage.read().await;

        // See if the file metadata is in the File Storage.
        let result = match read_file_storage
            .get_metadata(&file_key)
            .map_err(into_rpc_error)?
        {
            None => GetFileFromFileStorageResult::FileNotFound,
            Some(file_metadata) => {
                let stored_chunks = read_file_storage
                    .stored_chunks_count(&file_key)
                    .map_err(into_rpc_error)?;
                let total_chunks = file_metadata.chunks_count();
                if stored_chunks < total_chunks {
                    GetFileFromFileStorageResult::IncompleteFile(IncompleteFileStatus {
                        file_metadata,
                        stored_chunks,
                        total_chunks,
                    })
                } else if stored_chunks > total_chunks {
                    GetFileFromFileStorageResult::FileFoundWithInconsistency(file_metadata)
                } else {
                    GetFileFromFileStorageResult::FileFound(file_metadata)
                }
            }
        };

        info!(
            target: LOG_TARGET,
            "is_file_in_file_storage finished for file_key=[{:x}]. Result: {:?}",
            file_key,
            result
        );

        Ok(result)
    }

    async fn get_file_metadata(
        &self,
        forest_key: Option<shp_types::Hash>,
        file_key: shp_types::Hash,
    ) -> RpcResult<Option<FileMetadata>> {
        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        let fs = self
            .forest_storage_handler
            .get(&forest_key)
            .await
            .ok_or_else(|| {
                into_rpc_error(format!("Forest storage not found for key {:?}", forest_key))
            })?;

        let read_fs = fs.read().await;
        let result = read_fs
            .get_file_metadata(&file_key)
            .map_err(into_rpc_error)?;

        info!(
            target: LOG_TARGET,
            "get_file_metadata finished for forest_key=[{}], file_key=[{:x}]. Result: {:?}",
            hex::encode(forest_key),
            file_key,
            result
        );

        Ok(result)
    }

    async fn is_file_key_expected(&self, file_key: shp_types::Hash) -> RpcResult<bool> {
        let expected = self
            .file_transfer
            .is_file_expected(file_key.into())
            .await
            .map_err(into_rpc_error)?;

        info!(
            target: LOG_TARGET,
            "is_file_key_expected finished for file_key=[0x{:x}]. Result: {}",
            file_key,
            expected
        );

        Ok(expected)
    }

    async fn generate_forest_proof(
        &self,
        forest_key: Option<shp_types::Hash>,
        challenged_file_keys: Vec<shp_types::Hash>,
    ) -> RpcResult<Vec<u8>> {
        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        let fs = self
            .forest_storage_handler
            .get(&forest_key)
            .await
            .ok_or_else(|| {
                into_rpc_error(format!("Forest storage not found for key {:?}", forest_key))
            })?;

        let read_fs = fs.read().await;
        let forest_proof = read_fs
            .generate_proof(challenged_file_keys)
            .map_err(into_rpc_error)?;

        let encoded = forest_proof.encode();

        info!(
            target: LOG_TARGET,
            "generate_forest_proof finished for forest_key=[{}].",
            hex::encode(forest_key),
        );

        Ok(encoded)
    }

    async fn generate_proof(
        &self,
        provider_id: shp_types::Hash,
        seed: shp_types::Hash,
        checkpoint_challenges: Option<Vec<CheckpointChallenge>>,
    ) -> RpcResult<Vec<u8>> {
        // TODO: Get provider ID itself.
        debug!(target: LOG_TARGET, "Checkpoint challenges: {:?}", checkpoint_challenges);

        // Getting Runtime APIs
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

        // Generate challenges from seed.
        let random_challenges = api
            .get_forest_challenges_from_seed(at_hash, &seed, &provider_id)
            .unwrap();

        // Merge custom challenges with random challenges.
        let challenges = if let Some(custom_challenges) = checkpoint_challenges.as_ref() {
            let mut challenged_keys = custom_challenges
                .iter()
                .map(|custom_challenge| custom_challenge.file_key)
                .collect::<Vec<_>>();

            challenged_keys.extend(random_challenges.into_iter());

            challenged_keys
        } else {
            random_challenges
        };

        // Generate the Forest proof in a closure to drop the read lock on the Forest Storage.
        let proven_file_keys = {
            // The Forest Key is an empty vector since this is a BSP, therefore it doesn't
            // have multiple Forest keys.
            let fs = self
                .forest_storage_handler
                .get(&Vec::new().into())
                .await
                .ok_or_else(|| {
                    into_rpc_error(
                        "Forest storage not found for empty key. Make sure you're running a BSP."
                            .to_string(),
                    )
                })?;

            let p = fs
                .read()
                .await
                .generate_proof(challenges)
                .map_err(into_rpc_error)?;

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
                        return Err(into_rpc_error(
                            "Both left and right leaves in forest proof are None. This should not be possible.",
                        ));
                    }
                },
                Proven::Empty => {
                    return Err(into_rpc_error(
                        "Forest proof generated with empty forest. This should not be possible, as this provider shouldn't have been challenged with an empty forest.",
                    ));
                }
            }
        }

        // Construct key challenges and generate key proofs for them.
        let mut key_proofs = KeyProofs::<Runtime>::new();
        for file_key in &proven_keys {
            // If the file key is a checkpoint challenge for a file deletion, we should NOT generate a key proof for it.
            let should_generate_key_proof = if let Some(checkpoint_challenges) =
                checkpoint_challenges.as_ref()
            {
                if checkpoint_challenges.contains(&CheckpointChallenge {
                    file_key: *file_key,
                    should_remove_file: true,
                }) {
                    debug!(target: LOG_TARGET, "File key [{:x}] is a checkpoint challenge for a file deletion", file_key);
                    false
                } else {
                    debug!(target: LOG_TARGET, "File key [{:x}] is not a checkpoint challenge for a file deletion", file_key);
                    true
                }
            } else {
                debug!(target: LOG_TARGET, "No checkpoint challenges provided");
                false
            };

            if should_generate_key_proof {
                // Generate the key proof for each file key.
                let key_proof = generate_key_proof(
                    self.client.clone(),
                    self.file_storage.clone(),
                    *file_key,
                    provider_id,
                    Some(seed),
                    None,
                    None,
                )
                .await?;

                key_proofs.insert(*file_key, key_proof);
            };
        }

        // Construct full proof.
        let proof = StorageProof::<Runtime> {
            forest_proof: proven_file_keys.proof.into(),
            key_proofs,
        };
        let encoded = proof.encode();

        info!(
            target: LOG_TARGET,
            "generate_proof finished for provider_id=[{:x}].",
            provider_id
        );

        Ok(encoded)
    }

    async fn generate_file_key_proof_bsp_confirm(
        &self,
        bsp_id: shp_types::Hash,
        file_key: shp_types::Hash,
    ) -> RpcResult<Vec<u8>> {
        // Getting Runtime APIs
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

        // Generate chunk IDs to prove to confirm the file
        let chunks_to_prove: Vec<ChunkId> = api
            .query_bsp_confirm_chunks_to_prove_for_file(at_hash, bsp_id, file_key)
            .unwrap()
            .unwrap();

        let key_proof = generate_key_proof::<_, Runtime>(
            self.client.clone(),
            self.file_storage.clone(),
            file_key,
            bsp_id,
            None,
            Some(at_hash),
            Some(chunks_to_prove),
        )
        .await?;

        let encoded = key_proof.proof.encode();

        info!(
            target: LOG_TARGET,
            "generate_file_key_proof_bsp_confirm finished for bsp_id=[{:x}], file_key=[{:x}].",
            bsp_id,
            file_key
        );

        Ok(encoded)
    }

    async fn generate_file_key_proof_msp_accept(
        &self,
        msp_id: shp_types::Hash,
        file_key: shp_types::Hash,
    ) -> RpcResult<Vec<u8>> {
        // Getting Runtime APIs
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

        // Generate chunk IDs to prove to accept the file
        let chunks_to_prove: Vec<ChunkId> = api
            .query_msp_confirm_chunks_to_prove_for_file(at_hash, msp_id, file_key)
            .unwrap()
            .unwrap();

        let key_proof = generate_key_proof::<_, Runtime>(
            self.client.clone(),
            self.file_storage.clone(),
            file_key,
            msp_id,
            None,
            Some(at_hash),
            Some(chunks_to_prove),
        )
        .await?;

        let encoded = key_proof.proof.encode();

        info!(
            target: LOG_TARGET,
            "generate_file_key_proof_msp_accept finished for msp_id=[{:x}], file_key=[{:x}].",
            msp_id,
            file_key
        );

        Ok(encoded)
    }

    // TODO: Add support for other signature schemes.
    // If a seed is provided, we manually generate and persist it into the file system.
    // In the case a seed is not provided, we delegate generation and insertion to `sr25519_generate_new`, which
    // internally uses the block number as a seed.
    // See https://paritytech.github.io/polkadot-sdk/master/sc_keystore/struct.LocalKeystore.html#method.sr25519_generate_new
    async fn insert_bcsv_keys(&self, ext: &Extensions, seed: Option<String>) -> RpcResult<String> {
        check_if_safe(ext)?;

        let seed = seed.as_deref();

        let new_pub_key = match seed {
            None => self
                .keystore
                .sr25519_generate_new(BCSV_KEY_TYPE, seed)
                .map_err(into_rpc_error)?,
            Some(seed) => {
                let new_pair = Sr25519Pair::from_string(seed, None).map_err(into_rpc_error)?;
                let new_pub_key = new_pair.public();
                self.keystore
                    .insert(BCSV_KEY_TYPE, seed, &new_pub_key)
                    .map_err(into_rpc_error)?;

                new_pub_key
            }
        };

        info!(
            target: LOG_TARGET,
            "insert_bcsv_keys finished, generated new public key."
        );

        Ok(new_pub_key.to_string())
    }

    // Deletes all files with keys of type BCSV from the Keystore.
    async fn remove_bcsv_keys(&self, ext: &Extensions, keystore_path: String) -> RpcResult<()> {
        check_if_safe(ext)?;

        let pub_keys = self.keystore.keys(BCSV_KEY_TYPE).map_err(into_rpc_error)?;
        let key_path = PathBuf::from(keystore_path);

        let total_keys = pub_keys.len();

        for pub_key in pub_keys {
            let mut key = key_path.clone();
            let key_name = key_file_name(&pub_key, BCSV_KEY_TYPE);
            key.push(key_name);

            // In case a key is not found we just ignore it
            // because there may be keys in memory that are not in the file system.
            let _ = fs::remove_file(key).await.map_err(|e| {
                error!(target: LOG_TARGET, "Failed to remove key: {:?}", e);
            });
        }

        info!(
            target: LOG_TARGET,
            "remove_bcsv_keys finished, attempted to remove {} keys.",
            total_keys
        );

        Ok(())
    }

    async fn add_to_exclude_list(
        &self,
        ext: &Extensions,
        file_key: shp_types::Hash,
        exclude_type: String,
    ) -> RpcResult<()> {
        check_if_safe(ext)?;

        let et = ExcludeType::from_str(&exclude_type).map_err(into_rpc_error)?;

        let mut write_file_storage = self.file_storage.write().await;
        write_file_storage
            .add_to_exclude_list(file_key, et)
            .map_err(into_rpc_error)?;

        drop(write_file_storage);

        info!(
            target: LOG_TARGET,
            "add_to_exclude_list finished for file_key=[{:x}], exclude_type={}",
            file_key,
            exclude_type
        );

        Ok(())
    }

    async fn remove_from_exclude_list(
        &self,
        ext: &Extensions,
        file_key: shp_types::Hash,
        exclude_type: String,
    ) -> RpcResult<()> {
        check_if_safe(ext)?;

        let et = ExcludeType::from_str(&exclude_type).map_err(into_rpc_error)?;

        let mut write_file_storage = self.file_storage.write().await;
        write_file_storage
            .remove_from_exclude_list(&file_key, et)
            .map_err(into_rpc_error)?;

        drop(write_file_storage);

        info!(
            target: LOG_TARGET,
            "remove_from_exclude_list finished for file_key=[{:x}], exclude_type={}",
            file_key,
            exclude_type
        );

        Ok(())
    }

    // TODO: Remove this RPC implementation once legacy upload is deprecated
    async fn receive_backend_file_chunks(
        &self,
        ext: &Extensions,
        file_key: shp_types::Hash,
        file_key_proof: Vec<u8>,
    ) -> RpcResult<Vec<u8>> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Parse inputs
        let file_key: FileKey = file_key.into();
        let proof: FileKeyProof = codec::Decode::decode(&mut &file_key_proof[..])
            .map_err(|e| into_rpc_error(format!("Failed to decode FileKeyProof: {:?}", e)))?;

        // Forward via FileTransferService's local `ReceiveBackendFileChunksRequest` command
        let (raw, _proto) = self
            .file_transfer
            .receive_backend_file_chunks_request(file_key, proof)
            .await
            .map_err(into_rpc_error)?;

        // Return the raw response
        info!(
            target: LOG_TARGET,
            "receive_backend_file_chunks finished for file_key=[{:x}].",
            file_key
        );

        Ok(raw)
    }

    async fn get_provider_id(&self, ext: &Extensions) -> RpcResult<RpcProviderId> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Derive the provider ID from the keystore.
        let at_hash = self.client.info().best_hash;
        let provider =
            get_provider_id_from_keystore::<Runtime>(&self.client, &self.keystore, &at_hash)
                .map_err(into_rpc_error)?;

        // Convert the provider ID to the expected format.
        let result = match provider {
            None => RpcProviderId::NotAProvider,
            Some(StorageProviderId::BackupStorageProvider(id)) => RpcProviderId::Bsp(id.into()),
            Some(StorageProviderId::MainStorageProvider(id)) => RpcProviderId::Msp(id.into()),
        };

        info!(
            target: LOG_TARGET,
            "get_provider_id finished with result={:?}",
            result
        );

        Ok(result)
    }

    async fn get_value_propositions(
        &self,
        ext: &Extensions,
    ) -> RpcResult<GetValuePropositionsResult> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Get the node's provider ID.
        let provider_id = self.get_provider_id(ext).await?;

        // Check if the node is an MSP, and extract its provider ID.
        if let RpcProviderId::Msp(msp_id) = provider_id {
            // If the node is indeed an MSP, get its value propositions.

            // First, get the runtime APIs.
            let api = self.client.runtime_api();
            let at_hash = self.client.info().best_hash;

            // Then, get the value propositions for the MSP and encode them.
            let value_propositions = api
                .query_value_propositions_for_msp(at_hash, &msp_id)
                .map_err(into_rpc_error)?
                .into_iter()
                .map(|vp| vp.encode())
                .collect::<Vec<_>>();

            let result = GetValuePropositionsResult::Success(value_propositions);

            info!(
                target: LOG_TARGET,
                "get_value_propositions finished for MSP=[{:x}].",
                msp_id
            );

            Ok(result)
        } else {
            info!(
                target: LOG_TARGET,
                "get_value_propositions called on a node that is not an MSP"
            );
            Ok(GetValuePropositionsResult::NotAnMsp)
        }
    }
}

/// Get the file name for the given public key and key type.
fn key_file_name(public: &[u8], key_type: KeyTypeId) -> PathBuf {
    let mut buf = PathBuf::new();
    let key_type = array_bytes::bytes2hex("", &key_type.0);
    let key = array_bytes::bytes2hex("", public);
    buf.push(key_type + key.as_str());
    buf
}

/// Converts into the expected kind of error for `jsonrpsee`'s `RpcResult<_>`.
fn into_rpc_error(e: impl Debug) -> JsonRpseeError {
    error!("into_rpc_error called with error: {:?}", e);
    JsonRpseeError::owned(
        INTERNAL_ERROR_CODE,
        INTERNAL_ERROR_MSG,
        Some(format!("{:?}", e)),
    )
}

/// Converts RemoteFileError into RPC error, preserving original IO error messages.
fn remote_file_error_to_rpc_error(e: remote_file::RemoteFileError) -> JsonRpseeError {
    JsonRpseeError::owned(
        INTERNAL_ERROR_CODE,
        INTERNAL_ERROR_MSG,
        Some(format!("{e}")),
    )
}

async fn generate_key_proof<FL, Runtime>(
    client: Arc<StorageHubClient<Runtime::RuntimeApi>>,
    file_storage: Arc<RwLock<FL>>,
    file_key: shp_types::Hash,
    provider_id: ProofsDealerProviderId<Runtime>,
    seed: Option<RandomnessOutput<Runtime>>,
    at: Option<BlockHash>,
    chunks_to_prove: Option<Vec<ChunkId>>,
) -> RpcResult<KeyProof<Runtime>>
where
    Runtime: StorageEnableRuntime,
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync + 'static,
{
    // Getting Runtime APIs
    let api = client.runtime_api();
    let at_hash = at.unwrap_or_else(|| client.info().best_hash);

    // Get the metadata for the file.
    let read_file_storage = file_storage.read().await;
    let metadata = read_file_storage
        .get_metadata(&file_key)
        .map_err(|e| into_rpc_error(format!("Error retrieving file metadata: {:?}", e)))?
        .ok_or_else(|| {
            into_rpc_error(format!("File metadata not found for key [{:x}]", file_key))
        })?;
    // Release the file storage read lock as soon as possible.
    drop(read_file_storage);

    // Calculate the number of challenges for this file.
    let challenge_count = metadata.chunks_to_check();

    // Get the chunks to prove.
    let chunks_to_prove = match chunks_to_prove {
        Some(chunks) => chunks,
        None => {
            // Generate the challenges for this file.
            let seed = seed.ok_or_else(|| {
                into_rpc_error("Seed is required to generate challenges if chunk IDs are missing")
            })?;
            let file_key_challenges = api
                .get_challenges_from_seed(at_hash, &seed, &provider_id, challenge_count)
                .map_err(|e| {
                    into_rpc_error(format!("Failed to generate challenges from seed: {:?}", e))
                })?;

            // Convert the challenges to chunk IDs.
            let chunks_count = metadata.chunks_count();
            file_key_challenges
                .iter()
                .map(|challenge| ChunkId::from_challenge(challenge.as_ref(), chunks_count))
                .collect::<Vec<_>>()
        }
    };

    // Construct file key proofs for the challenges.
    let read_file_storage = file_storage.read().await;
    let file_key_proof = read_file_storage
        .generate_proof(&file_key, &HashSet::from_iter(chunks_to_prove))
        .map_err(|e| {
            into_rpc_error(format!(
                "File is not in storage, or proof does not exist: {:?}",
                e
            ))
        })?;
    // Release the file storage read lock as soon as possible.
    drop(read_file_storage);

    // Return the key proof.
    Ok(KeyProof::<Runtime> {
        proof: file_key_proof,
        challenge_count,
    })
}

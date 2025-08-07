use std::{collections::HashSet, fmt::Debug, path::PathBuf, str::FromStr, sync::Arc};

use futures::StreamExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    types::error::{ErrorObjectOwned as JsonRpseeError, INTERNAL_ERROR_CODE, INTERNAL_ERROR_MSG},
    Extensions,
};
use log::{debug, error, info};
use tokio::{fs, io::AsyncReadExt, sync::RwLock};

use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use sc_rpc_api::check_if_safe;
use shc_common::{consts::CURRENT_FOREST_KEY, types::*};
use shc_file_manager::traits::{ExcludeType, FileDataTrie, FileStorage, FileStorageError};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{sr25519::Pair as Sr25519Pair, Encode, Pair, H256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{traits::Block as BlockT, AccountId32, Deserialize, KeyTypeId, Serialize};
use sp_runtime_interface::pass_by::PassByInner;

pub mod remote_file;
use remote_file::{RemoteFileConfig, RemoteFileHandlerFactory};

const LOG_TARGET: &str = "storage-hub-client-rpc";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointChallenge {
    pub file_key: H256,
    pub should_remove_file: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadFileInStorageResult {
    pub file_key: H256,
    pub file_metadata: FileMetadata,
}

/// RPC configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Remote file configuration options
    pub remote_file: RemoteFileConfig,
}

pub struct StorageHubClientRpcConfig<FL, FSH> {
    pub file_storage: Arc<RwLock<FL>>,
    pub forest_storage_handler: FSH,
    pub keystore: KeystorePtr,
    pub config: RpcConfig,
}

impl<FL, FSH: Clone> Clone for StorageHubClientRpcConfig<FL, FSH> {
    fn clone(&self) -> Self {
        Self {
            file_storage: self.file_storage.clone(),
            forest_storage_handler: self.forest_storage_handler.clone(),
            keystore: self.keystore.clone(),
            config: self.config.clone(),
        }
    }
}

impl<FL, FSH> StorageHubClientRpcConfig<FL, FSH>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Send + Sync,
{
    pub fn new(
        file_storage: Arc<RwLock<FL>>,
        forest_storage_handler: FSH,
        keystore: KeystorePtr,
        config: RpcConfig,
    ) -> Self {
        Self {
            file_storage,
            forest_storage_handler,
            keystore,
            config,
        }
    }

    pub fn with_remote_file_config(mut self, config: RemoteFileConfig) -> Self {
        self.remote_file_config = config;
        self
    }
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
        owner: AccountId32,
        bucket_id: H256,
    ) -> RpcResult<LoadFileInStorageResult>;

    /// Remove a list of files from the file storage.
    ///
    /// This is useful to allow BSPs and MSPs to manually adjust their file storage to match
    /// the state of the network if any inconsistencies are found.
    #[method(name = "removeFilesFromFileStorage", with_extensions)]
    async fn remove_files_from_file_storage(&self, file_key: Vec<H256>) -> RpcResult<()>;

    /// Remove all files under a certain prefix from the file storage.
    ///
    /// This is useful to allow MSPs to manually adjust their file storage to match
    /// the state of the network if any inconsistencies are found, allowing them
    /// to remove all files that belong to a bucket without having to call `removeFileFromFileStorage`
    /// for each file.
    #[method(name = "removeFilesWithPrefixFromFileStorage", with_extensions)]
    async fn remove_files_with_prefix_from_file_storage(&self, prefix: H256) -> RpcResult<()>;

    #[method(name = "saveFileToDisk", with_extensions)]
    async fn save_file_to_disk(
        &self,
        file_key: H256,
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
        forest_key: Option<H256>,
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
        forest_key: Option<H256>,
        file_keys: Vec<H256>,
    ) -> RpcResult<RemoveFilesFromForestStorageResult>;

    /// Get the root hash of a forest.
    ///
    /// In the case of an BSP node, the forest key is empty since it only maintains a single forest.
    /// In the case of an MSP node, the forest key is a bucket id.
    #[method(name = "getForestRoot")]
    async fn get_forest_root(&self, forest_key: Option<H256>) -> RpcResult<Option<H256>>;

    #[method(name = "isFileInForest")]
    async fn is_file_in_forest(&self, forest_key: Option<H256>, file_key: H256) -> RpcResult<bool>;

    #[method(name = "isFileInFileStorage")]
    async fn is_file_in_file_storage(
        &self,
        file_key: H256,
    ) -> RpcResult<GetFileFromFileStorageResult>;

    #[method(name = "getFileMetadata")]
    async fn get_file_metadata(
        &self,
        forest_key: Option<H256>,
        file_key: H256,
    ) -> RpcResult<Option<FileMetadata>>;

    // Note: this RPC method returns a Vec<u8> because the `ForestProof` struct is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    #[method(name = "generateForestProof")]
    async fn generate_forest_proof(
        &self,
        forest_key: Option<H256>,
        challenged_file_keys: Vec<H256>,
    ) -> RpcResult<Vec<u8>>;

    // Note: this RPC method returns a Vec<u8> because the `StorageProof` struct is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    // Note: This RPC method is only meant for nodes running a BSP.
    #[method(name = "generateProof")]
    async fn generate_proof(
        &self,
        provider_id: H256,
        seed: H256,
        checkpoint_challenges: Option<Vec<CheckpointChallenge>>,
    ) -> RpcResult<Vec<u8>>;

    // Note: this RPC method returns a Vec<u8> because the KeyVerifier Proof type is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    #[method(name = "generateFileKeyProofBspConfirm")]
    async fn generate_file_key_proof_bsp_confirm(
        &self,
        bsp_id: BackupStorageProviderId,
        file_key: H256,
    ) -> RpcResult<Vec<u8>>;

    // Note: this RPC method returns a Vec<u8> because the KeyVerifier Proof type is not serializable.
    // so we SCALE-encode it. The user of this RPC will have to decode it.
    #[method(name = "generateFileKeyProofMspAccept")]
    async fn generate_file_key_proof_msp_accept(
        &self,
        msp_id: MainStorageProviderId,
        file_key: H256,
    ) -> RpcResult<Vec<u8>>;

    #[method(name = "insertBcsvKeys", with_extensions)]
    async fn insert_bcsv_keys(&self, seed: Option<String>) -> RpcResult<String>;

    #[method(name = "removeBcsvKeys", with_extensions)]
    async fn remove_bcsv_keys(&self, keystore_path: String) -> RpcResult<()>;

    // Note: This RPC method allow BSP administrator to add a file to the exclude list (and later
    // buckets, users or file fingerprint). This method is required to call before deleting a file to
    // avoid re-uploading a file that has just been deleted.
    #[method(name = "addToExcludeList", with_extensions)]
    async fn add_to_exclude_list(&self, file_key: H256, exclude_type: String) -> RpcResult<()>;

    // Note: This RPC method allow BSP administrator to remove a file from the exclude list (allowing
    // the BSP to volunteer for this specific file key again). Later it will allow to remove from the exclude
    // list ban users, bucket or even file fingerprint.
    #[method(name = "removeFromExcludeList", with_extensions)]
    async fn remove_from_exclude_list(&self, file_key: H256, exclude_type: String)
        -> RpcResult<()>;
}

/// Stores the required objects to be used in our RPC method.
pub struct StorageHubClientRpc<FL, FSH, C, Block> {
    client: Arc<C>,
    file_storage: Arc<RwLock<FL>>,
    forest_storage_handler: FSH,
    keystore: KeystorePtr,
    config: RpcConfig,
    _block_marker: std::marker::PhantomData<Block>,
}

impl<FL, FSH, C, Block> StorageHubClientRpc<FL, FSH, C, Block>
where
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Send + Sync,
{
    pub fn new(
        client: Arc<C>,
        storage_hub_client_rpc_config: StorageHubClientRpcConfig<FL, FSH>,
    ) -> Self {
        Self {
            client,
            file_storage: storage_hub_client_rpc_config.file_storage,
            forest_storage_handler: storage_hub_client_rpc_config.forest_storage_handler,
            keystore: storage_hub_client_rpc_config.keystore,
            config: storage_hub_client_rpc_config.config,
            _block_marker: Default::default(),
        }
    }
}

/// Interface generated by the `rpc` macro from our `StorageHubClientApi` trait.
// TODO: Currently the UserSendsFile task will react to all runtime events triggered by
// file uploads, even if the file is not in its storage. So we need a way to inform the task
// to only react to its file.
#[async_trait]
impl<FL, FSH, C, Block> StorageHubClientApiServer for StorageHubClientRpc<FL, FSH, C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: ProofsDealerRuntimeApi<
            Block,
            ProofsDealerProviderId,
            BlockNumber,
            ForestLeaf,
            RandomnessOutput,
            CustomChallenge,
        > + FileSystemRuntimeApi<
            Block,
            BackupStorageProviderId,
            MainStorageProviderId,
            H256,
            BlockNumber,
            ChunkId,
            BucketId,
            StorageRequestMetadata,
        >,
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Send + Sync + 'static,
{
    async fn load_file_in_storage(
        &self,
        ext: &Extensions,
        file_path: String,
        location: String,
        owner: AccountId32,
        bucket_id: H256,
    ) -> RpcResult<LoadFileInStorageResult> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

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
            <AccountId32 as AsRef<[u8]>>::as_ref(&owner).to_vec(),
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
        file_keys: Vec<H256>,
    ) -> RpcResult<()> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Acquire a write lock for the file storage.
        let mut write_file_storage = self.file_storage.write().await;

        // Remove the files from the file storage.
        for file_key in file_keys {
            write_file_storage
                .delete_file(&file_key)
                .map_err(into_rpc_error)?;
        }

        Ok(())
    }

    async fn remove_files_with_prefix_from_file_storage(
        &self,
        ext: &Extensions,
        prefix: H256,
    ) -> RpcResult<()> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Acquire a write lock for the file storage.
        let mut write_file_storage = self.file_storage.write().await;

        // Remove all files with the given prefix from the file storage.
        write_file_storage
            .delete_files_with_prefix(&prefix.inner())
            .map_err(into_rpc_error)?;

        Ok(())
    }

    async fn save_file_to_disk(
        &self,
        ext: &Extensions,
        file_key: H256,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk> {
        // Check if the execution is safe.
        check_if_safe(ext)?;

        // Acquire FileStorage read lock.
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

        // Create file handler for writing to local or remote destination.
        let remote_file_config = self.config.remote_file.clone();
        let (handler, _url) =
            RemoteFileHandlerFactory::create_from_string(&file_path, remote_file_config)
                .map_err(|e| into_rpc_error(format!("Failed to create file handler: {:?}", e)))?;

        // TODO: Optimize memory usage for large file transfers
        // Current implementation loads all chunks into memory before streaming to remote location.
        // This can cause memory exhaustion for large files.
        //
        // Proposed solution: Implement true streaming by:
        // 1. Create a custom Stream implementation that reads chunks on-demand
        // 2. Then, pass this stream directly to the remote handler
        // 3. This would allow chunks to be read from source and written to destination
        //    without buffering the entire file in memory
        //
        // This has the problem of holding onto the file storage read lock, perhaps that's ok?
        // If it is we would need to shield against slow peers. We already have timeouts but not on the transfer as a whole
        // We also might need to allow pagination to resume transfer
        let mut chunks = Vec::new();
        for chunk_idx in 0..total_chunks {
            let chunk_id = ChunkId::new(chunk_idx);
            let chunk = read_file_storage
                .get_chunk(&file_key, &chunk_id)
                .map_err(into_rpc_error)?;
            chunks.push(chunk);
        }
        drop(read_file_storage);

        let chunks = futures::stream::iter(chunks.into_iter().map(Ok::<_, std::io::Error>));

        let reader =
            tokio_util::io::StreamReader::new(chunks.map(|result| result.map(bytes::Bytes::from)));
        let boxed_reader = Box::new(reader) as _;

        let file_size = file_metadata.file_size();
        // Write file data to destination (local or remote).
        handler
            .upload_file(boxed_reader, file_size, None)
            .await
            .map_err(remote_file_error_to_rpc_error)?;

        Ok(SaveFileToDisk::Success(file_metadata))
    }

    async fn add_files_to_forest_storage(
        &self,
        ext: &Extensions,
        forest_key: Option<H256>,
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

        Ok(AddFilesToForestStorageResult::Success)
    }

    async fn remove_files_from_forest_storage(
        &self,
        ext: &Extensions,
        forest_key: Option<H256>,
        file_keys: Vec<H256>,
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
        for file_key in file_keys {
            write_fs
                .delete_file_key(&file_key)
                .map_err(into_rpc_error)?;
        }

        Ok(RemoveFilesFromForestStorageResult::Success)
    }

    async fn get_forest_root(&self, forest_key: Option<H256>) -> RpcResult<Option<H256>> {
        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        // return None if not found
        let fs = match self.forest_storage_handler.get(&forest_key).await {
            Some(fs) => fs,
            None => return Ok(None),
        };

        let read_fs = fs.read().await;

        Ok(Some(read_fs.root()))
    }

    async fn is_file_in_forest(&self, forest_key: Option<H256>, file_key: H256) -> RpcResult<bool> {
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
        Ok(read_fs
            .contains_file_key(&file_key)
            .map_err(into_rpc_error)?)
    }

    async fn is_file_in_file_storage(
        &self,
        file_key: H256,
    ) -> RpcResult<GetFileFromFileStorageResult> {
        // Acquire FileStorage read lock.
        let read_file_storage = self.file_storage.read().await;

        // See if the file metadata is in the File Storage.
        match read_file_storage
            .get_metadata(&file_key)
            .map_err(into_rpc_error)?
        {
            None => Ok(GetFileFromFileStorageResult::FileNotFound),
            Some(file_metadata) => {
                let stored_chunks = read_file_storage
                    .stored_chunks_count(&file_key)
                    .map_err(into_rpc_error)?;
                let total_chunks = file_metadata.chunks_count();
                if stored_chunks < total_chunks {
                    Ok(GetFileFromFileStorageResult::IncompleteFile(
                        IncompleteFileStatus {
                            file_metadata,
                            stored_chunks,
                            total_chunks,
                        },
                    ))
                } else if stored_chunks > total_chunks {
                    Ok(GetFileFromFileStorageResult::FileFoundWithInconsistency(
                        file_metadata,
                    ))
                } else {
                    Ok(GetFileFromFileStorageResult::FileFound(file_metadata))
                }
            }
        }
    }

    // Note: this method could use either the file storage or the forest storage, but it's using the forest storage.
    // WARNING: Right now, forests don't have the file metadata saved to them, so don't expect to get the file
    // metadata from this method until that's fixed.
    async fn get_file_metadata(
        &self,
        forest_key: Option<H256>,
        file_key: H256,
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
        Ok(read_fs
            .get_file_metadata(&file_key)
            .map_err(into_rpc_error)?)
    }

    async fn generate_forest_proof(
        &self,
        forest_key: Option<H256>,
        challenged_file_keys: Vec<H256>,
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

        Ok(forest_proof.encode())
    }

    async fn generate_proof(
        &self,
        provider_id: H256,
        seed: H256,
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
                        return Err(into_rpc_error("Both left and right leaves in forest proof are None. This should not be possible."));
                    }
                },
                Proven::Empty => {
                    return Err(into_rpc_error("Forest proof generated with empty forest. This should not be possible, as this provider shouldn't have been challenged with an empty forest."));
                }
            }
        }

        // Construct key challenges and generate key proofs for them.
        let mut key_proofs = KeyProofs::new();
        for file_key in &proven_keys {
            // If the file key is a checkpoint challenge for a file deletion, we should NOT generate a key proof for it.
            let should_generate_key_proof = if let Some(checkpoint_challenges) =
                checkpoint_challenges.as_ref()
            {
                if checkpoint_challenges.contains(&CheckpointChallenge {
                    file_key: *file_key,
                    should_remove_file: true,
                }) {
                    debug!(target: LOG_TARGET, "File key {} is a checkpoint challenge for a file deletion", file_key);
                    false
                } else {
                    debug!(target: LOG_TARGET, "File key {} is not a checkpoint challenge for a file deletion", file_key);
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
        let proof = StorageProof {
            forest_proof: proven_file_keys.proof,
            key_proofs,
        };

        Ok(proof.encode())
    }

    async fn generate_file_key_proof_bsp_confirm(
        &self,
        bsp_id: BackupStorageProviderId,
        file_key: H256,
    ) -> RpcResult<Vec<u8>> {
        // Getting Runtime APIs
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

        // Generate chunk IDs to prove to confirm the file
        let chunks_to_prove: Vec<ChunkId> = api
            .query_bsp_confirm_chunks_to_prove_for_file(at_hash, bsp_id.into(), file_key)
            .unwrap()
            .unwrap();

        let key_proof = generate_key_proof(
            self.client.clone(),
            self.file_storage.clone(),
            file_key,
            bsp_id,
            None,
            None,
            Some(chunks_to_prove),
        )
        .await?;

        Ok(key_proof.proof.encode())
    }

    async fn generate_file_key_proof_msp_accept(
        &self,
        msp_id: MainStorageProviderId,
        file_key: H256,
    ) -> RpcResult<Vec<u8>> {
        // Getting Runtime APIs
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

        // Generate chunk IDs to prove to accept the file
        let chunks_to_prove: Vec<ChunkId> = api
            .query_msp_confirm_chunks_to_prove_for_file(at_hash, msp_id.into(), file_key)
            .unwrap()
            .unwrap();

        let key_proof = generate_key_proof(
            self.client.clone(),
            self.file_storage.clone(),
            file_key,
            msp_id,
            None,
            None,
            Some(chunks_to_prove),
        )
        .await?;

        Ok(key_proof.proof.encode())
    }

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

        Ok(new_pub_key.to_string())
    }

    // Deletes all files with keys of type BCSV from the Keystore.
    async fn remove_bcsv_keys(&self, ext: &Extensions, keystore_path: String) -> RpcResult<()> {
        check_if_safe(ext)?;

        let pub_keys = self.keystore.keys(BCSV_KEY_TYPE).map_err(into_rpc_error)?;
        let key_path = PathBuf::from(keystore_path);

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

        Ok(())
    }

    async fn add_to_exclude_list(
        &self,
        ext: &Extensions,
        file_key: H256,
        exclude_type: String,
    ) -> RpcResult<()> {
        check_if_safe(ext)?;

        let et = ExcludeType::from_str(&exclude_type).map_err(into_rpc_error)?;

        let mut write_file_storage = self.file_storage.write().await;
        write_file_storage
            .add_to_exclude_list(file_key, et)
            .map_err(into_rpc_error)?;

        drop(write_file_storage);

        Ok(())
    }

    async fn remove_from_exclude_list(
        &self,
        ext: &Extensions,
        file_key: H256,
        exclude_type: String,
    ) -> RpcResult<()> {
        check_if_safe(ext)?;

        let et = ExcludeType::from_str(&exclude_type).map_err(into_rpc_error)?;

        let mut write_file_storage = self.file_storage.write().await;
        write_file_storage
            .remove_from_exclude_list(&file_key, et)
            .map_err(into_rpc_error)?;

        drop(write_file_storage);

        Ok(())
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

async fn generate_key_proof<FL, C, Block>(
    client: Arc<C>,
    file_storage: Arc<RwLock<FL>>,
    file_key: H256,
    provider_id: ProofsDealerProviderId,
    seed: Option<RandomnessOutput>,
    at: Option<Block::Hash>,
    chunks_to_prove: Option<Vec<ChunkId>>,
) -> RpcResult<KeyProof>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: ProofsDealerRuntimeApi<
        Block,
        ProofsDealerProviderId,
        BlockNumber,
        ForestLeaf,
        RandomnessOutput,
        CustomChallenge,
    >,
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
        .ok_or_else(|| into_rpc_error(format!("File metadata not found for key {:?}", file_key)))?;
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
    Ok(KeyProof {
        proof: file_key_proof,
        challenge_count,
    })
}

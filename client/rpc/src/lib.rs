use std::{
    collections::HashSet,
    fmt::Debug,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

pub mod remote_file;

#[cfg(test)]
mod tests;

use futures::StreamExt;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    types::error::{ErrorObjectOwned as JsonRpseeError, INTERNAL_ERROR_CODE, INTERNAL_ERROR_MSG},
    Extensions,
};
use log::{debug, error, info};
use sc_rpc_api::check_if_safe;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use tokio::{fs, io::AsyncReadExt, sync::RwLock};

use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use remote_file::{local, RemoteFileConfig, RemoteFileHandler, RemoteFileHandlerFactory};
use shc_common::{consts::CURRENT_FOREST_KEY, types::*};
use shc_file_manager::traits::{ExcludeType, FileDataTrie, FileStorage, FileStorageError};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::{sr25519::Pair as Sr25519Pair, Encode, Pair, H256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{traits::Block as BlockT, AccountId32, Deserialize, KeyTypeId, Serialize};
use sp_runtime_interface::pass_by::PassByInner;

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

pub struct StorageHubClientRpcConfig<FL, FSH> {
    pub file_storage: Arc<RwLock<FL>>,
    pub forest_storage_handler: FSH,
    pub keystore: KeystorePtr,
}

impl<FL, FSH: Clone> Clone for StorageHubClientRpcConfig<FL, FSH> {
    fn clone(&self) -> Self {
        Self {
            file_storage: self.file_storage.clone(),
            forest_storage_handler: self.forest_storage_handler.clone(),
            keystore: self.keystore.clone(),
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
    ) -> Self {
        Self {
            file_storage,
            forest_storage_handler,
            keystore,
        }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AddFilesToForestStorageResult {
    ForestNotFound,
    Success,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RemoveFilesFromForestStorageResult {
    ForestNotFound,
    Success,
}

/// Storage Hub client RPC interface.
#[rpc(server, namespace = "storagehubclient")]
pub trait StorageHubClientApi {
    /// Load file from local path or remote URL into storage.
    #[method(name = "loadFileInStorage", with_extensions)]
    async fn load_file_in_storage(
        &self,
        file_path: String,
        location: String,
        owner: AccountId32,
        bucket_id: H256,
    ) -> RpcResult<LoadFileInStorageResult>;

    /// Remove files from file storage.
    #[method(name = "removeFilesFromFileStorage", with_extensions)]
    async fn remove_files_from_file_storage(&self, file_key: Vec<H256>) -> RpcResult<()>;

    /// Remove all files with a given prefix from file storage.
    #[method(name = "removeFilesWithPrefixFromFileStorage", with_extensions)]
    async fn remove_files_with_prefix_from_file_storage(&self, prefix: H256) -> RpcResult<()>;

    /// Save file to disk or upload to remote location.
    #[method(name = "saveFileToDisk", with_extensions)]
    async fn save_file_to_disk(
        &self,
        file_key: H256,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk>;

    /// Add files to forest storage. Forest key is empty for BSP, bucket ID for MSP.
    #[method(name = "addFilesToForestStorage", with_extensions)]
    async fn add_files_to_forest_storage(
        &self,
        forest_key: Option<H256>,
        metadata_of_files_to_add: Vec<FileMetadata>,
    ) -> RpcResult<AddFilesToForestStorageResult>;

    /// Remove files from forest storage. Forest key is empty for BSP, bucket ID for MSP.
    #[method(name = "removeFilesFromForestStorage", with_extensions)]
    async fn remove_files_from_forest_storage(
        &self,
        forest_key: Option<H256>,
        file_keys: Vec<H256>,
    ) -> RpcResult<RemoveFilesFromForestStorageResult>;

    /// Get forest root hash. Forest key is empty for BSP, bucket ID for MSP.
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

    /// Returns SCALE-encoded ForestProof.
    #[method(name = "generateForestProof")]
    async fn generate_forest_proof(
        &self,
        forest_key: Option<H256>,
        challenged_file_keys: Vec<H256>,
    ) -> RpcResult<Vec<u8>>;

    /// Returns SCALE-encoded StorageProof. BSP nodes only.
    #[method(name = "generateProof")]
    async fn generate_proof(
        &self,
        provider_id: H256,
        seed: H256,
        checkpoint_challenges: Option<Vec<CheckpointChallenge>>,
    ) -> RpcResult<Vec<u8>>;

    /// Returns SCALE-encoded KeyVerifier proof.
    #[method(name = "generateFileKeyProofBspConfirm")]
    async fn generate_file_key_proof_bsp_confirm(
        &self,
        bsp_id: BackupStorageProviderId,
        file_key: H256,
    ) -> RpcResult<Vec<u8>>;

    /// Returns SCALE-encoded KeyVerifier proof.
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

    /// Add file to exclude list to prevent re-uploading after deletion.
    #[method(name = "addToExcludeList", with_extensions)]
    async fn add_to_exclude_list(&self, file_key: H256, exclude_type: String) -> RpcResult<()>;

    /// Remove file from exclude list.
    #[method(name = "removeFromExcludeList", with_extensions)]
    async fn remove_from_exclude_list(&self, file_key: H256, exclude_type: String)
        -> RpcResult<()>;
}

pub struct StorageHubClientRpc<FL, FSH, C, Block> {
    client: Arc<C>,
    file_storage: Arc<RwLock<FL>>,
    forest_storage_handler: FSH,
    keystore: KeystorePtr,
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
            _block_marker: Default::default(),
        }
    }
}

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
        check_if_safe(ext)?;

        let config = RemoteFileConfig::default();
        let handler = RemoteFileHandlerFactory::create_from_string(&file_path, config)
            .map_err(|e| into_rpc_error(format!("Failed to create file handler: {:?}", e)))?;

        let url = url::Url::parse(&file_path)
            .or_else(|_| url::Url::parse(&format!("file://{}", file_path)))
            .map_err(|e| into_rpc_error(format!("Invalid file path or URL: {:?}", e)))?;

        let (file_size, _content_type) = handler
            .fetch_metadata(&url)
            .await
            .map_err(|e| into_rpc_error(format!("Failed to fetch file metadata: {:?}", e)))?;

        if file_size == 0 {
            return Err(into_rpc_error(FileStorageError::FileIsEmpty));
        }

        let mut stream = handler
            .stream_file(&url)
            .await
            .map_err(|e| into_rpc_error(format!("Failed to stream file: {:?}", e)))?;

        let mut file_data_trie = self.file_storage.write().await.new_file_data_trie();
        let mut chunk_id: u64 = 0;

        loop {
            let mut chunk = vec![0u8; FILE_CHUNK_SIZE as usize];

            match stream.read(&mut chunk).await {
                Ok(0) => {
                    debug!(target: LOG_TARGET, "Finished reading file");
                    break;
                }
                Ok(bytes_read) => {
                    debug!(target: LOG_TARGET, "Read {} bytes from file", bytes_read);

                    chunk.truncate(bytes_read);

                    file_data_trie
                        .write_chunk(&ChunkId::new(chunk_id), &chunk)
                        .map_err(into_rpc_error)?;
                    chunk_id += 1;
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

        let root = file_data_trie.get_root();

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&owner).to_vec(),
            bucket_id.as_ref().to_vec(),
            location.clone().into(),
            file_size,
            root.as_ref().into(),
        )
        .map_err(into_rpc_error)?;

        let file_key = file_metadata.file_key::<HashT<StorageProofsMerkleTrieLayout>>();

        let mut file_storage_write_lock = self.file_storage.write().await;

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
        check_if_safe(ext)?;

        let mut write_file_storage = self.file_storage.write().await;

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
        check_if_safe(ext)?;

        let mut write_file_storage = self.file_storage.write().await;

        write_file_storage
            .delete_files_with_prefix(&prefix.inner())
            .map_err(into_rpc_error)?;

        Ok(())
    }

    /// Saves a file from storage to disk or uploads it to a remote location.
    /// 
    /// This method supports both local file paths and remote URLs:
    /// - Local paths: Files are saved directly to the filesystem
    /// - HTTP/HTTPS URLs: Files are uploaded via HTTP PUT/POST
    /// - FTP/FTPS URLs: Files are uploaded via FTP
    /// - file:// URLs: Treated as local file paths
    /// 
    /// # Arguments
    /// * `ext` - RPC extensions for safety checks
    /// * `file_key` - The key identifying the file in storage
    /// * `file_path` - The destination path (local or remote URL)
    /// 
    /// # Returns
    /// * `SaveFileToDisk::Success` - File was successfully saved/uploaded
    /// * `SaveFileToDisk::FileNotFound` - File key not found in storage
    /// * `SaveFileToDisk::IncompleteFile` - File is incomplete (missing chunks)
    /// * Error - If the operation fails
    async fn save_file_to_disk(
        &self,
        ext: &Extensions,
        file_key: H256,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk> {
        check_if_safe(ext)?;

        let read_file_storage = self.file_storage.read().await;

        let file_metadata = match read_file_storage
            .get_metadata(&file_key)
            .map_err(into_rpc_error)?
        {
            None => return Ok(SaveFileToDisk::FileNotFound),
            Some(metadata) => metadata,
        };

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

        let config = RemoteFileConfig::default();
        let handler = if let Ok(url) = url::Url::parse(&file_path) {
            RemoteFileHandlerFactory::create(&url, config)
                .map_err(|e| into_rpc_error(format!("Failed to create file handler: {}", e)))?
        } else {
            Arc::new(local::LocalFileHandler::new()) as Arc<dyn RemoteFileHandler>
        };

        let mut chunks = Vec::new();
        for chunk_idx in 0..total_chunks {
            let chunk_id = ChunkId::new(chunk_idx);
            let chunk = read_file_storage
                .get_chunk(&file_key, &chunk_id)
                .map_err(into_rpc_error)?;
            chunks.push(chunk);
        }
        
        drop(read_file_storage);
        
        let chunks_stream = futures::stream::iter(chunks.into_iter().map(Ok::<_, std::io::Error>));

        let reader = tokio_util::io::StreamReader::new(
            chunks_stream.map(|result| result.map(bytes::Bytes::from))
        );
        let boxed_reader: Box<dyn tokio::io::AsyncRead + Send + Unpin> = Box::new(reader);

        let file_size = file_metadata.file_size();
        handler
            .upload_file(&file_path, boxed_reader, file_size, None)
            .await
            .map_err(|e| into_rpc_error(format!("Failed to save file: {}", e)))?;

        Ok(SaveFileToDisk::Success(file_metadata))
    }

    async fn add_files_to_forest_storage(
        &self,
        ext: &Extensions,
        forest_key: Option<H256>,
        metadata_of_files_to_add: Vec<FileMetadata>,
    ) -> RpcResult<AddFilesToForestStorageResult> {
        check_if_safe(ext)?;

        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        let fs = match self.forest_storage_handler.get(&forest_key).await {
            Some(fs) => fs,
            None => return Ok(AddFilesToForestStorageResult::ForestNotFound),
        };

        let mut write_fs = fs.write().await;

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
        check_if_safe(ext)?;

        let forest_key = match forest_key {
            Some(forest_key) => forest_key.as_ref().to_vec().into(),
            None => CURRENT_FOREST_KEY.to_vec().into(),
        };

        let fs = match self.forest_storage_handler.get(&forest_key).await {
            Some(fs) => fs,
            None => return Ok(RemoveFilesFromForestStorageResult::ForestNotFound),
        };

        let mut write_fs = fs.write().await;

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
        let read_file_storage = self.file_storage.read().await;

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
        debug!(target: LOG_TARGET, "Checkpoint challenges: {:?}", checkpoint_challenges);

        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

        let random_challenges = api
            .get_forest_challenges_from_seed(at_hash, &seed, &provider_id)
            .unwrap();

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

        let proven_file_keys = {
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

        let mut key_proofs = KeyProofs::new();
        for file_key in &proven_keys {
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
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

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
        let api = self.client.runtime_api();
        let at_hash = self.client.info().best_hash;

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

    async fn remove_bcsv_keys(&self, ext: &Extensions, keystore_path: String) -> RpcResult<()> {
        check_if_safe(ext)?;

        let pub_keys = self.keystore.keys(BCSV_KEY_TYPE).map_err(into_rpc_error)?;
        let key_path = PathBuf::from(keystore_path);

        for pub_key in pub_keys {
            let mut key = key_path.clone();
            let key_name = key_file_name(&pub_key, BCSV_KEY_TYPE);
            key.push(key_name);

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

fn key_file_name(public: &[u8], key_type: KeyTypeId) -> PathBuf {
    let mut buf = PathBuf::new();
    let key_type = array_bytes::bytes2hex("", &key_type.0);
    let key = array_bytes::bytes2hex("", public);
    buf.push(key_type + key.as_str());
    buf
}

fn into_rpc_error(e: impl Debug) -> JsonRpseeError {
    JsonRpseeError::owned(
        INTERNAL_ERROR_CODE,
        INTERNAL_ERROR_MSG,
        Some(format!("{:?}", e)),
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
    let api = client.runtime_api();
    let at_hash = at.unwrap_or_else(|| client.info().best_hash);

    let read_file_storage = file_storage.read().await;
    let metadata = read_file_storage
        .get_metadata(&file_key)
        .map_err(|e| into_rpc_error(format!("Error retrieving file metadata: {:?}", e)))?
        .ok_or_else(|| into_rpc_error(format!("File metadata not found for key {:?}", file_key)))?;
    drop(read_file_storage);

    let challenge_count = metadata.chunks_to_check();

    let chunks_to_prove = match chunks_to_prove {
        Some(chunks) => chunks,
        None => {
            let seed = seed.ok_or_else(|| {
                into_rpc_error("Seed is required to generate challenges if chunk IDs are missing")
            })?;
            let file_key_challenges = api
                .get_challenges_from_seed(at_hash, &seed, &provider_id, challenge_count)
                .map_err(|e| {
                    into_rpc_error(format!("Failed to generate challenges from seed: {:?}", e))
                })?;

            let chunks_count = metadata.chunks_count();
            file_key_challenges
                .iter()
                .map(|challenge| ChunkId::from_challenge(challenge.as_ref(), chunks_count))
                .collect::<Vec<_>>()
        }
    };

    let read_file_storage = file_storage.read().await;
    let file_key_proof = read_file_storage
        .generate_proof(&file_key, &HashSet::from_iter(chunks_to_prove))
        .map_err(|e| {
            into_rpc_error(format!(
                "File is not in storage, or proof does not exist: {:?}",
                e
            ))
        })?;
    drop(read_file_storage);

    Ok(KeyProof {
        proof: file_key_proof,
        challenge_count,
    })
}

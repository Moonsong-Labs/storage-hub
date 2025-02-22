use std::{
    collections::HashSet,
    fmt::Debug,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

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
use tokio::{fs, fs::create_dir_all, sync::RwLock};

use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use shc_common::{
    consts::CURRENT_FOREST_KEY,
    types::{
        BackupStorageProviderId, BlockNumber, BucketId, ChunkId, CustomChallenge, FileMetadata,
        ForestLeaf, HashT, KeyProof, KeyProofs, MainStorageProviderId, ProofsDealerProviderId,
        Proven, RandomnessOutput, StorageProof, StorageProofsMerkleTrieLayout, BCSV_KEY_TYPE,
        FILE_CHUNK_SIZE,
    },
};
use shc_file_manager::traits::{ExcludeType, FileDataTrie, FileStorage, FileStorageError};
use shc_forest_manager::traits::{ForestStorage, ForestStorageHandler};
use sp_core::{sr25519::Pair as Sr25519Pair, Encode, Pair, H256};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::{traits::Block as BlockT, AccountId32, Deserialize, KeyTypeId, Serialize};

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

/// Provides an interface with the desired RPC method.
/// Used by the `rpc` macro from `jsonrpsee`
/// to generate the trait that is actually going to be implemented.
#[rpc(server, namespace = "storagehubclient")]
pub trait StorageHubClientApi {
    #[method(name = "loadFileInStorage")]
    async fn load_file_in_storage(
        &self,
        file_path: String,
        location: String,
        owner: AccountId32,
        bucket_id: H256,
    ) -> RpcResult<LoadFileInStorageResult>;

    #[method(name = "saveFileToDisk")]
    async fn save_file_to_disk(
        &self,
        file_key: H256,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk>;

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
        >,
    FL: FileStorage<StorageProofsMerkleTrieLayout> + Send + Sync,
    FSH: ForestStorageHandler + Send + Sync + 'static,
{
    async fn load_file_in_storage(
        &self,
        file_path: String,
        location: String,
        owner: AccountId32,
        bucket_id: H256,
    ) -> RpcResult<LoadFileInStorageResult> {
        // Open file in the local file system.
        let mut file = File::open(PathBuf::from(file_path.clone())).map_err(into_rpc_error)?;

        // Instantiate an "empty" [`FileDataTrie`] so we can write the file chunks into it.
        let mut file_data_trie = self.file_storage.write().await.new_file_data_trie();
        // A chunk id is simply an integer index.
        let mut chunk_id: u64 = 0;

        // Read file in chunks of [`FILE_CHUNK_SIZE`] into buffer then push buffer into a vector.
        // Loops until EOF or until some error that is NOT `ErrorKind::Interrupted` is found.
        // If `ErrorKind::Interrupted` is found, the operation is simply retried, as per
        // https://doc.rust-lang.org/std/io/trait.Read.html#errors-1
        loop {
            let mut chunk = Vec::with_capacity(FILE_CHUNK_SIZE as usize);
            let read_result = <File as Read>::by_ref(&mut file)
                .take(FILE_CHUNK_SIZE)
                .read_to_end(&mut chunk);
            match read_result {
                // Reached EOF, break loop.
                Ok(0) => {
                    debug!(target: LOG_TARGET, "Finished reading file");
                    break;
                }
                // Haven't reached EOF yet, continue loop.
                Ok(bytes_read) => {
                    debug!(target: LOG_TARGET, "Read {} bytes from file", bytes_read);

                    // Build the actual [`FileDataTrie`] by inserting each chunk into it.
                    file_data_trie
                        .write_chunk(&ChunkId::new(chunk_id), &chunk)
                        .map_err(into_rpc_error)?;
                    chunk_id += 1;
                }
                Err(e) => {
                    error!(target: LOG_TARGET, "Error when trying to read file: {:?}", e);
                    return Err(into_rpc_error(e));
                }
            }
        }

        // Generate the necessary metadata so we can insert file into the File Storage.
        let root = file_data_trie.get_root();
        let fs_metadata = file.metadata().map_err(into_rpc_error)?;

        if fs_metadata.len() == 0 {
            return Err(into_rpc_error(FileStorageError::FileIsEmpty));
        }

        // Build StorageHub's [`FileMetadata`]
        let file_metadata = FileMetadata {
            owner: <AccountId32 as AsRef<[u8]>>::as_ref(&owner).to_vec(),
            bucket_id: bucket_id.as_ref().to_vec(),
            file_size: fs_metadata.len(),
            fingerprint: root.as_ref().into(),
            location: location.clone().into(),
        };
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

    async fn save_file_to_disk(
        &self,
        file_key: H256,
        file_path: String,
    ) -> RpcResult<SaveFileToDisk> {
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

        let file_path = PathBuf::from(file_path.clone());

        // Create parent directories if they don't exist.
        create_dir_all(&file_path.parent().unwrap())
            .await
            .map_err(into_rpc_error)?;

        // Open file in the local file system.
        let mut file = File::create(PathBuf::from(file_path.clone())).map_err(into_rpc_error)?;

        // Write file data to disk.
        for chunk_id in 0..total_chunks {
            let chunk_id = ChunkId::new(chunk_id);
            let chunk = read_file_storage
                .get_chunk(&file_key, &chunk_id)
                .map_err(into_rpc_error)?;
            file.write_all(&chunk).map_err(into_rpc_error)?;
        }

        Ok(SaveFileToDisk::Success(file_metadata))
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

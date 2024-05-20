use jsonrpsee::core::async_trait;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::ErrorObjectOwned as JsonRpseeError;
use jsonrpsee::types::error::INTERNAL_ERROR_CODE;
use jsonrpsee::types::error::INTERNAL_ERROR_MSG;
use jsonrpsee::types::ErrorObjectOwned;

use sc_transaction_pool_api::TransactionPool;
use sc_transaction_pool_api::TransactionSource;
use sc_client_api::backend::{Backend, StorageProvider};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::BoundedVec;
use sp_runtime::AccountId32;
use sp_trie::MemoryDB;
use sp_trie::TrieDBMutBuilder;
use sp_runtime::generic::UncheckedExtrinsic;
use storage_hub_infra::constants::FILE_CHUNK_SIZE;
use storage_hub_infra::types::Metadata;
use storage_hub_runtime::AccountId;
use storage_hub_runtime::RuntimeCall;

use file_manager::traits::FileStorage;
use forest_manager::traits::ForestStorage;

use log::debug;
use log::error;
use serde::de::DeserializeOwned;
use storage_hub_runtime::Signature;
use storage_hub_runtime::SignedExtra;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "file-system-rpc";


// CHANGE NAME
#[rpc(server, namespace = "filesystem")]
#[async_trait]
pub trait FileSystemApi {
    #[method(name = "sendFile")]
    async fn upload_file(&self, file_path: String, location: String, owner: AccountId32) -> RpcResult<()>;
}

pub struct FileSystemRpc<FL> {
    file_storage: Arc<RwLock<FL>>,
}

impl<FL> FileSystemRpc<FL> {
    pub fn new(
        file_storage: Arc<RwLock<FL>>,
    ) -> Self {
        Self {
            file_storage,
        }
    }
}

#[async_trait]
impl<FL> FileSystemApiServer for FileSystemRpc<FL>
where
    FL: Send + Sync + FileStorage,
{
    async fn upload_file(&self, file_path: String, location: String, owner: AccountId32) -> RpcResult<()> {

        let mut file = File::open(PathBuf::from(file_path.clone())).map_err(into_rpc_error)?;
        let mut file_chunks = Vec::new();

        // Read file in chunks of `FILE_CHUNK_SIZE` into buffer then push buffer into a vector.
        // Loops until EOF or until some error that is NOT `ErrorKind::Interrupted` is found.
        // https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end
        loop {
            let mut buffer = Vec::with_capacity(FILE_CHUNK_SIZE);
            let read_result = file
                .by_ref()
                .take(FILE_CHUNK_SIZE as u64)
                .read_to_end(&mut buffer);
            match read_result {
                // Reached EOF, break loop.
                Ok(0) => {
                    debug!(target: LOG_TARGET, "Finished reading file");
                    break;
                }
                // Haven't reached EOF yet, continue loop.
                Ok(bytes_read) => {
                    debug!(target: LOG_TARGET, "Read {} bytes from file", bytes_read);
                    file_chunks.push(buffer)
                }
                Err(e) => { 
                    error!(target: LOG_TARGET, "Error when trying to read file: {:?}", e);
                    return Err(into_rpc_error(e)) 
                },
            }
        }

        // let mut memdb = MemoryDB::<FL>::default();
        // let root = Default::default();
        // {
        //     let mut t = TrieDBMutBuilder::<FL>::new(&mut memdb, &mut root).build();
        //     for (chunk_id, chunk) in file_chunks.iter().enumerate() {
        //         let chunk_id = chunk_id as u64;
        //         t.insert(chunk_id, chunk).expect("error");
        //     }
        // }

        let mut file_storage_lock = self.file_storage.write().await;

        let fs_metadata = file.metadata().map_err(into_rpc_error)?;
        let file_size = fs_metadata.len();

        let file_metadata = Metadata {
            size: file_size,
            // TODO(Arthur/Alexandru): Fingerprint is a missing piece right now.
            // We will get it from `FileData`.
            fingerprint: H256::default(),
            owner: owner.to_string(),
            location: location.clone().into(),
        };

        let file_key = file_metadata.key();

        for (chunk_id, chunk) in file_chunks.iter().enumerate() { 
            let chunk_id = chunk_id as u64;
            file_storage_lock
                .write_chunk(&file_key, &chunk_id, &chunk.to_vec())
                .map_err(into_rpc_error)?;
        }

        Ok(())
    }
}

/// Converts into an RPC error.
fn into_rpc_error(e: impl Debug) -> JsonRpseeError {
    ErrorObjectOwned::owned(
        INTERNAL_ERROR_CODE,
        INTERNAL_ERROR_MSG,
        Some(format!("{:?}", e)),
    )
}
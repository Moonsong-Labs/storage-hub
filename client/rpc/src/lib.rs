use file_manager::traits::FileStorage;
use forest_manager::traits::ForestStorage;

use storage_hub_infra::constants::FILE_CHUNK_SIZE;
use storage_hub_infra::types::Metadata;

use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::INTERNAL_ERROR_CODE;
use jsonrpsee::types::error::INTERNAL_ERROR_MSG;
use jsonrpsee::types::error::ErrorObjectOwned as JsonRpseeError;
use jsonrpsee::types::ErrorObjectOwned;

use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_core::Blake2Hasher;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::DeserializeOwned;

use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::io::Read;
use log::debug;

const LOG_TARGET: &str = "file-system-rpc";

#[rpc(server)]
pub trait FileSystemApi<BlockHash> {
    #[method(name = "filesystem_sendFile")]
    fn send_file(&self, at: Option<BlockHash>, location: String) -> RpcResult<()>;
}

pub struct FileSystemRpc<C, B, FL, FS> {
    client: Arc<C>,
    file_storage: Arc<RwLock<FL>>,
    forest_storage: Arc<RwLock<FS>>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B, FL, FS> FileSystemRpc<C, B, FL, FS> {
    pub fn new(client: Arc<C>, file_storage: Arc<RwLock<FL>>, forest_storage: Arc<RwLock<FS>>) -> Self {
        Self {
            client,
            file_storage,
            forest_storage,
            _marker: Default::default(),
        }
    }
}

impl<C, B, FL, FS> FileSystemApiServer<<B as BlockT>::Hash> for FileSystemRpc<C, B, FL, FS>
where
    B: BlockT,
    C: Send + Sync + 'static + HeaderBackend<B>,
    FL: Send + Sync + FileStorage,
    FS: Send + Sync + ForestStorage<LookupKey = H256, Value = Metadata>,
{
    fn send_file(&self, at: Option<<B as BlockT>::Hash>, location: String) -> RpcResult<()> {
        let _at = at.unwrap_or_else(|| self.client.info().best_hash);

        let mut file = File::open(PathBuf::from(location.clone())).map_err(into_rpc_error)?;
        let fs_metadata = file.metadata().map_err(into_rpc_error)?;
        let file_size = fs_metadata.len();
        // let fingerprint = ();
        // let owner = ();
        let file_metadata = Metadata {
            size: file_size,
            fingerprint: H256::default(),
            owner: "Owner".to_string(),
            location: location.into(),
        };
        let mut file_chunks = Vec::new();

        // Read file in chunks of `FILE_CHUNK_SIZE` into buffer and push them into a vector.
        // Loops until EOF or until some error that is not `ErrorKind::Interrupted` is found.
        loop {
            let mut buffer = Vec::with_capacity(FILE_CHUNK_SIZE);
            let result = file.by_ref().take(FILE_CHUNK_SIZE as u64).read_to_end(&mut buffer);
            match result {
                // Reached EOF.
                Ok(0) => { 
                    debug!(target: LOG_TARGET, "Finished reading file");    
                    break 
                },
                // Haven't reached EOF yet. Keep looping.
                Ok(bytes_read) => {
                    debug!(target: LOG_TARGET, "Read {} bytes from file", bytes_read);
                    file_chunks.push(buffer) 
                },
                Err(e) => { return Err(into_rpc_error(e)) }
            }
        }

        let mut file_storage_lock = self.file_storage.write().map_err(into_rpc_error)?;
        let mut forest_storage_lock = self.forest_storage.write().map_err(into_rpc_error)?;

        for (chunk_id, chunk) in file_chunks.iter().enumerate() {
            let key = H256::default();
            let chunk_id = chunk_id as u64;
            forest_storage_lock.insert_file_key(&key, &file_metadata).map_err(into_rpc_error)?;
            file_storage_lock.write_chunk(&key, &chunk_id, &chunk.to_vec()).map_err(into_rpc_error)?;
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
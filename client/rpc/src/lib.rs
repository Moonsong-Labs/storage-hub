use file_manager::traits::FileStorage;
use jsonrpsee::types::error::INTERNAL_ERROR_CODE;
use jsonrpsee::types::error::INTERNAL_ERROR_MSG;
use jsonrpsee::types::error::ErrorObjectOwned as JsonRpseeError;
use jsonrpsee::types::ErrorObjectOwned;
use sp_core::H256;
use storage_hub_infra::constants::FILE_CHUNK_SIZE;

use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;

use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::io::Read;

#[rpc(server)]
pub trait FileSystemApi<BlockHash> {
    #[method(name = "filesystem_sendFile")]
    fn send_file(&self, at: Option<BlockHash>, location: String) -> RpcResult<()>;
}

pub struct FileSystemRpc<C, B, FL> {
    client: Arc<C>,
    file_storage: Arc<RwLock<FL>>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B, FL> FileSystemRpc<C, B, FL> {
    pub fn new(client: Arc<C>, file_storage: Arc<RwLock<FL>>) -> Self {
        Self {
            client,
            file_storage,
            _marker: Default::default(),
        }
    }
}

impl<C, B, FL> FileSystemApiServer<<B as BlockT>::Hash> for FileSystemRpc<C, B, FL>
where
    B: BlockT,
    C: Send + Sync + 'static + HeaderBackend<B>,
    FL: Send + Sync + FileStorage,
{
    fn send_file(&self, at: Option<<B as BlockT>::Hash>, location: String) -> RpcResult<()> {
        let _at = at.unwrap_or_else(|| self.client.info().best_hash);

        let mut file = File::open(PathBuf::from(location)).map_err(into_rpc_error)?;
        let file_metadata = file.metadata().map_err(into_rpc_error)?;
        let file_size = file_metadata.len();
        let mut file_chunks = Vec::new();

        // Read file in chunks of `FILE_CHUNK_SIZE` into a buffer and push them into a vector.
        // Loops until EOF or if some error different from `ErrorKind::Interrupted` is found.
        loop {
            let mut buffer = Vec::with_capacity(FILE_CHUNK_SIZE);
            let result = file.by_ref().take(FILE_CHUNK_SIZE as u64).read_to_end(&mut buffer);
            match result {
                // Reached EOF.
                Ok(0) => break, 
                Ok(_) => file_chunks.push(buffer),
                Err(e) => { return Err(into_rpc_error(e)) }
            }
        }

        let mut lock = self.file_storage.write().map_err(into_rpc_error)?;

        for (chunk_id, chunk) in file_chunks.iter().enumerate() {
            let key = H256::default();
            let chunk_id = chunk_id as u64;
            let _ = lock.write_chunk(&key, &chunk_id, &chunk.to_vec());
        }

        Ok(())
    }
}

/// Converts into an RPC error.
fn into_rpc_error(e: impl std::fmt::Debug) -> JsonRpseeError {
    ErrorObjectOwned::owned(
        INTERNAL_ERROR_CODE,
        INTERNAL_ERROR_MSG,
        Some(format!("{:?}", e)),
    )
}
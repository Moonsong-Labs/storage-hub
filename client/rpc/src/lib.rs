use jsonrpsee::types::error::ErrorObjectOwned as JsonRpseeError;
use jsonrpsee::core::{async_trait, RpcResult};
use jsonrpsee::proc_macros::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::AccountId32;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::fs::File;
use file_manager::traits::FileStorage;

#[rpc(server)]
pub trait FileSystemApi<BlockHash> {
    #[method(name = "filesystem_sendFile")]
    fn send_file(&self, at: Option<BlockHash>, location: String) -> RpcResult<()>;
}

pub struct FileSystem<C, B, FL> {
    client: Arc<C>,
    file_storage: Arc<RwLock<FL>>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B, FL> FileSystem<C, B, FL> {
    pub fn new(client: Arc<C>, file_storage: Arc<RwLock<FL>>) -> Self {
        Self {
            client, file_storage, _marker: Default::default()
        } 
    }
}

impl<C, B, FL> FileSystemApiServer<<B as BlockT>::Hash> for FileSystem<C, B, FL>
where
    B: BlockT,
    C: Send + Sync + 'static + HeaderBackend<B>,
    FL: Send + Sync + FileStorage
{
     fn send_file(&self, at: Option<<B as BlockT>::Hash>, location: String) -> RpcResult<()> {
        let at = at.unwrap_or_else(||self.client.info().best_hash);

        // TODO: deal with result.
        let file = File::open(PathBuf::from(location)).expect("Can't open file.");

        // TODO: deal with result.
        let lock = self.file_storage.write().expect("Can't acquire lock");

        // let _ = lock.write_chunk();

        Ok(())
    }
}

use file_manager::traits::FileStorage;
use sp_core::H256;
use storage_hub_infra::constants::FILE_CHUNK_SIZE;

use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;

use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

use std::fs::File;
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

        // TODO: deal with result.
        let mut file = File::open(PathBuf::from(location)).expect("Can't open file.");
        let file_metadata = file.metadata().expect("Can't get metadata.");
        let file_size = file_metadata.len();
        let mut file_chunks = Vec::<[u8; FILE_CHUNK_SIZE]>::new();
        let mut buffer = [1; FILE_CHUNK_SIZE];

        loop {
            file.read_exact(&mut buffer).expect("Can't read file in chunks");
            file_chunks.push(buffer)
        }
        let lock = self.file_storage.write().expect("Can't acquire lock");

        for (chunk_id, chunk) in file_chunks.iter().enumerate() {
            let key = H256::default();
            let chunk_id = chunk_id as u64;
            let _ = lock.write_chunk(&key, &chunk_id, &chunk.to_vec());

        }

        Ok(())
    }
}

use jsonrpsee::core::async_trait;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::ErrorObjectOwned as JsonRpseeError;
use jsonrpsee::types::error::INTERNAL_ERROR_CODE;
use jsonrpsee::types::error::INTERNAL_ERROR_MSG;
use jsonrpsee::types::ErrorObjectOwned;

use sp_core::H256;

use sp_runtime::AccountId32;

use sp_trie::MemoryDB;
use sp_trie::TrieDBMutBuilder;
use sp_trie::TrieLayout;
use sp_trie::TrieMut;
use shc_common::types::FILE_CHUNK_SIZE;
use shc_common::types::Metadata;
use shc_common::types::Fingerprint;

use file_manager::traits::FileStorage;

use log::debug;
use log::error;

use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

const LOG_TARGET: &str = "file-system-rpc";

// CHANGE NAME
// add doc comments
// check alphabetical order in imports in files and .toml

#[rpc(server, namespace = "filesystem")]
#[async_trait]
pub trait FileSystemApi {
    #[method(name = "sendFile")]
    async fn upload_file(
        &self,
        file_path: String,
        location: String,
        owner: AccountId32,
    ) -> RpcResult<Metadata>;
}

pub struct FileSystemRpc<FL, T> {
    file_storage: Arc<RwLock<FL>>,
    _marker: PhantomData<T>,
}

impl<FL, T> FileSystemRpc<FL, T> {
    pub fn new(file_storage: Arc<RwLock<FL>>) -> Self {
        Self { file_storage, _marker: Default::default() }
    }
}

#[async_trait]
impl<FL, T> FileSystemApiServer for FileSystemRpc<FL, T>
where
    FL: Send + Sync + FileStorage<T>,
    T: Send + Sync + TrieLayout + 'static,
    <T::Hash as sp_core::Hasher>::Out: Into<[u8; 32]>
{
    async fn upload_file(
        &self,
        file_path: String,
        location: String,
        owner: AccountId32,
    ) -> RpcResult<Metadata> {
        let mut file = File::open(PathBuf::from(file_path.clone())).map_err(into_rpc_error)?;
        let mut file_chunks = Vec::new();

        // Read file in chunks of `FILE_CHUNK_SIZE` into buffer then push buffer into a vector.
        // Loops until EOF or until some error that is NOT `ErrorKind::Interrupted` is found.
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
                    return Err(into_rpc_error(e));
                }
            }
        }

        let mut memdb = MemoryDB::<T::Hash>::default();
        let mut root = Default::default();
        {
            let mut t = TrieDBMutBuilder::<T>::new(&mut memdb, &mut root).build();
            for (chunk_id, chunk) in file_chunks.iter().enumerate() {
                let chunk_id = chunk_id.to_be_bytes();
                t.insert(&chunk_id, chunk).expect("error");
            }
        }

        let mut file_storage_lock = self.file_storage.write().await;

        let fs_metadata = file.metadata().map_err(into_rpc_error)?;
        let file_size = fs_metadata.len();

        let file_metadata = Metadata {
            size: file_size,
            fingerprint: Fingerprint::new(root.into()),
            owner: owner.to_string(),
            location: location.clone().into(),
        };

        let file_key = file_metadata.key::<T::Hash>();

        for (chunk_id, chunk) in file_chunks.iter().enumerate() {
            let chunk_id = chunk_id as u64;
            file_storage_lock
                .write_chunk(&file_key, &chunk_id, &chunk.to_vec())
                .map_err(into_rpc_error)?;
        }

        Ok(file_metadata)
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

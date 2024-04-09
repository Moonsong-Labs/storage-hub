use crate::types::{FileChunk, FileKey, FileMetadata};

/// Storage interface to be implemented by the storage providers.
pub trait Storage: Clone + Send + Sync + 'static {
    /// Get metadata for a file.
    fn get_metadata(
        &self,
        key: &FileKey,
    ) -> impl std::future::Future<Output = Option<FileMetadata>> + Send;

    /// Set metadata for a file.
    fn set_metadata(
        &self,
        key: &FileKey,
        metadata: &FileMetadata,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Get a file chunk from storage.
    fn get_chunk(
        &self,
        key: &FileKey,
        chunk: u64,
    ) -> impl std::future::Future<Output = Option<FileChunk>> + Send;

    /// Write a file chunk in storage.
    fn write_chunk(
        &self,
        key: &str,
        chunk: u64,
        data: &FileChunk,
    ) -> impl std::future::Future<Output = ()> + Send;
}

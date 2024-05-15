use shc_common::types::{Chunk, ChunkId, FileProof, HasherOutT, Metadata};
use trie_db::TrieLayout;

#[derive(Debug)]
pub enum FileStorageWriteError {
    /// The requested file does not exist.
    FileDoesNotExist,
    /// File chunk already exists.
    FileChunkAlreadyExists,
    /// Failed to insert the file chunk.
    FailedToInsertFileChunk,
    /// Failed to get file chunk.
    FailedToGetFileChunk,
    /// File metadata fingerprint does not match the stored file fingerprint.
    FingerprintAndStoredFileMismatch,
}

#[derive(Debug)]
pub enum FileStorageError {
    /// File chunk already exists.
    FileChunkAlreadyExists,
    /// File chunk does not exist.
    FileChunkDoesNotExist,
    /// Failed to insert the file chunk.
    FailedToInsertFileChunk,
    /// Failed to get file chunk.
    FailedToGetFileChunk,
    /// Failed to generate proof.
    FailedToGenerateCompactProof,
    /// The requested file does not exist.
    FileDoesNotExist,
    /// File metadata fingerprint does not match the stored file fingerprint.
    FingerprintAndStoredFileMismatch,
    /// The requested file is incomplete and a proof is impossible to generate.
    IncompleteFile,
}

#[derive(Debug)]
pub enum FileStorageWriteOutcome {
    /// The file storage was completed after this write.
    /// All chunks for the file are stored and the fingerprints match too.
    FileComplete,
    /// The file was not completed after this chunk write.
    FileIncomplete,
}

/// Storage interface to be implemented by the storage providers.
pub trait FileStorage<T: TrieLayout>: 'static {
    /// Generate proof for a chunk of a file. If the file does not exists or any chunk is missing,
    /// no proof will be returned.
    fn generate_proof(
        &self,
        key: &HasherOutT<T>,
        chunk_id: &ChunkId,
    ) -> Result<FileProof, FileStorageError>;

    /// Remove a file from storage.
    fn delete_file(&mut self, key: &HasherOutT<T>);

    /// Get metadata for a file.
    fn get_metadata(&self, key: &HasherOutT<T>) -> Result<Metadata, FileStorageError>;

    /// Set metadata for a file. This should be called before you start adding chunks since it
    /// will overwrite any previous Metadata and delete already stored file chunks.
    fn set_metadata(&mut self, key: HasherOutT<T>, metadata: Metadata);

    /// Get a file chunk from storage.
    fn get_chunk(&self, key: &HasherOutT<T>, chunk_id: &ChunkId)
        -> Result<Chunk, FileStorageError>;

    /// Write a file chunk in storage. It is expected that you verify the associated proof that the
    /// [`Chunk`] is part of the file before writing it.
    fn write_chunk(
        &mut self,
        key: &HasherOutT<T>,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<FileStorageWriteOutcome, FileStorageWriteError>;
}

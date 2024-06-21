use shc_common::types::{Chunk, ChunkId, FileMetadata, FileProof, HasherOutT};
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
    /// File already exists.
    FileAlreadyExists,
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

    FailedToReadStorage,
    FailedToWriteToStorage,
    FailedToParseKey,
    ExpectingRootToBeInStorage,
}

#[derive(Debug)]
pub enum FileStorageWriteOutcome {
    /// The file storage was completed after this write.
    /// All chunks for the file are stored and the fingerprints match too.
    FileComplete,
    /// The file was not completed after this chunk write.
    FileIncomplete,
}

pub trait FileDataTrie<T: TrieLayout> {
    /// Get the root of the trie.
    fn get_root(&self) -> &HasherOutT<T>;

    /// Get the number of stored chunks in the trie.
    fn stored_chunks_count(&self) -> u64;

    /// Generate proof for a chunk of a file. Returns error if the chunk does not exist.
    fn generate_proof(&self, chunk_ids: &Vec<ChunkId>) -> Result<FileProof, FileStorageError>;

    /// Get a file chunk from storage. Returns error if the chunk does not exist.
    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError>;

    /// Write a file chunk in storage updating the root hash of the trie.
    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError>;

    /// Removes all chunks from storage.
    fn delete(&mut self) -> Result<(), FileStorageError>;
}

/// Storage interface to be implemented by the storage providers.
pub trait FileStorage<T: TrieLayout>: 'static {
    type FileDataTrie: FileDataTrie<T> + Send + Sync + Default;

    /// Generate proof for a chunk of a file. If the file does not exists or any chunk is missing,
    /// no proof will be returned.
    fn generate_proof(
        &self,
        key: &HasherOutT<T>,
        chunk_ids: &Vec<ChunkId>,
    ) -> Result<FileProof, FileStorageError>;

    /// Remove a file from storage.
    fn delete_file(&mut self, key: &HasherOutT<T>) -> Result<(), FileStorageError>;

    /// Get metadata for a file.
    fn get_metadata(&self, key: &HasherOutT<T>) -> Result<FileMetadata, FileStorageError>;

    // TODO: check if this method is necessary and what is its use case.
    /// Inserts a new file. If the file already exists, it will return an error.
    /// It is expected that the file key is indeed computed from the [Metadata].
    /// This method does not require the actual data, file [`Chunk`]s being inserted separately.
    fn insert_file(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
    ) -> Result<(), FileStorageError>;

    /// Inserts a new file with the associated trie data. If the file already exists, it will
    /// return an error.
    fn insert_file_with_data(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
        file_data: Self::FileDataTrie,
    ) -> Result<(), FileStorageError>;

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

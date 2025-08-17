use std::{collections::HashSet, str::FromStr};

use trie_db::TrieLayout;

use shc_common::types::{Chunk, ChunkId, FileKeyProof, FileMetadata, FileProof, HasherOutT};

#[derive(Debug)]
pub enum FileStorageWriteError {
    /// The requested file does not exist.
    FileDoesNotExist,
    /// File chunk ID already exists.
    FileChunkAlreadyExists,
    /// Failed to insert the file chunk.
    FailedToInsertFileChunk,
    /// Failed to get file chunk.
    FailedToGetFileChunk,
    /// File metadata fingerprint does not match the stored file fingerprint.
    FingerprintAndStoredFileMismatch,
    /// Failed to construct file trie.
    FailedToContructFileTrie,
    /// Failed to construct iterator for trie.
    FailedToConstructTrieIter,
    /// Failed to commit changes in overlay to disk.
    FailedToPersistChanges,
    /// Failed to delete root.
    FailedToDeleteRoot,
    /// Failed to delete chunk.
    FailedToDeleteChunk,
    /// Failed to convert raw bytes into [`FileMetadata`].
    FailedToParseFileMetadata,
    /// Failed to access storage for reading.
    FailedToReadStorage,
    /// Failed to convert raw bytes into [`Fingerprint`].
    FailedToParseFingerprint,
    /// Failed to update root after a chunk was written.
    FailedToUpdatePartialRoot,
    /// Failed to convert raw bytes into partial root.
    FailedToParsePartialRoot,
    /// Failed to get chunks count in storage.
    FailedToGetStoredChunksCount,
    /// Reached chunk count limit (overflow)
    ChunkCountOverflow,
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
    /// Failed to access storage for reading.
    FailedToReadStorage,
    /// Failed to access storage for writing.
    FailedToWriteToStorage,
    /// Failed to convert raw bytes into [`FileKey`].
    FailedToParseKey,
    /// Failed to construct iterator for trie.
    FailedToConstructTrieIter,
    /// Failed to convert raw bytes into [`FileMetadata`].
    FailedToParseFileMetadata,
    /// Failed to convert raw bytes into [`Fingerprint`].
    FailedToParseFingerprint,
    /// Failed to convert raw bytes into [`ChunkWithId`].
    FailedToParseChunkWithId,
    /// Failed to delete chunk from storage.
    FailedToDeleteFileChunk,
    /// Failed to convert raw bytes into partial root.
    FailedToParsePartialRoot,
    /// Failed to convert raw bytes into [`HasherOutT`].
    FailedToHasherOutput,
    /// File has size zero.
    FileIsEmpty,
    /// Failed to add entity to the exclude list.
    FailedToAddEntityToExcludeList,
    /// Failed to remove entity from the exclude list.
    FailedToAddEntityFromExcludeList,
    /// Trying to parse unknown exclude type.
    ErrorParsingExcludeType,
    /// Failed to get file key proof from file metadata.
    FailedToConstructFileKeyProof,
}

#[derive(Debug)]
pub enum FileStorageWriteOutcome {
    /// The file storage was completed after this write.
    /// All chunks for the file are stored and the fingerprints match too.
    FileComplete,
    /// The file was not completed after this chunk write.
    FileIncomplete,
}

#[derive(Eq, Hash, PartialEq)]
pub enum ExcludeType {
    File,
    User,
    Bucket,
    Fingerprint,
}

impl FromStr for ExcludeType {
    type Err = FileStorageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" => Ok(ExcludeType::File),
            "user" => Ok(ExcludeType::User),
            "bucket" => Ok(ExcludeType::Bucket),
            "fingerprint" => Ok(ExcludeType::Fingerprint),
            _ => Err(FileStorageError::ErrorParsingExcludeType),
        }
    }
}

pub trait FileDataTrie<T: TrieLayout> {
    /// Get the root of the trie.
    fn get_root(&self) -> &HasherOutT<T>;

    /// Generate proof for a set of chunks of a file. Returns error if the chunk does not exist.
    fn generate_proof(&self, chunk_ids: &HashSet<ChunkId>) -> Result<FileProof, FileStorageError>;

    // TODO: make it accept a list of chunks to be retrieved
    /// Get a file chunk from storage. Returns error if the chunk does not exist.
    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError>;

    // TODO: make it accept a list of chunks to be retrieved
    /// Write a file chunk in storage updating the root hash of the trie.
    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError>;

    /// Removes all references to chunks in the trie data and removes
    /// chunks themselves from storage.
    fn delete(&mut self) -> Result<(), FileStorageWriteError>;
}

/// Storage interface to be implemented by the storage providers.
pub trait FileStorage<T: TrieLayout>: 'static {
    type FileDataTrie: FileDataTrie<T> + Send + Sync;

    /// Creates a new [`FileDataTrie`] with no data and empty default root.
    /// Should be used as the default way of generating new tries.
    fn new_file_data_trie(&self) -> Self::FileDataTrie;

    /// Generate proof for a chunk of a file. If the file does not exists or any chunk is missing,
    /// no proof will be returned.
    fn generate_proof(
        &self,
        key: &HasherOutT<T>,
        chunk_ids: &HashSet<ChunkId>,
    ) -> Result<FileKeyProof, FileStorageError>;

    /// Remove a file from storage.
    fn delete_file(&mut self, key: &HasherOutT<T>) -> Result<(), FileStorageError>;

    fn delete_files_with_prefix(&mut self, prefix: &[u8; 32]) -> Result<(), FileStorageError>;

    /// Get metadata for a file.
    fn get_metadata(&self, key: &HasherOutT<T>) -> Result<Option<FileMetadata>, FileStorageError>;

    /// Check if a file is completely stored.
    fn is_file_complete(&self, key: &HasherOutT<T>) -> Result<bool, FileStorageError>;

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

    /// Get the number of stored chunks for a file key.
    fn stored_chunks_count(&self, key: &HasherOutT<T>) -> Result<u64, FileStorageError>;

    // TODO: Return Result<Option> instead of Result only
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

    fn is_allowed(
        &self,
        key: &HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<bool, FileStorageError>;

    fn add_to_exclude_list(
        &mut self,
        key: HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<(), FileStorageError>;

    fn remove_from_exclude_list(
        &mut self,
        key: &HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<(), FileStorageError>;
}

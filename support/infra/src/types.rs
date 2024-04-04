use crate::constants::FILE_CHUNK_SIZE;

// TODO: this is currently a placeholder in order to define Storage interface.
/// Hash type used by the Merkle tree.
pub struct Hash(pub [u8; 32]);

// TODO: this is currently a placeholder in order to define Storage interface.
/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
pub struct FileKey(pub Hash);

// TODO: this is currently a placeholder in order to define Storage interface.
/// Metadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
pub struct FileMetadata {
    pub size: u64,
    pub fingerprint: Hash,
}

// TODO: this is currently a placeholder in order to define Storage interface.
/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub struct FileChunk(pub [u8; FILE_CHUNK_SIZE]);

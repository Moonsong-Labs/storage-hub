use crate::constants::FILE_CHUNK_SIZE;

/// Typed version of u16 for the port number.
pub type Port = u16;

/// Placeholder for the (Merkle) hash type.
pub struct Hash(pub [u8; 32]);

/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
pub struct FileKey(pub Hash);

/// Metadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
pub struct FileMetadata {
    pub size: u64,
    pub fingerprint: Hash,
}

/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub struct FileChunk(pub [u8; FILE_CHUNK_SIZE]);

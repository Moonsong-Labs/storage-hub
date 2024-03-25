/// Typed version of u16 for the port number.
pub type Port = u16;

/// Placeholder for the (Merkle) hash type.
pub struct Hash([u8; 32]);

/// FileKey is the identifier for a file.
/// Computed as the hash of the FileMetadata.
pub struct FileKey(Hash);

/// Metadata contains information about a file.
/// Most importantly, the fingerprint which is the root Merkle hash of the file.
pub struct FileMetadata {
    pub size: u64,
    pub fingerprint: Hash,
}

const CHUNK_SIZE: usize = 1024 * 1024;

/// Typed chunk of a file. This is what is stored in the leaf of the stored Merkle tree.
pub struct FileChunk([u8; CHUNK_SIZE]);

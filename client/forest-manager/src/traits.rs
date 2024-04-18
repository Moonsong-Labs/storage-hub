use sp_core::H256;
use sp_trie::CompactProof;
use storage_hub_infra::types::{Key, Metadata, Proven};

pub struct ForestProof {
    /// The file key that was proven.
    pub proven: Proven<Metadata>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root_hash: H256,
}

/// Forest storage interface to be implemented by the storage providers.
pub trait ForestStorage: 'static {
    /// Generate proof for file key(s).
    fn generate_proof(&self, challenged_file_key: &Key) -> ForestProof;

    /// Insert a file key and generate a proof for it.
    fn insert_file_key(&self, file_key: &Key) -> ForestProof;

    /// Delete a file key and generate a proof for it.
    fn delete_file_key(&self, file_key: &Key) -> ForestProof;
}

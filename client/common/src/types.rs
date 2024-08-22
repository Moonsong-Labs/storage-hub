use std::fmt::Debug;

use codec::{Decode, Encode};
use sc_executor::WasmExecutor;
use sc_network::NetworkService;
use sc_service::TFullClient;
pub use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
pub use shp_file_metadata::{Chunk, ChunkId, Leaf};
use shp_traits::CommitmentVerifier;
use sp_core::Hasher;
use sp_runtime::{traits::Block as BlockT, KeyTypeId};
use sp_std::collections::btree_map::BTreeMap;
use sp_trie::CompactProof;
use storage_hub_runtime::{opaque::Block, Runtime, RuntimeApi};
use trie_db::TrieLayout;

/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

/// Following types are shared between the client and the runtime.
/// They are defined as generic types in the runtime and made concrete using the runtime config
/// here to be used by the node/client.
pub type FileKeyVerifier = <Runtime as pallet_proofs_dealer::Config>::KeyVerifier;
pub type FileKeyProof = <FileKeyVerifier as CommitmentVerifier>::Proof;
pub type Hash = shp_file_metadata::Hash<H_LENGTH>;
pub type Fingerprint = shp_file_metadata::Fingerprint<H_LENGTH>;
pub type FileMetadata =
    shp_file_metadata::FileMetadata<H_LENGTH, FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES>;
pub type FileKey = shp_file_metadata::FileKey<H_LENGTH>;
pub type BlockNumber = frame_system::pallet_prelude::BlockNumberFor<Runtime>;
pub type StorageData = pallet_file_system::types::StorageData<Runtime>;
pub type FileLocation = pallet_file_system::types::FileLocation<Runtime>;
pub type PeerIds = pallet_file_system::types::PeerIds<Runtime>;
pub type BucketId = pallet_storage_providers::types::MerklePatriciaRoot<Runtime>;
pub type ProviderId = pallet_proofs_dealer::types::ProviderIdFor<Runtime>;
pub type RandomnessOutput = pallet_proofs_dealer::types::RandomnessOutputFor<Runtime>;
pub type ForestLeaf = pallet_proofs_dealer::types::KeyFor<Runtime>;
pub type ForestRoot = pallet_proofs_dealer::types::ForestRootFor<Runtime>;
pub type TrieRemoveMutation = shp_traits::TrieRemoveMutation;
pub type StorageProofsMerkleTrieLayout = storage_hub_runtime::StorageProofsMerkleTrieLayout;
pub type StorageProof = pallet_proofs_dealer::types::Proof<Runtime>;
pub type ForestVerifierProof = pallet_proofs_dealer::types::ForestVerifierProofFor<Runtime>;
pub type KeyProof = pallet_proofs_dealer::types::KeyProof<Runtime>;
pub type KeyProofs = BTreeMap<ForestLeaf, KeyProof>;

#[cfg(not(feature = "runtime-benchmarks"))]
type HostFunctions = (
    // TODO: change this to `cumulus_client_service::ParachainHostFunctions` once it is part of the next release
    sp_io::SubstrateHostFunctions,
    cumulus_client_service::storage_proof_size::HostFunctions,
);

#[cfg(feature = "runtime-benchmarks")]
type HostFunctions = (
    // TODO: change this to `cumulus_client_service::ParachainHostFunctions` once it is part of the next release
    sp_io::SubstrateHostFunctions,
    cumulus_client_service::storage_proof_size::HostFunctions,
    frame_benchmarking::benchmarking::HostFunctions,
);

pub type ParachainExecutor = WasmExecutor<HostFunctions>;
pub type ParachainClient = TFullClient<Block, RuntimeApi, ParachainExecutor>;
pub type ParachainNetworkService = NetworkService<Block, <Block as BlockT>::Hash>;

/// The type of key used for [`BlockchainService`]` operations.
pub const BCSV_KEY_TYPE: KeyTypeId = KeyTypeId(*b"bcsv");

/// Proving either the exact key or the neighbour keys of the challenged key.
pub enum Proven<K, D: Debug> {
    Empty,
    ExactKey(Leaf<K, D>),
    NeighbourKeys((Option<Leaf<K, D>>, Option<Leaf<K, D>>)),
}

impl<K, D: Debug> Proven<K, D> {
    pub fn new_exact_key(key: K, data: D) -> Self {
        Proven::ExactKey(Leaf { key, data })
    }

    pub fn new_neighbour_keys(
        left: Option<Leaf<K, D>>,
        right: Option<Leaf<K, D>>,
    ) -> Result<Self, &'static str> {
        match (left, right) {
            (None, None) => Err("Both left and right leaves cannot be None"),
            (left, right) => Ok(Proven::NeighbourKeys((left, right))),
        }
    }
}

/// Proof of file key(s) in the forest trie.
pub struct ForestProof<T: TrieLayout> {
    /// The file key that was proven.
    pub proven: Vec<Proven<HasherOutT<T>, ()>>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root: HasherOutT<T>,
}

#[derive(Clone, Encode, Decode)]
pub struct FileProof {
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie, also known as the fingerprint of the file.
    pub fingerprint: Fingerprint,
}

impl FileProof {
    pub fn to_file_key_proof(&self, file_metadata: FileMetadata) -> FileKeyProof {
        FileKeyProof::new(
            file_metadata.owner.clone(),
            file_metadata.bucket_id.clone(),
            file_metadata.location.clone(),
            file_metadata.file_size,
            file_metadata.fingerprint,
            self.proof.clone(),
        )
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct DownloadRequestId(u64);

impl DownloadRequestId {
    pub fn new(id: u64) -> Self {
        DownloadRequestId(id)
    }

    pub fn next(&self) -> Self {
        let next = self.0 + 1;
        DownloadRequestId(next)
    }
}

use std::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

use codec::{Decode, Encode};
use frame_system::{Config, EventRecord};
use sc_executor::WasmExecutor;
use sc_service::TFullClient;
pub use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
pub use shp_file_metadata::{Chunk, ChunkId, ChunkWithId, Leaf};
use shp_opaque::Block;
use shp_traits::CommitmentVerifier;
use sp_core::Hasher;
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::{traits::Block as BlockT, KeyTypeId};
use sp_std::collections::btree_map::BTreeMap;
use sp_trie::CompactProof;
use sp_trie::LayoutV1;
use storage_hub_runtime::Runtime;
use trie_db::TrieLayout;

/// Size of each batch in bytes (2 MiB)
/// This is the maximum size of a batch of chunks that can be uploaded in a single call
/// (request-response round-trip).
pub const BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE: usize = 2 * 1024 * 1024;

pub trait StorageHubRuntime {
    type Runtime: pallet_file_system::Config;
    type BlockNumber;
}

// impl StorageHubRuntime for storage_hub_runtime::Runtime {
//     type Runtime = storage_hub_runtime::Runtime;

//     type BlockNumber = frame_system::pallet_prelude::BlockNumberFor<Runtime>;
// }

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
pub type BlockNumber<T> = frame_system::pallet_prelude::BlockNumberFor<T>;
pub type TickNumber<T> = pallet_file_system::types::TickNumber<T>;
pub type StorageData<T> = pallet_file_system::types::StorageDataUnit<T>;
pub type FileLocation<T> = pallet_file_system::types::FileLocation<T>;
pub type StorageRequestMspBucketResponse<T> =
    pallet_file_system::types::StorageRequestMspBucketResponse<T>;
pub type StorageRequestMspResponse<T> = pallet_file_system::types::StorageRequestMspResponse<T>;
pub type MaxUsersToCharge<T> = pallet_payment_streams::types::MaxUsersToChargeFor<T>;
pub type RejectedStorageRequestReason = pallet_file_system::types::RejectedStorageRequestReason;
pub type RejectedStorageRequest<T> = pallet_file_system::types::RejectedStorageRequest<T>;
pub type StorageRequestMspAcceptedFileKeys<T> =
    pallet_file_system::types::StorageRequestMspAcceptedFileKeys<T>;
pub type FileKeyWithProof<T> = pallet_file_system::types::FileKeyWithProof<T>;
pub type PeerIds<T> = pallet_file_system::types::PeerIds<T>;
pub type BucketId<T> = pallet_storage_providers::types::ProviderIdFor<T>;
pub type ValuePropId<T> = pallet_storage_providers::types::ValuePropId<T>;
pub type StorageProviderId<T> = pallet_storage_providers::types::StorageProviderId<T>;
pub type BackupStorageProviderId<T> = pallet_storage_providers::types::BackupStorageProviderId<T>;
pub type BackupStorageProviderInfo<T> = pallet_storage_providers::types::BackupStorageProvider<T>;
pub type MainStorageProviderId<T> = pallet_storage_providers::types::MainStorageProviderId<T>;
pub type ProviderId<T> = pallet_storage_providers::types::ProviderIdFor<T>;
pub type ProofsDealerProviderId<T> = pallet_proofs_dealer::types::ProviderIdFor<T>;
pub type Multiaddresses<T> = pallet_storage_providers::types::Multiaddresses<T>;
pub type MultiAddress<T> = pallet_storage_providers::types::MultiAddress<T>;
pub type RandomnessOutput<T> = pallet_proofs_dealer::types::RandomnessOutputFor<T>;
pub type ForestLeaf<T> = pallet_proofs_dealer::types::KeyFor<T>;
pub type ForestRoot<T> = pallet_proofs_dealer::types::ForestRootFor<T>;
pub type CustomChallenge<T> = pallet_proofs_dealer::types::CustomChallenge<T>;
pub type TrieMutation = shp_traits::TrieMutation;
pub type TrieRemoveMutation = shp_traits::TrieRemoveMutation;
pub type TrieAddMutation = shp_traits::TrieAddMutation;
pub type StorageProofsMerkleTrieLayout = LayoutV1<BlakeTwo256>;
pub type StorageProof<T> = pallet_proofs_dealer::types::Proof<T>;
pub type ForestVerifierProof<T> = pallet_proofs_dealer::types::ForestVerifierProofFor<T>;
pub type KeyProof<T> = pallet_proofs_dealer::types::KeyProof<T>;
pub type KeyProofs<T> = BTreeMap<ForestLeaf<T>, KeyProof<T>>;
pub type Balance<T> = pallet_storage_providers::types::BalanceOf<T>;
pub type OpaqueBlock = shp_opaque::Block;
pub type BlockHash = <OpaqueBlock as BlockT>::Hash;
pub type PeerId<T> = pallet_file_system::types::PeerId<T>;
pub type StorageRequestMetadata<T> = pallet_file_system::types::StorageRequestMetadata<T>;
pub type MaxBatchConfirmStorageRequests<T> =
    <T as pallet_file_system::Config>::MaxBatchConfirmStorageRequests;
pub type ValuePropositionWithId<T> = pallet_storage_providers::types::ValuePropositionWithId<T>;

/// Type alias for the events vector.
///
/// The events vector is a storage element in the FRAME system pallet, which stores all the events
/// that have occurred in a block. This is syntactic sugar to make the code more readable.
pub type StorageHubEventsVec<T> = Vec<
    Box<EventRecord<<T as frame_system::Config>::RuntimeEvent, <T as frame_system::Config>::Hash>>,
>;

#[cfg(not(feature = "runtime-benchmarks"))]
type HostFunctions = cumulus_client_service::ParachainHostFunctions;

#[cfg(feature = "runtime-benchmarks")]
type HostFunctions = (
    cumulus_client_service::ParachainHostFunctions,
    frame_benchmarking::benchmarking::HostFunctions,
);

pub type ParachainExecutor = WasmExecutor<HostFunctions>;
pub type ParachainClient<RuntimeApi> = TFullClient<Block, RuntimeApi, ParachainExecutor>;

/// The type of key used for [`BlockchainService`]` operations.
pub const BCSV_KEY_TYPE: KeyTypeId = KeyTypeId(*b"bcsv");

/// Proving either the exact key or the neighbour keys of the challenged key.
#[derive(Clone, Debug)]
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
#[derive(Clone, Encode, Decode, Debug)]
pub struct ForestProof<T: TrieLayout> {
    /// The file keys that were proven.
    #[codec(skip)]
    pub proven: Vec<Proven<HasherOutT<T>, ()>>,
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie.
    pub root: HasherOutT<T>,
}

impl<T: TrieLayout> ForestProof<T> {
    /// Returns whether a file key was found in the forest proof.
    pub fn contains_file_key(&self, file_key: &HasherOutT<T>) -> bool {
        self.proven.iter().any(|proven| match proven {
            Proven::ExactKey(leaf) => leaf.key.as_ref() == file_key.as_ref(),
            _ => false,
        })
    }
}

#[derive(Clone, Encode, Decode)]
pub struct FileProof {
    /// The compact proof.
    pub proof: CompactProof,
    /// The root hash of the trie, also known as the fingerprint of the file.
    pub fingerprint: Fingerprint,
}

impl FileProof {
    pub fn to_file_key_proof(
        &self,
        file_metadata: FileMetadata,
    ) -> Result<FileKeyProof, FileProofError> {
        FileKeyProof::new(
            file_metadata.owner().clone(),
            file_metadata.bucket_id().clone(),
            file_metadata.location().clone(),
            file_metadata.file_size(),
            *file_metadata.fingerprint(),
            self.proof.clone(),
        )
        .map_err(|_| FileProofError::InvalidFileMetadata)
    }
}

#[derive(Debug, Clone)]
pub enum FileProofError {
    InvalidFileMetadata,
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct DownloadRequestId(u64);

impl DownloadRequestId {
    pub fn new(id: u64) -> Self {
        DownloadRequestId(id)
    }

    pub fn next(&self) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        DownloadRequestId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct UploadRequestId(u64);

impl UploadRequestId {
    pub fn new(id: u64) -> Self {
        UploadRequestId(id)
    }

    pub fn next(&self) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        UploadRequestId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

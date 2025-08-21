use std::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

use codec::{Decode, Encode};
use frame_system::EventRecord;
use sc_executor::WasmExecutor;
use sc_service::TFullClient;
pub use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
pub use shp_file_metadata::{Chunk, ChunkId, ChunkWithId, Leaf};
use shp_opaque::Block;
use shp_traits::CommitmentVerifier;
use sp_core::Hasher;
use sp_runtime::{generic, traits::Block as BlockT, KeyTypeId};
use sp_std::collections::btree_map::BTreeMap;
use sp_trie::CompactProof;
use storage_hub_runtime::Runtime;
use trie_db::TrieLayout;

use crate::traits::{ExtensionOperations, StorageEnableRuntime};

/// Size of each batch in bytes (2 MiB)
/// This is the maximum size of a batch of chunks that can be uploaded in a single call
/// (request-response round-trip).
pub const BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE: usize = 2 * 1024 * 1024;

/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

/// Following types are shared between the client and the runtime.
/// They are defined as generic types in the runtime and made concrete using the runtime config
/// here to be used by the node/client.
pub type AccountId<Runtime> = <Runtime as frame_system::Config>::AccountId;
pub type FileKeyVerifier = <Runtime as pallet_proofs_dealer::Config>::KeyVerifier;
pub type FileKeyProof = <FileKeyVerifier as CommitmentVerifier>::Proof;
pub type Hash = shp_file_metadata::Hash<H_LENGTH>;
pub type Fingerprint = shp_file_metadata::Fingerprint<H_LENGTH>;
pub type FileMetadata =
    shp_file_metadata::FileMetadata<H_LENGTH, FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES>;
pub type FileKey = shp_file_metadata::FileKey<H_LENGTH>;
pub type BlockNumber = frame_system::pallet_prelude::BlockNumberFor<Runtime>;
pub type TickNumber = pallet_file_system::types::TickNumber<Runtime>;
pub type StorageData = pallet_file_system::types::StorageDataUnit<Runtime>;
pub type FileLocation = pallet_file_system::types::FileLocation<Runtime>;
pub type StorageRequestMspBucketResponse =
    pallet_file_system::types::StorageRequestMspBucketResponse<Runtime>;
pub type StorageRequestMspResponse = pallet_file_system::types::StorageRequestMspResponse<Runtime>;
pub type MaxUsersToCharge = pallet_payment_streams::types::MaxUsersToChargeFor<Runtime>;
pub type RejectedStorageRequestReason = pallet_file_system::types::RejectedStorageRequestReason;
pub type RejectedStorageRequest = pallet_file_system::types::RejectedStorageRequest<Runtime>;
pub type StorageRequestMspAcceptedFileKeys =
    pallet_file_system::types::StorageRequestMspAcceptedFileKeys<Runtime>;
pub type FileKeyWithProof = pallet_file_system::types::FileKeyWithProof<Runtime>;
pub type PeerIds = pallet_file_system::types::PeerIds<Runtime>;
pub type FileOperationIntention = pallet_file_system::types::FileOperationIntention<Runtime>;
pub type FileOperation = pallet_file_system::types::FileOperation;
pub type BucketId = pallet_storage_providers::types::ProviderIdFor<Runtime>;
pub type ValuePropId = pallet_storage_providers::types::ValuePropId<Runtime>;
pub type StorageProviderId = pallet_storage_providers::types::StorageProviderId<Runtime>;
pub type BackupStorageProviderId =
    pallet_storage_providers::types::BackupStorageProviderId<Runtime>;
pub type BackupStorageProviderInfo =
    pallet_storage_providers::types::BackupStorageProvider<Runtime>;
pub type MainStorageProviderId = pallet_storage_providers::types::MainStorageProviderId<Runtime>;
pub type ProviderId = pallet_storage_providers::types::ProviderIdFor<Runtime>;
pub type ProofsDealerProviderId = pallet_proofs_dealer::types::ProviderIdFor<Runtime>;
pub type Multiaddresses = pallet_storage_providers::types::Multiaddresses<Runtime>;
pub type MultiAddress = pallet_storage_providers::types::MultiAddress<Runtime>;
pub type RandomnessOutput = pallet_proofs_dealer::types::RandomnessOutputFor<Runtime>;
pub type ForestLeaf = pallet_proofs_dealer::types::KeyFor<Runtime>;
pub type ForestRoot = pallet_proofs_dealer::types::ForestRootFor<Runtime>;
pub type CustomChallenge = pallet_proofs_dealer::types::CustomChallenge<Runtime>;
pub type TrieMutation = shp_traits::TrieMutation;
pub type TrieRemoveMutation = shp_traits::TrieRemoveMutation;
pub type TrieAddMutation = shp_traits::TrieAddMutation;
pub type StorageProofsMerkleTrieLayout = storage_hub_runtime::StorageProofsMerkleTrieLayout;
pub type StorageProof = pallet_proofs_dealer::types::Proof<Runtime>;
pub type ForestVerifierProof = pallet_proofs_dealer::types::ForestVerifierProofFor<Runtime>;
pub type KeyProof = pallet_proofs_dealer::types::KeyProof<Runtime>;
pub type KeyProofs = BTreeMap<ForestLeaf, KeyProof>;
pub type Balance = pallet_storage_providers::types::BalanceOf<Runtime>;
pub type OpaqueBlock = storage_hub_runtime::opaque::Block;
pub type BlockHash = <OpaqueBlock as BlockT>::Hash;
pub type PeerId = pallet_file_system::types::PeerId<Runtime>;
pub type StorageRequestMetadata = pallet_file_system::types::StorageRequestMetadata<Runtime>;
pub type MaxBatchConfirmStorageRequests =
    <Runtime as pallet_file_system::Config>::MaxBatchConfirmStorageRequests;
pub type ValuePropositionWithId = pallet_storage_providers::types::ValuePropositionWithId<Runtime>;
pub type Tip = pallet_transaction_payment::ChargeTransactionPayment<Runtime>;

/// Type alias for the events vector.
///
/// The events vector is a storage element in the FRAME system pallet, which stores all the events
/// that have occurred in a block. This is syntactic sugar to make the code more readable.
pub type StorageHubEventsVec = Vec<
    Box<
        EventRecord<
            <storage_hub_runtime::Runtime as frame_system::Config>::RuntimeEvent,
            <storage_hub_runtime::Runtime as frame_system::Config>::Hash,
        >,
    >,
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

pub struct MinimalExtension {
    pub era: generic::Era,
    pub nonce: u32,
    pub tip: Tip,
}

impl MinimalExtension {
    pub fn new(era: generic::Era, nonce: u32, tip: Tip) -> Self {
        Self { era, nonce, tip }
    }
}

//TODO: This should be moved to the runtime crate once the SH Client is abstracted
//TODO: from the runtime. If we put it there now, we will have a cyclic dependency.
impl StorageEnableRuntime for storage_hub_runtime::Runtime {
    type Address = storage_hub_runtime::Address;
    type Call = storage_hub_runtime::RuntimeCall;
    type Signature = storage_hub_runtime::Signature;
    type Extension = storage_hub_runtime::SignedExtra;
    type RuntimeApi = storage_hub_runtime::apis::RuntimeApi;
}

//TODO: This should be moved to the runtime crate once the SH Client is abstracted
//TODO: from the runtime. If we put it there now, we will have a cyclic dependency.
impl ExtensionOperations<storage_hub_runtime::RuntimeCall> for storage_hub_runtime::SignedExtra {
    type Hash = storage_hub_runtime::Hash;

    fn from_minimal_extension(minimal: MinimalExtension) -> Self {
        (
            frame_system::CheckNonZeroSender::<Runtime>::new(),
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(minimal.era),
            frame_system::CheckNonce::<Runtime>::from(minimal.nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            minimal.tip,
            cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim::<Runtime>::new(),
            frame_metadata_hash_extension::CheckMetadataHash::new(false),
        )
    }

    fn implicit(genesis_block_hash: Self::Hash, current_block_hash: Self::Hash) -> Self::Implicit {
        (
            (),
            storage_hub_runtime::VERSION.spec_version,
            storage_hub_runtime::VERSION.transaction_version,
            genesis_block_hash,
            current_block_hash,
            (),
            (),
            (),
            (),
            None,
        )
    }
}

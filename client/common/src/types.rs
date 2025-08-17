use std::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

use codec::{Decode, Encode};
use sc_executor::WasmExecutor;
use sc_service::TFullClient;
pub use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
pub use shp_file_metadata::{Chunk, ChunkId, ChunkWithId, Leaf};
use shp_traits::ProofsDealerInterface;
use sp_core::Hasher;
use sp_runtime::{generic, KeyTypeId};
use sp_std::collections::btree_map::BTreeMap;
use sp_trie::CompactProof;
use trie_db::TrieLayout;

/// Size of each batch in bytes (2 MiB)
/// This is the maximum size of a batch of chunks that can be uploaded in a single call
/// (request-response round-trip).
pub const BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE: usize = 2 * 1024 * 1024;

/// The hash type of trie node keys
pub type HashT<T> = <T as TrieLayout>::Hash;
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

// The following are concrete types that are defined in the pallets/primitives.
// These are type aliases for convenience to use in the SH Client.
pub type MetadataHash = shp_file_metadata::Hash<H_LENGTH>;
pub type Fingerprint = shp_file_metadata::Fingerprint<H_LENGTH>;
pub type FileMetadata =
    shp_file_metadata::FileMetadata<H_LENGTH, FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES>;
pub type FileKey = shp_file_metadata::FileKey<H_LENGTH>;
pub type RejectedStorageRequestReason = pallet_file_system::types::RejectedStorageRequestReason;
pub type FileOperation = pallet_file_system::types::FileOperation;
pub type TrieMutation = shp_traits::TrieMutation;
pub type TrieRemoveMutation = shp_traits::TrieRemoveMutation;
pub type TrieAddMutation = shp_traits::TrieAddMutation;
pub type OpaqueBlock = shp_opaque::Block;
pub type BlockHash = shp_opaque::Hash;
pub type StorageProofsMerkleTrieLayout = shp_types::StorageProofsMerkleTrieLayout;
pub type FileKeyVerifier = shp_file_key_verifier::FileKeyVerifier<
    StorageProofsMerkleTrieLayout,
    H_LENGTH,
    FILE_CHUNK_SIZE,
    FILE_SIZE_TO_CHALLENGES,
>;
pub type FileKeyProof =
    shp_file_key_verifier::types::FileKeyProof<H_LENGTH, FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES>;
pub type ForestVerifier =
    shp_forest_verifier::ForestVerifier<StorageProofsMerkleTrieLayout, H_LENGTH>;
pub type ForestVerifierProof = <ForestVerifier as shp_traits::CommitmentVerifier>::Proof;

// The following are abstracted types that depend on a runtime implementing the `Config`
// traits of the various pallets.
// These are type aliases for convenience to use in the SH Client.
pub type Hash<Runtime> = <Runtime as frame_system::Config>::Hash;
pub type AccountId<Runtime> = <Runtime as frame_system::Config>::AccountId;
pub type BlockNumber<Runtime> = frame_system::pallet_prelude::BlockNumberFor<Runtime>;
pub type TickNumber<Runtime> =
    <pallet_proofs_dealer::Pallet<Runtime> as ProofsDealerInterface>::TickNumber;
pub type StorageDataUnit<Runtime> = <Runtime as pallet_storage_providers::Config>::StorageDataUnit;
pub type FileLocation<Runtime> = pallet_file_system::types::FileLocation<Runtime>;
pub type StorageRequestMspBucketResponse<Runtime> =
    pallet_file_system::types::StorageRequestMspBucketResponse<Runtime>;
pub type StorageRequestMspResponse<Runtime> =
    pallet_file_system::types::StorageRequestMspResponse<Runtime>;
pub type MaxUsersToCharge<Runtime> = pallet_payment_streams::types::MaxUsersToChargeFor<Runtime>;
pub type RejectedStorageRequest<Runtime> =
    pallet_file_system::types::RejectedStorageRequest<Runtime>;
pub type StorageRequestMspAcceptedFileKeys<Runtime> =
    pallet_file_system::types::StorageRequestMspAcceptedFileKeys<Runtime>;
pub type FileKeyWithProof<Runtime> = pallet_file_system::types::FileKeyWithProof<Runtime>;
pub type PeerIds<Runtime> = pallet_file_system::types::PeerIds<Runtime>;
pub type FileOperationIntention<Runtime> =
    pallet_file_system::types::FileOperationIntention<Runtime>;
pub type BucketId<Runtime> = pallet_storage_providers::types::ProviderIdFor<Runtime>;
pub type ValuePropId<Runtime> = pallet_storage_providers::types::ValuePropId<Runtime>;
pub type StorageProviderId<Runtime> = pallet_storage_providers::types::StorageProviderId<Runtime>;
pub type BackupStorageProviderId<Runtime> =
    pallet_storage_providers::types::BackupStorageProviderId<Runtime>;
pub type BackupStorageProviderInfo<Runtime> =
    pallet_storage_providers::types::BackupStorageProvider<Runtime>;
pub type MainStorageProviderId<Runtime> =
    pallet_storage_providers::types::MainStorageProviderId<Runtime>;
pub type ProviderId<Runtime> = pallet_storage_providers::types::ProviderIdFor<Runtime>;
pub type ProofsDealerProviderId<Runtime> = pallet_proofs_dealer::types::ProviderIdFor<Runtime>;
pub type Multiaddresses<Runtime> = pallet_storage_providers::types::Multiaddresses<Runtime>;
pub type MultiAddress<Runtime> = pallet_storage_providers::types::MultiAddress<Runtime>;
pub type RandomnessOutput<Runtime> = pallet_proofs_dealer::types::RandomnessOutputFor<Runtime>;
pub type ForestLeaf<Runtime> = pallet_proofs_dealer::types::KeyFor<Runtime>;
pub type ForestRoot<Runtime> = pallet_proofs_dealer::types::ForestRootFor<Runtime>;
pub type CustomChallenge<Runtime> = pallet_proofs_dealer::types::CustomChallenge<Runtime>;
pub type StorageProof<Runtime> = pallet_proofs_dealer::types::Proof<Runtime>;
pub type KeyProof<Runtime> = pallet_proofs_dealer::types::KeyProof<Runtime>;
pub type KeyProofs<Runtime> = BTreeMap<ForestLeaf<Runtime>, KeyProof<Runtime>>;
pub type Balance<Runtime> = <Runtime as pallet_balances::Config>::Balance;
pub type PeerId<Runtime> = pallet_file_system::types::PeerId<Runtime>;
pub type StorageRequestMetadata<Runtime> =
    pallet_file_system::types::StorageRequestMetadata<Runtime>;
pub type MaxBatchConfirmStorageRequests<Runtime> =
    <Runtime as pallet_file_system::Config>::MaxBatchConfirmStorageRequests;
pub type ValuePropositionWithId<Runtime> =
    pallet_storage_providers::types::ValuePropositionWithId<Runtime>;

/// Type alias for the events vector.
///
/// The events vector is a storage element in the FRAME system pallet, which stores all the events
/// that have occurred in a block. This is syntactic sugar to make the code more readable.
pub type StorageHubEventsVec<Runtime> = Vec<
    Box<
        frame_system::EventRecord<
            <Runtime as frame_system::Config>::RuntimeEvent,
            <Runtime as frame_system::Config>::Hash,
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

// TODO: Consider not using `ParachainExecutor` as the runtime might not be a parachain.
pub type ParachainExecutor = WasmExecutor<HostFunctions>;
pub type ParachainClient<RuntimeApi> =
    TFullClient<shp_opaque::Block, RuntimeApi, ParachainExecutor>;

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
    pub fn new(proof: CompactProof, fingerprint: Fingerprint) -> Self {
        Self { proof, fingerprint }
    }

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
    pub tip: u128,
}

impl MinimalExtension {
    pub fn new(era: generic::Era, nonce: u32, tip: u128) -> Self {
        Self { era, nonce, tip }
    }
}

/// Set of pallets and their events that are relevant to the StorageHub Client.
///
/// This enum serves to convert the runtime's `RuntimeEvent` into a known set of
/// storage-related events that the client cares about. It allows the
/// client to match on these events without having to know about every pallet
/// that may exist in the runtime.
///
/// The enum intentionally includes a catch-all `Other` variant so that
/// unrecognized or out-of-scope events can be ignored without breaking
/// client logic.
///
/// Conversion from the concrete runtime event is up to each StorageHub-compatible
/// runtime to implement.
#[derive(Debug, Clone)]
pub enum StorageEnableEvents<Runtime>
where
    Runtime: frame_system::Config
        + pallet_storage_providers::Config
        + pallet_proofs_dealer::Config
        + pallet_payment_streams::Config
        + pallet_file_system::Config
        + pallet_transaction_payment::Config
        + pallet_balances::Config
        + pallet_bucket_nfts::Config
        + pallet_randomness::Config,
{
    /// Events emitted by the [frame_system](https://docs.rs/frame-system/latest/frame_system/) pallet.
    System(frame_system::Event<Runtime>),
    /// Events from [`pallet_storage_providers`].
    StorageProviders(pallet_storage_providers::Event<Runtime>),
    /// Events from [`pallet_proofs_dealer`].
    ProofsDealer(pallet_proofs_dealer::Event<Runtime>),
    /// Events from [`pallet_payment_streams`].
    PaymentStreams(pallet_payment_streams::Event<Runtime>),
    /// Events from [`pallet_file_system`].
    FileSystem(pallet_file_system::Event<Runtime>),
    /// Events from [`pallet_transaction_payment`](https://docs.rs/pallet-transaction-payment/latest/pallet_transaction_payment/).
    TransactionPayment(pallet_transaction_payment::Event<Runtime>),
    /// Events from [`pallet_balances`](https://docs.rs/pallet-balances/latest/pallet_balances/index.html).
    Balances(pallet_balances::Event<Runtime>),
    /// Events from [`pallet_bucket_nfts`].
    BucketNfts(pallet_bucket_nfts::Event<Runtime>),
    /// Events from [`pallet_randomness`].
    Randomness(pallet_randomness::Event<Runtime>),
    /// Catch-all for events that we do not care in the SH Client.
    Other(<Runtime as frame_system::Config>::RuntimeEvent),
}

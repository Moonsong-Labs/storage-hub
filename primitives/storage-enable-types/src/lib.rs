#![cfg_attr(not(feature = "std"), no_std)]

use shp_constants::{FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES, H_LENGTH};
use shp_traits::ProofsDealerInterface;
use sp_core::H256;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::LayoutV1;

/// A hash of some data used by the chain.
pub type Hash = H256;

/// The hashing algorithm used.
pub type Hashing = BlakeTwo256;

/// The layout of the storage proofs merkle trie.
pub type StorageProofsMerkleTrieLayout = LayoutV1<BlakeTwo256>;

/// Type representing the storage data units in StorageHub.
pub type StorageDataUnit = u64;

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
pub type HashFor<Runtime> = <Runtime as frame_system::Config>::Hash;
pub type AccountId<Runtime> = <Runtime as frame_system::Config>::AccountId;
pub type BlockNumber<Runtime> = frame_system::pallet_prelude::BlockNumberFor<Runtime>;
pub type TickNumber<Runtime> =
    <pallet_proofs_dealer::Pallet<Runtime> as ProofsDealerInterface>::TickNumber;
pub type StorageDataUnitFor<Runtime> =
    <Runtime as pallet_storage_providers::Config>::StorageDataUnit;
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
pub type Balance<Runtime> = <Runtime as pallet_balances::Config>::Balance;
pub type PeerId<Runtime> = pallet_file_system::types::PeerId<Runtime>;
pub type StorageRequestMetadata<Runtime> =
    pallet_file_system::types::StorageRequestMetadata<Runtime>;
pub type MaxBatchConfirmStorageRequests<Runtime> =
    <Runtime as pallet_file_system::Config>::MaxBatchConfirmStorageRequests;
pub type ValuePropositionWithId<Runtime> =
    pallet_storage_providers::types::ValuePropositionWithId<Runtime>;

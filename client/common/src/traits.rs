use crate::events::EventsStorageEnable;
use crate::types::*;
use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_payment_streams_runtime_api::PaymentStreamsApi as PaymentStreamsRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi as StorageProvidersRuntimeApi;
use polkadot_primitives::AccountId;
use polkadot_primitives::Nonce;
use shp_traits::ProofsDealerInterface;
use shp_traits::ReadChallengeableProvidersInterface;
use shp_traits::ReadProvidersInterface;
use shp_traits::ReadStorageProvidersInterface;
use sp_trie::CompactProof;
use storage_hub_runtime::opaque;

use core::default::Default;
use core::fmt::Debug;
use core::marker::{Send, Sync};
use sc_service::TFullClient;
use shp_opaque::Block;
use shp_traits::ReadBucketsInterface;
use sp_api::ConstructRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_core::H256;
use sp_runtime::traits::AsSystemOriginSigner;
use sp_runtime::traits::Get;
use sp_runtime::AccountId32;

pub trait StorageEnableRuntimeConfig:
    frame_system::Config<
        RuntimeCall: core::marker::Send,
        RuntimeEvent: Into<EventsStorageEnable<Self>>,
        AccountId = AccountId32,
        Hash = H256,
        Block = opaque::Block,
        Nonce = u32,
        RuntimeOrigin: AsSystemOriginSigner<sp_runtime::AccountId32>,
    > + pallet_file_system::Config<
        RuntimeEvent: Into<EventsStorageEnable<Self>>,
        Fingerprint = Fingerprint,
        Providers: ReadProvidersInterface<ProviderId = H256, MerkleHash = H256>
                       + ReadStorageProvidersInterface<
            ValuePropId = H256,
            StorageDataUnit = u64,
            MultiAddress: core::marker::Send + core::marker::Sync,
            MaxNumberOfMultiAddresses: core::marker::Send + core::marker::Sync,
        > + ReadBucketsInterface<
            BucketNameLimit: core::marker::Send + core::marker::Sync,
        >,
        ProofDealer: ProofsDealerInterface<TickNumber = u32>,
        MaxNumberOfPeerIds: Get<u32> + core::marker::Send + core::marker::Sync,
        MaxPeerIdSize: Get<u32> + core::marker::Send + core::marker::Sync,
        MaxBatchConfirmStorageRequests: core::marker::Send + core::marker::Sync,
        MaxFilePathSize: core::marker::Send + core::marker::Sync,
        Nfts: frame_support::traits::tokens::nonfungibles_v2::Inspect<
            sp_runtime::AccountId32,
            CollectionId = u128,
        >,
    > + pallet_storage_providers::Config<
        RuntimeEvent: Into<EventsStorageEnable<Self>>,
        StorageDataUnit = u64,
        ProviderId = H256,
        MerklePatriciaRoot = H256,
        MaxCommitmentSize: Get<u32> + core::marker::Send + core::marker::Sync,
        MaxMultiAddressSize: Get<u32> + core::marker::Send + core::marker::Sync,
        NativeBalance = <Self as pallet_payment_streams::Config>::NativeBalance,
        MaxMultiAddressAmount: core::marker::Send + core::marker::Sync,
    > + pallet_proofs_dealer::Config<
        RuntimeEvent: Into<EventsStorageEnable<Self>>,
        ProvidersPallet: ReadChallengeableProvidersInterface<ProviderId = H256>,
        MerkleTrieHash = H256,
        KeyVerifier = shp_file_key_verifier::FileKeyVerifier<
            StorageProofsMerkleTrieLayout,
            { shp_constants::H_LENGTH },
            { shp_constants::FILE_CHUNK_SIZE },
            { shp_constants::FILE_SIZE_TO_CHALLENGES },
        >,
        MaxCustomChallengesPerBlock: Get<u32> + core::marker::Sync + core::marker::Send,
        ForestVerifier: shp_traits::CommitmentVerifier<Proof = CompactProof>,
    > + pallet_bucket_nfts::Config<RuntimeEvent: Into<EventsStorageEnable<Self>>>
    + pallet_transaction_payment::Config
    + pallet_payment_streams::Config<
        ProvidersPallet: ReadProvidersInterface<ProviderId = H256>,
        MaxUsersToCharge: core::marker::Send,
    > + pallet_randomness::Config
    + Default
    + Send
    + Sync
    + Debug
{
}

impl<T> StorageEnableRuntimeConfig for T where
    T: frame_system::Config<
            RuntimeCall: core::marker::Send,
            RuntimeEvent: Into<EventsStorageEnable<Self>>,
            AccountId = AccountId32,
            Hash = H256,
            Block = opaque::Block,
            Nonce = u32,
            RuntimeOrigin: AsSystemOriginSigner<sp_runtime::AccountId32>,
        > + pallet_file_system::Config<
            RuntimeEvent: Into<EventsStorageEnable<Self>>,
            Fingerprint = Fingerprint,
            Providers: ReadProvidersInterface<ProviderId = H256, MerkleHash = H256>
                           + ReadStorageProvidersInterface<
                ValuePropId = H256,
                StorageDataUnit = u64,
                MultiAddress: core::marker::Send + core::marker::Sync,
                MaxNumberOfMultiAddresses: core::marker::Send + core::marker::Sync,
            > + ReadBucketsInterface<
                BucketNameLimit: core::marker::Send + core::marker::Sync,
            >,
            ProofDealer: ProofsDealerInterface<TickNumber = u32>,
            MaxNumberOfPeerIds: Get<u32> + core::marker::Send + core::marker::Sync,
            MaxPeerIdSize: Get<u32> + core::marker::Send + core::marker::Sync,
            MaxBatchConfirmStorageRequests: core::marker::Send + core::marker::Sync,
            MaxFilePathSize: core::marker::Send + core::marker::Sync,
            Nfts: frame_support::traits::tokens::nonfungibles_v2::Inspect<
                sp_runtime::AccountId32,
                CollectionId = u128,
            >,
        > + pallet_storage_providers::Config<
            RuntimeEvent: Into<EventsStorageEnable<Self>>,
            StorageDataUnit = u64,
            ProviderId = H256,
            MerklePatriciaRoot = H256,
            MaxCommitmentSize: Get<u32> + core::marker::Send + core::marker::Sync,
            MaxMultiAddressSize: Get<u32> + core::marker::Send + core::marker::Sync,
            NativeBalance = <Self as pallet_payment_streams::Config>::NativeBalance,
            MaxMultiAddressAmount: core::marker::Send + core::marker::Sync,
        > + pallet_proofs_dealer::Config<
            RuntimeEvent: Into<EventsStorageEnable<Self>>,
            ProvidersPallet: ReadChallengeableProvidersInterface<ProviderId = H256>,
            MerkleTrieHash = H256,
            KeyVerifier = shp_file_key_verifier::FileKeyVerifier<
                StorageProofsMerkleTrieLayout,
                { shp_constants::H_LENGTH },
                { shp_constants::FILE_CHUNK_SIZE },
                { shp_constants::FILE_SIZE_TO_CHALLENGES },
            >,
            MaxCustomChallengesPerBlock: Get<u32> + core::marker::Sync + core::marker::Send,
            ForestVerifier: shp_traits::CommitmentVerifier<Proof = CompactProof>,
        > + pallet_bucket_nfts::Config<RuntimeEvent: Into<EventsStorageEnable<Self>>>
        + pallet_transaction_payment::Config
        + pallet_payment_streams::Config<
            ProvidersPallet: ReadProvidersInterface<ProviderId = H256>,
            MaxUsersToCharge: core::marker::Send,
        > + pallet_randomness::Config
        + Default
        + Send
        + Sync
        + Debug
{
}

pub trait StorageEnableApiCollection<Runtime: StorageEnableRuntimeConfig>:
    // pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance<Runtime>>
    substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
    + BlockBuilder<Block>
    + ProofsDealerRuntimeApi<
        Block,
        ProofsDealerProviderId<Runtime>,
        BlockNumber<Runtime>,
        ForestLeaf<Runtime>,
        RandomnessOutput<Runtime>,
        CustomChallenge<Runtime>,
    > + FileSystemRuntimeApi<
        Block,
        BackupStorageProviderId<Runtime>,
        MainStorageProviderId<Runtime>,
        H256,
        BlockNumber<Runtime>,
        ChunkId,
        BucketId<Runtime>,
        StorageRequestMetadata<Runtime>,
    > + StorageProvidersRuntimeApi<
        Block,
        BlockNumber<Runtime>,
        BackupStorageProviderId<Runtime>,
        BackupStorageProviderInfo<Runtime>,
        MainStorageProviderId<Runtime>,
        AccountId,
        ProviderId<Runtime>,
        StorageProviderId<Runtime>,
        StorageData<Runtime>,
        Balance<Runtime>,
        BucketId<Runtime>,
        Multiaddresses<Runtime>,
        ValuePropositionWithId<Runtime>,
    > + PaymentStreamsRuntimeApi<Block, ProviderId<Runtime>, Balance<Runtime>, AccountId>
{
}

impl<T, Runtime: StorageEnableRuntimeConfig> StorageEnableApiCollection<Runtime> for T where
    T: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + BlockBuilder<Block>
        + ProofsDealerRuntimeApi<
            Block,
            ProofsDealerProviderId<Runtime>,
            BlockNumber<Runtime>,
            ForestLeaf<Runtime>,
            RandomnessOutput<Runtime>,
            CustomChallenge<Runtime>,
        > + FileSystemRuntimeApi<
            Block,
            BackupStorageProviderId<Runtime>,
            MainStorageProviderId<Runtime>,
            H256,
            BlockNumber<Runtime>,
            ChunkId,
            BucketId<Runtime>,
            StorageRequestMetadata<Runtime>,
        > + StorageProvidersRuntimeApi<
            Block,
            BlockNumber<Runtime>,
            BackupStorageProviderId<Runtime>,
            BackupStorageProviderInfo<Runtime>,
            MainStorageProviderId<Runtime>,
            AccountId,
            ProviderId<Runtime>,
            StorageProviderId<Runtime>,
            StorageData<Runtime>,
            Balance<Runtime>,
            BucketId<Runtime>,
            Multiaddresses<Runtime>,
            ValuePropositionWithId<Runtime>,
        > + PaymentStreamsRuntimeApi<Block, ProviderId<Runtime>, Balance<Runtime>, AccountId>
{
}

pub trait StorageEnableRuntimeApi:
    ConstructRuntimeApi<Block, TFullClient<Block, Self, ParachainExecutor>>
    + Sized
    + Send
    + Sync
    + 'static
{
}

impl<T> StorageEnableRuntimeApi for T where
    T: ConstructRuntimeApi<Block, TFullClient<Block, Self, ParachainExecutor>>
        + Sized
        + Send
        + Sync
        + 'static
{
}

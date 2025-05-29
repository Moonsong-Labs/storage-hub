use crate::types::*;
use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_payment_streams_runtime_api::PaymentStreamsApi as PaymentStreamsRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi as StorageProvidersRuntimeApi;
use polkadot_primitives::AccountId;
use polkadot_primitives::Nonce;

use sc_service::TFullClient;
use shp_opaque::Block;
use sp_api::ConstructRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_core::H256;

pub trait StorageEnableApiCollection<
    Runtime: pallet_file_system::Config
        + pallet_storage_providers::Config
        + pallet_proofs_dealer::Config
        + pallet_storage_providers::Config,
>:
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

impl<
        T,
        Runtime: pallet_file_system::Config
            + pallet_storage_providers::Config
            + pallet_proofs_dealer::Config
            + pallet_storage_providers::Config,
    > StorageEnableApiCollection<Runtime> for T
where
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
        > + PaymentStreamsRuntimeApi<Block, ProviderId<Runtime>, Balance<Runtime>, AccountId>,
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

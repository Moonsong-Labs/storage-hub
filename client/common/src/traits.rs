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

pub trait StorageEnableApiCollection:
    pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
    + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
    + BlockBuilder<Block>
    + ProofsDealerRuntimeApi<
        Block,
        ProofsDealerProviderId,
        BlockNumber,
        ForestLeaf,
        RandomnessOutput,
        CustomChallenge,
    > + FileSystemRuntimeApi<
        Block,
        BackupStorageProviderId,
        MainStorageProviderId,
        H256,
        BlockNumber,
        ChunkId,
        BucketId,
        StorageRequestMetadata,
    > + StorageProvidersRuntimeApi<
        Block,
        BlockNumber,
        BackupStorageProviderId,
        BackupStorageProviderInfo,
        MainStorageProviderId,
        AccountId,
        ProviderId,
        StorageProviderId,
        StorageData,
        Balance,
        BucketId,
        Multiaddresses,
        ValuePropositionWithId,
    > + PaymentStreamsRuntimeApi<Block, ProviderId, Balance, AccountId>
{
}

impl<T> StorageEnableApiCollection for T where
    T: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + BlockBuilder<Block>
        + ProofsDealerRuntimeApi<
            Block,
            ProofsDealerProviderId,
            BlockNumber,
            ForestLeaf,
            RandomnessOutput,
            CustomChallenge,
        > + FileSystemRuntimeApi<
            Block,
            BackupStorageProviderId,
            MainStorageProviderId,
            H256,
            BlockNumber,
            ChunkId,
            BucketId,
            StorageRequestMetadata,
        > + StorageProvidersRuntimeApi<
            Block,
            BlockNumber,
            BackupStorageProviderId,
            BackupStorageProviderInfo,
            MainStorageProviderId,
            AccountId,
            ProviderId,
            StorageProviderId,
            StorageData,
            Balance,
            BucketId,
            Multiaddresses,
            ValuePropositionWithId,
        > + PaymentStreamsRuntimeApi<Block, ProviderId, Balance, AccountId>
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

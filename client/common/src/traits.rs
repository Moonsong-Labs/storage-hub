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

/// A trait bundle that ensures a runtime API includes all storage-related capabilities.
///
/// This trait acts as a "capability bundle" that groups together all runtime APIs required
/// for StorageHub's storage operations. It provides a single trait bound that guarantees
/// access to all necessary storage subsystem APIs, simplifying client-side service implementations.
///
/// # Purpose
///
/// Instead of requiring multiple trait bounds on every function or struct that needs to
/// interact with the storage runtime, this trait provides a single, comprehensive bound
/// that ensures all storage-related APIs are available.
///
/// # Usage Patterns
///
/// ## In Service Definitions
/// ```ignore
/// pub struct BlockchainService<RuntimeApi>
/// where
///     RuntimeApi::RuntimeApi: StorageEnableApiCollection,
/// {
///     // Service implementation
/// }
/// ```
///
/// ## In Function Signatures
/// ```ignore
/// fn spawn_blockchain_service<RuntimeApi>(client: Arc<ParachainClient<RuntimeApi>>)
/// where
///     RuntimeApi::RuntimeApi: StorageEnableApiCollection,
/// {
///     // Can now use all storage APIs: FileSystemApi, StorageProvidersApi, etc.
/// }
/// ```
///
/// ## In RPC Setup
/// ```ignore
/// pub fn create_full<C>(client: Arc<C>) -> RpcModule<()>
/// where
///     C::Api: StorageEnableApiCollection,
/// {
///     // RPC methods can access all storage runtime APIs
/// }
/// ```
///
/// # Included APIs
///
/// - [`TransactionPaymentRuntimeApi`]: For fee calculations and payment handling
/// - [`AccountNonceApi`]: For transaction nonce management
/// - [`BlockBuilder`]: For block construction operations
/// - [`ProofsDealerRuntimeApi`]: For storage proof challenges and verification
/// - [`FileSystemRuntimeApi`]: For file operations and bucket management
/// - [`StorageProvidersRuntimeApi`]: For BSP/MSP provider operations
/// - [`PaymentStreamsRuntimeApi`]: For payment stream management
///
/// # Implementation
///
/// This trait has a blanket implementation for any type that implements all the
/// required runtime APIs. This means runtime developers don't need to explicitly
/// implement this trait - it's automatically available when all component APIs
/// are implemented.
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

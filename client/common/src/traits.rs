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
use sp_core::{crypto::KeyTypeId, sr25519, H256};

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

/// A read-only keystore trait that provides access to public keys without signing capabilities.
///
/// This trait is designed for services that only need to query public keys from the keystore
/// without the ability to generate signatures. It provides a type-safe way to restrict
/// keystore access to read-only operations.
///
/// # Purpose
///
/// The indexer service needs to detect provider IDs by querying sr25519 public keys
/// from the keystore, but it never needs to sign anything. This trait enforces that
/// restriction at the type level, making the code more secure and its intentions clearer.
///
/// # Usage
///
/// ```ignore
/// fn get_provider_id_from_keystore<K: ReadOnlyKeystore>(
///     keystore: &K,
///     block_hash: &H256,
/// ) -> Result<Option<StorageProviderId>, GetProviderIdError> {
///     let keys = keystore.sr25519_public_keys(BCSV_KEY_TYPE);
///     // ... process keys
/// }
/// ```
pub trait ReadOnlyKeystore: Send + Sync {
    /// Returns all sr25519 public keys for the given key type.
    ///
    /// This is the only operation the indexer service needs from the keystore.
    fn sr25519_public_keys(&self, key_type: KeyTypeId) -> Vec<sr25519::Public>;
}

/// Blanket implementation for any type that implements the full Keystore trait.
///
/// This allows existing `KeystorePtr` instances to be used wherever `ReadOnlyKeystore`
/// is required, maintaining backward compatibility while enforcing read-only access
/// at the type level.
impl<T> ReadOnlyKeystore for T
where
    T: sp_keystore::Keystore + ?Sized,
{
    fn sr25519_public_keys(&self, key_type: KeyTypeId) -> Vec<sr25519::Public> {
        sp_keystore::Keystore::sr25519_public_keys(self, key_type)
    }
}

/// Trait for abstracting key type operations to support multiple cryptographic schemes.
///
/// This trait provides a unified interface for working with different key types (sr25519, ecdsa)
/// in the StorageHub client. It abstracts the differences between key types, allowing
/// generic code to work with any supported cryptographic scheme.
///
/// # Purpose
///
/// Different cryptographic schemes have different key sizes and signing mechanisms.
/// For example, sr25519 public keys are 32 bytes while ecdsa public keys are 33 bytes
/// (compressed format). This trait provides a consistent interface for:
/// - Retrieving public keys from the keystore
/// - Signing messages
/// - Converting between key types and runtime types
///
/// # Type Parameters
///
/// - `Public`: The public key type (e.g., `sp_core::sr25519::Public`)
/// - `Signature`: The signature type (e.g., `sp_core::sr25519::Signature`)
///
/// # Usage
///
/// ```ignore
/// fn sign_extrinsic<T: KeyTypeOperations>(keystore: KeystorePtr) -> UncheckedExtrinsic {
///     let public_key = T::public_keys(&keystore, BCSV_KEY_TYPE).pop().unwrap();
///     let signature = T::sign(&keystore, BCSV_KEY_TYPE, &public_key, &payload).unwrap();
///     // ... construct extrinsic
/// }
/// ```
pub trait KeyTypeOperations: Sized {
    /// The public key type associated with this key type
    type Public;

    /// The signature type associated with this key type
    type Signature;

    /// Get all public keys of this type from the keystore
    fn public_keys(keystore: &sp_keystore::KeystorePtr, key_type: KeyTypeId) -> Vec<Self::Public>;

    /// Sign a message with the given public key
    fn sign(
        keystore: &sp_keystore::KeystorePtr,
        key_type: KeyTypeId,
        public: &Self::Public,
        msg: &[u8],
    ) -> Option<Self::Signature>;

    /// Convert the signature to the runtime signature type
    fn to_runtime_signature(signature: Self::Signature) -> polkadot_primitives::Signature;
    
    /// Convert the public key to AccountId32
    fn public_to_account_id(public: &Self::Public) -> sp_runtime::AccountId32;
}

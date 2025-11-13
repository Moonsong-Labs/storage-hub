use std::fmt::{Debug, Display};

use bigdecimal::BigDecimal;
use codec::{Decode, Encode};
use pallet_file_system_runtime_api::FileSystemApi as FileSystemRuntimeApi;
use pallet_payment_streams_runtime_api::PaymentStreamsApi as PaymentStreamsRuntimeApi;
use pallet_proofs_dealer_runtime_api::ProofsDealerApi as ProofsDealerRuntimeApi;
use pallet_storage_providers_runtime_api::StorageProvidersApi as StorageProvidersRuntimeApi;
use polkadot_primitives::Nonce;
use sc_service::TFullClient;
use scale_info::StaticTypeInfo;
use shp_opaque::Block;
use shp_tx_implicits_runtime_api::TxImplicitsApi as TxImplicitsRuntimeApi;
use sp_api::ConstructRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_core::{crypto::KeyTypeId, H256};
use sp_rpc::number::NumberOrHex;
use sp_runtime::traits::{
    Block as BlockT, Dispatchable, IdentifyAccount, MaybeDisplay, Member, TransactionExtension,
    Verify,
};

use crate::types::*;

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
pub trait StorageEnableApiCollection<Runtime>:
    pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance<Runtime>>
    + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId<Runtime>, Nonce>
    + BlockBuilder<Block>
    + TxImplicitsRuntimeApi<Block>
    + ProofsDealerRuntimeApi<
        Block,
        ProofsDealerProviderId<Runtime>,
        BlockNumber<Runtime>,
        ForestLeaf<Runtime>,
        RandomnessOutput<Runtime>,
        CustomChallenge<Runtime>,
    > + FileSystemRuntimeApi<
        Block,
        AccountId<Runtime>,
        BackupStorageProviderId<Runtime>,
        MainStorageProviderId<Runtime>,
        Runtime::Hash,
        BlockNumber<Runtime>,
        ChunkId,
        BucketId<Runtime>,
        StorageRequestMetadata<Runtime>,
        BucketId<Runtime>,
        StorageDataUnit<Runtime>,
        Runtime::Hash,
    > + StorageProvidersRuntimeApi<
        Block,
        BlockNumber<Runtime>,
        BackupStorageProviderId<Runtime>,
        BackupStorageProviderInfo<Runtime>,
        MainStorageProviderId<Runtime>,
        AccountId<Runtime>,
        ProviderId<Runtime>,
        StorageProviderId<Runtime>,
        StorageDataUnit<Runtime>,
        Balance<Runtime>,
        BucketId<Runtime>,
        Multiaddresses<Runtime>,
        ValuePropositionWithId<Runtime>,
    > + PaymentStreamsRuntimeApi<Block, ProviderId<Runtime>, Balance<Runtime>, AccountId<Runtime>>
where
    Runtime: frame_system::Config
        + pallet_storage_providers::Config
        + pallet_proofs_dealer::Config
        + pallet_file_system::Config
        + pallet_balances::Config<Balance: MaybeDisplay>,
{
}

impl<T, Runtime> StorageEnableApiCollection<Runtime> for T
where
    T: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance<Runtime>>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId<Runtime>, Nonce>
        + BlockBuilder<Block>
        + TxImplicitsRuntimeApi<Block>
        + ProofsDealerRuntimeApi<
            Block,
            ProofsDealerProviderId<Runtime>,
            BlockNumber<Runtime>,
            ForestLeaf<Runtime>,
            RandomnessOutput<Runtime>,
            CustomChallenge<Runtime>,
        > + FileSystemRuntimeApi<
            Block,
            AccountId<Runtime>,
            BackupStorageProviderId<Runtime>,
            MainStorageProviderId<Runtime>,
            Runtime::Hash,
            BlockNumber<Runtime>,
            ChunkId,
            BucketId<Runtime>,
            StorageRequestMetadata<Runtime>,
            BucketId<Runtime>,
            StorageDataUnit<Runtime>,
            Runtime::Hash,
        > + StorageProvidersRuntimeApi<
            Block,
            BlockNumber<Runtime>,
            BackupStorageProviderId<Runtime>,
            BackupStorageProviderInfo<Runtime>,
            MainStorageProviderId<Runtime>,
            AccountId<Runtime>,
            ProviderId<Runtime>,
            StorageProviderId<Runtime>,
            StorageDataUnit<Runtime>,
            Balance<Runtime>,
            BucketId<Runtime>,
            Multiaddresses<Runtime>,
            ValuePropositionWithId<Runtime>,
        > + PaymentStreamsRuntimeApi<Block, ProviderId<Runtime>, Balance<Runtime>, AccountId<Runtime>>,
    Runtime: frame_system::Config
        + pallet_storage_providers::Config
        + pallet_proofs_dealer::Config
        + pallet_file_system::Config
        + pallet_balances::Config<Balance: MaybeDisplay>,
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

/// The trait that a runtime must implement to be compatible with the StorageHub Client.
///
/// This trait describes the concrete associated types and pallet constraints that a
/// runtime must satisfy for the StorageHub Client to work. It allows the client to
/// remain generic over runtimes while still relying on a consistent set of capabilities.
///
/// - Fixes the fundamental runtime types used by the client: `Address`, `Call`,
///   `Signature`, `Extension`, and `RuntimeApi`.
/// - Requires the presence of specific pallets via trait bounds.
/// - Requires `RuntimeEvent: Into<StorageEnableEvents<Self>>` so the client can map the
///   concrete runtime event type into a known set of events.
///
/// # Associated Types
///
/// - `Address` - The account address format used by the runtime
/// - `Call` - The dispatchable call type for submitting extrinsics
/// - `Signature` - The signature type used for signing transactions
/// - `Extension` - The transaction extension type for additional transaction logic
/// - `RuntimeApi` - The set of runtime APIs that must be available to the client
pub trait StorageEnableRuntime:
    // TODO: Consider removing the restriction that `Hash = H256`.
    frame_system::Config<
        Hash = shp_types::Hash,
        AccountId: for<'a> TryFrom<&'a [u8]> + AsRef<[u8]>,
        RuntimeEvent: Into<StorageEnableEvents<Self>>,
        Block: BlockT<Hash = shp_types::Hash>,
    >
        + pallet_storage_providers::Config<
            MerklePatriciaRoot = shp_types::Hash,
            ValuePropId = shp_types::Hash,
            ProviderId = shp_types::Hash,
            BucketNameLimit: Send + Sync,
            MaxCommitmentSize: Send + Sync,
            MaxMultiAddressSize: Send + Sync,
            MaxMultiAddressAmount: Send + Sync,
            StorageDataUnit: Into<BigDecimal>,
        >
        + pallet_proofs_dealer::Config<
            ProvidersPallet = pallet_storage_providers::Pallet<Self>,
            MerkleTrieHash = shp_types::Hash,
            ForestVerifier = ForestVerifier,
            KeyVerifier = FileKeyVerifier,
            MaxCustomChallengesPerBlock: Send + Sync,
        >
        + pallet_payment_streams::Config<
            ProvidersPallet = pallet_storage_providers::Pallet<Self>,
            NativeBalance = pallet_balances::Pallet<Self>,
            MaxUsersToCharge: Send + Sync,
        >
        + pallet_file_system::Config<
            Providers = pallet_storage_providers::Pallet<Self>,
            ProofDealer = pallet_proofs_dealer::Pallet<Self>,
            PaymentStreams = pallet_payment_streams::Pallet<Self>,
            Nfts = pallet_nfts::Pallet<Self>,
            Fingerprint = shp_types::Hash,
            OffchainSignature: Send + Sync,
            MaxBatchConfirmStorageRequests: Send + Sync,
            MaxFilePathSize: Send + Sync,
            MaxNumberOfPeerIds: Send + Sync,
            MaxPeerIdSize: Send + Sync,
            MaxReplicationTarget: Send + Sync,
            MaxFileDeletionsPerExtrinsic: Send + Sync,
        >
        + pallet_transaction_payment::Config
        + pallet_balances::Config<Balance: Into<BigDecimal> + Into<NumberOrHex> + MaybeDisplay>
        + pallet_nfts::Config<CollectionId: Send + Sync + Display>
        + pallet_bucket_nfts::Config
        + pallet_randomness::Config
        + Copy
        + Debug
        + Send
        + Sync
        + 'static
{
    /// The address format used to identify accounts in the runtime.
    /// Must support type information, encoding/decoding, and debug formatting.
    type Address: StaticTypeInfo + Decode + Encode + core::fmt::Debug + Send;

    /// The dispatchable call type representing extrinsics that can be submitted to the runtime.
    /// Must be a member type that supports encoding/decoding and dispatching.
    type Call: StaticTypeInfo
        + Decode
        + Encode
        + Member
        + Dispatchable
        + From<frame_system::Call<Self>>
        + From<pallet_storage_providers::Call<Self>>
        + From<pallet_proofs_dealer::Call<Self>>
        + From<pallet_payment_streams::Call<Self>>
        + From<pallet_file_system::Call<Self>>;

    /// The signature type used for signing transactions.
    /// Must support verification and key operations that produce the associated `Address` type.
    type Signature: StaticTypeInfo
        + Decode
        + Encode
        + Member
        + Verify<Signer: IdentifyAccount<AccountId = <Self as frame_system::Config>::AccountId>>
        + KeyTypeOperations<
            Address = Self::Address,
            Public: Into<<Self as frame_system::Config>::AccountId>,
        >;

    /// The transaction extension type for additional validation and transaction logic.
    /// Extensions can modify transaction behaviour and must support the runtime's call type.
    type Extension: StaticTypeInfo
        + Decode
        + Encode
        + TransactionExtension<Self::Call>
        // TODO: Consider removing the `Hash` constraint.
        + ExtensionOperations<Self::Call, Self, Hash = H256>
        + Clone
        + core::fmt::Debug;

    /// The runtime API type that provides access to all StorageHub-specific runtime functions.
    /// Must support construction and provide complete access to all required runtime APIs
    /// including file system, storage providers, proofs dealer, and payment streams functionality.
    type RuntimeApi: StorageEnableRuntimeApi<RuntimeApi: StorageEnableApiCollection<Self>>;
}

/// Trait for abstracting key type operations to support multiple cryptographic schemes.
///
/// This trait provides a unified interface for working with different signature types (sr25519, ecdsa)
/// in the StorageHub client. It abstracts the differences between signature schemes, allowing
/// generic code to work with any supported cryptographic scheme.
///
/// # Purpose
///
/// Different cryptographic schemes have different key sizes and signing mechanisms.
/// For example, sr25519 public keys are 32 bytes while ecdsa public keys are 33 bytes
/// (compressed format). This trait provides a consistent interface for:
/// - Retrieving public keys from the keystore
/// - Signing messages
/// - Converting signatures to runtime types
/// - Converting public keys to address types
///
/// # Type Parameters
///
/// - `Public`: The public key type (e.g., `sp_core::sr25519::Public`, `sp_core::ecdsa::Public`)
/// - `Address`: The address type (e.g., `MultiAddress<AccountId32, ()>`, `AccountId20`)
///
/// # Implementations
///
/// StorageHub provides two implementations:
/// - `MultiSignature`: Uses sr25519 keys with `MultiAddress<AccountId32, ()>`
/// - `EthereumSignature`: Uses ECDSA keys with `AccountId20` (Ethereum-compatible)
///
/// # Usage
///
/// ## Getting Public Keys
/// ```ignore
/// pub fn caller_pub_key<S: KeyTypeOperations>(keystore: KeystorePtr) -> S::Public {
///     S::public_keys(&keystore, BCSV_KEY_TYPE).pop()
///         .expect("At least one key should exist in the keystore")
/// }
/// ```
pub trait KeyTypeOperations: Sized {
    /// The public key type associated with this signature type.
    ///
    /// For example:
    /// - `sr25519::Public` for sr25519 signatures
    /// - `ecdsa::Public` for ECDSA signatures
    type Public: Debug + Send;

    /// The address type used to identify accounts on-chain.
    ///
    /// Common implementations:
    /// - `MultiAddress<AccountId32, ()>`: Standard Substrate/Polkadot address format
    /// - `AccountId20`: Ethereum-compatible 20-byte address format
    type Address;

    /// Retrieves all public keys of this signature type from the keystore.
    ///
    /// # Parameters
    /// - `keystore`: Reference to the keystore containing the keys
    /// - `key_type`: The key type identifier to filter keys (e.g., `BCSV_KEY_TYPE`)
    ///
    /// # Returns
    /// A vector of public keys found in the keystore for the specified key type.
    /// May be empty if no keys are found.
    fn public_keys(keystore: &sp_keystore::KeystorePtr, key_type: KeyTypeId) -> Vec<Self::Public>;

    /// Signs a message using the specified public key from the keystore.
    ///
    /// # Parameters
    /// - `keystore`: Reference to the keystore containing the private key
    /// - `key_type`: The key type identifier for the signing key
    /// - `public`: The public key whose corresponding private key will be used
    /// - `msg`: The message bytes to sign
    ///
    /// # Returns
    /// - `Some(Self)`: The signature if signing was successful
    /// - `None`: If the private key is not found or signing fails
    fn sign(
        keystore: &sp_keystore::KeystorePtr,
        key_type: KeyTypeId,
        public: &Self::Public,
        msg: &[u8],
    ) -> Option<Self>;

    /// Converts this signature to the Polkadot runtime signature type.
    ///
    /// This enables compatibility with the Polkadot runtime which expects
    /// signatures in the `polkadot_primitives::Signature` enum format.
    ///
    /// # Note
    /// The implementation for `EthereumSignature` uses encoding/decoding as a workaround
    /// since `EthereumSignature` doesn't expose its inner `ecdsa::Signature` directly.
    fn to_runtime_signature(self) -> polkadot_primitives::Signature;

    /// Converts a public key to its corresponding on-chain address.
    ///
    /// # Parameters
    /// - `public`: The public key to convert
    ///
    /// # Returns
    /// The address representation of the public key, which varies by implementation:
    /// - For sr25519: Direct conversion to `AccountId32` wrapped in `MultiAddress`
    /// - For ECDSA: Ethereum-style address (last 20 bytes of keccak256 hash)
    fn public_to_address(public: &Self::Public) -> Self::Address;
}

/// Trait for abstracting transaction extension operations across different runtime configurations.
///
/// This trait provides a unified interface for working with transaction extensions (formerly known
/// as "signed extensions") in Substrate-based blockchains. It abstracts the creation and management
/// of extensions that provide additional transaction metadata and validation logic.
///
/// # Purpose
///
/// Transaction extensions are used to attach additional data to transactions and perform
/// validation checks. Examples include:
/// - Checking account nonces to prevent replay attacks
/// - Validating transaction mortality (timeouts)
/// - Handling transaction fees and tips
/// - Adding runtime-specific metadata
///
/// This trait allows generic code to work with different extension configurations by providing:
/// - A way to construct extensions from minimal data
/// - Methods to generate implicit data required by the extension
///
/// # Type Parameters
///
/// - `Call`: The runtime call type that this extension validates
/// - `Hash`: The block hash type used for mortality and genesis checks
///
/// # Required Traits
///
/// Implementers must also implement `TransactionExtension<Call>` which provides the core
/// extension functionality including validation and metadata generation.
///
/// # Usage
///
/// ## Generic Extrinsic Construction
/// ```ignore
/// pub fn construct_extrinsic<E>(&self, function: Call) -> UncheckedExtrinsic
/// where
///     E: ExtensionOperations<Call, Hash = H256>,
/// {
///     let extension = E::from_minimal_extension(MinimalExtension {
///         era: self.era,
///         nonce: self.nonce,
///         tip: self.tip,
///     });
///
///     let implicit = E::implicit(self.genesis_hash, self.block_hash);
///
///     generic::UncheckedExtrinsic::new_signed(
///         function,
///         address,
///         signature,
///         extension,
///     )
/// }
/// ```
pub trait ExtensionOperations<
    Call: Encode + Dispatchable,
    Runtime: pallet_transaction_payment::Config,
>: TransactionExtension<Call>
{
    /// The block hash type used by this extension.
    ///
    /// This is typically `H256` for most Substrate chains, but could vary
    /// for chains with different hashing algorithms.
    type Hash;

    /// Creates a transaction extension from minimal required data.
    ///
    /// This method constructs a full extension from just the essential fields
    /// that vary between transactions (era, nonce, tip). Other fields are
    /// typically set to default or zero values.
    ///
    /// # Parameters
    /// - `minimal`: The minimal extension data containing era, nonce, and tip
    ///
    /// # Returns
    /// A fully constructed extension ready for use in transaction signing
    fn from_minimal_extension(minimal: MinimalExtension) -> Self;
}

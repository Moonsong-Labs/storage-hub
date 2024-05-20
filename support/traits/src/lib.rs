#![cfg_attr(not(feature = "std"), no_std)]

use codec::{FullCodec, HasCompact};
use frame_support::dispatch::DispatchResult;
use frame_support::pallet_prelude::{
    Decode, Encode, MaxEncodedLen, MaybeSerializeDeserialize, Member,
};
use frame_support::sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps};
use frame_support::traits::fungible;
use frame_support::Parameter;
use scale_info::prelude::{fmt::Debug, vec::Vec};
use sp_core::Get;
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_runtime::{BoundedVec, DispatchError};

#[cfg(feature = "std")]
pub trait MaybeDebug: Debug {}
#[cfg(feature = "std")]
impl<T: Debug> MaybeDebug for T {}
#[cfg(not(feature = "std"))]
pub trait MaybeDebug {}
#[cfg(not(feature = "std"))]
impl<T> MaybeDebug for T {}

/// A trait to lookup registered Providers.
///
/// It is abstracted over the `AccountId` type, `Provider` type.
pub trait ProvidersInterface {
    /// The type corresponding to the staking balance of a registered Provider.
    type Balance: fungible::Inspect<Self::AccountId> + fungible::hold::Inspect<Self::AccountId>;
    /// The type which can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a registered Provider's ID.
    type ProviderId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + Copy;
    /// The type corresponding to the root of a registered Provider.
    type MerkleHash: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleBitOps
        + Ord
        + Default
        + Copy
        + CheckEqual
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;
    /// Check if an account is a registered Provider.
    fn is_provider(who: Self::ProviderId) -> bool;

    /// Get the ProviderId from AccountId, if it is a registered Provider.
    fn get_provider_id(who: Self::AccountId) -> Option<Self::ProviderId>;

    /// Get the payment account of a registered Provider.
    fn get_provider_payment_account(who: Self::ProviderId) -> Option<Self::AccountId>;

    /// Get the root for a registered Provider.
    fn get_root(who: Self::ProviderId) -> Option<Self::MerkleHash>;

    /// Get the stake for a registered  Provider.
    fn get_stake(
        who: Self::ProviderId,
    ) -> Option<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;
}

/// A trait to lookup registered Providers, their Merkle Patricia Trie roots and their stake.
pub trait ReadProvidersInterface: ProvidersInterface {
    /// Type that represents the total number of registered Storage Providers.
    type SpCount: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Ord
        + AtLeast32BitUnsigned
        + FullCodec
        + Copy
        + Default
        + Debug
        + scale_info::TypeInfo
        + MaxEncodedLen;

    /// Type that represents the multiaddress of a Storage Provider.
    type MultiAddress: Parameter
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + Default
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;

    /// Maximum number of multiaddresses a provider can have.
    type MaxNumberOfMultiAddresses: Get<u32>;

    /// Check if provider is a BSP.
    fn is_bsp(who: &Self::ProviderId) -> bool;

    /// Check if provider is a MSP.
    fn is_msp(who: &Self::ProviderId) -> bool;

    /// Get number of registered BSPs.
    fn get_number_of_bsps() -> Self::SpCount;

    /// Get multiaddresses of a BSP.
    fn get_bsp_multiaddresses(
        who: &Self::ProviderId,
    ) -> Result<BoundedVec<Self::MultiAddress, Self::MaxNumberOfMultiAddresses>, DispatchError>;
}

/// Interface to allow the File System pallet to modify the data used by the Storage Providers pallet.
pub trait MutateProvidersInterface {
    /// The type which can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a registered Provider.
    type ProviderId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// Data type for the measurement of storage size
    type StorageData: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Default
        + MaybeDisplay
        + AtLeast32BitUnsigned
        + Copy
        + MaxEncodedLen
        + HasCompact;
    /// The type of ID that uniquely identifies a Merkle Trie Holder (BSPs/Buckets) from an AccountId
    type BucketId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleBitOps
        + Ord
        + Default
        + Copy
        + CheckEqual
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;
    /// The type of the Merkle Patricia Root of the storage trie for BSPs and MSPs' buckets (a hash).
    type MerklePatriciaRoot: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleBitOps
        + Ord
        + Default
        + Copy
        + CheckEqual
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;

    /// Increase the used data of a Storage Provider (generic, MSP or BSP).
    fn increase_data_used(who: &Self::AccountId, delta: Self::StorageData) -> DispatchResult;

    /// Decrease the used data of a Storage Provider (generic, MSP or BSP).
    fn decrease_data_used(who: &Self::AccountId, delta: Self::StorageData) -> DispatchResult;

    /// Add a new Bucket as a Provider
    fn add_bucket(
        msp_id: Self::ProviderId,
        user_id: Self::AccountId,
        bucket_id: Self::BucketId,
        bucket_root: Self::MerklePatriciaRoot,
    ) -> DispatchResult;

    /// Change the root of a bucket
    fn change_root_bucket(
        bucket_id: Self::BucketId,
        new_root: Self::MerklePatriciaRoot,
    ) -> DispatchResult;

    /// Change the root of a BSP
    fn change_root_bsp(
        bsp_id: Self::ProviderId,
        new_root: Self::MerklePatriciaRoot,
    ) -> DispatchResult;

    /// Remove a root from a bucket of a MSP, removing the whole bucket from storage
    fn remove_root_bucket(bucket_id: Self::BucketId) -> DispatchResult;
}

/// The interface to subscribe to updates on the Storage Providers pallet.
pub trait SubscribeProvidersInterface {
    /// The type which represents a registered Provider.
    type ProviderId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Subscribe to the sign off of a BSP.
    fn subscribe_bsp_sign_off(who: &Self::ProviderId) -> DispatchResult;

    /// Subscribe to the sign up of a BSP.
    fn subscribe_bsp_sign_up(who: &Self::ProviderId) -> DispatchResult;
}

/// The interface for the ProofsDealer pallet.
///
/// It is abstracted over the `Provider` type, `Proof` type and `MerkleHash` type.
/// It provides the functions to verify a proof, submit a new proof challenge and
/// submit a new challenge with priority.
pub trait ProofsDealerInterface {
    /// The type which represents a registered Provider.
    type ProviderId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type that represents the proof.
    type Proof: Parameter + Member + Debug;
    /// The type corresponding to the root of a registered Provider.
    type MerkleHash: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleBitOps
        + Ord
        + Default
        + Copy
        + CheckEqual
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaxEncodedLen
        + FullCodec;

    /// Verify a proof for a given Provider, who should have a given Root.
    fn verify_proof(
        who: &Self::ProviderId,
        root: &Self::MerkleHash,
        proof: &Self::Proof,
    ) -> DispatchResult;

    /// Submit a new proof challenge.
    fn challenge(key_challenged: &Self::MerkleHash) -> DispatchResult;

    /// Submit a new challenge with priority.
    fn challenge_with_priority(key_challenged: &Self::MerkleHash) -> DispatchResult;
}

/// A trait to verify proofs based on commitments and challenges.
///
/// It is abstracted over the `Proof` and `Key` type.
pub trait CommitmentVerifier {
    /// The type that represents the proof.
    type Proof: Parameter + Member + Debug;
    /// The type that represents the commitment (e.g. a Merkle root) and the keys representing nodes
    /// in a Merkle tree which are also passed as challenges.
    type Key: MaybeDebug + Ord + Default + Copy + AsRef<[u8]> + AsMut<[u8]>;

    /// Verify a proof based on a commitment and a set of challenges.
    ///
    /// The function returns a vector of keys that are verified by the proof, or an error if the proof
    /// is invalid.
    fn verify_proof(
        commitment: &Self::Key,
        challenges: &[Self::Key],
        proof: &Self::Proof,
    ) -> Result<Vec<Self::Key>, DispatchError>;
}

/// The interface of the Payment Streams pallet.
///
/// It is to be used by other pallets to interact with the Payment Streams pallet to create, update and delete payment streams.
pub trait PaymentStreamsInterface {
    /// The type which represents the balance of the runtime.
    type Balance: fungible::Inspect<Self::AccountId>
        + fungible::Mutate<Self::AccountId>
        + fungible::hold::Inspect<Self::AccountId>
        + fungible::hold::Mutate<Self::AccountId>;
    /// The type which represents an account identifier.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a provider identifier.
    type ProviderId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + Copy;
    /// The type which represents a block number.
    type BlockNumber: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a payment stream.
    type PaymentStream: Encode
        + Decode
        + Parameter
        + Member
        + Debug
        + MaxEncodedLen
        + PartialEq
        + Clone;

    /// Create a new payment stream from a user to a Provider.
    fn create_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        rate: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult;

    /// Update the rate of an existing payment stream.
    fn update_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        rate: <Self::Balance as fungible::Inspect<Self::AccountId>>::Balance,
    ) -> DispatchResult;

    /// Delete a payment stream.
    fn delete_payment_stream(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> DispatchResult;

    /// Get the payment stream information for a user and a Backup Storage Provider.
    fn get_payment_stream_info(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
    ) -> Option<Self::PaymentStream>;
}

/// The interface of a Payment Manager, which has to be made aware of the last block for which a charge of a payment can be made by a provider.
/// Example: the Proofs Dealer pallet uses this interface to update the block when a Storage Provider last submitted a valid proof for the Payment Streams pallet.
pub trait PaymentManager {
    /// The type which represents an account identifier.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a provider identifier.
    type ProviderId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + Ord
        + MaxEncodedLen
        + Copy;
    /// The type which represents a block number.
    type BlockNumber: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;

    /// Update the last valid block for which a charge of a payment can be made
    fn update_last_chargeable_block(
        provider_id: &Self::ProviderId,
        user_account: &Self::AccountId,
        last_chargeable_block: Self::BlockNumber,
    ) -> DispatchResult;
}

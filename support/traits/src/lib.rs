#![cfg_attr(not(feature = "std"), no_std)]

use codec::{FullCodec, HasCompact};
use frame_support::dispatch::DispatchResult;
use frame_support::pallet_prelude::{MaxEncodedLen, MaybeSerializeDeserialize, Member};
use frame_support::sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps};
use frame_support::traits::fungible;
use frame_support::Parameter;
use scale_info::prelude::fmt::Debug;
use sp_runtime::traits::AtLeast32BitUnsigned;

/// A trait to lookup registered Providers, their Merkle Patricia Trie roots and their stake.
///
/// It is abstracted over the `AccountId` type, `Provider` type, `Balance` type and `MerkleHash` type.
pub trait ReadProvidersInterface {
    /// The type which can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a registered Provider.
    type Provider: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type corresponding to the staking balance of a registered Provider.
    type Balance: fungible::Inspect<Self::AccountId> + fungible::hold::Inspect<Self::AccountId>;
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
    fn is_provider(who: Self::Provider) -> bool;

    // Get Provider from AccountId, if it is a registered Provider.
    fn get_provider(who: Self::AccountId) -> Option<Self::Provider>;

    /// Get the root for a registered Provider.
    fn get_root(who: Self::Provider) -> Option<Self::MerkleHash>;

    /// Get the stake for a registered  Provider.
    fn get_stake(
        who: Self::Provider,
    ) -> Option<<Self::Balance as fungible::Inspect<Self::AccountId>>::Balance>;
}

/// Interface to allow the File System pallet to modify the data used by the Storage Providers pallet.
pub trait MutateProvidersInterface {
    /// The type which can be used to identify accounts.
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a registered Provider.
    type Provider: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
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

    /// Change the used data of a Storage Provider (generic, MSP or BSP).
    fn change_data_used(who: &Self::AccountId, data_change: Self::StorageData) -> DispatchResult;

    /// Add a new Bucket as a Provider
    fn add_bucket(
        msp_id: Self::Provider,
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
        bsp_id: Self::Provider,
        new_root: Self::MerklePatriciaRoot,
    ) -> DispatchResult;

    /// Remove a root from a bucket of a MSP, removing the whole bucket from storage
    fn remove_root_bucket(bucket_id: Self::BucketId) -> DispatchResult;

    /// Remove a root from a BSP. It will remove the whole BSP from storage, so it should only be called when the BSP is being removed.
    /// todo!("If the only way to remove a BSP is by this pallet (bsp_sign_off), then is this function actually needed?")
    fn remove_root_bsp(who: &Self::AccountId) -> DispatchResult;
}

/// The interface for the ProofsDealer pallet.
///
/// It is abstracted over the `Provider` type, `Proof` type and `MerkleHash` type.
/// It provides the functions to verify a proof, submit a new proof challenge and
/// submit a new challenge with priority.
pub trait ProofsDealerInterface {
    /// The type which represents a registered Provider.
    type Provider: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
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
        who: &Self::Provider,
        root: &Self::MerkleHash,
        proof: &Self::Proof,
    ) -> DispatchResult;

    /// Submit a new proof challenge.
    fn challenge(key_challenged: &Self::MerkleHash) -> DispatchResult;

    /// Submit a new challenge with priority.
    fn challenge_with_priority(key_challenged: &Self::MerkleHash) -> DispatchResult;
}

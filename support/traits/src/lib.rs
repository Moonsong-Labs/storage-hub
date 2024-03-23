#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use frame_support::dispatch::DispatchResult;
use frame_support::pallet_prelude::{MaxEncodedLen, MaybeSerializeDeserialize, Member};
use frame_support::sp_runtime::traits::{CheckEqual, MaybeDisplay, SimpleBitOps};
use frame_support::traits::fungible;
use frame_support::Parameter;
use scale_info::prelude::fmt::Debug;

/// A trait to lookup registered Providers, their Merkle Patricia Trie roots and their stake.
///
/// It is abstracted over the `AccountId` type, `Provider` type, `Balance` type and `MerkleHash` type.
pub trait ProvidersInterface {
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

/// A trait to verify proofs based on commitments and challenges.
///
/// It is abstracted over the `Proof` type, `Commitment` type and `Challenge` type.
pub trait CommitmentVerifier {
    /// The type that represents the proof.
    type Proof: Parameter + Member + Debug;
    /// The type corresponding to the commitment, generally some hash.
    /// For example, in vector commitments like Merkle proofs, this would be the root hash.
    type Commitment: Parameter
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
    /// The type corresponding to a challenge, generally some hash.
    /// For example, in vector commitments like Merkle proofs, this would be the
    /// leaf hash being challenged.
    type Challenge: Parameter
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

    /// Verify a proof based on a commitment and a set of challenges.
    fn verify_proof(
        commitment: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &Self::Proof,
    ) -> bool;
}

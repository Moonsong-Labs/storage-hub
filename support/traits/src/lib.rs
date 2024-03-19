#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
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

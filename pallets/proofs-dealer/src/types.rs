use codec::{Decode, Encode};
use frame_support::traits::fungible;
use scale_info::TypeInfo;
use storage_hub_traits::ProvidersInterface;

#[derive(Debug, Clone, PartialEq, Decode, Encode, TypeInfo)]
pub enum ProofRejectionReason {
    /// The proof was rejected because the root does not match the
    /// Merkle Patricia Forest root.
    RootMismatch,
    /// The proof was rejected because the previous and next existing
    /// leaves to a challenge were not consecutive, i.e. there is at
    /// least one more existing leaf in between.
    NotConsecutiveLeaves,
}

// ****************************************************************************
// ********************* Syntactic sugar for types ****************************
// ****************************************************************************

/// Syntactic sugar for the AccountId type used in the proofs pallet.
pub type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

/// The type for keys that identify a file within a Merkle Patricia Forest.
/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type FileKeyFor<T> = <T as crate::Config>::MerkleHash;

/// The type for a root of a Merkle Patricia Forest.
/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type ForestRootFor<T> = <T as crate::Config>::MerkleHash;

/// Syntactic sugar for the MaxChallengesPerBlock type used in the proofs pallet.
pub type MaxChallengesPerBlockFor<T> = <T as crate::Config>::MaxChallengesPerBlock;

/// Syntactic sugar for the MaxSpsChallengedPerBlock type used in the proofs pallet.
pub type MaxSpsChallengedPerBlockFor<T> = <T as crate::Config>::MaxProvidersChallengedPerBlock;

/// Syntactic sugar for the ChallengesQueueLength type used in the proofs pallet.
pub type ChallengesQueueLengthFor<T> = <T as crate::Config>::ChallengesQueueLength;

/// Syntactic sugar for the Providers type used in the proofs pallet.
pub type ProvidersPalletFor<T> = <T as crate::Config>::ProvidersPallet;

/// Syntactic sugar for the Provider type used in the proofs pallet.
pub type ProviderFor<T> = <<T as crate::Config>::ProvidersPallet as ProvidersInterface>::Provider;

/// Syntactic sugar for the type of Balance used in the NativeBalances pallet.
pub type BalanceFor<T> = <<T as crate::Config>::NativeBalance as fungible::Inspect<
    <T as frame_system::Config>::AccountId,
>>::Balance;

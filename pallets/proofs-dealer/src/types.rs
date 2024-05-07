use codec::{Decode, Encode};
use frame_support::traits::fungible;
use scale_info::TypeInfo;
use sp_std::{vec::Vec, fmt::{Debug, Formatter, Result}};
use storage_hub_traits::{CommitmentVerifier, ProvidersInterface};

#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct Proof<T: crate::Config> {
    pub forest_proof: ForestVerifierProofFor<T>,
    pub key_proofs: Vec<KeyVerifierProofFor<T>>,
}

impl<T: crate::Config> Debug for Proof<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Proof {{ forest_proof: {:?}, key_proofs: {:?} }}", self.forest_proof, self.key_proofs)
    }
}

// ****************************************************************************
// ********************* Syntactic sugar for types ****************************
// ****************************************************************************

/// Syntactic sugar for the AccountId type used in the proofs pallet.
pub type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type MerkleHashFor<T> = <T as crate::Config>::MerkleHash;

/// The type for keys that identify a file within a Merkle Patricia Forest.
/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type KeyFor<T> = <T as crate::Config>::MerkleHash;

/// The type for a root of a Merkle Patricia Forest.
/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type ForestRootFor<T> = <T as crate::Config>::MerkleHash;

/// Syntactic sugar for the RandomChallengesPerBlock type used in the proofs pallet.
pub type RandomChallengesPerBlockFor<T> = <T as crate::Config>::RandomChallengesPerBlock;

/// Syntactic sugar for the MaxCustomChallengesPerBlock type used in the proofs pallet.
pub type MaxCustomChallengesPerBlockFor<T> = <T as crate::Config>::MaxCustomChallengesPerBlock;

/// Syntactic sugar for the MaxProvidersChallengedPerBlock type used in the proofs pallet.
pub type MaxProvidersChallengedPerBlockFor<T> =
    <T as crate::Config>::MaxProvidersChallengedPerBlock;

/// Syntactic sugar for the ChallengesQueueLength type used in the proofs pallet.
pub type ChallengesQueueLengthFor<T> = <T as crate::Config>::ChallengesQueueLength;

/// Syntactic sugar for the ChallengesFee type used in the proofs pallet.
pub type ChallengesFeeFor<T> = <T as crate::Config>::ChallengesFee;

/// Syntactic sugar for the StakeToChallengePeriod type used in the proofs pallet.
pub type StakeToChallengePeriodFor<T> = <T as crate::Config>::StakeToChallengePeriod;

/// Syntactic sugar for the ChallengeHistoryLength type used in the proofs pallet.
pub type ChallengeHistoryLengthFor<T> = <T as crate::Config>::ChallengeHistoryLength;

/// Syntactic sugar for the Treasury type used in the proofs pallet.
pub type TreasuryAccountFor<T> = <T as crate::Config>::Treasury;

/// Syntactic sugar for the Providers type used in the proofs pallet.
pub type ProvidersPalletFor<T> = <T as crate::Config>::ProvidersPallet;

/// Syntactic sugar for the Provider type used in the proofs pallet.
pub type ProviderFor<T> = <<T as crate::Config>::ProvidersPallet as ProvidersInterface>::Provider;

/// Syntactic sugar for the ForestVerifier type used in the proofs pallet.
pub type ForestVerifierFor<T> = <T as crate::Config>::ForestVerifier;

/// Syntactic sugar for the ForestVerifier::Proof type used in the proofs pallet.
pub type ForestVerifierProofFor<T> =
    <<T as crate::Config>::ForestVerifier as CommitmentVerifier>::Proof;

/// Syntactic sugar for the KeyVerifier type used in the proofs pallet.
pub type KeyVerifierFor<T> = <T as crate::Config>::KeyVerifier;

/// Syntactic sugar for the KeyVerifier::Proof type used in the proofs pallet.
pub type KeyVerifierProofFor<T> = <<T as crate::Config>::KeyVerifier as CommitmentVerifier>::Proof;

/// Syntactic sugar for the type of NativeBalance pallet.
pub type BalancePalletFor<T> = <T as crate::Config>::NativeBalance;

/// Syntactic sugar for the type of Balance used in the NativeBalances pallet.
pub type BalanceFor<T> = <<T as crate::Config>::NativeBalance as fungible::Inspect<
    <T as frame_system::Config>::AccountId,
>>::Balance;

/// Syntactic sugar for the type of RandomnessProvider type used in the proofs pallet.
pub type RandomnessProviderFor<T> = <T as crate::Config>::RandomnessProvider;

/// Syntactic sugar for the Randomness Output type used in the proofs pallet.
pub type RandomnessOutputFor<T> = <T as frame_system::Config>::Hash;

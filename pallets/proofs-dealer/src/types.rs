use codec::{Decode, Encode};
use frame_support::traits::fungible;
use scale_info::TypeInfo;
use shp_traits::{CommitmentVerifier, ReadChallengeableProvidersInterface};
use sp_std::{
    collections::btree_map::BTreeMap,
    fmt::{Debug, Formatter, Result},
};

/// Type that encapsulates the proof a Provider submits.
///
/// The proof consists of a forest proof and a set of key proofs.
/// A good proof would have a forest proof that proves that some keys belong to a
/// Merkle Patricia Forest of a Provider, and the corresponding key proofs for those keys.
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct Proof<T: crate::Config> {
    /// The proof that the Provider submits to prove that the keys belong to their Merkle
    /// Patricia Forest.
    pub forest_proof: ForestVerifierProofFor<T>,
    /// A mapping from the keys to the key proofs that are included in the `forest_proof`.
    pub key_proofs: BTreeMap<KeyFor<T>, KeyProof<T>>,
}

/// Implement Debug for Proof. Cannot derive Debug directly because of compiler issues
/// with the generic type.
impl<T: crate::Config> Debug for Proof<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Proof {{ forest_proof: {:?}, key_proofs: {:?} }}",
            self.forest_proof, self.key_proofs
        )
    }
}

/// Type that encapsulates the proof a Provider submits for a single key within a Merkle Patricia
/// Forest.
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct KeyProof<T: crate::Config> {
    /// The actual key proof.
    pub proof: KeyVerifierProofFor<T>,
    /// Determines how many challenges this key proof responds to.
    pub challenge_count: u32,
}

/// Implement Debug for KeyProof. Cannot derive Debug directly because of compiler issues
/// with the generic type.
impl<T: crate::Config> Debug for KeyProof<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "KeyProof {{ proof: {:?}, challenge_count: {:?} }}",
            self.proof, self.challenge_count
        )
    }
}

// ****************************************************************************
// ********************* Syntactic sugar for types ****************************
// ****************************************************************************

/// Syntactic sugar for the AccountId type used in the proofs pallet.
pub type AccountIdFor<T> = <T as frame_system::Config>::AccountId;

/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type MerkleHashFor<T> = <T as crate::Config>::MerkleTrieHash;

/// The type for keys that identify a file within a Merkle Patricia Forest.
/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type KeyFor<T> = <T as crate::Config>::MerkleTrieHash;

/// The type for a root of a Merkle Patricia Forest.
/// Syntactic sugar for the MerkleHash type used in the proofs pallet.
pub type ForestRootFor<T> = <T as crate::Config>::MerkleTrieHash;

/// Syntactic sugar for the RandomChallengesPerBlock type used in the proofs pallet.
pub type RandomChallengesPerBlockFor<T> = <T as crate::Config>::RandomChallengesPerBlock;

/// Syntactic sugar for the MaxCustomChallengesPerBlock type used in the proofs pallet.
pub type MaxCustomChallengesPerBlockFor<T> = <T as crate::Config>::MaxCustomChallengesPerBlock;

/// Syntactic sugar for the MaxSubmittersPerBlock type used in the proofs pallet.
pub type MaxSubmittersPerTickFor<T> = <T as crate::Config>::MaxSubmittersPerTick;

/// Syntactic sugar for the TargetBlocksStorageOfSubmitters type used in the proofs pallet.
pub type TargetTicksStorageOfSubmittersFor<T> =
    <T as crate::Config>::TargetTicksStorageOfSubmitters;

/// Syntactic sugar for the ChallengesQueueLength type used in the proofs pallet.
pub type ChallengesQueueLengthFor<T> = <T as crate::Config>::ChallengesQueueLength;

/// Syntactic sugar for the ChallengesFee type used in the proofs pallet.
pub type ChallengesFeeFor<T> = <T as crate::Config>::ChallengesFee;

/// Syntactic sugar for the StakeToChallengePeriod type used in the proofs pallet.
pub type StakeToChallengePeriodFor<T> = <T as crate::Config>::StakeToChallengePeriod;

/// Syntactic sugar for the MinChallengePeriod type used in the proofs pallet.
pub type MinChallengePeriodFor<T> = <T as crate::Config>::MinChallengePeriod;

/// Syntactic sugar for the ChallengeHistoryLength type used in the proofs pallet.
pub type ChallengeHistoryLengthFor<T> = <T as crate::Config>::ChallengeHistoryLength;

/// Syntactic sugar for the CheckpointChallengePeriod type used in the proofs pallet.
pub type CheckpointChallengePeriodFor<T> = <T as crate::Config>::CheckpointChallengePeriod;

/// Syntactic sugar for the ChallengeTicksTolerance type used in the proofs pallet.
pub type ChallengeTicksToleranceFor<T> = <T as crate::Config>::ChallengeTicksTolerance;

/// Syntactic sugar for the Treasury type used in the proofs pallet.
pub type TreasuryAccountFor<T> = <T as crate::Config>::Treasury;

/// Syntactic sugar for the Providers type used in the proofs pallet.
pub type ProvidersPalletFor<T> = <T as crate::Config>::ProvidersPallet;

/// Syntactic sugar for the Provider ID type used in the proofs pallet.
pub type ProviderIdFor<T> =
    <<T as crate::Config>::ProvidersPallet as ReadChallengeableProvidersInterface>::ProviderId;

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

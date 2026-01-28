extern crate alloc;

use alloc::collections::BTreeMap;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::fmt::{Debug, Formatter, Result};
use frame_support::traits::fungible;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use shp_traits::{CommitmentVerifier, ReadChallengeableProvidersInterface};

/// Type that encapsulates the proof a Provider submits.
///
/// The proof consists of a forest proof and a set of key proofs.
/// A good proof would have a forest proof that proves that some keys belong to a
/// Merkle Patricia Forest of a Provider, and the corresponding key proofs for those keys.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Clone)]
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
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Clone)]
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

/// Information to keep track of the Provider's challenge cycle.
///
/// This stores the last tick the Provider submitted a proof for, and the next tick
/// the Provider should submit a proof for. Normally the difference between these two
/// ticks is equal to the Provider's challenge period, but if the Provider's period
/// is changed, this change only affects the next cycle. In other words, for one
/// cycle, `next_tick_to_submit_proof_for - last_tick_proven ≠ provider_challenge_period`.
///
/// Similarly, if the Provider is slashed, its `last_tick_proven` won't be updated, while
/// its `next_tick_to_submit_proof_for` will be accordingly updated.
#[derive(Debug, Encode, Decode, TypeInfo, PartialEq, Eq, Clone, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct ProofSubmissionRecord<T: crate::Config> {
    /// The last tick the Provider submitted a proof for.
    ///
    /// Or in other words,
    /// the last proof submitted by the Provider, was a response to the challenge
    /// seed in this tick.
    pub last_tick_proven: BlockNumberFor<T>,
    /// The next tick the Provider should submit a proof for.
    ///
    /// When the Provider submits a valid proof, this is calculated as:
    /// ```ignore
    /// next_tick_to_submit_proof_for = last_tick_proven + provider_challenge_period
    /// ```
    /// Where `provider_challenge_period` is the Provider's challenge period at the time
    /// it submits a proof.
    ///
    /// If the Provider is slashed, this is calculated as:
    /// ```ignore
    /// next_tick_to_submit_proof_for = old_next_tick_to_submit_proof_for + provider_challenge_period
    /// ```
    /// Where `old_next_tick_to_submit_proof_for` is the challenge missed, and `provider_challenge_period`
    /// is the Provider's challenge period at the time it is marked as slashable.
    pub next_tick_to_submit_proof_for: BlockNumberFor<T>,
}

/// A custom challenge that can be included in a checkpoint challenge round.
///
/// It contains the key being challenged and a boolean indicating whether the key should be removed
/// from the Merkle Patricia Forest. This key will be removed if `should_remove_key` is `true` and
/// if when the Provider responds to this challenge with a proof, in that proof there is an inclusion
/// proof for that key (i.e. the key is in the Merkle Patricia Forest).
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct CustomChallenge<T: crate::Config> {
    /// The key being challenged.
    pub key: KeyFor<T>,
    /// Whether the key should be removed from the Merkle Patricia Forest.
    pub should_remove_key: bool,
}

/// Implement Debug for CustomChallenge. Cannot derive Debug directly because of compiler issues
/// with the generic type.
impl<T: crate::Config> Debug for CustomChallenge<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "CustomChallenge {{ key: {:?}, should_remove_key: {:?} }}",
            self.key, self.should_remove_key
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

/// Syntactic sugar for the MerkleTrieHashing type used in the proofs pallet.
pub type MerkleTrieHashingFor<T> = <T as crate::Config>::MerkleTrieHashing;

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

/// Syntactic sugar for the PriorityChallengesFee type used in the proofs pallet.
pub type PriorityChallengesFeeFor<T> = <T as crate::Config>::PriorityChallengesFee;

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

/// Syntactic sugar for BlockFullnessPeriod type used in the ProofsDealer pallet.
pub type BlockFullnessPeriodFor<T> = <T as crate::Config>::BlockFullnessPeriod;

/// Syntactic sugar for BlockFullnessHeadroom type used in the ProofsDealer pallet.
pub type BlockFullnessHeadroomFor<T> = <T as crate::Config>::BlockFullnessHeadroom;

/// Syntactic sugar for MinNotFullBlocksRatio type used in the ProofsDealer pallet.
pub type MinNotFullBlocksRatioFor<T> = <T as crate::Config>::MinNotFullBlocksRatio;

/// Syntactic sugar for MaxSlashableProvidersPerTick type used in the ProofsDealer pallet.
pub type MaxSlashableProvidersPerTickFor<T> = <T as crate::Config>::MaxSlashableProvidersPerTick;

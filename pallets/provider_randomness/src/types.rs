use codec::{Decode, Encode};
use frame_support::{traits::fungible::Inspect, RuntimeDebugNoBound};
use scale_info::TypeInfo;
use shp_traits::ReadChallengeableProvidersInterface;

/// Type that encapsulates the commitment a Provider submitted before with the seed that verifies it.
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, RuntimeDebugNoBound)]
#[scale_info(skip_type_params(T))]
pub struct CommitmentWithSeed<T: crate::Config> {
    /// The commitment for the seed.
    pub commitment: SeedCommitmentFor<T>,
    /// The seed that verifies the commitment.
    pub seed: SeedFor<T>,
}

// ****************************************************************************
// ********************* Syntactic sugar for types ****************************
// ****************************************************************************

/// The Balances pallet of the runtime.
pub type BalancesPalletFor<T> = <T as pallet_proofs_dealer::Config>::NativeBalance;

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <BalancesPalletFor<T> as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// The Providers pallet from the Proofs Dealer configuration.
pub type ProvidersPalletFor<T> = <T as pallet_proofs_dealer::Config>::ProvidersPallet;

/// The type of the Provider ID for the given configuration.
pub type ProviderIdFor<T> = <<T as pallet_proofs_dealer::Config>::ProvidersPallet as ReadChallengeableProvidersInterface>::ProviderId;

/// The converter from a Balance to a Block Number.
pub type StakeToBlockNumberFor<T> = <T as pallet_proofs_dealer::Config>::StakeToBlockNumber;

/// The type of the Seed for the given configuration.
pub type SeedFor<T> = <T as crate::Config>::Seed;

/// The type of the Seed Commitment for the given configuration.
pub type SeedCommitmentFor<T> = <T as crate::Config>::SeedCommitment;

//! Various types to use in the Storage Providers pallet.

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::fungible::Inspect;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use storage_hub_traits::ProvidersInterface;

/// Structure that has the Fixed-Rate Payment Stream information
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct FixedRatePaymentStream<T: Config> {
    pub rate: BalanceOf<T>,
    pub last_charged_block: BlockNumberFor<T>,
    pub last_chargeable_block: BlockNumberFor<T>,
    pub user_deposit: BalanceOf<T>,
}

/// Structure that has the Dynamic-Rate Payment Stream information
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct DynamicRatePaymentStream<T: Config> {
    pub amount_provided: UnitsProvidedFor<T>,
    pub price_index_when_last_charged: BalanceOf<T>,
    pub price_index_at_last_chargeable_block: BalanceOf<T>,
    pub user_deposit: BalanceOf<T>,
}

// Type aliases:

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// UnitsProvidedFor is the type of the units provided by the provider.
pub type UnitsProvidedFor<T> = <T as Config>::Units;

/// Syntactic sugar for the ProviderId type used in the proofs pallet.
pub type ProviderIdFor<T> =
    <<T as crate::Config>::ProvidersPallet as ProvidersInterface>::ProviderId;

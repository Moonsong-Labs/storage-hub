//! Various types to use in the Storage Providers pallet.

use super::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::fungible::Inspect;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;
use shp_traits::ReadProvidersInterface;

/// Structure that has the Fixed-Rate Payment Stream information
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebugNoBound,
    PartialEq,
    Eq,
    Clone,
)]
#[scale_info(skip_type_params(T))]
pub struct FixedRatePaymentStream<T: Config> {
    pub rate: BalanceOf<T>,
    pub last_charged_tick: BlockNumberFor<T>,
    pub user_deposit: BalanceOf<T>,
    pub out_of_funds_tick: Option<BlockNumberFor<T>>,
}

/// Structure that has the Dynamic-Rate Payment Stream information
#[derive(
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebugNoBound,
    PartialEq,
    Eq,
    Clone,
)]
#[scale_info(skip_type_params(T))]
pub struct DynamicRatePaymentStream<T: Config> {
    pub amount_provided: UnitsProvidedFor<T>,
    pub price_index_when_last_charged: BalanceOf<T>,
    pub user_deposit: BalanceOf<T>,
    pub out_of_funds_tick: Option<BlockNumberFor<T>>,
}

/// Enum that represents a Payment Stream. It holds either a FixedRatePaymentStream or a DynamicRatePaymentStream,
/// allowing to operate generically with both types.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub enum PaymentStream<T: Config> {
    FixedRatePaymentStream(FixedRatePaymentStream<T>),
    DynamicRatePaymentStream(DynamicRatePaymentStream<T>),
}

/// Structure that holds the information of the last chargeable tick and price index for a Provider
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct ProviderLastChargeableInfo<T: Config> {
    pub last_chargeable_tick: BlockNumberFor<T>,
    pub price_index: BalanceOf<T>,
}
impl<T: pallet::Config> Default for ProviderLastChargeableInfo<T> {
    fn default() -> Self {
        Self {
            last_chargeable_tick: Default::default(),
            price_index: Default::default(),
        }
    }
}

// Type aliases:

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// UnitsProvidedFor is the type of the units provided by the provider.
pub type UnitsProvidedFor<T> = <T as Config>::Units;

/// Syntactic sugar for the ProviderId type used in the proofs pallet.
pub type ProviderIdFor<T> =
    <<T as crate::Config>::ProvidersPallet as ReadProvidersInterface>::ProviderId;

/// Syntactic sugar for the maximum amount of Users a Provider can charge in a batch.
pub type MaxUsersToChargeFor<T> = <T as Config>::MaxUsersToCharge;

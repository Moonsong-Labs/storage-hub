//! Various types to use in the Storage Providers pallet.

use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::fungible::Inspect;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::TypeInfo;

/// Structure that has the payment stream information
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebugNoBound, PartialEq, Eq, Clone)]
#[scale_info(skip_type_params(T))]
pub struct PaymentStream<T: Config> {
    pub rate: BalanceOf<T>,
    pub last_valid_proof: BlockNumberFor<T>,
    pub last_charged_proof: BlockNumberFor<T>,
    // todo!("add relevant fields here")
}

// Type aliases:

/// BalanceOf is the balance type of the runtime.
pub type BalanceOf<T> =
    <<T as Config>::NativeBalance as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// BackupStorageProviderId is the type that represents an ID of a Backup Storage Provider, uniquely linked with an AccountId
pub type BackupStorageProviderId<T> = <T as frame_system::Config>::Hash;

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait PaymentStreamsApi<ProviderId, Balance, AccountId>
    where
        ProviderId: codec::Codec,
        Balance: codec::Codec,
        AccountId: codec::Codec
    {
        fn get_users_with_debt_over_threshold(provider_id: &ProviderId, threshold: Balance) -> Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError>;
        fn get_users_of_payment_streams_of_provider(provider_id: &ProviderId) -> Vec<AccountId>;
        fn get_providers_with_payment_streams_with_user(user_account: &AccountId) -> Vec<ProviderId>;
    }
}

/// Error type for the `get_users_with_debt_over_threshold` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetUsersWithDebtOverThresholdError {
    ProviderNotRegistered,
    ProviderWithoutPaymentStreams,
    AmountToChargeOverflow,
    AmountToChargeUnderflow,
    DebtOverflow,
    InternalApiError,
}

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait StorageProvidersApi<BspId, BspInfo, AccountId, ProviderId>
    where
        BspId: codec::Codec,
        BspInfo: codec::Codec,
        AccountId: codec::Codec,
        ProviderId: codec::Codec,
    {
        fn get_bsp_info(bsp_id: &BspId) -> Result<BspInfo, GetBspInfoError>;
        fn get_storage_provider_id(who: &AccountId) -> Option<ProviderId>;
    }
}

/// Error type for the `get_bsp_info` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetBspInfoError {
    BspNotRegistered,
    InternalApiError,
}

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait StorageProvidersApi<BlockNumber, BspId, BspInfo, AccountId, ProviderId, StorageProviderId, StorageDataUnit>
    where
        BlockNumber: Codec,
        BspId: Codec,
        BspInfo: Codec,
        AccountId: Codec,
        ProviderId: Codec,
        StorageProviderId: Codec,
        StorageDataUnit: Codec,
    {
        fn get_bsp_info(bsp_id: &BspId) -> Result<BspInfo, GetBspInfoError>;
        fn get_storage_provider_id(who: &AccountId) -> Option<StorageProviderId>;
        fn query_storage_provider_capacity(who: &ProviderId) -> Result<StorageDataUnit, QueryStorageProviderCapacityError>;
        fn query_available_storage_capacity(who: &ProviderId) -> Result<StorageDataUnit, QueryAvailableStorageCapacityError>;
        fn query_earliest_change_capacity_block(who: &BspId) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError>;
    }
}

/// Error type for the `get_bsp_info` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetBspInfoError {
    BspNotRegistered,
    InternalApiError,
}

/// Error type for the `query_storage_provider_capacity` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryStorageProviderCapacityError {
    ProviderNotRegistered,
    InternalError,
}

/// Error type for the `query_available_storage_capacity` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryAvailableStorageCapacityError {
    ProviderNotRegistered,
    InternalError,
}

/// Error type for the `query_available_storage_capacity` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryEarliestChangeCapacityBlockError {
    ProviderNotRegistered,
    InternalError,
}

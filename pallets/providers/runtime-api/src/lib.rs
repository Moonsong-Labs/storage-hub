#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait StorageProvidersApi<BlockNumber, BspId, BspInfo, AccountId, ProviderId, StorageProviderId, StorageDataUnit, Balance, BucketId, Multiaddresses>
    where
        BlockNumber: Codec,
        BspId: Codec,
        BspInfo: Codec,
        AccountId: Codec,
        ProviderId: Codec,
        StorageProviderId: Codec,
        StorageDataUnit: Codec,
        Balance: Codec,
        BucketId: Codec,
        Multiaddresses: Codec,
    {
        fn get_bsp_info(bsp_id: &BspId) -> Result<BspInfo, GetBspInfoError>;
        fn get_storage_provider_id(who: &AccountId) -> Option<StorageProviderId>;
        fn query_provider_multiaddresses(provider_id: &ProviderId) -> Result<Multiaddresses, QueryProviderMultiaddressesError>;
        fn query_msp_id_of_bucket_id(bucket_id: &BucketId) -> Result<ProviderId, QueryMspIdOfBucketIdError>;
        fn query_storage_provider_capacity(provider_id: &ProviderId) -> Result<StorageDataUnit, QueryStorageProviderCapacityError>;
        fn query_available_storage_capacity(provider_id: &ProviderId) -> Result<StorageDataUnit, QueryAvailableStorageCapacityError>;
        fn query_earliest_change_capacity_block(bsp_id: &BspId) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError>;
        fn get_worst_case_scenario_slashable_amount(provider_id: ProviderId) -> Option<Balance>;
        fn get_slash_amount_per_max_file_size() -> Balance;
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

/// Error type for the `query_msp_id_of_bucket_id` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryMspIdOfBucketIdError {
    BucketNotFound,
    InternalError,
}

/// Error type for the `query_provider_multiaddresses` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryProviderMultiaddressesError {
    ProviderNotRegistered,
    InternalError,
}

/// Error type for the `get_stake` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GetStakeError {
    ProviderStakeNotFound,
    InternalError,
}

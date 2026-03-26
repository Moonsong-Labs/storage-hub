#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait StorageProvidersApi<BlockNumber, BspId, BspInfo, MspId, AccountId, ProviderId, StorageProviderId, StorageDataUnit, Balance, BucketId, Multiaddresses, ValuePropositionWithId, MerkleHash>
    where
        BlockNumber: Codec,
        BspId: Codec,
        BspInfo: Codec,
        MspId: Codec,
        AccountId: Codec,
        ProviderId: Codec,
        StorageProviderId: Codec,
        StorageDataUnit: Codec,
        Balance: Codec,
        BucketId: Codec,
        Multiaddresses: Codec,
        ValuePropositionWithId: Codec,
        MerkleHash: Codec,
    {
        fn get_bsp_info(bsp_id: &BspId) -> Result<BspInfo, GetBspInfoError>;
        fn get_storage_provider_id(who: &AccountId) -> Option<StorageProviderId>;
        fn query_provider_multiaddresses(provider_id: &ProviderId) -> Result<Multiaddresses, QueryProviderMultiaddressesError>;
        fn query_msp_id_of_bucket_id(bucket_id: &BucketId) -> Result<Option<ProviderId>, QueryMspIdOfBucketIdError>;
        fn query_storage_provider_capacity(provider_id: &ProviderId) -> Result<StorageDataUnit, QueryStorageProviderCapacityError>;
        fn query_available_storage_capacity(provider_id: &ProviderId) -> Result<StorageDataUnit, QueryAvailableStorageCapacityError>;
        fn query_earliest_change_capacity_block(bsp_id: &BspId) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError>;
        fn get_worst_case_scenario_slashable_amount(provider_id: ProviderId) -> Option<Balance>;
        fn get_slash_amount_per_max_file_size() -> Balance;
        fn query_value_propositions_for_msp(msp_id: &MspId) -> sp_runtime::Vec<ValuePropositionWithId>;
        fn get_bsp_stake(bsp_id: &BspId) -> Result<Balance, GetStakeError>;
        fn can_delete_provider(provider_id: &ProviderId) -> bool;
        fn query_buckets_for_msp(msp_id: &MspId) -> Result<sp_runtime::Vec<BucketId>, QueryBucketsForMspError>;
        fn query_buckets_of_user_stored_by_msp(msp_id: &ProviderId, user: &AccountId) -> Result<sp_runtime::Vec<BucketId>, QueryBucketsOfUserStoredByMspError>;
        fn query_bucket_root(bucket_id: &BucketId) -> Result<MerkleHash, QueryBucketRootError>;
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
    ProviderNotRegistered,
    InternalError,
}

/// Error type for the `query_buckets_for_msp` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryBucketsForMspError {
    ProviderNotRegistered,
    InternalError,
}

/// Error type for the `query_buckets_of_user_stored_by_msp` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryBucketsOfUserStoredByMspError {
    NotAnMsp,
    InternalError,
}

/// Error type for the `query_bucket_root` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryBucketRootError {
    BucketNotFound,
    InternalError,
}

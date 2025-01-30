#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

/// Error type for the `is_storage_request_open_to_volunteers` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum IsStorageRequestOpenToVolunteersError {
    StorageRequestNotFound,
    InternalError,
}

/// Error type for the `query_earliest_file_volunteer_tick` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryFileEarliestVolunteerTickError {
    FailedToEncodeFingerprint,
    FailedToEncodeBsp,
    ThresholdArithmeticError,
    StorageRequestNotFound,
    FailedToComputeEligibilityCriteria,
    InternalError,
}

/// Error type for the `query_bsp_confirm_chunks_to_prove_for_file` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryBspConfirmChunksToProveForFileError {
    StorageRequestNotFound,
    ConfirmChunks(QueryConfirmChunksToProveForFileError),
    InternalError,
}

/// Error type for the `query_msp_confirm_chunks_to_prove_for_file` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryMspConfirmChunksToProveForFileError {
    StorageRequestNotFound,
    ConfirmChunks(QueryConfirmChunksToProveForFileError),
    InternalError,
}

/// Error type for the `query_confirm_chunks_to_prove_for_file`.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryConfirmChunksToProveForFileError {
    ChallengedChunkToChunkIdError,
}

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait FileSystemApi<BackupStorageProviderId, MainStorageProviderId, FileKey, TickNumber, ChunkId>
    where
        BackupStorageProviderId: Codec,
        MainStorageProviderId: Codec,
        FileKey: Codec,
        TickNumber: Codec,
        ChunkId: Codec,
    {
        fn is_storage_request_open_to_volunteers(file_key: FileKey) -> Result<bool, IsStorageRequestOpenToVolunteersError>;
        fn query_earliest_file_volunteer_tick(bsp_id: BackupStorageProviderId, file_key: FileKey) -> Result<TickNumber, QueryFileEarliestVolunteerTickError>;
        fn query_bsp_confirm_chunks_to_prove_for_file(bsp_id: BackupStorageProviderId, file_key: FileKey) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>;
        fn query_msp_confirm_chunks_to_prove_for_file(msp_id: MainStorageProviderId, file_key: FileKey) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError>;
    }
}

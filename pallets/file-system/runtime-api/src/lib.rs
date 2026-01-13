#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::collections::btree_map::BTreeMap;

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

/// Error type for the `query_bsps_volunteered_for_file` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryBspsVolunteeredForFileError {
    StorageRequestNotFound,
    InternalError,
}

/// Error type for the `query_confirm_chunks_to_prove_for_file`.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryConfirmChunksToProveForFileError {
    ChallengedChunkToChunkIdError,
    FailedToCreateFileMetadata,
    FailedToGenerateChunkChallenges,
}

/// Error type for `decode_generic_apply_delta_event_info`.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum GenericApplyDeltaEventInfoError {
    DecodeError,
}

/// Error type for the `pending_storage_requests_by_msp`.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum StorageRequestsByMspError {
    FailedToRetrieveStorageRequests,
}

/// Error type for the `query_incomplete_storage_request_metadata`.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryIncompleteStorageRequestMetadataError {
    StorageNotFound,
    InternalError,
}

/// Response type for incomplete storage request metadata that can be safely passed across runtime boundary
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct IncompleteStorageRequestMetadataResponse<
    AccountId,
    BucketId,
    StorageDataUnit,
    Fingerprint,
    ProviderId,
> {
    /// File owner for validation
    pub owner: AccountId,
    /// Bucket containing the file
    pub bucket_id: BucketId,
    /// File location/path
    pub location: Vec<u8>,
    /// File size
    pub file_size: StorageDataUnit,
    /// File fingerprint
    pub fingerprint: Fingerprint,
    /// BSPs that still need to remove the file (as a Vec instead of BoundedVec)
    pub pending_bsp_removals: Vec<ProviderId>,
    /// Whether the file still needs to be removed from the bucket
    pub pending_bucket_removal: bool,
}

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait FileSystemApi<AccountId, BackupStorageProviderId, MainStorageProviderId, FileKey, TickNumber, ChunkId, GenericApplyDeltaEventInfo, StorageRequestMetadata, BucketId, StorageDataUnit, Fingerprint>
    where
        AccountId: Codec,
        BackupStorageProviderId: Codec,
        MainStorageProviderId: Codec,
        FileKey: Codec,
        TickNumber: Codec,
        ChunkId: Codec,
        GenericApplyDeltaEventInfo: Codec,
        StorageRequestMetadata: Codec,
        BucketId: Codec,
        StorageDataUnit: Codec,
        Fingerprint: Codec,
    {
        fn is_storage_request_open_to_volunteers(file_key: FileKey) -> Result<bool, IsStorageRequestOpenToVolunteersError>;
        fn query_earliest_file_volunteer_tick(bsp_id: BackupStorageProviderId, file_key: FileKey) -> Result<TickNumber, QueryFileEarliestVolunteerTickError>;
        fn query_bsp_confirm_chunks_to_prove_for_file(bsp_id: BackupStorageProviderId, file_key: FileKey) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>;
        fn query_msp_confirm_chunks_to_prove_for_file(msp_id: MainStorageProviderId, file_key: FileKey) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError>;
        fn query_bsps_volunteered_for_file(file_key: FileKey) -> Result<Vec<BackupStorageProviderId>, QueryBspsVolunteeredForFileError>;
        fn decode_generic_apply_delta_event_info(encoded_event_info: Vec<u8>) -> Result<GenericApplyDeltaEventInfo, GenericApplyDeltaEventInfoError>;
        fn storage_requests_by_msp(msp_id: MainStorageProviderId) -> BTreeMap<H256, StorageRequestMetadata>;
        fn pending_storage_requests_by_msp(msp_id: MainStorageProviderId) -> BTreeMap<H256, StorageRequestMetadata>;
        fn query_incomplete_storage_request_metadata(file_key: FileKey) -> Result<IncompleteStorageRequestMetadataResponse<AccountId, BucketId, StorageDataUnit, Fingerprint, BackupStorageProviderId>, QueryIncompleteStorageRequestMetadataError>;
        fn list_incomplete_storage_request_keys(start_after: Option<FileKey>, limit: u32) -> Vec<FileKey>;
        fn query_pending_bsp_confirm_storage_requests(bsp_id: BackupStorageProviderId, file_keys: Vec<FileKey>) -> Vec<FileKey>;
    }
}

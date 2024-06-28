#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

/// Error type for the `query_earliest_file_volunteer_block` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryFileEarliestVolunteerBlockError {
    FailedToEncodeFingerprint,
    FailedToEncodeBsp,
    ThresholdArithmeticError,
    StorageRequestNotFound,
    InternalError,
}

/// Error type for the `query_bsp_confirm_chunks_to_prove_for_file` runtime API call.
#[derive(Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum QueryBspConfirmChunksToProveForFileError {
    StorageRequestNotFound,
    InternalError,
}

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait FileSystemApi<ProviderId, FileKey, BlockNumber, ChunkId>
    where
        ProviderId: Codec,
        FileKey: Codec,
        BlockNumber: Codec,
        ChunkId: Codec,
    {
        fn query_earliest_file_volunteer_block(bsp_id: ProviderId, file_key: FileKey) -> Result<BlockNumber, QueryFileEarliestVolunteerBlockError>;
        fn query_bsp_confirm_chunks_to_prove_for_file(bsp_id: ProviderId, file_key: FileKey) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError>;
    }
}

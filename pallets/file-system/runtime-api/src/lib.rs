// #![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
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

sp_api::decl_runtime_apis! {
    #[api_version(1)]
    pub trait FileSystemApi<ProviderId, FileKey, BlockNumber>
    where
        ProviderId: codec::Codec,
        FileKey: codec::Codec,
        BlockNumber: codec::Codec,
    {
        fn query_earliest_file_volunteer_block(bsp_id: ProviderId, file_key: FileKey) -> Result<BlockNumber, QueryFileEarliestVolunteerBlockError>;
    }
}

use frame_support::dispatch::DispatchInfo;
use frame_system::EventRecord;
use sp_core::H256;
use sp_runtime::DispatchError;

/// Type alias for the events vector.
///
/// The events vector is a storage element in the FRAME system pallet, which stores all the events that have occurred
/// in a block. This is syntactic sugar to make the code more readable.
pub type EventsVec = Vec<
    Box<
        EventRecord<
            <storage_hub_runtime::Runtime as frame_system::Config>::RuntimeEvent,
            <storage_hub_runtime::Runtime as frame_system::Config>::Hash,
        >,
    >,
>;

/// Extrinsic struct.
///
/// This struct represents an extrinsic in the blockchain.
#[derive(Debug, Clone)]
pub struct Extrinsic {
    /// Extrinsic hash.
    pub hash: H256,
    /// Block hash.
    pub block_hash: H256,
    /// Events vector.
    pub events: EventsVec,
}

/// ExtrinsicResult enum.
///
/// This enum represents the result of an extrinsic execution. It can be either a success or a failure.
pub enum ExtrinsicResult {
    /// Success variant.
    ///
    /// This variant represents a successful extrinsic execution.
    Success {
        /// Dispatch info.
        dispatch_info: DispatchInfo,
    },
    /// Failure variant.
    ///
    /// This variant represents a failed extrinsic execution.
    Failure {
        /// Dispatch error.
        dispatch_error: DispatchError,
        /// Dispatch info.
        dispatch_info: DispatchInfo,
    },
}

/// Type alias for the RPC JSON response as Strings.
pub type RpcJsonResponse = String;

/// Type alias for the extrinsic hash.
pub type ExtrinsicHash = H256;

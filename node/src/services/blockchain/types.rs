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
/// This struct represents an extrinsic in the blockchain. It contains the following fields:
/// - `hash`: the hash of the extrinsic.
/// - `block_hash`: the hash of the block in which the extrinsic was included.
/// - `events`: the events that occurred during the execution of the extrinsic.
#[derive(Debug, Clone)]
pub struct Extrinsic {
    pub hash: H256,
    pub block_hash: H256,
    pub events: EventsVec,
}

/// ExtrinsicResult enum.
///
/// This enum represents the result of an extrinsic execution. It can be either a success or a failure. It contains the
/// following variants:
/// - `Success`: the extrinsic was executed successfully.
/// - `Failure`: the extrinsic execution failed.

pub enum ExtrinsicResult {
    Success {
        dispatch_info: DispatchInfo,
    },
    Failure {
        dispatch_error: DispatchError,
        dispatch_info: DispatchInfo,
    },
}

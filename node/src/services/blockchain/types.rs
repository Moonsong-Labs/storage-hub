use frame_system::EventRecord;
use sp_core::H256;

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

#[derive(Debug, Clone)]
pub struct Extrinsic {
    pub hash: H256,
    pub block_hash: H256,
    pub events: EventsVec,
}

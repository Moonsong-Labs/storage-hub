use frame_support::{StorageHasher, Twox128};
use lazy_static::lazy_static;
use std::sync::Arc;
use thiserror::Error;

use codec::Decode;
use sc_client_api::{backend::StorageProvider, StorageKey};
use sp_core::H256;

use crate::types::{ParachainClient, StorageHubEventsVec};

lazy_static! {
    // Would be cool to be able to do this...
    // let events_storage_key = frame_system::Events::<storage_hub_runtime::Runtime>::hashed_key();

    // Static and lazily initialised `events_storage_key`
    static ref EVENTS_STORAGE_KEY: Vec<u8> = {
        let key = [
            Twox128::hash(b"System").to_vec(),
            Twox128::hash(b"Events").to_vec(),
        ]
        .concat();
        key
    };
}

#[derive(Error, Debug)]
pub enum EventsRetrievalError {
    #[error("Failed to get Events storage element: {0}")]
    StorageRetrievalError(#[from] sp_blockchain::Error),
    #[error("Failed to decode Events storage element: {0}")]
    DecodeError(#[from] codec::Error),
    #[error("Events storage element not found")]
    StorageNotFound,
}

/// Get the events storage element for a given block.
pub fn get_events_at_block(
    client: &Arc<ParachainClient>,
    block_hash: &H256,
) -> Result<StorageHubEventsVec, EventsRetrievalError> {
    // Get the events storage.
    let raw_storage_opt = client.storage(*block_hash, &StorageKey(EVENTS_STORAGE_KEY.clone()))?;

    // Decode the events storage.
    raw_storage_opt
        .map(|raw_storage| StorageHubEventsVec::decode(&mut raw_storage.0.as_slice()))
        .transpose()?
        .ok_or(EventsRetrievalError::StorageNotFound)
}

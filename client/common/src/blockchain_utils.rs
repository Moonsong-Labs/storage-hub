use frame_support::{StorageHasher, Twox128};
use lazy_static::lazy_static;
use log::error;
use sc_network::Multiaddr;
use std::{str::FromStr, sync::Arc};
use thiserror::Error;

use codec::Decode;
use sc_client_api::{backend::StorageProvider, StorageKey};
use sp_core::H256;

use crate::{
    traits::{StorageEnableApiCollection, StorageEnableRuntimeApi, StorageEnableRuntimeConfig},
    types::{Multiaddresses, ParachainClient, StorageHubEventsVec},
};

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
pub fn get_events_at_block<
    RuntimeApi: StorageEnableRuntimeApi,
    Runtime: StorageEnableRuntimeConfig,
>(
    client: &Arc<ParachainClient<RuntimeApi>>,
    block_hash: &H256,
) -> Result<StorageHubEventsVec<Runtime>, EventsRetrievalError>
where
    RuntimeApi::RuntimeApi: StorageEnableApiCollection<Runtime>,
{
    // Get the events storage.
    let raw_storage_opt = client.storage(*block_hash, &StorageKey(EVENTS_STORAGE_KEY.clone()))?;

    // Decode the events storage.
    raw_storage_opt
        .map(|raw_storage| StorageHubEventsVec::<Runtime>::decode(&mut raw_storage.0.as_slice()))
        .transpose()?
        .ok_or(EventsRetrievalError::StorageNotFound)
}

/// Attempt to convert BoundedVec of BoundedVecs of bytes.
///
/// Returns a list of [`Multiaddr`] objects that have successfully been parsed from the raw bytes.
pub fn convert_raw_multiaddresses_to_multiaddr<Runtime: StorageEnableRuntimeConfig>(
    multiaddresses: Multiaddresses<Runtime>,
) -> Vec<Multiaddr> {
    let mut multiaddress_vec: Vec<Multiaddr> = Vec::new();
    for raw_multiaddr in multiaddresses.into_iter() {
        if let Some(multiaddress) = convert_raw_multiaddress_to_multiaddr(&raw_multiaddr) {
            multiaddress_vec.push(multiaddress);
        }
    }
    multiaddress_vec
}

pub fn convert_raw_multiaddress_to_multiaddr(raw_multiaddr: &[u8]) -> Option<Multiaddr> {
    match std::str::from_utf8(raw_multiaddr) {
        Ok(s) => match Multiaddr::from_str(s) {
            Ok(multiaddr) => Some(multiaddr),
            Err(e) => {
                error!("Failed to parse Multiaddress from string: {:?}", e);
                None
            }
        },
        Err(e) => {
            error!("Failed to parse Multiaddress from bytes: {:?}", e);
            None
        }
    }
}

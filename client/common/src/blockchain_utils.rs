use frame_support::{StorageHasher, Twox128};
use lazy_static::lazy_static;
use log::error;
use sc_network::Multiaddr;
use std::{str::FromStr, sync::Arc};
use thiserror::Error;

use codec::Decode;
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use sc_client_api::{backend::StorageProvider, StorageKey};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;

use crate::{
    traits::{KeyTypeOperations, StorageEnableRuntime},
    types::{
        Multiaddresses, ParachainClient, StorageHubEventsVec, StorageProviderId, BCSV_KEY_TYPE,
    },
};

lazy_static! {
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
pub fn get_events_at_block<Runtime: StorageEnableRuntime>(
    client: &Arc<ParachainClient<Runtime::RuntimeApi>>,
    block_hash: &H256,
) -> Result<StorageHubEventsVec<Runtime>, EventsRetrievalError> {
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
pub fn convert_raw_multiaddresses_to_multiaddr<Runtime>(
    multiaddresses: Multiaddresses<Runtime>,
) -> Vec<Multiaddr>
where
    Runtime: StorageEnableRuntime,
{
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

#[derive(Error, Debug)]
pub enum GetProviderIdError {
    #[error("Multiple provider IDs found for BCSV keys. Managing multiple providers at once is not supported.")]
    MultipleProviderIds,
    #[error("Runtime API error while getting Provider ID: {0}")]
    RuntimeApiError(String),
}

/// Get the Provider ID linked to the [`BCSV_KEY_TYPE`] keys in the keystore.
///
/// This function searches for all BCSV keys in the keystore and queries the runtime
/// to get the associated Provider ID for each key.
///
/// # Returns
/// - `Ok(None)` if no Provider ID is found for any BCSV key
/// - `Ok(Some(provider_id))` if exactly one Provider ID is found
/// - `Err(GetProviderIdError::MultipleProviderIds)` if multiple Provider IDs are found
/// - `Err(GetProviderIdError::RuntimeApiError)` if there's an error calling the runtime API
pub fn get_provider_id_from_keystore<Runtime>(
    client: &Arc<ParachainClient<Runtime::RuntimeApi>>,
    keystore: &sp_keystore::KeystorePtr,
    block_hash: &H256,
) -> Result<Option<StorageProviderId<Runtime>>, GetProviderIdError>
where
    Runtime: StorageEnableRuntime,
{
    let mut provider_ids_found = Vec::new();

    for key in Runtime::Signature::public_keys(keystore, BCSV_KEY_TYPE) {
        let maybe_provider_id = client
            .runtime_api()
            .get_storage_provider_id(*block_hash, &key.into())
            .map_err(|e| GetProviderIdError::RuntimeApiError(e.to_string()))?;

        if let Some(provider_id) = maybe_provider_id {
            provider_ids_found.push(provider_id);
        }
    }

    match provider_ids_found.len() {
        0 => Ok(None),
        1 => Ok(Some(provider_ids_found[0])),
        _ => Err(GetProviderIdError::MultipleProviderIds),
    }
}

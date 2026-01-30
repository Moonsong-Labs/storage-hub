use frame_support::{traits::PalletInfoAccess, StorageHasher, Twox128, Twox64Concat};
use lazy_static::lazy_static;
use log::error;
use sc_network::Multiaddr;
use std::{str::FromStr, sync::Arc};
use thiserror::Error;

use codec::{Decode, Encode};
use pallet_storage_providers_runtime_api::StorageProvidersApi;
use sc_client_api::{backend::StorageProvider, StorageKey};
use sp_api::ProvideRuntimeApi;
use sp_core::{H256, U256};

use crate::{
    traits::{KeyTypeOperations, StorageEnableRuntime},
    types::{
        Multiaddresses, StorageEnableErrors, StorageHubClient, StorageHubEventsVec,
        StorageProviderId, BCSV_KEY_TYPE,
    },
};

const LOG_TARGET: &str = "blockchain-utils";

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

#[derive(Error, Debug)]
pub enum ModuleErrorDecodeError {
    #[error("Unknown pallet index {0}: no matching pallet found")]
    UnknownPallet(u8),
    #[error("Failed to decode error for pallet index {0}: {1}")]
    DecodeError(u8, codec::Error),
}

/// Get the events storage element for a given block.
///
/// # Event Decoding Strategy
///
/// This function decodes `System.Events` locally using the compiled runtime's event types.
/// If a runtime upgrade introduces breaking event changes, the SCALE decode will fail and return an error.
///
/// To ensure backward compatibility for historical blocks, Runtimes which implement StorageHub pallets _**must**_ have fixed pallet indices via `#[runtime::pallet_index(N)]`
/// but this cannot be enforced at the StorageHub pallet level.
///
/// StorageHub pallets do enforce the following constraints on event and error variants:
/// - Fixed variant indices via `#[codec(index = N)]`
/// - Append-only variants (breaking changes use `Vx` suffixes)
///
/// # Errors
///
/// Returns `EventsRetrievalError::DecodeError` if the events payload exists but cannot be
/// SCALE-decoded into `StorageHubEventsVec<Runtime>`. A decode failure likely indicates an
/// incompatible runtime upgrade and the client may need to be upgraded.
pub fn get_events_at_block<Runtime: StorageEnableRuntime>(
    client: &Arc<StorageHubClient<Runtime::RuntimeApi>>,
    block_hash: &H256,
) -> Result<StorageHubEventsVec<Runtime>, EventsRetrievalError> {
    // Get the events storage.
    let raw_storage = client
        .storage(*block_hash, &StorageKey(EVENTS_STORAGE_KEY.clone()))?
        .ok_or(EventsRetrievalError::StorageNotFound)?;

    StorageHubEventsVec::<Runtime>::decode(&mut raw_storage.0.as_slice()).map_err(|e| {
        error!(
            target: LOG_TARGET,
            "Failed to decode System.Events at block {:?}. This likely indicates a breaking change in a possible runtime upgrade since an event was likely 
            added or even worse an existing event was removed or updated and cannot be decoded. Underlying error: {:?}",
            block_hash, e
        );
        EventsRetrievalError::DecodeError(e)
    })
}

/// Decode a [`sp_runtime::ModuleError`] into [`StorageEnableErrors`].
///
/// This uses compile-time pallet indices to determine which pallet the error originated from,
/// then decodes the error bytes into the appropriate error variant.
///
/// # Error Decoding Strategy
///
/// This function relies on fixed pallet indices and error variant indices to
/// decode the error into the appropriate error variant.
///
/// If a runtime upgrade changes the ordering of error variants or removes an error variant, decoding will fail.
pub fn decode_module_error<Runtime: StorageEnableRuntime>(
    module_error: sp_runtime::ModuleError,
) -> Result<StorageEnableErrors<Runtime>, ModuleErrorDecodeError> {
    let system_index = frame_system::Pallet::<Runtime>::index() as u8;
    let providers_index = pallet_storage_providers::Pallet::<Runtime>::index() as u8;
    let proofs_dealer_index = pallet_proofs_dealer::Pallet::<Runtime>::index() as u8;
    let payment_streams_index = pallet_payment_streams::Pallet::<Runtime>::index() as u8;
    let file_system_index = pallet_file_system::Pallet::<Runtime>::index() as u8;
    let balances_index = pallet_balances::Pallet::<Runtime>::index() as u8;
    let bucket_nfts_index = pallet_bucket_nfts::Pallet::<Runtime>::index() as u8;

    match module_error.index {
        i if i == system_index => {
            frame_system::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::System)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        i if i == providers_index => {
            pallet_storage_providers::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::StorageProviders)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        i if i == proofs_dealer_index => {
            pallet_proofs_dealer::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::ProofsDealer)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        i if i == payment_streams_index => {
            pallet_payment_streams::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::PaymentStreams)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        i if i == file_system_index => {
            pallet_file_system::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::FileSystem)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        i if i == balances_index => {
            pallet_balances::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::Balances)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        i if i == bucket_nfts_index => {
            pallet_bucket_nfts::Error::<Runtime>::decode(&mut &module_error.error[..])
                .map(StorageEnableErrors::BucketNfts)
                .map_err(|e| ModuleErrorDecodeError::DecodeError(i, e))
        }
        _ => Err(ModuleErrorDecodeError::UnknownPallet(module_error.index)),
    }
}

/// Get the Ethereum block hash from `pallet_ethereum::BlockHash` storage for a given block number.
///
/// # Parameters
/// - `client`: The blockchain client
/// - `block_hash`: The Substrate block hash to query storage at
/// - `block_number`: The block number to get the Ethereum block hash for
///
/// # Returns
/// - `Ok(Some(eth_block_hash))` if the Ethereum block hash exists in storage
/// - `Ok(None)` if the block hash doesn't exist (may have been pruned)
/// - `Err(...)` if there's an error accessing storage
pub fn get_ethereum_block_hash<RuntimeApi>(
    client: &Arc<StorageHubClient<RuntimeApi>>,
    block_hash: &H256,
    block_number: u32,
) -> Result<Option<H256>, sp_blockchain::Error> {
    // Construct the storage key for pallet_ethereum::BlockHash
    // StorageMap<_, Twox64Concat, U256, H256, ValueQuery>
    let pallet_prefix = Twox128::hash(b"Ethereum").to_vec();
    let storage_prefix = Twox128::hash(b"BlockHash").to_vec();

    // Encode and hash the map key (U256 block number) using Twox64Concat
    let key = U256::from(block_number);
    let encoded_key = key.encode();
    let hashed_key = Twox64Concat::hash(&encoded_key);

    // Construct the complete storage key for the received block number
    let storage_key: Vec<u8> = [pallet_prefix, storage_prefix, hashed_key.to_vec()].concat();

    // Read the block hash from storage
    let raw_storage_opt = client.storage(*block_hash, &StorageKey(storage_key))?;

    // Decode and return it if it exists
    Ok(raw_storage_opt.and_then(|raw_storage| H256::decode(&mut raw_storage.0.as_slice()).ok()))
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
                error!(target: LOG_TARGET, "Failed to parse Multiaddress from string: {:?}", e);
                None
            }
        },
        Err(e) => {
            error!(target: LOG_TARGET, "Failed to parse Multiaddress from bytes: {:?}", e);
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
    client: &Arc<StorageHubClient<Runtime::RuntimeApi>>,
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

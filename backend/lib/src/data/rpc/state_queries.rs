//! Contains the various state queries that the backend accesses
//!
//! This module provides a type-safe way to interact with chain storage through the
//! `StorageQueryTypes` trait, which encodes the key parameters and return types.
//!
//! TODO: consider refactoring this to use subxt or similar
//! and auto-import/generate the bindings from the runtime

use codec::Decode;
use shc_indexer_db::OnchainMspId;
use sp_core::storage::StorageKey;

use crate::runtime::MainStorageProvidersStorageMap;

// Type Aliases for Storage Types
pub type MspInfo = crate::runtime::MainStorageProvider;

/// Trait to associate key parameters and value types for each storage query.
///
/// This trait allows you to access the concrete types for storage keys and values
/// at compile time, ensuring type-safe storage access.
pub trait StorageQueryTypes {
    /// The parameter(s) needed to generate the storage key.
    /// This can be any type that's appropriate for the specific query.
    type KeyParams;

    /// The type of value stored at this storage location
    type Value: Decode;

    /// Generate the storage key from parameters
    fn storage_key(params: Self::KeyParams) -> StorageKey;
}

/// Query for MSP (Main Storage Provider) information
pub struct MspInfoQuery;
impl StorageQueryTypes for MspInfoQuery {
    type KeyParams = OnchainMspId;
    type Value = MspInfo;

    fn storage_key(provider: Self::KeyParams) -> StorageKey {
        let key = MainStorageProvidersStorageMap::hashed_key_for(provider.as_h256());
        StorageKey(key)
    }
}

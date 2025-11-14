//! Contains the various state queries that the backend accesses
//!
//! TODO: consider refactoring this to use subxt or similar
//! and auto-import/generate the bindings from the runtime

use shc_indexer_db::OnchainMspId;
use sp_core::storage::StorageKey;

use crate::runtime::MainStorageProvidersStorageMap;

pub type MspInfo = crate::runtime::MainStorageProvider;
pub fn msp_info_key(provider: OnchainMspId) -> StorageKey {
    let key = MainStorageProvidersStorageMap::hashed_key_for(provider.as_h256());
    StorageKey(key)
}

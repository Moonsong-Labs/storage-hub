use hash_db::Hasher;
use log::warn;
use shc_common::types::HasherOutT;
use trie_db::TrieLayout;

use crate::{
    error::{ErrorT, ForestStorageError},
    LOG_TARGET,
};

pub(crate) fn convert_raw_bytes_to_hasher_out<T: TrieLayout>(
    key: Vec<u8>,
) -> Result<HasherOutT<T>, ErrorT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    let key: [u8; 32] = key
        .try_into()
        .map_err(|_| ForestStorageError::FailedToParseKey)?;

    let key = HasherOutT::<T>::try_from(key).map_err(|_| {
        warn!(target: LOG_TARGET, "Failed to parse root from DB");
        ForestStorageError::FailedToParseKey
    })?;

    Ok(key)
}

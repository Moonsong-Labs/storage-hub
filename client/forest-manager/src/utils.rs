use common::types::HasherOutT;
use hash_db::Hasher;
use log::warn;
use trie_db::TrieLayout;

use crate::{
    error::{ErrorT, ForestStorageError},
    LOG_TARGET,
};

pub(crate) fn convert_raw_bytes_to_hasher_out<T: TrieLayout>(
    root: Vec<u8>,
) -> Result<HasherOutT<T>, ErrorT<T>>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    let root: [u8; 32] = root
        .try_into()
        .map_err(|_| ForestStorageError::FailedToParseRoot)?;

    let root = HasherOutT::<T>::try_from(root).map_err(|_| {
        warn!(target: LOG_TARGET, "Failed to parse root from DB");
        ForestStorageError::FailedToParseRoot
    })?;

    Ok(root)
}

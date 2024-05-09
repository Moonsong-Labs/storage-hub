use hash_db::Hasher;
use kvdb::DBValue;
use log::warn;
use sp_core::serde::{de::DeserializeOwned, Serialize};
use trie_db::TrieLayout;

use crate::{
    types::{ForestStorageErrors, HasherOutT},
    LOG_TARGET,
};

pub(crate) fn deserialize_value<T: DeserializeOwned>(
    data: &[u8],
) -> Result<T, ForestStorageErrors> {
    bincode::deserialize(data).map_err(|_| ForestStorageErrors::FailedToDeserializeValue)
}

pub(crate) fn serialize_value<T: Serialize>(value: &T) -> Result<DBValue, ForestStorageErrors> {
    bincode::serialize(value).map_err(|_| ForestStorageErrors::FailedToSerializeValue)
}

pub(crate) fn convert_raw_bytes_to_hasher_out<T: TrieLayout>(
    root: Vec<u8>,
) -> Result<<<T as TrieLayout>::Hash as Hasher>::Out, ForestStorageErrors>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    let root: [u8; 32] = root
        .try_into()
        .map_err(|_| ForestStorageErrors::FailedToParseRoot)?;

    HasherOutT::<T>::try_from(root).map_err(|_| {
        warn!(target: LOG_TARGET, "Failed to parse root from DB");
        ForestStorageErrors::FailedToParseRoot
    })
}

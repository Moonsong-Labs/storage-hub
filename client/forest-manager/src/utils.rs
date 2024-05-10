use codec::decode_from_bytes;
use common::types::{HasherOutT, Metadata};
use hash_db::Hasher;
use log::warn;
use trie_db::{Trie, TrieLayout};

use crate::{
    error::{Error, ForestStorageError},
    LOG_TARGET,
};

pub(crate) fn get_and_decode_value<T: TrieLayout>(
    trie: trie_db::TrieDB<T>,
    file_key: &HasherOutT<T>,
) -> Result<Option<Metadata>, Error> {
    let maybe_metadata = trie
        .get(file_key.as_ref())
        .map_err(|e| {
            warn!(target: "trie", "Failed to get file key: {:?}", e);
            ForestStorageError::FailedToGetFileKey
        })?
        .map(|raw_metadata| {
            decode_from_bytes(raw_metadata.into()).map_err(|_| {
                warn!(target: "trie", "Failed to decode metadata");
                ForestStorageError::FailedToDecodeValue
            })
        })
        .transpose()?;
    Ok(maybe_metadata)
}

pub(crate) fn convert_raw_bytes_to_hasher_out<T: TrieLayout>(
    root: Vec<u8>,
) -> Result<<<T as TrieLayout>::Hash as Hasher>::Out, Error>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    let root: [u8; 32] = root
        .try_into()
        .map_err(|_| ForestStorageError::FailedToParseRoot)?;

    HasherOutT::<T>::try_from(root).map_err(|_| {
        warn!(target: LOG_TARGET, "Failed to parse root from DB");
        ForestStorageError::FailedToParseRoot.into()
    })
}

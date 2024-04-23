use sp_core::serde::{de::DeserializeOwned, Serialize};

use crate::types::ForestStorageErrors;

pub(crate) fn deserialize_value<T: DeserializeOwned>(
    data: &[u8],
) -> Result<T, ForestStorageErrors> {
    bincode::deserialize(data).map_err(|_| ForestStorageErrors::FailedToDeserializeValue)
}

pub(crate) fn serialize_value<T: Serialize>(value: &T) -> Result<Vec<u8>, ForestStorageErrors> {
    bincode::serialize(value).map_err(|_| ForestStorageErrors::FailedToSerializeValue)
}

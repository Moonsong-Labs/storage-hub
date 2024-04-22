use sp_core::serde::de::DeserializeOwned;

use crate::types::ForestStorageErrors;

pub(crate) fn deserialize_value<T: DeserializeOwned>(
    data: &[u8],
) -> Result<T, ForestStorageErrors> {
    bincode::deserialize(data).map_err(|_| ForestStorageErrors::FailedToDeserializeValue)
}

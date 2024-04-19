use sp_core::serde::de::DeserializeOwned;

use crate::types::Errors;

pub(crate) fn deserialize_value<T: DeserializeOwned>(data: &[u8]) -> Result<T, Errors> {
    bincode::deserialize(data).map_err(|_| Errors::FailedToDeserializeValue)
}

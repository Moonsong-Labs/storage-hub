use diesel::{
    deserialize::{FromSql, Result as DeserializeResult},
    pg::Pg,
    serialize::{Output, Result as SerializeResult, ToSql},
    sql_types::Text,
    AsExpression, FromSqlRow,
};
use sp_core::H256;
use std::{fmt, io::Write};

/// Wrapper for onchain BSP IDs that automatically handles DB encoding
// TODO(Datahaven): Add `Runtime: StorageEnableRuntime` to be id length agnostic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct OnchainBspId(H256);

impl OnchainBspId {
    /// Create a new OnchainBspId from H256
    pub const fn new(id: H256) -> Self {
        Self(id)
    }

    /// Get as H256 for blockchain calls
    pub fn as_h256(&self) -> &H256 {
        &self.0
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_fixed_bytes()
    }

    /// Convert to H256
    pub fn into_h256(self) -> H256 {
        self.0
    }
}

impl From<H256> for OnchainBspId {
    fn from(id: H256) -> Self {
        Self(id)
    }
}

impl From<OnchainBspId> for H256 {
    fn from(id: OnchainBspId) -> H256 {
        id.0
    }
}

/// TODO: Do not assume account Ids are only 32 bytes long - we must be generic over the runtime
impl TryFrom<String> for OnchainBspId {
    type Error = String;

    fn try_from(id: String) -> Result<Self, Self::Error> {
        let hex_str = id.trim_start_matches("0x");
        let bytes =
            hex::decode(hex_str).map_err(|e| format!("Failed to decode BSP ID from hex: {}", e))?;
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Self(H256(array)))
    }
}

impl fmt::Display for OnchainBspId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl ToSql<Text, Pg> for OnchainBspId {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> SerializeResult {
        let hex_string = format!("{:#x}", self.0);
        out.write_all(hex_string.as_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

// TODO: Add unit tests demonstrating read and write operations
impl FromSql<Text, Pg> for OnchainBspId {
    fn from_sql(bytes: diesel::pg::PgValue) -> DeserializeResult<Self> {
        let hex_string = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        let hex_str = hex_string.trim_start_matches("0x");
        let bytes =
            hex::decode(hex_str).map_err(|e| format!("Failed to decode BSP ID from hex: {}", e))?;

        if bytes.len() != 32 {
            return Err(format!(
                "Invalid BSP ID length: expected 32 bytes, got {}",
                bytes.len()
            )
            .into());
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(OnchainBspId(H256::from(array)))
    }
}

/// Wrapper for onchain MSP IDs that automatically handles DB encoding
// TODO(Datahaven): Add `Runtime: StorageEnableRuntime` to be id length agnostic
#[derive(Debug, Clone, Copy, PartialEq, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct OnchainMspId(H256);

impl OnchainMspId {
    /// Create a new OnchainMspId from H256
    pub const fn new(id: H256) -> Self {
        Self(id)
    }

    /// Get as H256 for blockchain calls
    pub fn as_h256(&self) -> &H256 {
        &self.0
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_fixed_bytes()
    }

    /// Convert to H256
    pub fn into_h256(self) -> H256 {
        self.0
    }
}

impl From<H256> for OnchainMspId {
    fn from(id: H256) -> Self {
        Self(id)
    }
}

impl From<OnchainMspId> for H256 {
    fn from(id: OnchainMspId) -> H256 {
        id.0
    }
}

impl fmt::Display for OnchainMspId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl ToSql<Text, Pg> for OnchainMspId {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> SerializeResult {
        let hex_string = format!("{:#x}", self.0);
        out.write_all(hex_string.as_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

// TODO: Add unit tests demonstrating read and write operations
impl FromSql<Text, Pg> for OnchainMspId {
    fn from_sql(bytes: diesel::pg::PgValue) -> DeserializeResult<Self> {
        let hex_string = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        let hex_str = hex_string.trim_start_matches("0x");
        let bytes =
            hex::decode(hex_str).map_err(|e| format!("Failed to decode MSP ID from hex: {}", e))?;

        if bytes.len() != 32 {
            return Err(format!(
                "Invalid MSP ID length: expected 32 bytes, got {}",
                bytes.len()
            )
            .into());
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(OnchainMspId(H256::from(array)))
    }
}

use chrono::{DateTime, Utc};
use codec::{Decode, Encode};
use serde::Serialize;
use sp_core::{ConstU32, H256};
use sp_runtime::BoundedVec;

#[derive(Debug, Serialize)]
pub struct InfoResponse {
    pub client: String,
    pub version: String,
    #[serde(rename = "mspId")]
    pub msp_id: String,
    pub multiaddresses: Vec<String>,
    #[serde(rename = "ownerAccount")]
    pub owner_account: String,
    #[serde(rename = "paymentAccount")]
    pub payment_account: String,
    pub status: String,
    #[serde(rename = "activeSince")]
    pub active_since: u64,
    pub uptime: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsResponse {
    pub capacity: Capacity,
    #[serde(rename = "activeUsers")]
    pub active_users: u64,
    #[serde(rename = "lastCapacityChange")]
    pub last_capacity_change: String,
    #[serde(rename = "valuePropsAmount")]
    pub value_props_amount: String,
    #[serde(rename = "bucketsAmount")]
    pub buckets_amount: String,
    #[serde(rename = "filesAmount")]
    pub files_amount: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Capacity {
    #[serde(rename = "totalBytes")]
    pub total_bytes: String,
    #[serde(rename = "availableBytes")]
    pub available_bytes: String,
    #[serde(rename = "usedBytes")]
    pub used_bytes: String,
}

// TODO: We should update this to somehow use the types configured in the runtime.
// For now, I hardcoded them to match
#[derive(Debug, Serialize, Encode, Decode, Default)]
pub struct ValueProposition {
    #[serde(rename = "pricePerGbPerBlock")]
    pub price_per_giga_unit_of_data_per_block: u128,
    #[serde(rename = "commitment")]
    pub commitment: BoundedVec<u8, ConstU32<1000>>,
    #[serde(rename = "bucketDataLimit")]
    pub bucket_data_limit: u64,
    #[serde(rename = "available")]
    pub available: bool,
}

#[derive(Debug, Serialize, Encode, Decode, Default)]
pub struct ValuePropositionWithId {
    pub id: H256,
    pub value_prop: ValueProposition,
}

impl ValuePropositionWithId {
    pub fn new(id: H256, value_prop: ValueProposition) -> Self {
        Self { id, value_prop }
    }
}

impl ValueProposition {
    pub fn new(
        price_per_giga_unit_of_data_per_block: u128,
        commitment: BoundedVec<u8, ConstU32<1000>>,
        bucket_data_limit: u64,
        available: bool,
    ) -> Self {
        Self {
            price_per_giga_unit_of_data_per_block,
            commitment,
            bucket_data_limit,
            available,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MspHealthResponse {
    pub status: String,
    pub components: serde_json::Value,
    #[serde(rename = "lastChecked")]
    pub last_checked: DateTime<Utc>,
}

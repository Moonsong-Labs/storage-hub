use serde::Serialize;
use chrono::{DateTime, Utc};

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

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub capacity: Capacity,
    #[serde(rename = "activeUsers")]
    pub active_users: u64,
    #[serde(rename = "lastCapacityChange")]
    pub last_capacity_change: u64,
    #[serde(rename = "valuePropsAmount")]
    pub value_props_amount: u64,
    #[serde(rename = "BucketsAmount")]
    pub buckets_amount: u64,
}

#[derive(Debug, Serialize)]
pub struct Capacity {
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "availableBytes")]
    pub available_bytes: u64,
    #[serde(rename = "usedBytes")]
    pub used_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ValueProp {
    pub id: String,
    #[serde(rename = "pricePerGbBlock")]
    pub price_per_gb_block: f64,
    #[serde(rename = "dataLimitPerBucketBytes")]
    pub data_limit_per_bucket_bytes: u64,
    #[serde(rename = "isAvailable")]
    pub is_available: bool,
}

#[derive(Debug, Serialize)]
pub struct MspHealthResponse {
    pub status: String,
    pub components: serde_json::Value,
    #[serde(rename = "lastChecked")]
    pub last_checked: DateTime<Utc>,
}
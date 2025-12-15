use alloy_core::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceRequest {
    pub address: Address,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    /// Full URI to include in the message (e.g., "https://datahaven.app/testnet")
    /// Domain will be extracted from this URI automatically
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    #[serde(serialize_with = "crate::utils::serde::checksummed_address")]
    pub address: Address,
    pub ens: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub address: Address,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

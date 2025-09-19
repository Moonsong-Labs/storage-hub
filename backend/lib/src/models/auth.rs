use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceRequest {
    pub address: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
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
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub address: String,
    pub ens: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub address: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

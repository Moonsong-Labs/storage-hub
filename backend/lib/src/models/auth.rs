use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NonceRequest {
    pub address: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
}

#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub message: String,
    pub nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub address: String,
    pub exp: i64, // JWT expiration timestamp
    pub iat: i64, // JWT issued at timestamp
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub address: String,
    pub ens: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
}

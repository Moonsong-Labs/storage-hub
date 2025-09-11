use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NonceRequest {
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub message: String,
    pub nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub address: String,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub address: String,
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

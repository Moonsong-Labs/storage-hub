use axum_extra::headers::{authorization::Bearer, Authorization};
use base64::{engine::general_purpose, Engine};
use rand::Rng;
use serde_json::Value;

use crate::error::Error;

pub fn validate_eth_address(address: &str) -> Result<(), Error> {
    if address.starts_with("0x")
        && address.len() == 42
        && address[2..].chars().all(|c| c.is_ascii_hexdigit())
    {
        Ok(())
    } else {
        Err(Error::BadRequest("Invalid Ethereum address".to_string()))
    }
}

pub fn validate_hex_id(id: &str, expected_len: usize) -> Result<(), Error> {
    if id.len() == expected_len && id.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "Invalid hex ID, expected {} characters",
            expected_len
        )))
    }
}

pub fn generate_hex_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len / 2)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect()
}

pub fn generate_mock_jwt() -> String {
    format!(
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}.{}",
        general_purpose::STANDARD.encode("mock_payload"),
        general_purpose::STANDARD.encode("mock_signature")
    )
}

// TODO(MOCK): verify JWT signature
fn validate_token(jtw: Value) -> Result<Value, Error> {
    Ok(jtw)
}

/// Extracts, decodes and verifies the JWT
pub fn extract_bearer_token(auth: &Authorization<Bearer>) -> Result<Value, Error> {
    auth.token()
        .split('.') // JWT is header.payload.signature
        .nth(1)
        .ok_or_else(|| Error::Unauthorized("Invalid token payload format".to_string()))
        .and_then(|payload| {
            use base64::{engine::general_purpose, Engine};
            general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| Error::Unauthorized(format!("Base64 decode error: {}", e)))
        })
        .and_then(|bytes| {
            String::from_utf8(bytes)
                .map_err(|e| Error::Unauthorized(format!("UTF-8 decode error: {}", e)))
        })
        .and_then(|s| {
            serde_json::from_str::<serde_json::Value>(&s)
                .map_err(|e| Error::Unauthorized(format!("JSON decode error: {}", e)))
        })
        .and_then(validate_token)
}

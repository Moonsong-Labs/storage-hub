use axum_extra::headers::{authorization::Bearer, Authorization};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
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
    // Create a proper mock JWT with valid base64url encoding
    // TODO(MOCK): We manually construct the JWT instead of using jsonwebtoken::encode()
    // because encode() requires real cryptographic signing, which we're avoiding for mocks
    // Header: {"alg":"HS256","typ":"JWT"} already encoded
    let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";

    // Create a mock payload with proper structure
    let payload = serde_json::json!({
        // Standard JWT claims
        "sub": "0x1234567890123456789012345678901234567890", // Subject: user's ETH address
        "exp": 9999999999i64, // Expiration: far future for mock
        "iat": 1704067200i64, // Issued at: 2024-01-01

        // TODO(MOCK): Include relevant claim items here as siblings to exp, iat, sub
        // For example:
        // "iss": "storagehub-api",  // Issuer
        // "address": "0x...",       // User's actual Ethereum address
        // "ens": "user.eth",        // ENS domain name
        // "chain_id": 1,            // Network ID
        // "role": "user",           // User permissions
    });

    // Encode payload using base64url (no padding) - proper JWT format
    let payload_json = serde_json::to_string(&payload).unwrap();
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());

    // Mock signature (base64url encoded)
    let signature = URL_SAFE_NO_PAD.encode("mock_signature");

    format!("{}.{}.{}", header, payload_b64, signature)
}

/// Extracts, decodes and verifies the JWT
pub fn extract_bearer_token(auth: &Authorization<Bearer>) -> Result<Value, Error> {
    let token = auth.token();

    // TODO(MOCK): decode with verification
    let mut validation = Validation::new(Algorithm::HS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false; // Don't validate expiry for mocks

    // handles base64url decoding properly
    decode::<Value>(
        token,
        &DecodingKey::from_secret(b"mock_secret"), // TODO(MOCK): use configurable secret
        &validation,
    )
    .map_err(|e| Error::Unauthorized(format!("JWT decode error: {}", e)))?
    .map(|token| token.claims)
}

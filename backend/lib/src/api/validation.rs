use axum::http::HeaderMap;

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
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len / 2)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect()
}

pub fn generate_mock_jwt() -> String {
    use base64::{engine::general_purpose, Engine};
    format!(
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.{}.{}",
        general_purpose::STANDARD.encode("mock_payload"),
        general_purpose::STANDARD.encode("mock_signature")
    )
}

pub fn extract_bearer_token(auth_header: Option<&str>) -> Result<String, Error> {
    match auth_header {
        Some(header) if header.starts_with("Bearer ") => Ok(header[7..].to_string()),
        _ => Err(Error::Unauthorized(
            "Missing or invalid authorization header".to_string(),
        )),
    }
}

pub fn extract_token(headers: &HeaderMap) -> Result<String, Error> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    extract_bearer_token(auth_header)
}

pub fn validate_token(_token: &str) -> Result<(), Error> {
    // Mock validation - in production this would verify JWT
    Ok(())
}

use axum_extra::headers::{authorization::Bearer, Authorization};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use rand::Rng;
use serde_json::Value;

use crate::error::Error;

/// Validates that the passed in ethereum address is:
///
/// * a hex string
/// * is 42 characters long (0x + 20 bytes)
/// * all characters after the first 0x are valid ascii hex digits
//TODO: consider switching to validating with a dedicated library
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

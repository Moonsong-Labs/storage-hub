use axum_jwt::jsonwebtoken::{DecodingKey, EncodingKey};

use crate::{
    api::validation::{generate_mock_jwt, validate_eth_address},
    constants::mocks::MOCK_ADDRESS,
    error::Error,
    models::auth::{NonceResponse, ProfileResponse, TokenResponse, User, VerifyResponse},
};
use alloy_core::primitives::{eip191_hash_message, PrimitiveSignature};
use alloy_signer::utils::public_key_to_address;
use std::str::FromStr;

#[derive(Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    // TODO(MOCK): store nonces and sessions
}

impl AuthService {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
        }
    }

    /// Construct the message that should be signed for authentication
    fn construct_auth_message(nonce: &str) -> String {
        format!("Sign this message to authenticate: {}", nonce)
    }

    pub async fn generate_nonce(&self, address: &str) -> Result<NonceResponse, Error> {
        validate_eth_address(address)?;

        // TODO: Generate a random nonce and store it with expiration
        let nonce = "aBcDeF12345";
        let message = Self::construct_auth_message(nonce);

        Ok(NonceResponse {
            message,
            nonce: nonce.to_string(),
        })
    }

    pub async fn verify_eth_signature(
        &self,
        address: &str,
        nonce: &str,
        signature: &str,
    ) -> Result<VerifyResponse, Error> {
        // Validate the provided address format
        validate_eth_address(address)?;

        // TODO: Check if nonce exists in storage and hasn't expired
        // For now, we only accept the fixed nonce
        if nonce != "aBcDeF12345" {
            return Err(Error::Unauthorized("Invalid or expired nonce".to_string()));
        }

        // Reconstruct the message that should have been signed
        let message = Self::construct_auth_message(nonce);

        // Parse the signature
        let sig = PrimitiveSignature::from_str(signature)
            .map_err(|_| Error::Unauthorized("Invalid signature format".to_string()))?;

        // Hash the message with EIP-191 prefix (this adds "\x19Ethereum Signed Message:\n" + length + message)
        let message_hash = eip191_hash_message(message.as_bytes());

        // Recover the public key from the signature
        let recovered_pubkey = sig.recover_from_prehash(&message_hash).map_err(|_| {
            Error::Unauthorized("Failed to recover public key from signature".to_string())
        })?;

        // Convert the public key to an address
        let recovered_address = public_key_to_address(&recovered_pubkey);

        // Convert recovered address to string format
        let recovered_address_str = format!("{:#x}", recovered_address);

        // Verify that the recovered address matches the claimed address
        if recovered_address_str.to_lowercase() != address.to_lowercase() {
            return Err(Error::Unauthorized(
                "Signature doesn't match the provided address".to_string(),
            ));
        }

        // TODO: Store the session token in database
        // Generate JWT token and return response
        Ok(VerifyResponse {
            token: generate_mock_jwt(),
            user: User {
                address: recovered_address_str,
            },
        })
    }

    pub async fn refresh_token(&self, _old_token: &str) -> Result<TokenResponse, Error> {
        // TODO(MOCK): refresh token
        Ok(TokenResponse {
            token: generate_mock_jwt(),
        })
    }

    pub async fn get_profile(&self, _token: &str) -> Result<ProfileResponse, Error> {
        // TODO(MOCK): retrieve profile from token
        Ok(ProfileResponse {
            address: MOCK_ADDRESS.to_string(),
            ens: "user.eth".to_string(),
        })
    }

    pub async fn logout(&self, _token: &str) -> Result<(), Error> {
        // TODO(MOCK): invalidate the token
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verify_eth_signature() {
        let auth_service = AuthService::default();

        // Test with invalid nonce
        let result = auth_service
            .verify_eth_signature(
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9",
                "invalid_nonce",
                "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Unauthorized(msg)) => {
                assert_eq!(msg, "Invalid or expired nonce");
            }
            _ => panic!("Expected unauthorized error for invalid nonce"),
        }

        // Test with valid nonce but invalid signature format
        let result = auth_service
            .verify_eth_signature(
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9",
                "aBcDeF12345",
                "invalid_signature",
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid signature format"));
            }
            _ => panic!("Expected unauthorized error for invalid signature"),
        }
    }
}

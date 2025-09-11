use std::str::FromStr;

use alloy_core::primitives::{eip191_hash_message, PrimitiveSignature};
use alloy_signer::utils::public_key_to_address;
use axum_jwt::{
    jsonwebtoken::{self, Header, DecodingKey, EncodingKey, TokenData},
    Decoder,
};
use chrono::{Duration, Utc};

use crate::{
    api::validation::validate_eth_address,
    constants::mocks::MOCK_ADDRESS,
    error::Error,
    models::auth::{
        JwtClaims, NonceResponse, ProfileResponse, TokenResponse, User, VerifyResponse,
    },
};

#[derive(Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoder: Decoder,
    // TODO(MOCK): store nonces and sessions
}

impl AuthService {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoder: Decoder::from_key(DecodingKey::from_secret(secret)),
        }
    }

    pub fn jwt_decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// Construct the message that should be signed for authentication
    /// Construct a Sign-In with Ethereum (SIWE) compliant message for authentication
    ///
    /// This follows the EIP-4361 standard for Sign-In with Ethereum messages.
    /// The message format ensures compatibility with wallet signing interfaces
    /// and provides a standardized authentication flow.
    fn construct_auth_message(address: &str, domain: &str, nonce: &str, chain_id: u64) -> String {
        // SIWE message format as per EIP-4361
        let scheme = "https";
        let uri = format!("https://{}/auth/nonce", domain);
        let statement = "I authenticate with this MSP Backend with my address";
        let version = 1;
        let issued_at = chrono::Utc::now().to_rfc3339();

        format!(
            "{}://{} wants you to sign in with your Ethereum account:\n\
            {}\n\
            \n\
            {}\n\
            \n\
            URI: {}\n\
            Version: {}\n\
            Chain ID: {}\n\
            Nonce: {}\n\
            Issued At: {}",
            scheme, domain, address, statement, uri, version, chain_id, nonce, issued_at
        )
    }

    /// Generate a JWT token for the given address
    fn generate_jwt(&self, address: &str) -> Result<String, Error> {
        let now = Utc::now();
        let exp = now + Duration::minutes(10); // TODO: Make this configurable

        let claims = JwtClaims {
            address: address.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| Error::Internal)
    }

    pub async fn generate_nonce(
        &self,
        address: &str,
        chain_id: u64,
    ) -> Result<NonceResponse, Error> {
        // Validate address BEFORE generating message or storing in cache
        validate_eth_address(address)?;

        // TODO: generate randomly
        let nonce = "aBcDeF12345";
        // TODO: Make domain configurable
        let domain = "localhost";
        let message = Self::construct_auth_message(address, domain, nonce, chain_id);

        // TODO: Store message paired with validated address in storage
        // For now, we're using a fixed nonce and message
        // In production, this should be stored in a cache/database with:
        // - key: message
        // - value: { address, expiration_time }
        // The address stored here is guaranteed to be valid due to prior validation

        Ok(NonceResponse {
            message,
            nonce: nonce.to_string(),
        })
    }

    pub async fn verify_eth_signature(
        &self,
        message: &str,
        signature: &str,
    ) -> Result<VerifyResponse, Error> {
        // TODO: Retrieve the stored address for this message from storage
        // For now, we'll extract it from the message (this is temporary)
        // In production, this should:
        // 1. Look up the message in storage
        // 2. Get the associated address
        // 3. Check if the message hasn't expired
        // 4. Remove the message from storage after successful verification

        // Extract address from message (temporary solution)
        let address_line = message
            .lines()
            .nth(1)
            .ok_or_else(|| Error::Unauthorized("Invalid message format".to_string()))?;
        let stored_address = address_line.trim();

        // Validate the stored address format
        validate_eth_address(stored_address)?;

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

        // Verify that the recovered address matches the stored address
        if recovered_address_str.to_lowercase() != stored_address.to_lowercase() {
            return Err(Error::Unauthorized(
                "Signature doesn't match the provided address".to_string(),
            ));
        }

        // Generate JWT token
        let token = self.generate_jwt(&recovered_address_str)?;

        // TODO: Store the session token in database
        // Return response with the generated JWT
        Ok(VerifyResponse {
            token,
            user: User {
                address: recovered_address_str,
            },
        })
    }

    pub async fn refresh_token(
        &self,
        token_data: &TokenData<JwtClaims>,
    ) -> Result<TokenResponse, Error> {
        // Generate new token with refreshed expiry using the address from the validated token
        let new_token = self.generate_jwt(&token_data.claims.address)?;

        Ok(TokenResponse { token: new_token })
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
    use crate::config::Config;

    use super::*;

    #[tokio::test]
    async fn test_verify_eth_signature() {
        let cfg = Config::default();
        let auth_service = AuthService::new(cfg.auth.jwt_secret.as_bytes());

        // Generate a test message
        let test_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9";
        let test_message =
            AuthService::construct_auth_message(test_address, "localhost", "aBcDeF12345", 1);

        // Test with invalid message format
        let result = auth_service
            .verify_eth_signature(
                "Invalid message",
                "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid message format"));
            }
            _ => panic!("Expected unauthorized error for invalid message format"),
        }

        // Test with valid message but invalid signature format
        let result = auth_service
            .verify_eth_signature(&test_message, "invalid_signature")
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

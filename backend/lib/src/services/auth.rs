use std::{str::FromStr, sync::Arc};

use alloy_core::primitives::{eip191_hash_message, PrimitiveSignature};
use alloy_signer::utils::public_key_to_address;
use axum_jwt::{
    jsonwebtoken::{self, DecodingKey, EncodingKey, Header},
    Decoder,
};
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};

use crate::{
    api::validation::validate_eth_address,
    data::storage::BoxedStorage,
    error::Error,
    models::auth::{
        JwtClaims, NonceResponse, ProfileResponse, TokenResponse, User, VerifyResponse,
    },
};

#[derive(Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoder: Decoder,
    validate_signature: bool,
    storage: Arc<dyn BoxedStorage>,
}

impl AuthService {
    /// Crete a new instance of `AuthService` with the configured secret.
    ///
    /// Arguments:
    /// * `secret`: secret to use to initialize the JWT encoding and decoding keys
    /// * `validate_signature`: used to enable Eth and JWT signature validation. Recommended set to true.
    /// * `storage`: reference to the storage service to use to store nonce information
    pub fn new(secret: &[u8], validate_signature: bool, storage: Arc<dyn BoxedStorage>) -> Self {
        let mut validation = jsonwebtoken::Validation::default();
        if !validate_signature {
            validation.insecure_disable_signature_validation();
        }

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoder: Decoder::new(DecodingKey::from_secret(secret), validation),
            storage,
            validate_signature,
        }
    }

    pub fn jwt_decoder(&self) -> &Decoder {
        &self.decoder
    }

    /// Generate a random SIWE-compliant nonce (at least 8 alphanumeric characters)
    fn generate_random_nonce() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16) // 16 characters for better security
            .map(char::from)
            .collect()
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

        // Generate a random SIWE-compliant nonce
        let nonce = Self::generate_random_nonce();
        // TODO: Make domain configurable
        let domain = "localhost";
        let message = Self::construct_auth_message(address, domain, &nonce, chain_id);

        // Store message paired with validated address in storage
        // Using message as key and address as value with 5 minute expiration
        const NONCE_EXPIRATION_SECONDS: u64 = 300; // 5 minutes

        self.storage
            .store_nonce(
                message.clone(),
                address.to_string(),
                NONCE_EXPIRATION_SECONDS,
            )
            .await
            .map_err(|_| Error::Internal)?;

        Ok(NonceResponse { message })
    }

    pub async fn verify_eth_signature(
        &self,
        message: &str,
        signature: &str,
    ) -> Result<VerifyResponse, Error> {
        // Retrieve the stored address for this message from storage
        let address = self
            .storage
            .get_nonce(message)
            .await
            .map(|addr| addr.to_lowercase())
            .map_err(|_| Error::Internal)?
            .ok_or_else(|| Error::Unauthorized("Invalid or expired nonce".to_string()))?;

        // Validate the stored address format (defensive check)
        validate_eth_address(&address)?;

        if self.validate_signature {
            let sig = PrimitiveSignature::from_str(signature)
                .map_err(|_| Error::Unauthorized("Invalid signature format".to_string()))?;

            // Hash the message with EIP-191 prefix
            let message_hash = eip191_hash_message(message.as_bytes());

            // Recover the public key from the signature
            let recovered_pubkey = sig.recover_from_prehash(&message_hash).map_err(|_| {
                Error::Unauthorized("Failed to recover public key from signature".to_string())
            })?;

            let recovered_address = public_key_to_address(&recovered_pubkey)
                .to_string()
                .to_lowercase();

            // Verify that the recovered address matches the stored address
            if recovered_address.as_str() != address.as_str() {
                return Err(Error::Unauthorized(
                    "Signature doesn't match the provided address".to_string(),
                ));
            }
        }

        // Remove the nonce from storage after successful verification (one-time use)
        self.storage
            .remove_nonce(message)
            .await
            .map_err(|_| Error::Internal)?;

        // Generate JWT token
        let token = self.generate_jwt(&address)?;

        // TODO: Store the session token in database
        Ok(VerifyResponse {
            token,
            user: User { address },
        })
    }

    pub async fn refresh_token(&self, claims: JwtClaims) -> Result<TokenResponse, Error> {
        // Generate new token with refreshed expiry using the address from the validated token
        let new_token = self.generate_jwt(&claims.address)?;

        Ok(TokenResponse { token: new_token })
    }

    pub async fn get_profile(&self, claims: JwtClaims) -> Result<ProfileResponse, Error> {
        // TODO: verify claims expiry

        Ok(ProfileResponse {
            address: claims.address,
            // TODO(MOCK): retrieve ens from token
            ens: "user.eth".to_string(),
        })
    }

    pub async fn logout(&self, _token: &str) -> Result<(), Error> {
        // TODO: Invalidate the token in session storage
        // For now, the nonce cleanup happens automatically on expiration
        // or during verification (one-time use)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::Config,
        data::storage::{BoxedStorageWrapper, InMemoryStorage},
    };

    use super::*;

    #[tokio::test]
    async fn test_verify_eth_signature() {
        let cfg = Config::default();
        let storage = Arc::new(BoxedStorageWrapper::new(InMemoryStorage::new()));
        let auth_service = AuthService::new(cfg.auth.jwt_secret.as_bytes(), false, storage.clone());

        // Generate a test message
        let test_address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb9";
        let test_message =
            AuthService::construct_auth_message(test_address, "localhost", "aBcDeF12345", 1);

        // Test with message not in storage (nonce not found)
        let result = auth_service
            .verify_eth_signature(
                "Invalid message",
                "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(Error::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid or expired nonce"));
            }
            _ => panic!("Expected unauthorized error for invalid nonce"),
        }

        // Store the test message with address in storage
        storage
            .store_nonce(test_message.clone(), test_address.to_string(), 300)
            .await
            .unwrap();

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

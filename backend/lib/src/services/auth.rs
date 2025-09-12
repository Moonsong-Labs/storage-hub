use std::{str::FromStr, sync::Arc};

use alloy_core::primitives::{eip191_hash_message, PrimitiveSignature};
use alloy_signer::utils::public_key_to_address;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_jwt::{
    jsonwebtoken::{self, DecodingKey, EncodingKey, Header},
    Claims, Decoder,
};
use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};

use crate::{
    api::validation::validate_eth_address,
    constants::auth::{
        AUTH_NONCE_EXPIRATION_SECONDS, AUTH_SIWE_DOMAIN, JWT_EXPIRY_OFFSET, MOCK_ENS,
    },
    data::storage::BoxedStorage,
    error::Error,
    models::auth::{JwtClaims, NonceResponse, TokenResponse, User, VerifyResponse},
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
        // TODO: make rng configurable (OS's cryptorng / seeded rng for tests)
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
        let scheme = "https";

        // TODO: make uri match endpoint
        let uri = format!("{scheme}://{domain}/auth/nonce");
        let statement = "I authenticate with this MSP Backend with my address";
        let version = 1;
        let issued_at = chrono::Utc::now().to_rfc3339();

        // SIWE message format as per EIP-4361
        format!(
            "{scheme}://{domain} wants you to sign in with your Ethereum account:\n\
            {address}\n\
            \n\
            {statement}\n\
            \n\
            URI: {uri}\n\
            Version: {version}\n\
            Chain ID: {chain_id}\n\
            Nonce: {nonce}\n\
            Issued At: {issued_at}",
        )
    }

    /// Generate a JWT token for the given address
    fn generate_jwt(&self, address: &str) -> Result<String, Error> {
        let now = Utc::now();
        let exp = now + JWT_EXPIRY_OFFSET;

        let claims = JwtClaims {
            address: address.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| Error::Internal)
    }

    /// Generate a SIWE-compliant message for the user to sign
    ///
    /// The message will expire after a given time
    pub async fn challenge(&self, address: &str, chain_id: u64) -> Result<NonceResponse, Error> {
        // Validate address before generating message or storing in cache
        validate_eth_address(address)?;

        let nonce = Self::generate_random_nonce();
        let message = Self::construct_auth_message(address, AUTH_SIWE_DOMAIN, &nonce, chain_id);

        // Store message paired with address in storage
        // Using message as key and address as value
        self.storage
            .store_nonce(
                message.clone(),
                address.to_string(),
                AUTH_NONCE_EXPIRATION_SECONDS,
            )
            .await
            .map_err(|_| Error::Internal)?;

        Ok(NonceResponse { message })
    }

    fn verify_eth_signature(message: &str, signature: &str) -> Result<String, Error> {
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

        Ok(recovered_address)
    }

    /// Generate a JWT token if, and only if, the signature is for the given message.
    ///
    /// The signature should be a valid ETH signature. The message should be the same as the returned value from `generate_nonce`.
    /// The method will fail if `message` has expired
    pub async fn login(&self, message: &str, signature: &str) -> Result<VerifyResponse, Error> {
        // Retrieve the stored address for this message from storage
        let address = self
            .storage
            .get_nonce(message)
            .await
            .map_err(|_| Error::Internal)?
            .ok_or_else(|| Error::Unauthorized("Invalid or expired nonce".to_string()))?
            .to_lowercase();

        if self.validate_signature {
            let recovered_address = Self::verify_eth_signature(message, signature)?;

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

        // Finally, generate JWT token
        let token = self.generate_jwt(&address)?;

        // TODO: Store the session token in database
        Ok(VerifyResponse {
            token,
            user: User {
                address,
                ens: MOCK_ENS.to_string(),
            },
        })
    }

    /// Generate a new JWT token, matching the same address as the valid token passed in
    pub async fn refresh(&self, claims: JwtClaims) -> Result<TokenResponse, Error> {
        let user = AuthenticatedUser::from_claims(claims)?;

        // Since the claims were valid, we can just generate a new token
        let token = self.generate_jwt(&user.address)?;

        Ok(TokenResponse { token })
    }

    /// Retrieve the user profile from the corresponding `JwtClaims`
    pub async fn profile(&self, claims: JwtClaims) -> Result<User, Error> {
        let user = AuthenticatedUser::from_claims(claims)?;

        Ok(User {
            address: user.address,
            ens: MOCK_ENS.to_string(),
        })
    }

    pub async fn logout(&self, _claims: JwtClaims) -> Result<(), Error> {
        // TODO: Invalidate the token in session storage
        // For now, the nonce cleanup happens automatically on expiration
        // or during verification (one-time use)
        Ok(())
    }
}

/// Axum extractor to verify the authenticated user.
///
/// Will error if the JWT token is expired or it is otherwise invalid
pub struct AuthenticatedUser {
    pub address: String,
}

impl AuthenticatedUser {
    /// Verifies the passed in `JwtClaims`
    ///
    /// Returns the user information if the claims are valid and not expired
    pub fn from_claims(claims: JwtClaims) -> Result<Self, Error> {
        let now = Utc::now();
        let exp = DateTime::<Utc>::from_timestamp(claims.exp, 0)
            .ok_or_else(|| Error::Unauthorized(format!("Invalid JWT expiry")))?;

        if now >= exp {
            return Err(Error::Unauthorized(format!("Expired JWT")));
        }

        Ok(AuthenticatedUser {
            address: claims.address,
        })
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    Decoder: FromRef<S>,
    S: Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = Claims::<JwtClaims>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::Unauthorized(format!("Invalid JWT: {e:?}")))?;

        Self::from_claims(claims.0)
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
            .login(
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
        let result = auth_service.login(&test_message, "invalid_signature").await;

        assert!(result.is_err());
        match result {
            Err(Error::Unauthorized(msg)) => {
                assert!(msg.contains("Invalid signature format"));
            }
            _ => panic!("Expected unauthorized error for invalid signature"),
        }
    }
}

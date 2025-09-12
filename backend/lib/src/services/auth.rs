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
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    use crate::{
        config::Config,
        constants::{auth::MOCK_ENS, mocks::MOCK_ADDRESS},
        data::storage::{BoxedStorageWrapper, InMemoryStorage},
        test_utils::auth::{eth_wallet, sign_message},
    };

    use super::*;

    /// Helper to create a test AuthService with configurable signature validation
    fn create_test_auth_service(
        validate_signature: bool,
    ) -> (AuthService, Arc<dyn BoxedStorage>, Config) {
        let cfg = Config::default();
        let storage: Arc<dyn BoxedStorage> =
            Arc::new(BoxedStorageWrapper::new(InMemoryStorage::new()));
        let jwt_secret = cfg
            .auth
            .jwt_secret
            .as_ref()
            .expect("JWT secret should be set in tests");
        let auth_service =
            AuthService::new(jwt_secret.as_bytes(), validate_signature, storage.clone());
        (auth_service, storage, cfg)
    }

    /// Helper to get a DecodingKey from the test config
    fn get_decoding_key(cfg: &Config) -> DecodingKey {
        let jwt_secret = cfg
            .auth
            .jwt_secret
            .as_ref()
            .expect("JWT secret should be set in tests");
        DecodingKey::from_secret(jwt_secret.as_bytes())
    }

    #[test]
    fn construct_auth_message_contains_address() {
        let address = MOCK_ADDRESS;
        let domain = "localhost";
        let nonce = "testNonce123";
        let chain_id = 1;

        let message = AuthService::construct_auth_message(address, domain, nonce, chain_id);

        // Check that message contains the address
        assert!(
            message.contains(address),
            "Message should contain the target address"
        );
        assert!(
            message.contains(domain),
            "Message should contain the domain"
        );
        assert!(message.contains(nonce), "Message should contain the nonce");
        assert!(
            message.contains(&chain_id.to_string()),
            "Message should contain the chain ID"
        );
    }

    #[test]
    fn generate_jwt_creates_valid_token() {
        let (auth_service, _, cfg) = create_test_auth_service(true);

        let address = MOCK_ADDRESS;
        let token = auth_service.generate_jwt(address).unwrap();

        // Try to decode the token
        let decoding_key = get_decoding_key(&cfg);
        let validation = Validation::new(Algorithm::HS256);

        let decoded = decode::<JwtClaims>(&token, &decoding_key, &validation).unwrap();

        assert_eq!(decoded.claims.address, address);
        assert!(decoded.claims.exp > decoded.claims.iat);
    }

    #[tokio::test]
    async fn challenge_rejects_invalid_address() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let invalid_address = "not_an_eth_address";
        let result = auth_service.challenge(invalid_address, 1).await;
        assert!(result.is_err(), "Should reject invalid eth address");
    }

    #[tokio::test]
    async fn challenge_stores_nonce_for_valid_address() {
        let (auth_service, storage, _) = create_test_auth_service(true);

        let result = auth_service.challenge(MOCK_ADDRESS, 1).await.unwrap();

        // Check that message was stored in storage
        let stored_address = storage.get_nonce(&result.message).await.unwrap();
        assert_eq!(stored_address, Some(MOCK_ADDRESS.to_string()));
    }

    #[test]
    fn verify_eth_signature_recovers_correct_address() {
        // Create a test signing key
        let (address, sk) = eth_wallet();

        let message = "Test message for signature verification";
        let sig_str = sign_message(&sk, message);

        // Test with correct signature
        let recovered = AuthService::verify_eth_signature(message, &sig_str).unwrap();
        assert_eq!(recovered, address.to_string().to_lowercase());
    }

    #[test]
    fn verify_eth_signature_rejects_invalid_format() {
        let result = AuthService::verify_eth_signature("test message", "invalid_signature");
        assert!(result.is_err(), "Should reject invalid signature format");
    }

    #[test]
    fn verify_eth_signature_wrong_message_recovers_different_address() {
        let (address, sk) = eth_wallet();
        let sig_str = sign_message(&sk, "original message");

        // Test with wrong message for the signature
        let result = AuthService::verify_eth_signature("different message", &sig_str);
        assert!(
            result.is_ok(),
            "Should recover an address, but it won't match"
        );
        assert_ne!(
            result.unwrap(),
            address.to_string().to_lowercase(),
            "Recovered address should not match"
        );
    }

    #[tokio::test]
    async fn login_fails_without_challenge() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let message = "random message";
        let (_, sk) = eth_wallet();
        let sig_str = sign_message(&sk, message);

        let result = auth_service.login(message, &sig_str).await;
        assert!(result.is_err(), "Should fail if challenge wasn't called");
        match result {
            Err(Error::Unauthorized(msg)) => assert!(msg.contains("Invalid or expired nonce")),
            _ => panic!("Expected unauthorized error for missing nonce"),
        }
    }

    #[tokio::test]
    async fn login_rejects_invalid_signature_when_validation_enabled() {
        let (auth_service, _, _) = create_test_auth_service(true);

        // Get challenge for test address
        let challenge = auth_service.challenge(MOCK_ADDRESS, 1).await.unwrap();

        // Give signature from different address
        let (_, sk) = eth_wallet();
        let wrong_sig_str = sign_message(&sk, &challenge.message);

        let result = auth_service.login(&challenge.message, &wrong_sig_str).await;
        assert!(
            result.is_err(),
            "Should reject with wrong signature when validate_signature is true"
        );
    }

    #[tokio::test]
    async fn login_accepts_invalid_signature_when_validation_disabled() {
        let (auth_service, _, _) = create_test_auth_service(false);

        let challenge_result = auth_service.challenge(MOCK_ADDRESS, 1).await.unwrap();
        let invalid_sig = format!("0x{}", hex::encode(&[0u8; 32]));

        let result = auth_service
            .login(&challenge_result.message, &invalid_sig)
            .await;
        assert!(
            result.is_ok(),
            "Should generate token even with invalid signature when validate_signature is false"
        );
    }

    #[tokio::test]
    async fn login_prevents_replay_attacks() {
        let (auth_service, _, _) = create_test_auth_service(true);

        // Create a test signing key
        let (address, sk) = eth_wallet();

        // Get challenge and sign it
        let challenge = auth_service.challenge(&address, 1).await.unwrap();
        let sig_str = sign_message(&sk, &challenge.message);

        // First login should succeed
        let first_login = auth_service.login(&challenge.message, &sig_str).await;
        assert!(first_login.is_ok(), "First login should succeed");

        // Second login with same message should fail
        let second_login = auth_service.login(&challenge.message, &sig_str).await;
        assert!(
            second_login.is_err(),
            "Second login with same message should fail"
        );
    }

    #[tokio::test]
    async fn refresh_rejects_expired_claims() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let expired_claims = JwtClaims {
            address: MOCK_ADDRESS.to_string(),
            exp: Utc::now().timestamp() - 3600, // 1 hour ago
            iat: Utc::now().timestamp() - 7200, // 2 hours ago
        };

        let result = auth_service.refresh(expired_claims).await;
        assert!(result.is_err(), "Should reject expired claims");
    }

    #[tokio::test]
    async fn refresh_generates_new_token_with_updated_timestamps() {
        let (auth_service, _, cfg) = create_test_auth_service(true);

        let valid_claims = JwtClaims {
            address: MOCK_ADDRESS.to_string(),
            exp: Utc::now().timestamp() + 3600, // 1 hour from now
            iat: Utc::now().timestamp(),
        };

        let result = auth_service.refresh(valid_claims.clone()).await.unwrap();

        // Decode and verify the new token
        let decoding_key = get_decoding_key(&cfg);
        let validation = Validation::new(Algorithm::HS256);
        let decoded = decode::<JwtClaims>(&result.token, &decoding_key, &validation).unwrap();

        assert_eq!(decoded.claims.address, valid_claims.address);
        assert!(
            decoded.claims.iat >= valid_claims.iat,
            "New token should have newer or equal iat"
        );
        assert!(
            decoded.claims.exp > valid_claims.iat,
            "New token should have exp after original iat"
        );
    }

    #[tokio::test]
    async fn profile_rejects_expired_claims() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let expired_claims = JwtClaims {
            address: MOCK_ADDRESS.to_string(),
            exp: Utc::now().timestamp() - 3600, // 1 hour ago
            iat: Utc::now().timestamp() - 7200, // 2 hours ago
        };

        let result = auth_service.profile(expired_claims).await;
        assert!(result.is_err(), "Should reject expired claims");
    }

    #[tokio::test]
    async fn profile_returns_user_data_for_valid_claims() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let address = MOCK_ADDRESS;
        let valid_claims = JwtClaims {
            address: address.to_string(),
            exp: Utc::now().timestamp() + 3600, // 1 hour from now
            iat: Utc::now().timestamp(),
        };

        let result = auth_service.profile(valid_claims).await.unwrap();
        assert_eq!(result.address, address, "Should return address from claims");
        assert_eq!(result.ens, MOCK_ENS, "Should return mock ENS");
    }
}

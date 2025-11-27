use std::{str::FromStr, sync::Arc};

use alloy_core::primitives::{eip191_hash_message, Address, PrimitiveSignature};
use alloy_signer::utils::public_key_to_address;
use axum_jwt::jsonwebtoken::{self, DecodingKey, EncodingKey, Header, Validation};
use chrono::{Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use tracing::{debug, error};

use crate::{
    config::AuthConfig,
    constants::auth::MOCK_ENS,
    data::storage::{BoxedStorage, WithExpiry},
    error::Error,
    models::auth::{JwtClaims, NonceResponse, TokenResponse, UserProfile, VerifyResponse},
};

/// Implements Axum extractors for authentication
mod axum;
pub use axum::*;

/// Authentication service for the backend
///
/// The intended authentication flow is as follows:
/// * User retrieves a nonce using [`AuthService::challenge`]
/// * User constructs an Eth personal message signature for the resulting message
/// * User submits the signature using [`AuthService::login`]
/// * If verification succeeds, the User obtains JWT representing the User's address which they can then use to authenticate with this service
#[derive(Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    validate_signature: bool,
    storage: Arc<dyn BoxedStorage>,

    /// The duration for generated JWTs
    session_duration: Duration,
    /// The duration for the stored nonces
    nonce_duration: Duration,
    /// The SIWE domain to use when generating messages
    siwe_domain: String,
}

impl AuthService {
    /// Create an instance of `AuthService` from the passed in `config`.
    ///
    /// Requires an existing `storage` instance
    pub fn from_config(config: &AuthConfig, storage: Arc<dyn BoxedStorage>) -> Self {
        let secret = config
            .jwt_secret
            .as_ref()
            .ok_or_else(|| {
                error!(target: "auth_service::from_config", "JWT_SECRET is not set. Please set it in the config file or as an environment variable.");
                "JWT_SECRET is not configured"
            })
            .and_then(|secret| {
                hex::decode(secret.trim_start_matches("0x"))
                    .map_err(|e| {
                        error!(target: "auth_service::from_config", error = %e, "Invalid JWT_SECRET format - must be a valid hex string");
                        "Invalid JWT_SECRET format"
                    })
            })
            .and_then(|decoded| {
                if decoded.len() < 32 {
                    error!(target: "auth_service::from_config", length = decoded.len(), "JWT_SECRET is too short - must be at least 32 bytes (64 hex characters)");
                    Err("JWT_SECRET must be at least 32 bytes")
                } else {
                    Ok(decoded)
                }
            })
            .expect("JWT secret configuration should be valid");

        let session_duration = Duration::minutes(config.session_expiration_minutes as _);
        let nonce_duration = Duration::seconds(config.nonce_expiration_seconds as _);

        // `Validation` is used by the underlying lib to determine how to decode
        // the JWT passed in
        let validation = Validation::default();

        #[cfg_attr(not(feature = "mocks"), allow(unused_mut))]
        let mut this = Self {
            encoding_key: EncodingKey::from_secret(secret.as_slice()),
            decoding_key: DecodingKey::from_secret(secret.as_slice()),
            validation,
            storage,
            validate_signature: true,
            session_duration,
            nonce_duration,
            siwe_domain: config.siwe_domain.clone(),
        };

        #[cfg(feature = "mocks")]
        {
            if config.mock_mode {
                this.insecure_disable_validation();
            }
        }

        this
    }

    /// Returns the configured JWT decoding key
    pub(crate) fn jwt_decoding_key(&self) -> &DecodingKey {
        debug!(target: "auth_service::jwt_decoding_key", "Returning JWT decoding key");

        &self.decoding_key
    }

    /// Returns the configured JWT validation parameter
    pub(crate) fn jwt_validation(&self) -> &Validation {
        debug!(target: "auth_service::jwt_validation", "Returning JWT validation parameter");

        &self.validation
    }

    /// Generate a random SIWE-compliant nonce (at least 8 alphanumeric characters)
    fn generate_random_nonce() -> String {
        debug!(target: "auth_service::generate_random_nonce", "Generating random nonce");

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
    fn construct_auth_message(
        address: &Address,
        domain: &str,
        nonce: &str,
        chain_id: u64,
        uri: &str,
    ) -> String {
        debug!(target: "auth_service::construct_auth_message", address = %address, domain = %domain, nonce = %nonce, chain_id = chain_id, "Constructing auth message");

        let statement = "I authenticate to this MSP Backend with my address";
        let version = 1;
        let issued_at = chrono::Utc::now().to_rfc3339();

        // SIWE message format as per EIP-4361
        format!(
            "{domain} wants you to sign in with your Ethereum account:\n\
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

    /// Generate a JWT for the given address
    ///
    /// The resulting JWT is already base64 encoded and signed by the service
    fn generate_jwt(&self, address: &Address) -> Result<String, Error> {
        debug!(target: "auth_service::generate_jwt", address = %address, "Generating JWT");

        let now = Utc::now();
        let exp = now + self.session_duration;

        let claims = JwtClaims {
            address: *address,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        // encodes the given claims and also produces a signature of it
        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| Error::Internal)
    }

    /// Generate a SIWE-compliant message for the user to sign
    ///
    /// The message will expire after a given time
    pub async fn challenge(
        &self,
        address: &Address,
        chain_id: u64,
        domain: &str,
        uri: &str,
    ) -> Result<NonceResponse, Error> {
        debug!(target: "auth_service::challenge", address = %address, chain_id = chain_id, "Generating challenge");

        let nonce = Self::generate_random_nonce();

        let message = Self::construct_auth_message(address, domain, &nonce, chain_id, uri);

        // Store message paired with address in storage
        // Using message as key and address as value
        self.storage
            .store_nonce(
                message.clone(),
                address,
                self.nonce_duration.num_seconds() as _,
            )
            .await
            .map_err(|_| Error::Internal)?;

        debug!(address = %address, "Generated auth challenge");
        Ok(NonceResponse { message })
    }

    /// Recovers the ethereum address that signed the EIP191 `message` and produced `signature`
    fn recover_eth_address_from_sig(message: &str, signature: &str) -> Result<Address, Error> {
        debug!(target: "auth_service::recover_eth_address_from_sig", message_len = message.len(), signature_len = signature.len(), "Recovering Ethereum address from signature");

        let sig = PrimitiveSignature::from_str(signature)
            .map_err(|_| Error::Unauthorized("Invalid signature format".to_string()))?;

        // Hash the message with EIP-191 prefix
        let message_hash = eip191_hash_message(message.as_bytes());

        // Recover the public key from the signature
        let recovered_pubkey = sig.recover_from_prehash(&message_hash).map_err(|e| {
            Error::Unauthorized(format!("Failed to recover public key from signature: {e}"))
        })?;

        let recovered_address = public_key_to_address(&recovered_pubkey);

        Ok(recovered_address)
    }

    /// Generate a JWT token if, and only if, the signature is for the given message.
    ///
    /// The signature should be a valid ETH signature. The message should be the same as the returned value from `generate_nonce`.
    /// The method will fail if `message` has expired
    pub async fn login(&self, message: &str, signature: &str) -> Result<VerifyResponse, Error> {
        debug!(target: "auth_service::login", message_len = message.len(), signature_len = signature.len(), "Logging in");

        // Retrieve (and remove) the stored address for this message from storage
        let address = self
            .storage
            .get_nonce(message)
            .await
            .map_err(|_| Error::Internal)
            .and_then(|entry| match entry {
                WithExpiry::Valid(address) => Ok(address),
                WithExpiry::Expired => Err(Error::Unauthorized("Expired nonce".to_string())),
                WithExpiry::NotFound => Err(Error::Unauthorized("Invalid nonce".to_string())),
            })?;

        if self.validate_signature {
            let recovered_address = Self::recover_eth_address_from_sig(message, signature)?;

            // Verify that the recovered address matches the stored address
            // NOTE: address comparison relies on the underlying library
            if recovered_address != address {
                // since verification failed, reinsert nonce
                self.storage
                    .store_nonce(
                        message.to_string(),
                        &address,
                        self.nonce_duration.num_seconds() as _,
                    )
                    .await
                    .map_err(|_| Error::Internal)?;

                return Err(Error::Unauthorized(
                    format!("Signature doesn't match the provided address: {recovered_address} != {address}"),
                ));
            }
        }

        // Finally, generate JWT token
        let token = self.generate_jwt(&address)?;

        debug!(address = %address, "Successful login");

        // TODO: Store the session token in database
        // to allow users to logout (invalidate their session)
        Ok(VerifyResponse {
            token,
            user: UserProfile {
                address,
                ens: MOCK_ENS.to_string(),
            },
        })
    }

    /// Generate a new JWT token, matching the same address as the valid token passed in
    // TODO: properly separate between the session and the refresh token
    pub async fn refresh(&self, user_address: &Address) -> Result<TokenResponse, Error> {
        debug!(target: "auth_service::refresh", address = %user_address, "Refreshing token");

        let token = self.generate_jwt(user_address)?;

        debug!(address = %user_address, "Token refreshed");
        Ok(TokenResponse { token })
    }

    /// Retrieve the user profile from the corresponding `JwtClaims`
    pub async fn profile(&self, user_address: &Address) -> Result<UserProfile, Error> {
        debug!(target: "auth_service::profile", address = %user_address, "Profile requested");

        Ok(UserProfile {
            address: *user_address,
            // TODO: retrieve ENS (lookup or cache?)
            ens: MOCK_ENS.to_string(),
        })
    }

    pub async fn logout(&self, user_address: &Address) -> Result<(), Error> {
        debug!(address = %user_address, "User logged out");
        // TODO: Invalidate the token in session storage
        // For now, the nonce cleanup happens automatically on expiration
        // or during verification (one-time use)
        Ok(())
    }
}

#[cfg(any(test, feature = "mocks"))]
impl AuthService {
    /// Disables Eth and JWT signature validation checks
    ///
    /// Will also enable fallaback authentication to [`MOCK_ADDRESS`]
    pub fn insecure_disable_validation(&mut self) {
        self.validation.insecure_disable_signature_validation();
        self.validate_signature = false;
    }

    /// Enables Eth and JWT signature validation checks
    ///
    /// Opposite operation of [`insecure_disable_validation`].
    pub fn enable_validation(&mut self) {
        self.validation = Validation::default();
        self.validate_signature = true;
    }
}

#[cfg(test)]
impl AuthService {
    /// Encodes the given claims as a JWT
    ///
    /// Used in tests that simulate token expiration
    pub fn encode_jwt(&self, claims: JwtClaims) -> Result<String, Error> {
        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| Error::Internal)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jsonwebtoken::{decode, Algorithm, Validation};

    use crate::{
        config::Config,
        constants::{
            auth::{DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS, MOCK_ENS},
            mocks::MOCK_ADDRESS,
        },
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

        let mut auth_service = AuthService::from_config(&cfg.auth, storage.clone());

        if !validate_signature {
            auth_service.insecure_disable_validation();
        } else {
            auth_service.enable_validation();
        }

        (auth_service, storage, cfg)
    }

    #[test]
    fn construct_auth_message_contains_address() {
        let address = MOCK_ADDRESS;
        let domain = "localhost";
        let nonce = "testNonce123";
        let chain_id = 1;

        let message = AuthService::construct_auth_message(&address, domain, nonce, chain_id);

        // Check that message contains the address
        assert!(
            message.contains(&address.to_string()),
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

    #[tokio::test]
    async fn generate_jwt_creates_valid_token() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let address = MOCK_ADDRESS;
        let token = auth_service.generate_jwt(&address).unwrap();

        // Try to decode the token
        let decoding_key = auth_service.jwt_decoding_key();
        let validation = Validation::new(Algorithm::HS256); // ensure we validate

        let decoded = decode::<JwtClaims>(&token, &decoding_key, &validation).unwrap();

        assert_eq!(decoded.claims.address, address);
        assert!(decoded.claims.exp > decoded.claims.iat);
    }

    #[tokio::test]
    async fn challenge_stores_nonce_for_valid_address() {
        let (auth_service, storage, _) = create_test_auth_service(true);

        let result = auth_service.challenge(&MOCK_ADDRESS, 1).await.unwrap();

        // Check that message was stored in storage
        let stored_address = storage.get_nonce(&result.message).await.unwrap();
        assert_eq!(stored_address, WithExpiry::Valid(MOCK_ADDRESS));
    }

    #[test]
    fn recovers_correct_eth_address() {
        // Create a test signing key
        let (address, sk) = eth_wallet();

        let message = "Test message for signature verification";
        let sig_str = sign_message(&sk, message);

        // Test with correct signature
        let recovered = AuthService::recover_eth_address_from_sig(message, &sig_str).unwrap();
        assert_eq!(recovered, address, "Should recover correct address");
    }

    #[test]
    fn recover_eth_address_from_sig_rejects_invalid_format() {
        let result = AuthService::recover_eth_address_from_sig("test message", "invalid_signature");
        assert!(result.is_err(), "Should reject invalid signature format");
    }

    #[test]
    fn recover_eth_address_from_sig_wrong_message_recovers_different_address() {
        let (address, sk) = eth_wallet();
        let sig_str = sign_message(&sk, "original message");

        // Test with wrong message for the signature
        let result = AuthService::recover_eth_address_from_sig("different message", &sig_str);
        assert!(
            result.is_ok(),
            "Should recover an address, but it won't match"
        );
        assert_ne!(
            result.unwrap(),
            address,
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
            Err(Error::Unauthorized(msg)) => assert!(msg.contains("Invalid nonce")),
            _ => panic!("Expected unauthorized error for missing nonce"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn login_fails_after_expiry() {
        let (auth_service, _, _) = create_test_auth_service(true);
        let (address, sk) = eth_wallet();

        let challenge = auth_service.challenge(&address, 1).await.unwrap();
        let sig_str = sign_message(&sk, &challenge.message);

        // Advance time to expire the nonce
        tokio::time::advance(Duration::from_secs(
            DEFAULT_AUTH_NONCE_EXPIRATION_SECONDS as u64 + 1,
        ))
        .await;

        let result = auth_service.login(&challenge.message, &sig_str).await;
        assert!(result.is_err(), "Should fail if nonce has expired");
        match result {
            Err(Error::Unauthorized(msg)) => assert!(msg.contains("Expired nonce")),
            _ => panic!("Expected unauthorized error for expired nonce"),
        }
    }

    #[tokio::test]
    async fn login_rejects_invalid_signature_when_validation_enabled() {
        let (auth_service, _, _) = create_test_auth_service(true);

        // Get challenge for test address
        let challenge = auth_service.challenge(&MOCK_ADDRESS, 1).await.unwrap();

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

        let challenge_result = auth_service.challenge(&MOCK_ADDRESS, 1).await.unwrap();
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
    async fn profile_returns_user_data_for_valid_claims() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let address = MOCK_ADDRESS;

        let result = auth_service.profile(&address).await.unwrap();
        assert_eq!(result.address, address, "Should return address from claims");
        assert_eq!(result.ens, MOCK_ENS, "Should return mock ENS");
    }
}

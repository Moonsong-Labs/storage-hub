use std::{str::FromStr, sync::Arc};

use alloy_core::primitives::{eip191_hash_message, PrimitiveSignature};
use alloy_signer::utils::public_key_to_address;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_jwt::{
    jsonwebtoken::{self, DecodingKey, EncodingKey, Header, Validation},
    Claims, Decoder,
};
use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};
use tracing::{debug, warn};

use crate::{
    api::validation::validate_eth_address,
    constants::{
        auth::{
            AUTH_NONCE_ENDPOINT, AUTH_NONCE_EXPIRATION_SECONDS, AUTH_SIWE_DOMAIN,
            JWT_EXPIRY_OFFSET, MOCK_ENS,
        },
        mocks::MOCK_ADDRESS,
    },
    data::storage::{BoxedStorage, WithExpiry},
    error::Error,
    models::auth::{JwtClaims, NonceResponse, TokenResponse, User, VerifyResponse},
    services::Services,
};

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
}

impl AuthService {
    /// Crete a new instance of `AuthService` with the configured secret.
    ///
    /// Arguments:
    /// * `secret`: secret to use to initialize the JWT encoding and decoding keys
    /// * `storage`: reference to the storage service to use to store nonce information
    pub fn new(secret: &[u8], storage: Arc<dyn BoxedStorage>) -> Self {
        // `Validation` is used by the underlying lib to determine how to decode
        // the JWT passed in
        let validation = Validation::default();

        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            storage,
            validate_signature: true,
        }
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
    fn construct_auth_message(address: &str, domain: &str, nonce: &str, chain_id: u64) -> String {
        debug!(target: "auth_service::construct_auth_message", address = %address, domain = %domain, nonce = %nonce, chain_id = chain_id, "Constructing auth message");

        let scheme = "https";

        // TODO: make uri match endpoint
        let uri = format!("{scheme}://{domain}{AUTH_NONCE_ENDPOINT}");
        let statement = "I authenticate to this MSP Backend with my address";
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

    /// Generate a JWT for the given address
    ///
    /// The resulting JWT is already base64 encoded and signed by the service
    fn generate_jwt(&self, address: &str) -> Result<String, Error> {
        debug!(target: "auth_service::generate_jwt", address = %address, "Generating JWT");

        let now = Utc::now();
        let exp = now + JWT_EXPIRY_OFFSET;

        let claims = JwtClaims {
            address: address.to_string(),
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
    pub async fn challenge(&self, address: &str, chain_id: u64) -> Result<NonceResponse, Error> {
        debug!(target: "auth_service::challenge", address = %address, chain_id = chain_id, "Generating challenge");

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

        debug!(address = %address, "Generated auth challenge");
        Ok(NonceResponse { message })
    }

    /// Recovers the ethereum address that signed the EIP191 `message` and produced `signature`
    fn recover_eth_address_from_sig(message: &str, signature: &str) -> Result<String, Error> {
        debug!(target: "auth_service::recover_eth_address_from_sig", message_len = message.len(), signature_len = signature.len(), "Recovering Ethereum address from signature");

        let sig = PrimitiveSignature::from_str(signature)
            .map_err(|_| Error::Unauthorized("Invalid signature format".to_string()))?;

        // Hash the message with EIP-191 prefix
        let message_hash = eip191_hash_message(message.as_bytes());

        // Recover the public key from the signature
        let recovered_pubkey = sig.recover_from_prehash(&message_hash).map_err(|e| {
            Error::Unauthorized(format!("Failed to recover public key from signature: {e}"))
        })?;

        // NOTE: we avoid lowercasing the address and instead use the canonical encoding
        let recovered_address = public_key_to_address(&recovered_pubkey).to_string();

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
            // NOTE: we compare the lowercase versions to avoid issues where the given user address is not
            // in the right casing, but would otherwise be the correct address.
            if recovered_address.as_str().to_lowercase() != address.as_str().to_lowercase() {
                // since verification failed, reinsert nonce
                self.storage
                    .store_nonce(
                        message.to_string(),
                        address.clone(),
                        AUTH_NONCE_EXPIRATION_SECONDS,
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
            user: User {
                address,
                ens: MOCK_ENS.to_string(),
            },
        })
    }

    /// Generate a new JWT token, matching the same address as the valid token passed in
    // TODO: properly separate between the session and the refresh token
    pub async fn refresh(&self, user_address: &str) -> Result<TokenResponse, Error> {
        debug!(target: "auth_service::refresh", address = %user_address, "Refreshing token");

        let token = self.generate_jwt(user_address)?;

        debug!(address = %user_address, "Token refreshed");
        Ok(TokenResponse { token })
    }

    /// Retrieve the user profile from the corresponding `JwtClaims`
    pub async fn profile(&self, user_address: &str) -> Result<User, Error> {
        debug!(target: "auth_service::profile", address = %user_address, "Profile requested");

        Ok(User {
            address: user_address.to_string(),
            // TODO: retrieve ENS (lookup or cache?)
            ens: MOCK_ENS.to_string(),
        })
    }

    pub async fn logout(&self, user_address: &str) -> Result<(), Error> {
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
    // TODO: user logout verification
    pub fn from_claims(claims: &JwtClaims) -> Result<Self, Error> {
        let now = Utc::now();
        let exp = DateTime::<Utc>::from_timestamp(claims.exp, 0)
            .ok_or_else(|| Error::Unauthorized("Invalid JWT expiry".to_string()))?;
        let iat = DateTime::<Utc>::from_timestamp(claims.iat, 0)
            .ok_or_else(|| Error::Unauthorized("Invalid JWT issuance time".to_string()))?;

        if now >= exp {
            return Err(Error::Unauthorized("Expired JWT".to_string()));
        }

        if iat > now {
            return Err(Error::Unauthorized("JWT issued in the future".to_string()));
        }

        Ok(AuthenticatedUser {
            address: claims.address.clone(),
        })
    }

    async fn from_request_parts_impl<S>(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, (Option<JwtClaims>, Error)>
    where
        S: Send + Sync,
        Decoder: FromRef<S>,
    {
        let claims = Claims::<JwtClaims>::from_request_parts(parts, state)
            .await
            .map_err(|e| (None, Error::Unauthorized(format!("Invalid JWT: {e:?}"))))?;

        Self::from_claims(&claims.0).map_err(|e| (Some(claims.0), e))
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    Decoder: FromRef<S>,
    Services: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let services = Services::from_ref(state);
        let maybe_auth = AuthenticatedUser::from_request_parts_impl(parts, state).await;

        match maybe_auth {
            Ok(ok) => Ok(ok),
            // if services are configured to not validate signature
            Err((claims, e)) if !services.auth.validate_signature => {
                warn!(target: "auth_service::from_request_parts", error = ?e, "Authentication failed");

                // if we were able to retrieve the claims then use the passed in address
                let address = claims
                    .map(|claims| claims.address)
                    .unwrap_or_else(|| MOCK_ADDRESS.to_string());
                debug!(target: "auth_service::from_request_parts", address = %address, "Bypassing authentication");

                return Ok(AuthenticatedUser { address });
            }
            Err((_, e)) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{decode, Algorithm, Validation};

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
        let mut auth_service = AuthService::new(jwt_secret.as_bytes(), storage.clone());

        if !validate_signature {
            auth_service.insecure_disable_validation();
        }

        (auth_service, storage, cfg)
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

    #[tokio::test]
    async fn generate_jwt_creates_valid_token() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let address = MOCK_ADDRESS;
        let token = auth_service.generate_jwt(address).unwrap();

        // Try to decode the token
        let decoding_key = auth_service.jwt_decoding_key();
        let validation = Validation::new(Algorithm::HS256); // ensure we validate

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
        assert_eq!(stored_address, WithExpiry::Valid(MOCK_ADDRESS.to_string()));
    }

    #[test]
    fn recovers_correct_eth_address() {
        // Create a test signing key
        let (address, sk) = eth_wallet();

        let message = "Test message for signature verification";
        let sig_str = sign_message(&sk, message);

        // Test with correct signature
        let recovered = AuthService::recover_eth_address_from_sig(message, &sig_str).unwrap();
        assert_eq!(recovered, address.to_string());
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
            address.to_string(),
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
    async fn profile_returns_user_data_for_valid_claims() {
        let (auth_service, _, _) = create_test_auth_service(true);

        let address = MOCK_ADDRESS;

        let result = auth_service.profile(address).await.unwrap();
        assert_eq!(result.address, address, "Should return address from claims");
        assert_eq!(result.ens, MOCK_ENS, "Should return mock ENS");
    }
}

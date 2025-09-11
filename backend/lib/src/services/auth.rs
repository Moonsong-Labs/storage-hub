use axum_jwt::jsonwebtoken::{DecodingKey, EncodingKey};

use crate::{
    api::validation::{generate_mock_jwt, validate_eth_address},
    constants::mocks::MOCK_ADDRESS,
    error::Error,
    models::auth::{NonceResponse, ProfileResponse, TokenResponse, User, VerifyResponse},
};

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

    pub async fn generate_nonce(
        &self,
        address: &str,
        chain_id: u64,
    ) -> Result<NonceResponse, Error> {
        validate_eth_address(address)?;

        Ok(NonceResponse {
            message: format!(
                "example.com wants you to sign in with your Ethereum account:\n{}\n\n\
                Sign in to access your account.\n\n\
                URI: https://example.com\n\
                Version: 1\n\
                Chain ID: {}\n\
                Nonce: aBcDeF12345\n\
                Issued At: 2024-01-01T00:00:00Z",
                address, chain_id
            ),
            nonce: "aBcDeF12345".to_string(),
        })
    }

    pub async fn verify_eth_signature(
        &self,
        _message: &str,
        signature: &str,
    ) -> Result<VerifyResponse, Error> {
        // TODO(MOCK): check that the address matches the signature
        if !signature.starts_with("0x") || signature.len() != 132 {
            return Err(Error::Unauthorized("Invalid signature".to_string()));
        }

        Ok(VerifyResponse {
            token: generate_mock_jwt(),
            user: User {
                address: MOCK_ADDRESS.to_string(),
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

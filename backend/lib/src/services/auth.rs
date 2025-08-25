use crate::{
    api::validation::{generate_mock_jwt, validate_eth_address},
    error::Error,
    models::*,
};

#[derive(Clone, Default)]
pub struct AuthService {
    // In real implementation, would store nonces and sessions
}

impl AuthService {
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

    pub async fn verify_signature(
        &self,
        _message: &str,
        signature: &str,
    ) -> Result<VerifyResponse, Error> {
        if !signature.starts_with("0x") || signature.len() != 132 {
            return Err(Error::Unauthorized("Invalid signature".to_string()));
        }

        Ok(VerifyResponse {
            token: generate_mock_jwt(),
            user: User {
                address: "0x1234567890123456789012345678901234567890".to_string(),
            },
        })
    }

    pub async fn refresh_token(&self, _old_token: &str) -> Result<TokenResponse, Error> {
        Ok(TokenResponse {
            token: generate_mock_jwt(),
        })
    }

    pub async fn get_profile(&self, _token: &str) -> Result<ProfileResponse, Error> {
        Ok(ProfileResponse {
            address: "0x1234567890123456789012345678901234567890".to_string(),
            ens: "user.eth".to_string(),
        })
    }

    pub async fn logout(&self, _token: &str) -> Result<(), Error> {
        // In real implementation, would invalidate the token
        Ok(())
    }
}

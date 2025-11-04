use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::debug;

use crate::{
    error::Error,
    models::auth::{NonceRequest, VerifyRequest},
    services::{auth::AuthenticatedUser, Services},
};

pub async fn nonce(
    State(services): State<Services>,
    Json(payload): Json<NonceRequest>,
) -> Result<impl IntoResponse, Error> {
    debug!(address = %payload.address, chain_id = payload.chain_id, "POST auth nonce");
    let response = services
        .auth
        .challenge(&payload.address, payload.chain_id)
        .await?;
    Ok(Json(response))
}

pub async fn verify(
    State(services): State<Services>,
    Json(payload): Json<VerifyRequest>,
) -> Result<impl IntoResponse, Error> {
    debug!("POST auth verify");
    let response = services
        .auth
        .login(&payload.message, &payload.signature)
        .await?;
    Ok(Json(response))
}

pub async fn refresh(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    debug!(user = %address, "POST auth refresh");
    let response = services.auth.refresh(&address).await?;
    Ok(Json(response))
}

pub async fn logout(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    debug!(user = %address, "POST auth logout");
    services.auth.logout(&address).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn profile(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    debug!(user = %address, "GET auth profile");
    let response = services.auth.profile(&address).await?;
    Ok(Json(response))
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use chrono::Utc;
    use jsonwebtoken::decode;

    use crate::{
        api::{create_app, mock_app},
        config::Config,
        constants::{
            auth::{AUTH_NONCE_ENDPOINT, MOCK_ENS},
            mocks::MOCK_ADDRESS,
        },
        models::auth::{
            JwtClaims, NonceRequest, NonceResponse, TokenResponse, UserProfile, VerifyRequest,
            VerifyResponse,
        },
        services::Services,
        test_utils::auth::{eth_wallet, sign_message},
    };

    #[tokio::test]
    async fn auth_flow_complete() {
        let mut cfg = Config::default();
        cfg.auth.mock_mode = false;

        let services = Services::mocks_with_config(cfg).await;
        let app = create_app(services.clone());
        let server = TestServer::new(app).unwrap();

        let (address, signing_key) = eth_wallet();

        // Step 1: Get nonce challenge
        let nonce_request = NonceRequest {
            address,
            chain_id: 1,
        };

        let response = server.post(AUTH_NONCE_ENDPOINT).json(&nonce_request).await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let nonce_response: NonceResponse = response.json();
        assert!(nonce_response.message.contains(&address.to_string()));

        // Step 2: Sign the message and login
        let signature = sign_message(&signing_key, &nonce_response.message);
        let verify_request = VerifyRequest {
            message: nonce_response.message,
            signature,
        };

        let response = server.post("/auth/verify").json(&verify_request).await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let verify_response: VerifyResponse = response.json();
        assert_eq!(verify_response.user.address, address);
        assert!(!verify_response.token.is_empty());

        // Decode and verify the JWT
        let jwt_key = services.auth.jwt_decoding_key();
        let jwt_validation = services.auth.jwt_validation();

        let decoded = decode::<JwtClaims>(&verify_response.token, jwt_key, jwt_validation)
            .expect("Failed to decode JWT");
        assert_eq!(decoded.claims.address, address);

        // inject delay to receive different timestamp
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Step 3: Use the JWT to refresh and get a new token
        let response = server
            .post("/auth/refresh")
            .add_header("Authorization", format!("Bearer {}", verify_response.token))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let token_response: TokenResponse = response.json();
        assert!(!token_response.token.is_empty());

        // Verify new token is different but valid
        assert_ne!(token_response.token, verify_response.token);

        let decoded_new = decode::<JwtClaims>(&token_response.token, jwt_key, jwt_validation)
            .expect("Failed to decode JWT");
        assert_eq!(decoded_new.claims.address, address);
        assert!(decoded_new.claims.iat >= decoded.claims.iat);

        // Step 4: Get profile with JWT
        let response = server
            .get("/auth/profile")
            .add_header("Authorization", format!("Bearer {}", verify_response.token))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let user: UserProfile = response.json();
        assert_eq!(user.address, address);
        assert_eq!(user.ens, MOCK_ENS);
    }

    #[tokio::test]
    async fn nonce_validates_address() {
        let app = mock_app().await;
        let server = TestServer::new(app).unwrap();

        let invalid_json = serde_json::json!({
            "address": "not_an_eth_address",
            "chainId": 1
        });

        let response = server.post(AUTH_NONCE_ENDPOINT).json(&invalid_json).await;

        assert_eq!(response.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn login_fails_without_nonce() {
        let app = mock_app().await;
        let server = TestServer::new(app).unwrap();
        let (_, signing_key) = eth_wallet();

        let message = "message not generated by backend";
        let signature = sign_message(&signing_key, message);

        let verify_request = VerifyRequest {
            message: message.to_string(),
            signature,
        };

        let response = server.post("/auth/verify").json(&verify_request).await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn login_fails_with_wrong_signature() {
        let mut cfg = Config::default();
        cfg.auth.mock_mode = false;

        let services = Services::mocks_with_config(cfg).await;
        let app = create_app(services);
        let server = TestServer::new(app).unwrap();

        let (address, _) = eth_wallet();
        let (wrong_address, wrong_signing_key) = eth_wallet(); // Different wallet
        assert_ne!(address, wrong_address);

        // Get valid nonce
        let nonce_request = NonceRequest {
            address,
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&nonce_request).await;

        let nonce_response: NonceResponse = response.json();

        // Sign with wrong key
        let wrong_signature = sign_message(&wrong_signing_key, &nonce_response.message);
        let verify_request = VerifyRequest {
            message: nonce_response.message,
            signature: wrong_signature,
        };

        let response = server.post("/auth/verify").json(&verify_request).await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn replay_attack_prevention() {
        let app = mock_app().await;
        let server = TestServer::new(app).unwrap();
        let (address, signing_key) = eth_wallet();

        // Get nonce and login once
        let nonce_request = NonceRequest {
            address,
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&nonce_request).await;

        let nonce_response: NonceResponse = response.json();
        let signature = sign_message(&signing_key, &nonce_response.message);

        let verify_request = VerifyRequest {
            message: nonce_response.message.clone(),
            signature: signature.clone(),
        };

        // First login should succeed
        let response = server.post("/auth/verify").json(&verify_request).await;

        assert_eq!(response.status_code(), StatusCode::OK);

        // Second login with same nonce should fail (replay attack)
        let response = server.post("/auth/verify").json(&verify_request).await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refresh_requires_valid_jwt() {
        let mut cfg = Config::default();
        cfg.auth.mock_mode = false;

        let services = Services::mocks_with_config(cfg).await;
        let app = create_app(services);
        let server = TestServer::new(app).unwrap();

        // Try refresh without token
        let response = server.post("/auth/refresh").await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

        // Try refresh with invalid token
        let response = server
            .post("/auth/refresh")
            .add_header("Authorization", "Bearer invalid_token")
            .await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn refresh_generates_new_token_with_updated_timestamps() {
        let mut cfg = Config::default();
        cfg.auth.mock_mode = false;

        let services = Services::mocks_with_config(cfg).await;
        let auth_service = services.auth.clone();

        let app = create_app(services);
        let server = TestServer::new(app).unwrap();

        // Unfortunately we can't easily advance system time
        // so instead we create an "old" token
        let old_claims = JwtClaims {
            address: MOCK_ADDRESS,
            iat: Utc::now().timestamp() - 10, // issued 10 seconds ago
            exp: Utc::now().timestamp() + 10, // expires in 10 seconds
        };
        let token = auth_service
            .encode_jwt(old_claims.clone())
            .expect("should be able to encode jwt");

        let response = server
            .post("/auth/refresh")
            .add_header("Authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let new_token: TokenResponse = response.json();
        let decoded_new = decode::<JwtClaims>(
            &new_token.token,
            &auth_service.jwt_decoding_key(),
            &auth_service.jwt_validation(),
        )
        .unwrap();

        assert_eq!(old_claims.address, decoded_new.claims.address);
        assert!(
            decoded_new.claims.iat > old_claims.iat,
            "New token should have newer IAT"
        );
        assert!(
            decoded_new.claims.exp > old_claims.iat,
            "New token should have EXP after original IAT"
        );
    }

    #[tokio::test]
    async fn profile_requires_valid_jwt() {
        let mut cfg = Config::default();
        cfg.auth.mock_mode = false;

        let services = Services::mocks_with_config(cfg).await;
        let app = create_app(services);
        let server = TestServer::new(app).unwrap();

        // Try profile without token
        let response = server.get("/auth/profile").await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

        // Try profile with invalid token
        let response = server
            .get("/auth/profile")
            .add_header("Authorization", "Bearer invalid_token")
            .await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn logout_clears_session() {
        let app = mock_app().await;
        let server = TestServer::new(app).unwrap();
        let (address, signing_key) = eth_wallet();

        // Complete login flow first
        let nonce_request = NonceRequest {
            address,
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&nonce_request).await;

        let nonce_response: NonceResponse = response.json();
        let signature = sign_message(&signing_key, &nonce_response.message);

        let verify_request = VerifyRequest {
            message: nonce_response.message,
            signature,
        };

        let response = server.post("/auth/verify").json(&verify_request).await;

        let verify_response: VerifyResponse = response.json();

        let response = server
            .post("/auth/logout")
            .add_header("Authorization", format!("Bearer {}", verify_response.token))
            .await;

        assert_eq!(response.status_code(), StatusCode::NO_CONTENT);

        // Token should still work for now (logout doesn't invalidate in current implementation)
        // This is a known limitation mentioned in the service code
        let response = server
            .get("/auth/profile")
            .add_header("Authorization", format!("Bearer {}", verify_response.token))
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn multiple_sessions() {
        let app = mock_app().await;
        let server = TestServer::new(app).unwrap();

        // Create two different wallets
        let (address1, signing_key1) = eth_wallet();
        let (address2, signing_key2) = eth_wallet();

        // Login with first wallet
        let nonce_request1 = NonceRequest {
            address: address1,
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&nonce_request1).await;

        let nonce_response1: NonceResponse = response.json();
        let signature1 = sign_message(&signing_key1, &nonce_response1.message);

        let verify_request1 = VerifyRequest {
            message: nonce_response1.message,
            signature: signature1,
        };

        let response = server.post("/auth/verify").json(&verify_request1).await;

        let verify_response1: VerifyResponse = response.json();

        // Login with second wallet
        let nonce_request2 = NonceRequest {
            address: address2,
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&nonce_request2).await;

        let nonce_response2: NonceResponse = response.json();
        let signature2 = sign_message(&signing_key2, &nonce_response2.message);

        let verify_request2 = VerifyRequest {
            message: nonce_response2.message,
            signature: signature2,
        };

        let response = server.post("/auth/verify").json(&verify_request2).await;

        let verify_response2: VerifyResponse = response.json();

        // Verify both tokens work independently
        let response = server
            .get("/auth/profile")
            .add_header(
                "Authorization",
                format!("Bearer {}", verify_response1.token),
            )
            .await;

        let user1: UserProfile = response.json();
        assert_eq!(user1.address, address1);

        let response = server
            .get("/auth/profile")
            .add_header(
                "Authorization",
                format!("Bearer {}", verify_response2.token),
            )
            .await;

        let user2: UserProfile = response.json();
        assert_eq!(user2.address, address2);
    }

    #[tokio::test]
    async fn rejects_expired() {
        let mut cfg = Config::default();
        cfg.auth.mock_mode = false;

        let services = Services::mocks_with_config(cfg).await;
        let auth_service = services.auth.clone();

        let app = create_app(services);
        let server = TestServer::new(app).unwrap();

        // unfortunately we can't easily advance system time
        // until the token is expired
        // so we create a token that's already expired
        let expired_claims = JwtClaims {
            address: MOCK_ADDRESS,
            exp: Utc::now().timestamp() - 3600, // 1 hour ago
            iat: Utc::now().timestamp() - 7200, // 2 hours ago
        };
        let token = auth_service
            .encode_jwt(expired_claims)
            .expect("should be able to encode jwt");

        let response = server
            .post("/auth/refresh")
            .add_header("Authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

        let response = server
            .get("/auth/profile")
            .add_header("Authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }
}

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_jwt::Claims;

use crate::{
    error::Error,
    models::auth::{JwtClaims, NonceRequest, VerifyRequest},
    services::Services,
};

pub async fn nonce(
    State(services): State<Services>,
    Json(payload): Json<NonceRequest>,
) -> Result<impl IntoResponse, Error> {
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
    let response = services
        .auth
        .login(&payload.message, &payload.signature)
        .await?;
    Ok(Json(response))
}

pub async fn refresh(
    State(services): State<Services>,
    Claims(user): Claims<JwtClaims>,
) -> Result<impl IntoResponse, Error> {
    let response = services.auth.refresh(user).await?;
    Ok(Json(response))
}

pub async fn logout(
    State(services): State<Services>,
    Claims(user): Claims<JwtClaims>,
) -> Result<impl IntoResponse, Error> {
    services.auth.logout(user).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn profile(
    State(services): State<Services>,
    Claims(user): Claims<JwtClaims>,
) -> Result<impl IntoResponse, Error> {
    let response = services.auth.profile(user).await?;
    Ok(Json(response))
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum_test::TestServer;
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    use crate::{
        api::mock_app,
        config::Config,
        constants::auth::MOCK_ENS,
        models::auth::{
            JwtClaims, NonceRequest, NonceResponse, TokenResponse, User, VerifyRequest,
            VerifyResponse,
        },
        test_utils::auth::{eth_wallet, sign_message},
    };

    #[tokio::test]
    async fn auth_flow_complete() {
        let app = mock_app();
        let server = TestServer::new(app).unwrap();
        let (address, signing_key) = eth_wallet();

        // Step 1: Get nonce challenge
        let nonce_request = NonceRequest {
            address: address.clone(),
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&nonce_request).await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let nonce_response: NonceResponse = response.json();
        assert!(nonce_response.message.contains(&address));

        // Step 2: Sign the message and login
        let signature = sign_message(&signing_key, &nonce_response.message);
        let verify_request = VerifyRequest {
            message: nonce_response.message,
            signature,
        };

        let response = server.post("/auth/verify").json(&verify_request).await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let verify_response: VerifyResponse = response.json();
        assert_eq!(verify_response.user.address, address.to_lowercase());
        assert!(!verify_response.token.is_empty());

        // Decode and verify the JWT
        let cfg = Config::default();
        let decoding_key = DecodingKey::from_secret(cfg.auth.jwt_secret.as_bytes());
        let validation = Validation::new(Algorithm::HS256);
        let decoded = decode::<JwtClaims>(&verify_response.token, &decoding_key, &validation)
            .expect("Failed to decode JWT");
        assert_eq!(decoded.claims.address, address.to_lowercase());

        // Step 3: Use the JWT to refresh and get a new token
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", verify_response.token)).unwrap(),
        );

        let response = server
            .post("/auth/refresh")
            .add_headers(headers.clone())
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let token_response: TokenResponse = response.json();
        assert!(!token_response.token.is_empty());

        // Verify new token is different but valid
        assert_ne!(token_response.token, verify_response.token);
        let decoded_new = decode::<JwtClaims>(&token_response.token, &decoding_key, &validation)
            .expect("Failed to decode new JWT");
        assert_eq!(decoded_new.claims.address, address.to_lowercase());
        assert!(decoded_new.claims.iat >= decoded.claims.iat);

        // Step 4: Get profile with JWT
        let response = server.get("/auth/profile").add_headers(headers).await;

        assert_eq!(response.status_code(), StatusCode::OK);
        let user: User = response.json();
        assert_eq!(user.address, address.to_lowercase());
        assert_eq!(user.ens, MOCK_ENS);
    }

    #[tokio::test]
    async fn nonce_validates_address() {
        let app = mock_app();
        let server = TestServer::new(app).unwrap();

        let invalid_request = NonceRequest {
            address: "not_an_eth_address".to_string(),
            chain_id: 1,
        };

        let response = server.post("/auth/nonce").json(&invalid_request).await;

        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn login_fails_without_nonce() {
        let app = mock_app();
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
        let app = mock_app();
        let server = TestServer::new(app).unwrap();
        let (address, _) = eth_wallet();
        let (_, wrong_signing_key) = eth_wallet(); // Different wallet

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
        let app = mock_app();
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
        let app = mock_app();
        let server = TestServer::new(app).unwrap();

        // Try refresh without token
        let response = server.post("/auth/refresh").await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

        // Try refresh with invalid token
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str("Bearer invalid_token").unwrap(),
        );

        let response = server.post("/auth/refresh").add_headers(headers).await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn profile_requires_valid_jwt() {
        let app = mock_app();
        let server = TestServer::new(app).unwrap();

        // Try profile without token
        let response = server.get("/auth/profile").await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);

        // Try profile with invalid token
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str("Bearer invalid_token").unwrap(),
        );

        let response = server.get("/auth/profile").add_headers(headers).await;

        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn logout_clears_session() {
        let app = mock_app();
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

        // Now logout with the token
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", verify_response.token)).unwrap(),
        );

        let response = server
            .post("/auth/logout")
            .add_headers(headers.clone())
            .await;

        assert_eq!(response.status_code(), StatusCode::NO_CONTENT);

        // Token should still work for now (logout doesn't invalidate in current implementation)
        // This is a known limitation mentioned in the service code
        let response = server.get("/auth/profile").add_headers(headers).await;

        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn multiple_sessions() {
        let app = mock_app();
        let server = TestServer::new(app).unwrap();

        // Create two different wallets
        let (address1, signing_key1) = eth_wallet();
        let (address2, signing_key2) = eth_wallet();

        // Login with first wallet
        let nonce_request1 = NonceRequest {
            address: address1.clone(),
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
            address: address2.clone(),
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
        let mut headers1 = HeaderMap::new();
        headers1.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", verify_response1.token)).unwrap(),
        );

        let mut headers2 = HeaderMap::new();
        headers2.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", verify_response2.token)).unwrap(),
        );

        let response = server.get("/auth/profile").add_headers(headers1).await;

        let user1: User = response.json();
        assert_eq!(user1.address, address1.to_lowercase());

        let response = server.get("/auth/profile").add_headers(headers2).await;

        let user2: User = response.json();
        assert_eq!(user2.address, address2.to_lowercase());
    }
}

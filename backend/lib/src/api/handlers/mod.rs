use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::{
    api::validation::extract_bearer_token,
    constants::mocks::MOCK_ADDRESS,
    error::Error,
    models::auth::{NonceRequest, VerifyRequest},
    services::Services,
};

pub mod buckets;
pub mod files;

// TODO: we could move from `TypedHeader` to axum-jwt (needs rust 1.88)

// ==================== Auth Handlers ====================

pub async fn nonce(
    State(services): State<Services>,
    Json(payload): Json<NonceRequest>,
) -> Result<impl IntoResponse, Error> {
    let response = services
        .auth
        .generate_nonce(&payload.address, payload.chain_id)
        .await?;
    Ok(Json(response))
}

pub async fn verify(
    State(services): State<Services>,
    Json(payload): Json<VerifyRequest>,
) -> Result<impl IntoResponse, Error> {
    let response = services
        .auth
        .verify_eth_signature(&payload.message, &payload.signature)
        .await?;
    Ok(Json(response))
}

pub async fn refresh(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let token = auth.token();
    let response = services.auth.refresh_token(token).await?;
    Ok(Json(response))
}

pub async fn logout(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let token = auth.token();
    services.auth.logout(token).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn profile(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let token = auth.token();
    let response = services.auth.get_profile(token).await?;
    Ok(Json(response))
}

// ==================== MSP Info Handlers ====================

pub async fn info(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_info().await?;
    Ok(Json(response))
}

pub async fn stats(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_stats().await?;
    Ok(Json(response))
}

pub async fn value_props(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_value_props().await?;
    Ok(Json(response))
}

pub async fn msp_health(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    let response = services.health.check_health().await;
    Ok(Json(response))
}

// ==================== Payment Handler ====================

pub async fn payment_stream(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let auth = extract_bearer_token(&auth)?;

    let address = auth
        .get("address")
        .and_then(|a| a.as_str())
        .unwrap_or(MOCK_ADDRESS);
    let response = services.msp.get_payment_stream(address).await?;
    Ok(Json(response))
}

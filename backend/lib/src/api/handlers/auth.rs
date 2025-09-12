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

pub async fn login(
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

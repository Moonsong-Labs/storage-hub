use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};

use crate::{
    error::Error,
    models::auth::{NonceRequest, VerifyRequest},
    services::Services,
};

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
    let _token = auth.token();
    // TODO: Wire up token decoding to JwtClaims
    let token_data = todo!("Wire up token decoding from handler");
    let response = services.auth.refresh_token(&token_data).await?;
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

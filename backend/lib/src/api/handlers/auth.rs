use axum::{
    extract::{FromRef, FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_jwt::{Claims, Decoder};

use crate::{
    error::Error,
    models::auth::{JwtClaims, NonceRequest, VerifyRequest},
    services::Services,
};

pub async fn message(
    State(services): State<Services>,
    Json(payload): Json<NonceRequest>,
) -> Result<impl IntoResponse, Error> {
    let response = services
        .auth
        .generate_nonce(&payload.address, payload.chain_id)
        .await?;
    Ok(Json(response))
}

pub async fn login(
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
    Claims(user): Claims<JwtClaims>,
) -> Result<impl IntoResponse, Error> {
    let response = services.auth.refresh_token(user).await?;
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
    let response = services.auth.get_profile(user).await?;
    Ok(Json(response))
}

/// Axum extractor to verify the authenticated user.
///
/// Will error if the token is expired or it is otherwise invalid
pub struct AuthenticatedUser {
    pub address: String,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    Services: FromRef<S>,
    Decoder: FromRef<S>,
    S: Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let services = Services::from_ref(state);
        let claims = Claims::<JwtClaims>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::Unauthorized(format!("Invalid JWT: {e:?}")))?;

        let profile = services.auth.get_profile(claims.0).await?;
        Ok(Self {
            address: profile.address,
        })
    }
}

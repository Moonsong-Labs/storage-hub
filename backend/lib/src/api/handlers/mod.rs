use axum::{extract::State, response::IntoResponse, Json};
use tracing::debug;

use crate::{
    error::Error,
    services::{auth::AuthenticatedUser, Services},
};

pub mod auth;
pub mod buckets;
pub mod files;

// ==================== MSP Info Handlers ====================

pub async fn info(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    debug!("GET MSP info");
    let response = services.msp.get_info().await?;
    Ok(Json(response))
}

pub async fn stats(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    debug!("GET MSP stats");
    let response = services.msp.get_stats().await?;
    Ok(Json(response))
}

pub async fn value_props(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    debug!("GET MSP value propositions");
    let response = services.msp.get_value_props().await?;
    Ok(Json(response))
}

pub async fn msp_health(State(services): State<Services>) -> Result<impl IntoResponse, Error> {
    debug!("GET health check");
    let response = services.health.check_health().await;
    Ok(Json(response))
}

// ==================== Payment Handler ====================

pub async fn payment_streams(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    debug!(user = %address, "GET payment streams");
    let response = services.msp.get_payment_streams(&address).await?;
    Ok(Json(response))
}

use alloy_core::primitives::Address;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::debug;

use crate::{
    error::Error,
    services::{auth::User, Services},
};

pub mod auth;
pub mod buckets;
pub mod files;

mod pagination;

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
    Ok(services.health.check_health().await)
}

// ==================== Payment Handler ====================

#[derive(Debug, Deserialize)]
pub struct PaymentStreamsQuery {
    pub address: Option<String>,
}

/// Returns payment streams for a given user.
///
/// The target address is resolved in order of precedence:
/// 1. `?address=` query parameter (no auth required)
/// 2. Authenticated user's address (via JWT)
/// 3. 400 Bad Request if neither is provided
pub async fn payment_streams(
    State(services): State<Services>,
    user: User,
    Query(query): Query<PaymentStreamsQuery>,
) -> Result<impl IntoResponse, Error> {
    let address = match query.address {
        Some(addr_str) => addr_str.parse::<Address>().map_err(|_| {
            Error::BadRequest(format!("Invalid address: {addr_str}"))
        })?,
        None => *user.address().map_err(|_| {
            Error::BadRequest(
                "Either provide an ?address= query parameter or authenticate via JWT".to_owned(),
            )
        })?,
    };

    debug!(user = %address, "GET payment streams");
    let response = services.msp.get_payment_streams(&address).await?;
    Ok(Json(response))
}

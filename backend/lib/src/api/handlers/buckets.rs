//! This module contains the handlers for the bucket management endpoints

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};

use crate::{
    api::validation::extract_bearer_token, constants::mocks::MOCK_ADDRESS, error::Error,
    models::files::FileListResponse, services::Services,
};

/// Retrieve
pub async fn list_buckets(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let payload = extract_bearer_token(&auth)?;
    let address = payload
        .get("address")
        .and_then(|a| a.as_str())
        .unwrap_or(MOCK_ADDRESS);

    let response = services.msp.list_user_buckets(address).await?;
    Ok(Json(response))
}

pub async fn get_bucket(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

    let response = services.msp.get_bucket(&bucket_id).await?;
    Ok(Json(response))
}

pub async fn get_files(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

    let file_tree = services.msp.get_file_tree(&bucket_id).await?;
    let response = FileListResponse {
        bucket_id: bucket_id.clone(),
        files: vec![file_tree],
    };
    Ok(Json(response))
}

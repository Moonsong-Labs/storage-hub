//! This module contains the handlers for the bucket management endpoints

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    error::Error, models::files::FileListResponse, services::auth::AuthenticatedUser,
    services::Services,
};

pub async fn list_buckets(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    let response = services
        .msp
        .list_user_buckets(&address)
        .await?
        .collect::<Vec<_>>();
    Ok(Json(response))
}

pub async fn get_bucket(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_bucket(&bucket_id, &address).await?;

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub path: Option<String>,
}

pub async fn get_files(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
    Path(bucket_id): Path<String>,
    Query(query): Query<FilesQuery>,
) -> Result<impl IntoResponse, Error> {
    let file_tree = services
        .msp
        .get_file_tree(&bucket_id, &address, query.path.as_deref().unwrap_or("/"))
        .await?;

    let response = FileListResponse {
        bucket_id: bucket_id.clone(),
        files: vec![file_tree],
    };

    Ok(Json(response))
}

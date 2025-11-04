//! This module contains the handlers for the bucket management endpoints

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::debug;

use crate::{
    api::handlers::pagination::Pagination,
    error::Error,
    models::files::FileListResponse,
    services::{
        auth::{AuthenticatedUser, User},
        Services,
    },
};

pub async fn list_buckets(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
    Pagination { limit, offset }: Pagination,
) -> Result<impl IntoResponse, Error> {
    debug!(user = %address, "GET list buckets");
    let response = services
        .msp
        .list_user_buckets(&address, offset, limit)
        .await?
        .collect::<Vec<_>>();
    Ok(Json(response))
}

pub async fn get_bucket(
    State(services): State<Services>,
    user: User,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    debug!(bucket_id = %bucket_id, %user, "GET bucket");

    let response = services
        .msp
        .get_bucket(&bucket_id, user.address().ok())
        .await?;

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub path: Option<String>,
}

pub async fn get_files(
    State(services): State<Services>,
    user: User,
    Path(bucket_id): Path<String>,
    Query(query): Query<FilesQuery>,
    Pagination { limit, offset }: Pagination,
) -> Result<impl IntoResponse, Error> {
    let path = query.path.as_deref().unwrap_or("/");
    debug!(
        bucket_id = %bucket_id,
        path = %path,
        %user,
        "GET bucket files"
    );

    let file_tree = services
        .msp
        .get_file_tree(&bucket_id, user.address().ok(), path, offset, limit)
        .await?;

    let response = FileListResponse {
        bucket_id: bucket_id.clone(),
        tree: file_tree,
    };

    Ok(Json(response))
}

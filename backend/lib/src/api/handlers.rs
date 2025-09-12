use std::io::Cursor;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::{extract::Multipart, response::file_stream::FileStream};
use serde::Deserialize;
use tokio_util::io::ReaderStream;

use crate::{
    error::Error,
    models::files::{FileListResponse, FileUploadResponse},
    services::{auth::AuthenticatedUser, Services},
};

pub mod auth;

// TODO: we could move from `TypedHeader` to axum-jwt (needs rust 1.88)

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
    let response = services.msp.get_health().await?;
    Ok(Json(response))
}

// ==================== Bucket Handlers ====================

pub async fn list_buckets(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.list_user_buckets(&address).await?;
    Ok(Json(response))
}

pub async fn get_bucket(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_bucket(&bucket_id).await?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub path: Option<String>,
}

pub async fn get_files(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path(bucket_id): Path<String>,
    Query(query): Query<FilesQuery>,
) -> Result<impl IntoResponse, Error> {
    let files = services
        .msp
        .get_files(&bucket_id, query.path.as_deref())
        .await?;
    let response = FileListResponse {
        bucket_id: bucket_id.clone(),
        files,
    };
    Ok(Json(response))
}

// ==================== File Handlers ====================

pub async fn download_by_location(
    State(_services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((_bucket_id, _file_location)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    // TODO(MOCK): return proper data
    let file_data = b"Mock file content for download".to_vec();
    let stream = ReaderStream::new(Cursor::new(file_data));
    let file_stream_resp = FileStream::new(stream).file_name("by_location.txt");

    Ok(file_stream_resp.into_response())
}

pub async fn download_by_key(
    State(_services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((_bucket_id, _file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    // TODO(MOCK): return proper data
    let file_data = b"Mock file content for download".to_vec();
    let stream = ReaderStream::new(Cursor::new(file_data));
    let file_stream_resp = FileStream::new(stream).file_name("by_key.txt");

    Ok(file_stream_resp.into_response())
}

pub async fn get_file_info(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_file_info(&bucket_id, &file_key).await?;
    Ok(Json(response))
}

pub async fn upload_file(
    State(_services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    // Extract file from multipart
    let mut file_data = Vec::new();
    let mut file_name = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            file_name = field
                .file_name()
                .ok_or_else(|| Error::BadRequest("Missing file name".to_string()))?
                .to_string();
            file_data = field
                .bytes()
                .await
                .map_err(|e| Error::BadRequest(e.to_string()))?
                .to_vec();
            break;
        }
    }

    if file_data.is_empty() {
        return Err(Error::BadRequest("No file provided".to_string()));
    }

    // TODO(MOCK): proper success response
    let response = FileUploadResponse {
        status: "upload_successful".to_string(),
        file_key: file_key.clone(),
        bucket_id: bucket_id.clone(),
        fingerprint: "5d7a3700e1f7d973c064539f1b18c988dace6b4f1a57650165e9b58305db090f".to_string(),
        // TODO: location is arbitrary, don't tie it to bucket_id and multipart filename
        location: format!("/files/{}/{}", bucket_id, file_name),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn distribute_file(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.distribute_file(&bucket_id, &file_key).await?;
    Ok(Json(response))
}

// ==================== Payment Handler ====================

pub async fn payment_stream(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_payment_stream(&address).await?;
    Ok(Json(response))
}

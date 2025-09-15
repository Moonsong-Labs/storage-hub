use std::io::Cursor;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::{
    extract::Multipart,
    headers::{authorization::Bearer, Authorization},
    response::file_stream::FileStream,
    TypedHeader,
};
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    api::validation::extract_bearer_token,
    constants::mocks::MOCK_ADDRESS,
    error::Error,
    models::{
        auth::{NonceRequest, VerifyRequest},
        files::{FileListResponse, FileUploadResponse},
    },
    services::Services,
};

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
    let response = services.msp.get_health().await?;
    Ok(Json(response))
}

// ==================== Bucket Handlers ====================

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

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub path: Option<String>,
}

pub async fn get_files(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(bucket_id): Path<String>,
    Query(query): Query<FilesQuery>,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

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
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path((_bucket_id, _file_location)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

    // TODO(MOCK): return proper data
    let file_data = b"Mock file content for download".to_vec();
    let stream = ReaderStream::new(Cursor::new(file_data));
    let file_stream_resp = FileStream::new(stream).file_name("by_location.txt");

    Ok(file_stream_resp.into_response())
}

// Used by the MSP RPC to upload a file to the backend
// The file is only temporary and will be deleted after the stream is closed
pub async fn internal_upload_by_key(
    State(_services): State<Services>,
    Path(file_key): Path<String>,
    body: Bytes,
) -> (StatusCode, impl IntoResponse) {
    // TODO: re-add auth
    // FIXME: make this only callable by the rpc itself
    // let _auth = extract_bearer_token(&auth)?;
    if let Err(e) = tokio::fs::create_dir_all("uploads").await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    // Validate file_key is a hex string
    let key = file_key.trim_start_matches("0x");
    if hex::decode(key).is_err() {
        return (StatusCode::BAD_REQUEST, "Invalid file key".to_string());
    }

    match tokio::fs::write(format!("uploads/{}", file_key), body).await {
        Ok(_) => (StatusCode::OK, "Upload successful".to_string()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn download_by_key(
    State(services): State<Services>,
    Path(file_key): Path<String>,
) -> Result<impl IntoResponse, Error> {
    // TODO: re-add auth
    // let _auth = extract_bearer_token(&auth)?;

    // Validate file_key is a hex string
    let key = file_key.trim_start_matches("0x");
    if hex::decode(key).is_err() {
        return Err(Error::BadRequest("Invalid file key".to_string()));
    }

    let download_result = services.msp.get_file_from_key(&file_key).await?;

    // Extract filename from location or use file_key as fallback
    let filename = download_result
        .location
        .split('/')
        .last()
        .unwrap_or(&file_key)
        .to_string();

    // Open file for streaming
    let file = File::open(&download_result.temp_path)
        .await
        .map_err(|e| Error::BadRequest(format!("Failed to open downloaded file: {}", e)))?;

    // On Unix, unlink the path immediately; the open fd remains valid for streaming
    // TODO: we should implement proper cleanup after the stream is closed
    // But as we will probably change implementation to just redirect the RPC stream to user, leaving it as is for now (not a problem if we run on unix).
    #[cfg(unix)]
    {
        let _ = tokio::fs::remove_file(&download_result.temp_path).await;
    }

    let stream = ReaderStream::new(file);
    let file_stream_resp = FileStream::new(stream).file_name(&filename).into_response();

    Ok(file_stream_resp)
}

pub async fn get_file_info(
    State(services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

    let response = services.msp.get_file_info(&bucket_id, &file_key).await?;
    Ok(Json(response))
}

pub async fn upload_file(
    State(_services): State<Services>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path((bucket_id, file_key)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

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
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let _auth = extract_bearer_token(&auth)?;

    let response = services.msp.distribute_file(&bucket_id, &file_key).await?;
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

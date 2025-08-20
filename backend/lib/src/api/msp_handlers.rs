use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::Multipart;

use crate::{api::validation::extract_bearer_token, error::Error, models::*, services::Services};

// Helper functions for token handling
fn extract_token(headers: &HeaderMap) -> Result<String, Error> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    extract_bearer_token(auth_header)
}

fn validate_token(_token: &str) -> Result<(), Error> {
    // Mock validation - in production this would verify JWT
    Ok(())
}

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
        .verify_signature(&payload.message, &payload.signature)
        .await?;
    Ok(Json(response))
}

pub async fn refresh(
    State(services): State<Services>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let response = services.auth.refresh_token(&token).await?;
    Ok(Json(response))
}

pub async fn logout(
    State(services): State<Services>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    services.auth.logout(&token).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn profile(
    State(services): State<Services>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let response = services.auth.get_profile(&token).await?;
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
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("")
        .split('.')
        .nth(1)
        .and_then(|payload| {
            use base64::{engine::general_purpose, Engine};
            general_purpose::STANDARD.decode(payload).ok()
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("address").and_then(|a| a.as_str()).map(String::from))
        .unwrap_or_else(|| "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string());

    let response = services.msp.list_user_buckets(&auth_header).await?;
    Ok(Json(response))
}

pub async fn get_bucket(
    State(services): State<Services>,
    headers: HeaderMap,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let response = services.msp.get_bucket(&bucket_id).await?;
    Ok(Json(response))
}

pub async fn get_files(
    State(services): State<Services>,
    headers: HeaderMap,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let file_tree = services.msp.get_file_tree(&bucket_id).await?;
    let response = FileListResponse {
        bucket_id: bucket_id.clone(),
        files: vec![file_tree],
    };
    Ok(Json(response))
}

// ==================== File Handlers ====================

pub async fn download_by_location(
    State(_services): State<Services>,
    headers: HeaderMap,
    Path((_bucket_id, _file_location)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    // Mock implementation - return dummy data
    let file_data = b"Mock file content for download".to_vec();

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        file_data,
    ))
}

pub async fn download_by_key(
    State(_services): State<Services>,
    headers: HeaderMap,
    Path((_bucket_id, _file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    // Mock implementation - return dummy data
    let file_data = b"Mock file content for download".to_vec();

    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        file_data,
    ))
}

pub async fn get_file_info(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let response = services.msp.get_file_info(&bucket_id, &file_key).await?;
    Ok(Json(response))
}

pub async fn upload_file(
    State(_services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

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

    // Mock implementation - return success response
    let response = FileUploadResponse {
        status: "upload_successful".to_string(),
        file_key: file_key.clone(),
        bucket_id: bucket_id.clone(),
        fingerprint: "5d7a3700e1f7d973c064539f1b18c988dace6b4f1a57650165e9b58305db090f".to_string(),
        location: format!("/files/{}/{}", bucket_id, file_name),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn distribute_file(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let response = services.msp.distribute_file(&bucket_id, &file_key).await?;
    Ok(Json(response))
}

// ==================== Payment Handler ====================

pub async fn payment_stream(
    State(services): State<Services>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;

    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("")
        .split('.')
        .nth(1)
        .and_then(|payload| {
            use base64::{engine::general_purpose, Engine};
            general_purpose::STANDARD.decode(payload).ok()
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("address").and_then(|a| a.as_str()).map(String::from))
        .unwrap_or_else(|| "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac".to_string());

    let response = services.msp.get_payment_stream(&auth_header).await?;
    Ok(Json(response))
}

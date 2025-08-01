use axum::{
    extract::{Path, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::Multipart;
use serde_json::json;
use crate::{
    error::Error,
    models::*,
    services::Services,
    api::validation::{validate_token, extract_token},
};

// ==================== Auth Handlers ====================

pub async fn nonce(
    State(services): State<Services>,
    Json(payload): Json<NonceRequest>,
) -> Result<impl IntoResponse, Error> {
    let response = services.auth.generate_nonce(&payload.address, payload.chain_id).await?;
    Ok(Json(response))
}

pub async fn verify(
    State(services): State<Services>,
    Json(payload): Json<VerifyRequest>,
) -> Result<impl IntoResponse, Error> {
    let response = services.auth.verify_signature(&payload.message, &payload.signature).await?;
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

pub async fn info(
    State(services): State<Services>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_info().await?;
    Ok(Json(response))
}

pub async fn stats(
    State(services): State<Services>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_stats().await?;
    Ok(Json(response))
}

pub async fn value_props(
    State(services): State<Services>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_value_props().await?;
    Ok(Json(response))
}

pub async fn msp_health(
    State(services): State<Services>,
) -> Result<impl IntoResponse, Error> {
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
    
    let response = services.msp.list_buckets(&token).await?;
    Ok(Json(response))
}

pub async fn get_bucket(
    State(services): State<Services>,
    headers: HeaderMap,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let response = services.msp.get_bucket(&token, &bucket_id).await?;
    Ok(Json(response))
}

pub async fn get_files(
    State(services): State<Services>,
    headers: HeaderMap,
    Path(bucket_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let response = services.msp.get_files(&token, &bucket_id).await?;
    Ok(Json(response))
}

// ==================== File Handlers ====================

pub async fn download_by_location(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_location)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let file_data = services.msp.download_by_location(&token, &bucket_id, &file_location).await?;
    
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        file_data
    ))
}

pub async fn download_by_key(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let file_data = services.msp.download_by_key(&token, &bucket_id, &file_key).await?;
    
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        file_data
    ))
}

pub async fn get_file_info(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let response = services.msp.get_file_info(&token, &bucket_id, &file_key).await?;
    Ok(Json(response))
}

pub async fn upload_file(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    // Extract file from multipart
    let mut file_data = Vec::new();
    let mut file_name = String::new();
    
    while let Some(field) = multipart.next_field().await.map_err(|e| Error::BadRequest(e.to_string()))? {
        if field.name() == Some("file") {
            file_name = field.file_name()
                .ok_or_else(|| Error::BadRequest("Missing file name".to_string()))?
                .to_string();
            file_data = field.bytes().await
                .map_err(|e| Error::BadRequest(e.to_string()))?
                .to_vec();
            break;
        }
    }
    
    if file_data.is_empty() {
        return Err(Error::BadRequest("No file provided".to_string()));
    }
    
    let response = services.msp.upload_file(&token, &bucket_id, &file_key, &file_name, file_data).await?;
    
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn distribute_file(
    State(services): State<Services>,
    headers: HeaderMap,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let response = services.msp.distribute_file(&token, &bucket_id, &file_key).await?;
    Ok(Json(response))
}

// ==================== Payment Handler ====================

pub async fn payment_stream(
    State(services): State<Services>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let token = extract_token(&headers)?;
    validate_token(&token)?;
    
    let response = services.msp.get_payment_stream(&token).await?;
    Ok(Json(response))
}
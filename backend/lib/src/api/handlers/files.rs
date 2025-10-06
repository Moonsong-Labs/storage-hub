//! This module contains the handlers for the file management endpoints
//!
//! TODO: move the rest of the endpoints as they are implemented

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::{
    extract::{multipart::Field, Multipart},
    response::FileStream,
};
use codec::Decode;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use shc_common::types::FileMetadata;

use crate::{
    error::Error,
    services::{auth::AuthenticatedUser, Services},
};

pub async fn get_file_info(
    State(services): State<Services>,
    AuthenticatedUser { address }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let response = services
        .msp
        .get_file_info(&bucket_id, &address, &file_key)
        .await?;
    Ok(Json(response))
}

// Internal endpoint used by the MSP RPC to upload a file to the backend
// The file is only temporary and will be deleted after the stream is closed
pub async fn internal_upload_by_key(
    State(_services): State<Services>,
    Path(file_key): Path<String>,
    body: Bytes,
) -> (StatusCode, impl IntoResponse) {
    // TODO: re-add auth
    // FIXME: make this only callable by the rpc itself
    // let _auth = extract_bearer_token(&auth)?;
    if let Err(e) = tokio::fs::create_dir_all("/tmp/uploads").await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    // Validate file_key is a hex string
    let key = file_key.trim_start_matches("0x");
    if hex::decode(key).is_err() {
        return (StatusCode::BAD_REQUEST, "Invalid file key".to_string());
    }

    let file_path = format!("/tmp/uploads/{}", file_key);
    match tokio::fs::write(&file_path, body).await {
        Ok(_) => (StatusCode::OK, "Upload successful".to_string()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn download_by_key(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path(file_key): Path<String>,
) -> Result<impl IntoResponse, Error> {
    // Validate file_key is a hex string
    let key = file_key.trim_start_matches("0x");
    if hex::decode(key).is_err() {
        return Err(Error::BadRequest("Invalid file key".to_string()));
    }

    // TODO(AUTH): verify that user has permissions to access this file
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

/// Streams a file upload from a user into a trie and then through P2P to the MSP.
///
/// This handler implements a streaming approach to file uploads that:
/// 1. Extracts the file data stream and the file metadata from the multipart form.
/// 2. Validates that the decoded file metadata matches the received bucket ID and file key.
/// 3. Streams the file data into a trie in memory, chunking it into FILE_CHUNK_SIZE chunks in the process.
/// 4. Processes the chunked file in batches of BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, generating proofs for each batch.
/// 5. Converts the generated proofs to FileKeyProofs to send to the MSP client.
/// 6. Sends the batches of chunks with their respective proofs to the MSP via batch uploads.
///
/// Expected multipart fields:
/// - `file`: The file data to upload
/// - `file_metadata`: Encoded FileMetadata (Vec<u8>) containing owner, bucket_id, location, file_size, and fingerprint
///
/// When running with the `mocks` feature enabled, this performs minimal validation
/// and returns a mock success response without actual file processing.
///
/// TODO: Further optimize this to avoid having to load the entire file into memory.
pub async fn upload_file(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    // TODO(AUTH): verify that user has permissions to access this file

    // Pre-check with MSP whether this file key is expected before doing heavy processing
    let is_expected = services
        .msp
        .is_msp_expecting_file_key(&file_key)
        .await
        .unwrap_or(false);
    if !is_expected {
        return Err(Error::BadRequest(
            "MSP is not expecting this file key".to_string(),
        ));
    }

    // Extract from the request the file data stream and file metadata.
    let mut file_data_stream: Option<Field> = None;
    let mut file_metadata: Option<FileMetadata> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::BadRequest(format!("Failed to parse multipart field: {}", e)))?
    {
        match field.name() {
            // NOTE: This is CRUCIAL. Only ONE field from a given Multipart instance may be live at once, and since
            // we want to stream and process the file data stream afterwards, the file metadata field must ALWAYS be sent first
            // by the requestor. For more details, see: https://github.com/tokio-rs/axum/blob/main/axum-extra/src/extract/multipart.rs#L55
            Some("file_metadata") => {
                // From the 'file_metadata' field we extract and decode the file metadata.
                let metadata_bytes = field.bytes().await.map_err(|e| {
                    Error::BadRequest(format!("Failed to read file_metadata: {}", e))
                })?;

                file_metadata = Some(FileMetadata::decode(&mut metadata_bytes.as_ref()).map_err(
                    |e| Error::BadRequest(format!("Failed to decode file_metadata: {:?}", e)),
                )?);
            }
            Some("file") => {
                // From the 'file' field of the multipart, we get the file data stream.
                file_data_stream = Some(field);

                // Since after this we can't process any more fields, we break out of the loop.
                break;
            }
            _ => {
                continue;
            }
        }
    }

    // Ensure both the file data stream and the file metadata were provided.
    let file_data_stream = file_data_stream
        .ok_or_else(|| Error::BadRequest("Missing 'file' field in multipart data".to_string()))?;

    let file_metadata = file_metadata.ok_or_else(|| {
        Error::BadRequest("Missing 'file_metadata' field in multipart data".to_string())
    })?;

    // Process and upload the file using the MSP service
    let response = services
        .msp
        .process_and_upload_file(&bucket_id, &file_key, file_data_stream, file_metadata)
        .await?;

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn distribute_file(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    // TODO(AUTH): verify that user has permissions to access this file

    let response = services.msp.distribute_file(&bucket_id, &file_key).await?;
    Ok(Json(response))
}

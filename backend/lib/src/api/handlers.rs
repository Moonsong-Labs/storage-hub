#[cfg(not(feature = "mocks"))]
use std::collections::HashSet;
use std::io::Cursor;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::{extract::Multipart, response::file_stream::FileStream};
use codec::Decode;
use serde::Deserialize;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use shc_common::types::FileMetadata;
#[cfg(not(feature = "mocks"))]
use shc_common::types::{
    ChunkId, StorageProofsMerkleTrieLayout, BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, FILE_CHUNK_SIZE,
};
#[cfg(not(feature = "mocks"))]
use shc_file_manager::{in_memory::InMemoryFileDataTrie, traits::FileDataTrie};
use shp_types::Hashing;

use crate::{
    error::Error,
    models::files::{FileListResponse, FileUploadResponse},
    services::Services,
};

pub mod auth;

use auth::AuthenticatedUser;

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
    let response = services.health.check_health().await;
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
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let response = services.msp.get_file_info(&bucket_id, &file_key).await?;
    Ok(Json(response))
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
#[cfg_attr(feature = "mocks", allow(unused_variables))]
pub async fn upload_file(
    State(services): State<Services>,
    AuthenticatedUser { address: _ }: AuthenticatedUser,
    Path((bucket_id, file_key)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
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

    // Validate that the bucket ID received in the URL matches the bucket ID in the file metadata.
    let expected_bucket_id = hex::encode(file_metadata.bucket_id());
    if bucket_id.trim_start_matches("0x") != expected_bucket_id {
        return Err(Error::BadRequest(
            "Bucket ID in URL does not match file metadata".to_string(),
        ));
    }

    // Generate the file key from the obtained file metadata and ensure it matches the file key received in the URL.
    let expected_file_key = hex::encode(file_metadata.file_key::<Hashing>());
    if file_key.trim_start_matches("0x") != expected_file_key {
        return Err(Error::BadRequest(
            "File key in URL does not match file metadata".to_string(),
        ));
    }

    process_upload_after_validation(
        file_data_stream,
        file_metadata,
        &bucket_id,
        &file_key,
        &services,
    )
    .await
}

#[cfg(feature = "mocks")]
async fn process_upload_after_validation(
    file_data_stream: Field,
    file_metadata: FileMetadata,
    bucket_id: &str,
    file_key: &str,
    _services: &Services,
) -> Result<impl IntoResponse, Error> {
    // Consume the file field to ensure it is correct
    let _file_bytes = file_data_stream
        .bytes()
        .await
        .map_err(|e| Error::BadRequest(format!("Failed to read file: {}", e)))?;

    // Return a mock success response
    let bytes_location = file_metadata.location().clone();
    let location = str::from_utf8(&bytes_location)
        .unwrap_or(file_key)
        .to_string();
    let response = FileUploadResponse {
        status: "upload_successful".to_string(),
        file_key: file_key.to_string(),
        bucket_id: bucket_id.to_string(),
        fingerprint: hex::encode(file_metadata.fingerprint().as_ref()),
        location,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[cfg(not(feature = "mocks"))]
async fn process_upload_after_validation(
    mut file_data_stream: Field,
    file_metadata: FileMetadata,
    bucket_id: &str,
    file_key: &str,
    services: &Services,
) -> Result<impl IntoResponse, Error> {
    // Initialize the trie that will hold the chunked file data.
    let mut trie = InMemoryFileDataTrie::<StorageProofsMerkleTrieLayout>::new();

    // Prepare the overflow buffer that will hold any data that doesn't exactly fit in a chunk.
    let mut overflow_buffer = Vec::new();

    // Initialize the chunk index.
    let mut chunk_index = 0;

    // Start streaming the file data into the trie, chunking it into FILE_CHUNK_SIZE chunks in the process.
    while let Some(bytes_read) = file_data_stream
        .chunk()
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?
    {
        // Load the bytes read from the file into the overflow buffer.
        overflow_buffer.extend_from_slice(&bytes_read);

        // While the overflow buffer is larger than FILE_CHUNK_SIZE, process a chunk.
        while overflow_buffer.len() >= FILE_CHUNK_SIZE as usize {
            let chunk = overflow_buffer[..FILE_CHUNK_SIZE as usize].to_vec();

            // Insert the chunk into the trie.
            trie.write_chunk(&ChunkId::new(chunk_index as u64), &chunk)
                .map_err(|e| Error::BadRequest(e.to_string()))?;

            // Increment the chunk index.
            chunk_index += 1;

            // Remove the chunk from the overflow buffer.
            overflow_buffer.drain(..FILE_CHUNK_SIZE as usize);
        }
    }

    // Check the overflow buffer to see if the file didn't fit exactly in an integer number of chunks.
    if !overflow_buffer.is_empty() {
        // Insert the chunk into the trie.
        trie.write_chunk(&ChunkId::new(chunk_index as u64), &overflow_buffer)
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        // Increment the chunk index to get the total amount of chunks.
        chunk_index += 1;
    }

    // Validate that the file fingerprint matches the trie root.
    let computed_root = trie.get_root();
    if computed_root.as_ref() != file_metadata.fingerprint().as_ref() {
        return Err(Error::BadRequest(format!(
            "File fingerprint mismatch. Expected: {}, Computed: {}",
            hex::encode(file_metadata.fingerprint().as_ref()),
            hex::encode(computed_root)
        )));
    }

    // Validate that the received amount of chunks matches the amount of chunks corresponding to the file size in the metadata.
    let total_chunks = file_metadata.chunks_count();
    if chunk_index != total_chunks {
        return Err(Error::BadRequest(format!(
            "Received amount of chunks {} does not match the amount of chunks {} corresponding to the file size in the metadata",
            chunk_index, total_chunks
        )));
    }

    // At this point, the trie contains the entire file data and we can start generating the proofs for the chunk batches
    // and sending them to the MSP.

    // Get how many chunks fit in a batch of BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE, rounding down.
    const CHUNKS_PER_BATCH: u64 = BATCH_CHUNK_FILE_TRANSFER_MAX_SIZE as u64 / FILE_CHUNK_SIZE;

    // Initialize the index of the initial chunk to process in this batch.
    let mut batch_start_chunk_index = 0;

    // Start processing batches, until all chunks have been processed.
    while batch_start_chunk_index < total_chunks {
        // Get the chunks to send in this batch, capping at the total amount of chunks of the file.
        let chunks = (batch_start_chunk_index
            ..(batch_start_chunk_index + CHUNKS_PER_BATCH).min(total_chunks))
            .map(|chunk_index| ChunkId::new(chunk_index as u64))
            .collect::<HashSet<_>>();
        let chunks_in_batch = chunks.len() as u64;

        // Generate the proof for the batch.
        let file_proof = trie
            .generate_proof(&chunks)
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        // Convert the generated proof to a FileKeyProof and send it to the MSP.
        let file_key_proof = file_proof
            .to_file_key_proof(file_metadata.clone())
            .map_err(|e| Error::BadRequest(format!("Failed to convert proof: {:?}", e)))?;

        services
            .msp
            .upload_to_msp(&chunks, &file_key_proof)
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;

        // Update the initial chunk index for the next batch.
        batch_start_chunk_index += chunks_in_batch;
    }

    // If the complete file was uploaded to the MSP successfully, we can return the response.
    let bytes_location = file_metadata.location().clone();
    let location = str::from_utf8(&bytes_location)
        .unwrap_or(file_key)
        .to_string();
    let response = FileUploadResponse {
        status: "upload_successful".to_string(),
        file_key: file_key.to_string(),
        bucket_id: bucket_id.to_string(),
        fingerprint: format!("0x{}", hex::encode(trie.get_root())),
        location,
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

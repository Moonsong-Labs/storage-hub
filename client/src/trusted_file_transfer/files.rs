//! File encoding/decoding utilities

use axum::body::Body;
use sc_tracing::tracing::info;
use shc_common::types::{ChunkId, FILE_CHUNK_SIZE};
use shc_file_manager::traits::FileStorageWriteOutcome;
use shp_file_metadata::Chunk;
use tokio_stream::StreamExt;

use crate::types::FileStorageT;

use super::server::Context;

pub const CHUNK_ID_SIZE: usize = 8; // sizeof(u64)

const LOG_TARGET: &str = "trusted-file-transfer-files";

/// Encodes a chunk ID and data pair into the wire format.
pub fn encode_chunk(chunk_id: ChunkId, chunk_data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(CHUNK_ID_SIZE + chunk_data.len());
    encoded.extend_from_slice(&chunk_id.as_u64().to_le_bytes());
    encoded.extend_from_slice(chunk_data);
    encoded
}

/// Process a stream of chunks from the http request body
pub(crate) async fn process_chunk_stream<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
    request_body: Body,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
    FSH: shc_forest_manager::traits::ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: shc_common::traits::StorageEnableRuntime,
{
    let mut request_stream = request_body.into_data_stream();
    let mut buffer = Vec::new();
    let mut last_write_outcome = FileStorageWriteOutcome::FileIncomplete;

    // Process request stream, storing chunks as they are received
    while let Some(try_bytes) = request_stream.next().await {
        let bytes = try_bytes?;
        buffer.extend_from_slice(&bytes);

        while buffer.len() >= CHUNK_ID_SIZE + (FILE_CHUNK_SIZE as usize) {
            let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, true)?;
            last_write_outcome = write_chunk(context, file_key, &chunk_id, &chunk_data).await?;
        }
    }

    // Check if there's remaining data for the last chunk (This is the case when data is
    // smaller than FILE_CHUNK_SIZE)
    if !buffer.is_empty() {
        let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, false)?;
        last_write_outcome = write_chunk(context, file_key, &chunk_id, &chunk_data).await?;
    }

    // Verify the file is complete using the last write outcome
    if matches!(last_write_outcome, FileStorageWriteOutcome::FileComplete) {
        info!(
            target: LOG_TARGET,
            file_key = %file_key,
            "File upload complete"
        );
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "File incomplete after processing all chunks"
        ))
    }
}

/// Gets next chunk from buffer. If cap_at_file_chunk_size is set to true,
/// it will get FILE_CHUNK_SIZE as data size, else it will use the remainder
/// of the buffer (used for last chunk when data is not a multiple of FILE_CHUNK_SIZE).
fn get_next_chunk(
    buffer: &mut Vec<u8>,
    cap_at_file_chunk_size: bool,
) -> anyhow::Result<(ChunkId, Vec<u8>)> {
    let min_data_size: usize = if cap_at_file_chunk_size {
        FILE_CHUNK_SIZE as usize
    } else {
        1
    };
    let min_buffer_size = CHUNK_ID_SIZE + min_data_size;
    if buffer.len() < min_buffer_size {
        return Err(anyhow::anyhow!(
            "Not enough bytes to extract chunk from buffer. Required at least {} bytes got {}.",
            min_buffer_size,
            buffer.len()
        ));
    }

    let chunk_id_bytes: [u8; CHUNK_ID_SIZE] = buffer
        .drain(..CHUNK_ID_SIZE)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse chunk ID"))?;
    let chunk_id_value = u64::from_le_bytes(chunk_id_bytes);
    let chunk_id = ChunkId::new(chunk_id_value);

    let chunk_data = if cap_at_file_chunk_size {
        let size = FILE_CHUNK_SIZE as usize;
        buffer.drain(..size).collect()
    } else {
        std::mem::take(buffer)
    };
    Ok((chunk_id, chunk_data))
}

async fn write_chunk<FL, FSH, Runtime>(
    context: &Context<FL, FSH, Runtime>,
    file_key: &sp_core::H256,
    chunk_id: &ChunkId,
    chunk_data: &Chunk,
) -> anyhow::Result<FileStorageWriteOutcome>
where
    FL: FileStorageT,
    FSH: shc_forest_manager::traits::ForestStorageHandler<Runtime> + Clone + Send + Sync + 'static,
    Runtime: shc_common::traits::StorageEnableRuntime,
{
    let mut file_storage = context.file_storage.write().await;
    file_storage
        .write_chunk(file_key, chunk_id, chunk_data)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to write chunk {} to storage: {}",
                chunk_id.as_u64(),
                e
            )
        })
}

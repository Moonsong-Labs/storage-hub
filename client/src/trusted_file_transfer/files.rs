//! File encoding/decoding utilities

use axum::body::Body;
use sc_tracing::tracing::info;
use shc_common::types::{ChunkId, FILE_CHUNK_SIZE};
use shc_file_manager::traits::FileStorageWriteOutcome;
use shp_file_metadata::Chunk;
use tokio_stream::StreamExt;

use crate::types::FileStorageT;

use tokio::sync::RwLock;

pub const CHUNK_ID_SIZE: usize = 8; // sizeof(u64)

const LOG_TARGET: &str = "trusted-file-transfer-files";

/// Encodes a chunk ID and data pair into the wire format.
pub fn encode_chunk(chunk_id: ChunkId, chunk_data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(CHUNK_ID_SIZE + chunk_data.len());
    encoded.extend_from_slice(&chunk_id.as_u64().to_le_bytes());
    encoded.extend_from_slice(chunk_data);
    encoded
}

/// Get chunks from a request body as a stream and write them to storage
pub(crate) async fn process_chunk_stream<FL>(
    file_storage: &RwLock<FL>,
    file_key: &sp_core::H256,
    request_body: Body,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
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
            last_write_outcome =
                write_chunk(file_storage, file_key, &chunk_id, &chunk_data).await?;
        }
    }

    // Check if there's remaining data for the last chunk (This is the case when data is
    // smaller than FILE_CHUNK_SIZE)
    if !buffer.is_empty() {
        let (chunk_id, chunk_data) = get_next_chunk(&mut buffer, false)?;
        last_write_outcome = write_chunk(file_storage, file_key, &chunk_id, &chunk_data).await?;
    }

    // Verify the file is complete using the last write outcome
    if matches!(last_write_outcome, FileStorageWriteOutcome::FileComplete) {
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

async fn write_chunk<FL>(
    file_storage: &RwLock<FL>,
    file_key: &sp_core::H256,
    chunk_id: &ChunkId,
    chunk_data: &Chunk,
) -> anyhow::Result<FileStorageWriteOutcome>
where
    FL: FileStorageT,
{
    let mut file_storage = file_storage.write().await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use shc_common::types::{FileMetadata, StorageProofsMerkleTrieLayout};
    use shc_file_manager::in_memory::InMemoryFileStorage;
    use shc_file_manager::traits::FileStorage;
    use sp_core::{blake2_256, H256};
    use sp_runtime::AccountId32;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_process_chunk_stream_exact_multiple_of_chunk_size() {
        let chunk_count = 3;
        let file_size = FILE_CHUNK_SIZE * chunk_count;

        let mut encoded_data = Vec::new();
        for i in 0..chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk_data = vec![i as u8; FILE_CHUNK_SIZE as usize];
            encoded_data.extend_from_slice(&encode_chunk(chunk_id, &chunk_data));
        }

        let body = Body::from(encoded_data);
        let mut file_storage = InMemoryFileStorage::<StorageProofsMerkleTrieLayout>::new();
        let file_key = H256::from(blake2_256(b"test_file"));

        let metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            b"test_location".to_vec(),
            file_size,
            [0u8; 32],
        )
        .unwrap();

        file_storage.insert_file(file_key, metadata).unwrap();
        let file_storage = Arc::new(RwLock::new(file_storage));

        let result = process_chunk_stream(&file_storage, &file_key, body)
            .await
            .unwrap();

        let storage = file_storage.read().await;
        for i in 0..chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk = storage.get_chunk(&file_key, &chunk_id);
            assert!(chunk.is_ok(), "Chunk {} not found", i);
            assert_eq!(chunk.unwrap(), vec![i as u8; FILE_CHUNK_SIZE as usize]);
        }
    }

    #[tokio::test]
    async fn test_process_chunk_stream_not_multiple_of_chunk_size() {
        let full_chunk_count = 3;
        let partial_chunk_size = 512;
        let file_size = (FILE_CHUNK_SIZE * full_chunk_count) + partial_chunk_size;

        let mut encoded_data = Vec::new();

        for i in 0..full_chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk_data = vec![i as u8; FILE_CHUNK_SIZE as usize];
            encoded_data.extend_from_slice(&encode_chunk(chunk_id, &chunk_data));
        }

        let last_chunk_id = ChunkId::new(full_chunk_count);
        let last_chunk_data = vec![99u8; partial_chunk_size as usize];
        encoded_data.extend_from_slice(&encode_chunk(last_chunk_id, &last_chunk_data));

        let body = Body::from(encoded_data);
        let mut file_storage = InMemoryFileStorage::<StorageProofsMerkleTrieLayout>::new();

        let file_key = H256::from(blake2_256(b"test_file_partial"));
        let metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            b"test_location".to_vec(),
            file_size,
            [0u8; 32],
        )
        .unwrap();

        file_storage.insert_file(file_key, metadata).unwrap();
        let file_storage = Arc::new(RwLock::new(file_storage));

        // Process the chunk stream
        let result = process_chunk_stream(&file_storage, &file_key, body).await;
        assert!(
            result.is_ok(),
            "process_chunk_stream failed: {:?}",
            result.err()
        );

        // Verify all full chunks were written
        let storage = file_storage.read().await;
        for i in 0..full_chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk = storage.get_chunk(&file_key, &chunk_id);
            assert!(chunk.is_ok(), "Full chunk {} not found", i);
            assert_eq!(chunk.unwrap(), vec![i as u8; FILE_CHUNK_SIZE as usize]);
        }

        // Verify partial chunk was written
        let last_chunk = storage.get_chunk(&file_key, &last_chunk_id);
        assert!(last_chunk.is_ok(), "Partial chunk not found");
        assert_eq!(last_chunk.unwrap(), vec![99u8; partial_chunk_size as usize]);
    }
}

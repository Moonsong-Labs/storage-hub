//! File encoding/decoding utilities

use axum::body::Body;
use bytes::BytesMut;
use log::{error, info};
use shc_common::{
    trusted_file_transfer::{read_chunk_with_id_from_buffer, CHUNK_ID_SIZE},
    types::ChunkId,
};
use shc_file_manager::traits::FileStorageWriteOutcome;
use shp_constants::FILE_CHUNK_SIZE;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use crate::{trusted_file_transfer::server::LOG_TARGET, types::FileStorageT};

/// Get chunks from a request body as a stream and write them to storage
pub(crate) async fn process_chunk_stream<FL>(
    file_storage: &RwLock<FL>,
    file_key: &sp_core::H256,
    batch_target_bytes: usize,
    request_body: Body,
) -> anyhow::Result<()>
where
    FL: FileStorageT,
{
    let mut request_stream = request_body.into_data_stream();
    let mut buffer = BytesMut::new();
    let mut last_write_outcome = FileStorageWriteOutcome::FileIncomplete;
    let mut pending: Vec<(ChunkId, Vec<u8>)> = Vec::new();
    let mut pending_bytes: usize = 0;

    // Process request stream, storing chunks as they are received
    while let Some(try_bytes) = request_stream.next().await {
        let bytes = try_bytes?;
        buffer.extend_from_slice(bytes.as_ref());

        while buffer.len() >= CHUNK_ID_SIZE + (FILE_CHUNK_SIZE as usize) {
            let (chunk_id, chunk_data) = read_chunk_with_id_from_buffer(&mut buffer, true)?;
            pending_bytes += CHUNK_ID_SIZE + chunk_data.len();
            pending.push((chunk_id, chunk_data));

            if pending_bytes >= batch_target_bytes {
                let batch = std::mem::take(&mut pending);
                pending_bytes = 0;
                last_write_outcome = write_chunk_batch(file_storage, file_key, batch).await?;
            }
        }
    }

    // Now that we have read all the "full" chunks, and there is no more data being streamed,
    // we know that if there is data left in the buffer, it represents the last chunk whose data size
    // is smaller than FILE_CHUNK_SIZE (in the case where it is exactly equal, it will be processed
    // in the loop above, and the buffer will be empty).
    if !buffer.is_empty() {
        let (chunk_id, chunk_data) = read_chunk_with_id_from_buffer(&mut buffer, false)?;
        pending.push((chunk_id, chunk_data));
    }

    if !pending.is_empty() {
        let batch = std::mem::take(&mut pending);
        last_write_outcome = write_chunk_batch(file_storage, file_key, batch).await?;
    }

    // Verify the file is complete using the last write outcome
    if matches!(last_write_outcome, FileStorageWriteOutcome::FileComplete) {
        info!(
            target: LOG_TARGET,
            "File [{:x}] processed successfully",
            file_key
        );
        Ok(())
    } else {
        error!(
            target: LOG_TARGET,
            "File [{:x}] incomplete after processing all data streamed",
            file_key
        );
        Err(anyhow::anyhow!(
            "File [{:x}] incomplete after processing all data streamed",
            file_key
        ))
    }
}

async fn write_chunk_batch<FL>(
    file_storage: &RwLock<FL>,
    file_key: &sp_core::H256,
    batch: Vec<(ChunkId, Vec<u8>)>,
) -> anyhow::Result<FileStorageWriteOutcome>
where
    FL: FileStorageT,
{
    let mut storage = file_storage.write().await;
    storage
        .write_chunks_batched_trusted(file_key, batch)
        .map_err(|e| anyhow::anyhow!("Failed to write chunk batch to storage: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shc_common::{
        trusted_file_transfer::encode_chunk_with_id,
        types::{FileMetadata, StorageProofsMerkleTrieLayout},
    };
    use shc_file_manager::{in_memory::InMemoryFileStorage, traits::FileStorage};
    use sp_core::{blake2_256, H256};
    use sp_runtime::AccountId32;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_process_chunk_stream_exact_multiple_of_chunk_size() {
        use shc_file_manager::in_memory::InMemoryFileDataTrie;
        use shc_file_manager::traits::FileDataTrie;

        let chunk_count = 3;
        let file_size = FILE_CHUNK_SIZE * chunk_count;

        let mut temp_trie = InMemoryFileDataTrie::<StorageProofsMerkleTrieLayout>::new();
        for i in 0..chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk_data = vec![i as u8; FILE_CHUNK_SIZE as usize];
            temp_trie.write_chunk(&chunk_id, &chunk_data).unwrap();
        }
        let expected_fingerprint = temp_trie.get_root().as_ref();

        let mut encoded_data = Vec::new();
        for i in 0..chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk_data = vec![i as u8; FILE_CHUNK_SIZE as usize];
            encoded_data.extend_from_slice(&encode_chunk_with_id(chunk_id, &chunk_data));
        }

        let body = Body::from(encoded_data);
        let mut file_storage = InMemoryFileStorage::<StorageProofsMerkleTrieLayout>::new();
        let file_key = H256::from(blake2_256(b"test_file"));

        let metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            b"test_location".to_vec(),
            file_size,
            expected_fingerprint.into(),
        )
        .unwrap();

        file_storage.insert_file(file_key, metadata).unwrap();
        let file_storage = Arc::new(RwLock::new(file_storage));

        process_chunk_stream(
            &file_storage,
            &file_key,
            crate::trusted_file_transfer::server::DEFAULT_BATCH_TARGET_BYTES,
            body,
        )
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
        use shc_file_manager::in_memory::InMemoryFileDataTrie;
        use shc_file_manager::traits::FileDataTrie;

        let full_chunk_count = 3;
        let partial_chunk_size = 512;
        let file_size = (FILE_CHUNK_SIZE * full_chunk_count) + partial_chunk_size;

        let mut temp_trie = InMemoryFileDataTrie::<StorageProofsMerkleTrieLayout>::new();
        for i in 0..full_chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk_data = vec![i as u8; FILE_CHUNK_SIZE as usize];
            temp_trie.write_chunk(&chunk_id, &chunk_data).unwrap();
        }
        let last_chunk_id = ChunkId::new(full_chunk_count);
        let last_chunk_data = vec![99u8; partial_chunk_size as usize];
        temp_trie
            .write_chunk(&last_chunk_id, &last_chunk_data)
            .unwrap();
        let expected_fingerprint = temp_trie.get_root().as_ref();

        let mut encoded_data = Vec::new();

        for i in 0..full_chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk_data = vec![i as u8; FILE_CHUNK_SIZE as usize];
            encoded_data.extend_from_slice(&encode_chunk_with_id(chunk_id, &chunk_data));
        }

        encoded_data.extend_from_slice(&encode_chunk_with_id(last_chunk_id, &last_chunk_data));

        let body = Body::from(encoded_data);
        let mut file_storage = InMemoryFileStorage::<StorageProofsMerkleTrieLayout>::new();

        let file_key = H256::from(blake2_256(b"test_file_partial"));
        let metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            b"test_location".to_vec(),
            file_size,
            expected_fingerprint.into(),
        )
        .unwrap();

        file_storage.insert_file(file_key, metadata).unwrap();
        let file_storage = Arc::new(RwLock::new(file_storage));

        process_chunk_stream(
            &file_storage,
            &file_key,
            crate::trusted_file_transfer::server::DEFAULT_BATCH_TARGET_BYTES,
            body,
        )
        .await
        .unwrap();

        let storage = file_storage.read().await;
        for i in 0..full_chunk_count {
            let chunk_id = ChunkId::new(i);
            let chunk = storage.get_chunk(&file_key, &chunk_id);
            assert!(chunk.is_ok(), "Full chunk {} not found", i);
            assert_eq!(chunk.unwrap(), vec![i as u8; FILE_CHUNK_SIZE as usize]);
        }

        let last_chunk = storage.get_chunk(&file_key, &last_chunk_id);
        assert!(last_chunk.is_ok(), "Partial chunk not found");
        assert_eq!(last_chunk.unwrap(), vec![99u8; partial_chunk_size as usize]);
    }
}

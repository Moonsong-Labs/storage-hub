//! File encoding/decoding utilities

use axum::body::Body;
use shc_common::{
    trusted_file_transfer::{read_chunk_with_id_from_buffer, CHUNK_ID_SIZE},
    types::{ChunkId, FILE_CHUNK_SIZE},
};
use shc_file_manager::traits::FileStorageWriteOutcome;
use shp_file_metadata::Chunk;
use tokio_stream::StreamExt;

use crate::types::FileStorageT;

use tokio::sync::RwLock;

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
            let (chunk_id, chunk_data) = read_chunk_with_id_from_buffer(&mut buffer, true)?;
            last_write_outcome =
                write_chunk(file_storage, file_key, &chunk_id, &chunk_data).await?;
        }
    }

    // Check if there's remaining data for the last chunk (This is the case when data is
    // smaller than FILE_CHUNK_SIZE)
    if !buffer.is_empty() {
        let (chunk_id, chunk_data) = read_chunk_with_id_from_buffer(&mut buffer, false)?;
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
    use shc_common::trusted_file_transfer::encode_chunk_with_id;
    use shc_common::types::{FileMetadata, StorageProofsMerkleTrieLayout};
    use shc_file_manager::in_memory::InMemoryFileStorage;
    use shc_file_manager::traits::FileStorage;
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

        process_chunk_stream(&file_storage, &file_key, body)
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

        process_chunk_stream(&file_storage, &file_key, body)
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

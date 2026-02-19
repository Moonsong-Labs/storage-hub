use crate::types::{ChunkId, FILE_CHUNK_SIZE};
use bytes::BytesMut;

pub const CHUNK_ID_SIZE: usize = 8; // sizeof(u64)

/// Encodes a chunk ID and data pair into the wire format.
pub fn encode_chunk_with_id(chunk_id: ChunkId, chunk_data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(CHUNK_ID_SIZE + chunk_data.len());
    encoded.extend_from_slice(&chunk_id.as_u64().to_le_bytes());
    encoded.extend_from_slice(chunk_data);
    encoded
}

/// Reads the next chunk record from a `BytesMut` buffer in O(1) time.
///
/// This avoids `Vec::drain(..)` shifting costs and is suitable for high-throughput streaming.
///
/// Wire format:
/// `[ChunkId: 8 bytes (u64, little-endian)][Chunk data: N bytes]...`
///
/// If `cap_at_file_chunk_size` is set to true, it will read `FILE_CHUNK_SIZE` bytes of data.
/// Otherwise it will read the remainder of the buffer (used for the last chunk when data is not a
/// multiple of `FILE_CHUNK_SIZE`).
pub fn read_chunk_with_id_from_buffer(
    buffer: &mut BytesMut,
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

    let chunk_id_bytes = buffer.split_to(CHUNK_ID_SIZE);
    let chunk_id_bytes: [u8; CHUNK_ID_SIZE] = chunk_id_bytes
        .as_ref()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse chunk ID"))?;
    let chunk_id_value = u64::from_le_bytes(chunk_id_bytes);
    let chunk_id = ChunkId::new(chunk_id_value);

    let chunk_data = if cap_at_file_chunk_size {
        let size = FILE_CHUNK_SIZE as usize;
        buffer.split_to(size).to_vec()
    } else {
        buffer.split().to_vec()
    };

    Ok((chunk_id, chunk_data))
}

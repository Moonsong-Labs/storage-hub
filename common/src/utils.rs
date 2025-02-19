/// Calculates the size of a chunk at a given index for a file.
///
/// # Arguments
/// * `chunk_idx` - The index of the chunk (0-based)
/// * `total_chunks` - Total number of chunks in the file
/// * `file_size` - Total size of the file in bytes
/// * `chunk_size` - Size of a standard chunk in bytes
///
/// # Returns
/// The size of the chunk in bytes
///
/// This function handles the special case where the file size is an exact multiple
/// of the chunk size, ensuring the last chunk is properly sized.
pub fn calculate_chunk_size(
    chunk_idx: u64,
    total_chunks: u64,
    file_size: u64,
    chunk_size: u64,
) -> usize {
    if chunk_idx == total_chunks - 1 {
        // For the last chunk
        let remaining = file_size - (chunk_idx * chunk_size);
        remaining as usize
    } else {
        // For all other chunks
        chunk_size as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_chunk_size() {
        const CHUNK_SIZE: u64 = 1024;

        // Test case 1: File size is exact multiple of chunk size
        let file_size = 2048; // 2 chunks exactly
        assert_eq!(calculate_chunk_size(0, 2, file_size, CHUNK_SIZE), 1024);
        assert_eq!(calculate_chunk_size(1, 2, file_size, CHUNK_SIZE), 1024);

        // Test case 2: File size is not multiple of chunk size
        let file_size = 2500; // 2 full chunks + 1 partial
        assert_eq!(calculate_chunk_size(0, 3, file_size, CHUNK_SIZE), 1024);
        assert_eq!(calculate_chunk_size(1, 3, file_size, CHUNK_SIZE), 1024);
        assert_eq!(calculate_chunk_size(2, 3, file_size, CHUNK_SIZE), 452);

        // Test case 3: Single chunk file
        let file_size = 500;
        assert_eq!(calculate_chunk_size(0, 1, file_size, CHUNK_SIZE), 500);
    }
}

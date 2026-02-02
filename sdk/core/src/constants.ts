/// The file chunk size in bytes. This is the size of the leaf nodes in the Merkle
/// Patricia Trie that is constructed for each file.
/// Each chunk is 1 kB.
export const CHUNK_SIZE = 1024;
/**
 * Batch size for fast zero-copy fingerprinting (bytes), aligned to CHUNK_SIZE
 * Default: 128 MiB
 */
export const BATCH_SIZE_BYTES = Math.floor((128 * 1024 * 1024) / CHUNK_SIZE) * CHUNK_SIZE;
/**
 * Upper bound for in-WASM fingerprinting. Above this size, the in-memory
 * Merkle-Patricia trie risks exhausting 32-bit WASM linear memory.
 */
export const MAX_WASM_FINGERPRINT_BYTES = 1_610_612_736; // ≈ 1.5 GiB


// Encryiption constants
export const ENCRYPTION_CHUNK_SIZE = 16 * 1024 * 1024; // 16 MB
export const NONCE_SIZE = 12;
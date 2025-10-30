/// The file chunk size in bytes. This is the size of the leaf nodes in the Merkle
/// Patricia Trie that is constructed for each file.
/// Each chunk is 1 kB.
export const CHUNK_SIZE = 1024;
/**
 * Upper bound for in-WASM fingerprinting. Above this size, the in-memory
 * Merkle-Patricia trie risks exhausting 32-bit WASM linear memory.
 */
export const MAX_WASM_FINGERPRINT_BYTES = 1_610_612_736; // ≈ 1.5 GiB

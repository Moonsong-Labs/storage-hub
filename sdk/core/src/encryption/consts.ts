// Base encryption primitives
export const NONCE_SIZE = 12;
export const DEK_SIZE = 32;
export const IKM_SIZE = 32;
export const SALT_SIZE = 32;
export const MIN_PASSWORD_SIZE = 8;

// File encryption framing
export const ENCRYPTION_CHUNK_SIZE = 16 * 1024 * 1024; // 16 MiB
export const AEAD_TAG_SIZE_BYTES = 16;

// Per-chunk AAD layout: [version(1), kind(1), chunk_index_u64_be(8), header_hash(32)]
export const HEADER_HASH_SIZE_BYTES = 32;
export const CHUNK_AAD_SIZE_BYTES = 1 + 1 + 8 + HEADER_HASH_SIZE_BYTES;
export const CHUNK_AAD_VERSION = 1;
export const CHUNK_AAD_KIND_DATA = 0;
export const CHUNK_AAD_KIND_COMMIT = 1;

// Authenticated commit trailer to bind final plaintext totals.
export const COMMIT_MAGIC: Readonly<Uint8Array> = new TextEncoder().encode("SHC1");
export const COMMIT_PLAINTEXT_SIZE_BYTES = COMMIT_MAGIC.length + 8 + 8;
export const COMMIT_CIPHERTEXT_SIZE_BYTES = COMMIT_PLAINTEXT_SIZE_BYTES + AEAD_TAG_SIZE_BYTES;

export const MAX_SAFE_INTEGER_BIGINT = BigInt(Number.MAX_SAFE_INTEGER);

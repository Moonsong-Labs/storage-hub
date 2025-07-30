# 1. File Key Construction using Complete FileMetadata

Date: 2025-07-30

## Status

Accepted

## Context

StorageHub uses a merkle trie structure (known as the merkle forest) to organize and verify file storage across BSPs and buckets stored by MSPs. In this structure:

- **File keys** serve as leaf node keys in the merkle trie
- **File metadata** serves as the corresponding leaf node values
- The fisherman service constructs and maintains these tries for verification purposes

The file key construction method must be deterministic and consistent across all network participants to ensure merkle trie integrity and enable proper fisherman verification.

## Decision

File keys will be constructed using the complete `FileMetadata` structure as defined in the codebase:

```rust
pub struct FileMetadata<const H_LENGTH: usize, const CHUNK_SIZE: u64, const SIZE_TO_CHALLENGES: u64> {
    owner: Vec<u8>,           // Account ID of the file owner
    bucket_id: Vec<u8>,       // Identifier of the containing bucket
    location: Vec<u8>,        // File path/location within the bucket
    #[codec(compact)]
    file_size: u64,           // Size of the file in bytes
    fingerprint: Fingerprint<H_LENGTH>, // Content-based fingerprint
}
```

**All fields are required** for file key construction to serve as leaf node keys in the merkle forest, with the FileMetadata itself serving as the leaf node values.

## Consequences

**Positive:**

- Enables deterministic merkle trie construction across all network participants
- File keys uniquely identify files within the merkle forest structure
- Fisherman service can verify BSP and bucket integrity using consistent trie construction
- Supports the ephemeral trie approach for efficient fisherman catch-up

**Negative:**

- File key construction requires complete metadata, not just content fingerprint
- Any service constructing file keys must have access to all FileMetadata fields
- Events must include complete FileMetadata to enable trie construction (addressed by RFC-02)

**Risk Management:**

- Services must ensure FileMetadata consistency to maintain merkle trie integrity
- Event structures must provide complete metadata to prevent fisherman service failures

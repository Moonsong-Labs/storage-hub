# Fisherman Ephemeral Merkle Trie Specification

## Overview

The Fisherman service requires the ability to construct ephemeral (in-memory) Merkle tries on-demand to generate proofs of inclusion for file deletion operations. This specification defines how the fisherman service will build these tries dynamically using a two-phase approach: first by fetching all file keys from finalized data in the indexer database, then by applying a catch-up process to sync to the best block.

## Motivation

The Fisherman role in StorageHub cannot maintain persistent forest storage because:

1. It must track unfinalized blockchain data across multiple forks
2. Forest state changes rapidly between finalized and best blocks
3. Each deletion proof must reflect the current state at the best block

## Requirements

### Functional Requirements

1. **Dynamic Trie Construction**: Build ephemeral Merkle tries on-demand for each deletion target
2. **Comprehensive File Key Retrieval**: Fetch all file keys associated with a BSP or Bucket from finalized indexer data
3. **Catch-Up Mechanism**: Apply file key changes from finalized to best block
4. **Proof Generation**: Generate valid proofs of inclusion for file keys to be deleted at the best block
5. **Memory-Only Operation**: Use only in-memory forest storage without persistence

### Non-Functional Requirements

1. **Memory Efficiency**: Operate within available system RAM constraints
2. **Correctness**: Ensure proof validity for on-chain verification
3. **Isolation**: Each deletion target gets its own independent trie

## Technical Design

### File Key Retrieval

The indexer database must provide methods to retrieve all file keys for:

- A specific BSP (by onchain BSP ID)
- A specific Bucket (by onchain bucket ID)

These queries operate on finalized blockchain data only.

### Two-Phase Trie Construction Process

#### Phase 1: Build from Finalized Data

1. **Get Last Finalized Block**: Determine the current finalized block number
2. **Fetch File Keys**: Query the indexer for all file keys belonging to the deletion target (finalized data)
3. **Create Ephemeral Storage**: Instantiate a new `InMemoryForestStorage` instance
4. **Insert File Keys**: Insert all retrieved file keys into the ephemeral trie

#### Phase 2: Catch-Up to Best Block

5. **Get File Key Changes**: Call the Fisherman command to get file key operations from finalized to best block
6. **Apply Operations**: For each file key returned, either insert (add) or remove (delete) from the trie
7. **Generate Proof**: Create proof of inclusion for the specific file key to be deleted using the updated trie

### Forest Storage Configuration

The Fisherman service will be configured to use only in-memory forest storage:

- No RocksDB or persistent storage options
- Forest storage handler will create fresh instances per deletion operation
- No configuration flags needed - in-memory is the only supported mode

## API Contracts

### Indexer Database Interface

Existing query methods for finalized data:

```rust
// In client/indexer-db/src/models/bsp.rs
impl BspFile {
    /// Get all file keys stored by a specific BSP (from finalized data)
    pub async fn get_all_file_keys_for_bsp<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: &str,
    ) -> Result<Vec<Vec<u8>>, diesel::result::Error>;
}

// In client/indexer-db/src/models/file.rs
impl File {
    /// Get all file keys in a specific bucket (from finalized data)
    pub async fn get_all_file_keys_for_bucket<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: &[u8],
    ) -> Result<Vec<Vec<u8>>, diesel::result::Error>;
}
```

### Fisherman Command Interface

New command for catch-up process:

```rust
// Pseudo-code for new Fisherman command
pub enum FileKeyOperation {
    Add,
    Remove,
}

pub struct FileKeyChange {
    pub file_key: Vec<u8>,
    pub operation: FileKeyOperation,
}

// In fisherman service commands
pub async fn get_file_key_changes_since_block(
    from_block: BlockNumber,
    provider: DeletionTarget, // BSP ID or Bucket ID
) -> Result<Vec<FileKeyChange>, Error>;
```

### Process Flow

```mermaid
flowchart TD
    A[File Deletion Event] --> B[Identify Deletion Targets]
    B --> C{For Each Target}
    C --> D[Get Last Finalized Block]
    D --> E[Query Indexer for All File Keys<br/>(Finalized Data)]
    E --> F[Create Ephemeral Forest Storage]
    F --> G[Insert All File Keys into Trie]
    G --> H[Call GetFileKeyChangesSinceBlock<br/>(from finalized to best)]
    H --> I[Apply Add/Remove Operations]
    I --> J[Generate Proof for File to Delete]
    J --> K[Submit Proof to Blockchain]
    C --> C
```

## Implementation Considerations

### Memory Management

- Each trie construction allocates memory proportional to the number of file keys
- No memory limits enforced - relies on available system RAM
- Parallel trie construction (for bucket + multiple BSPs) may cause high memory usage

### Concurrency

- Multiple deletion targets within a single task build tries in parallel
- Each target gets its own independent ephemeral trie
- No caching or trie reuse between operations

### Error Handling

- Out of memory conditions will cause task failure
- No partial trie construction - all file keys must be inserted successfully
- Failed proof generation aborts the deletion operation
- Catch-up errors fail the specific thread for that BSP/bucket while allowing other threads to continue

## Future Considerations

### Race Conditions

Multiple concurrent fisherman tasks processing deletions for the same bucket may experience race conditions:

- First task succeeds with valid proof
- Second task's proof becomes invalid due to changed forest root
- May require task coordination or retry mechanisms in the future

### Performance Optimizations

- **Pagination**: For very large BSPs/buckets, implement paginated queries to reduce memory pressure
- **Memory Monitoring**: Add guardrails to pause/wait when system memory is critically low
- **Batching**: Consider batching multiple deletions into single trie construction where possible

### Scalability

As the network grows, consider:

- Maximum file key limits per trie
- Streaming trie construction for extremely large datasets
- Memory pool management for parallel operations

## Testing Strategy

1. **Unit Tests**: Test ephemeral trie construction with various file key counts
2. **Integration Tests**: Verify proof generation against known forest states
3. **Memory Tests**: Validate behavior under memory pressure scenarios
4. **Concurrency Tests**: Ensure parallel trie construction works correctly

## Related Architecture Decisions

This specification assumes the existence of:

- Indexer database with comprehensive file tracking (finalized data)
- In-memory forest storage implementation
- Fisherman service actor framework with command pattern
- Blockchain service for best block tracking

Future ADRs may address:

- Memory management strategies for StorageHub services
- Concurrency control for forest mutations
- Proof generation optimization techniques
- Handling of blockchain reorganizations during catch-up

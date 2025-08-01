# RFC: Ephemeral Merkle Trie Design for Fisherman Service

**Title**: Ephemeral Merkle Trie Construction for File Deletion Proofs
**Author**: StorageHub Team
**Date**: 2025-01-29
**Status**: Superseded by [RFC-01](./01-RFC-finalized-data-catchup.md)

## Summary

This RFC proposes a design for constructing ephemeral (in-memory) Merkle tries within the Fisherman service to generate proofs of inclusion for file deletion operations. The design enables dynamic trie construction by fetching all relevant file keys from the indexer database at the time of deletion.

## Motivation

The Fisherman service requires proof generation capabilities but cannot maintain persistent forest storage due to:

- Tracking unfinalized blockchain data across multiple forks
- Rapidly changing file associations that invalidate cached state
- Need for point-in-time accurate proofs

Current implementation attempts to use persistent forest storage which doesn't contain all necessary file keys, making proof generation impossible.

## Detailed Design

### Core Components

#### 1. Indexer Query Interface

Extend the indexer database models with bulk file key retrieval:

```rust
// Pseudo-code for indexer extensions
FUNCTION get_all_file_keys_for_bsp(bsp_id: String) -> Vec<FileKey>:
    SELECT file.file_key
    FROM bsp_file
    JOIN file ON bsp_file.file_id = file.id
    JOIN bsp ON bsp_file.bsp_id = bsp.id
    WHERE bsp.onchain_bsp_id = bsp_id

FUNCTION get_all_file_keys_for_bucket(bucket_id: Bytes) -> Vec<FileKey>:
    SELECT file_key
    FROM file
    JOIN bucket ON file.bucket_id = bucket.id
    WHERE bucket.onchain_bucket_id = bucket_id
```

#### 2. Ephemeral Trie Construction

Modified `process_deletion_for_target` workflow:

```rust
// Pseudo-code for ephemeral trie construction
FUNCTION process_deletion_for_target(target: DeletionTarget, file_key: FileKey):
    // Step 1: Fetch all file keys for the target
    file_keys = MATCH target:
        BSP(id) -> indexer.get_all_file_keys_for_bsp(id)
        Bucket(id) -> indexer.get_all_file_keys_for_bucket(id)

    // Step 2: Create ephemeral forest storage
    ephemeral_forest = InMemoryForestStorage::new()

    // Step 3: Build trie by inserting all file keys
    FOR EACH key IN file_keys:
        metadata = create_minimal_file_metadata(key)
        ephemeral_forest.insert_file_metadata(metadata)

    // Step 4: Generate proof for the specific file key
    proof = ephemeral_forest.generate_proof([file_key])

    // Step 5: Use proof for deletion extrinsic
    RETURN proof
```

#### 3. Forest Storage Handler Configuration

The Fisherman service initialization:

```rust
// Pseudo-code for service configuration
FUNCTION create_fisherman_service():
    // Always use in-memory forest storage handler
    forest_handler = ForestStorageCaching<Vec<u8>, InMemoryForestStorage>::new()

    // No configuration options - in-memory only
    fisherman_service = FishermanService {
        forest_storage_handler: forest_handler,
        ...
    }
```

### Memory Management Strategy

Since there's no cap on trie size and we rely on available RAM:

1. **Sequential Processing**: Process deletion targets one at a time within reason
2. **Immediate Cleanup**: Release ephemeral tries after proof generation
3. **Parallel Limits**: Allow parallel construction for bucket + BSPs within single task only

### Error Handling

- **Memory Exhaustion**: Task fails and error propagates to event system
- **Missing File Keys**: If indexer returns empty set, generate proof for empty trie
- **Database Errors**: Retry with exponential backoff or fail task

## Alternatives Considered

### 1. Persistent Forest Caching

**Approach**: Maintain persistent forests updated from blockchain events
**Rejected Because**: Cannot handle unfinalized data and multiple forks correctly

### 2. Incremental Trie Updates

**Approach**: Start with cached trie and apply deltas
**Rejected Because**: Complexity of tracking deltas across forks outweighs benefits

### 3. Proof Request Service

**Approach**: Separate service maintains forests and provides proofs on request
**Rejected Because**: Adds architectural complexity and potential bottleneck

## Unresolved Questions

1. **Memory Limits**: Should we implement soft limits with warnings before hard failures?
2. **Metrics**: What operational metrics would be useful without adding monitoring overhead?
3. **Proof Caching**: Could we cache proofs (not tries) for recently seen file keys?

## Implementation Plan

### Phase 1: Indexer Extensions

- Add bulk file key query methods
- Optimize queries for large result sets
- Add appropriate database indexes

### Phase 2: Ephemeral Trie Integration

- Modify `process_deletion_for_target` to use ephemeral tries
- Remove forest storage handler usage from deletion flow
- Add proper error handling

### Phase 3: Testing and Optimization

- Unit tests for trie construction
- Integration tests with real indexer data
- Performance profiling with large datasets

### Future Phases

- Implement race condition handling
- Add memory usage guardrails
- Consider pagination for extremely large tries

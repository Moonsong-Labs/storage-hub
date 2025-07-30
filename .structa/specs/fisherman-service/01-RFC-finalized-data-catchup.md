# RFC: Finalized Data with Catch-Up Mechanism for Fisherman Service

**Title**: Two-Phase Ephemeral Trie Construction with Finalized Data and Catch-Up
**Author**: StorageHub Team
**Date**: 2025-01-29
**Status**: Implemented

## Summary

This RFC proposes an evolution of the ephemeral Merkle trie design to use a two-phase approach: first building from finalized indexer data, then applying a catch-up mechanism to sync to the best block. This ensures proofs are valid for the current chain tip while leveraging the stability of finalized data.

## Motivation

The original design (RFC-00) assumed the indexer would track unfinalized data. However, the indexer only tracks finalized blockchain data. Since deletion proofs must be valid at the best block (not finalized), we need a mechanism to bridge this gap.

Key challenges:

- Indexer data lags behind the best block by finalization delay
- File operations occurring between finalized and best blocks must be accounted for
- Proofs generated from outdated state will be rejected on-chain

## Detailed Design

### Overview

The solution introduces a catch-up phase after building the initial trie from finalized data:

1. **Phase 1**: Build ephemeral trie from all file keys in finalized indexer data
2. **Phase 2**: Query recent blocks for file key changes and apply them to the trie
3. Generate proof from the updated trie that reflects best block state

### Core Components

#### 1. Fisherman Command: GetFileKeyChangesSinceBlock

A new command that tracks file key operations across recent blocks:

```rust
// Pseudo-code for new Fisherman command
pub enum FileKeyOperation {
    // Add operation with optional metadata (when events include it)
    Add(Option<FileMetadata>),
    Remove,
}

pub struct FileKeyChange {
    pub file_key: Vec<u8>,
    pub operation: FileKeyOperation,
}

FUNCTION get_file_key_changes_since_block(
    from_block: BlockNumber,
    provider: DeletionTarget  // BSP ID or Bucket ID
) -> Vec<FileKeyChange>:

    // Get current best block
    best_block = blockchain_service.get_best_block()

    // Fetch all blocks between from_block and best_block
    blocks = blockchain_service.get_blocks_range(from_block + 1, best_block)

    // Track file key operations
    file_key_states = HashMap<FileKey, FileKeyOperation>()

    FOR EACH block IN blocks:
        events = get_fisherman_relevant_events(block, provider)

        FOR EACH event IN events:
            MATCH event:
                FileUploadSuccess(key) ->
                    file_key_states[key] = Add
                FileDeletionSuccess(key) ->
                    file_key_states[key] = Remove
                BspStorageSuccess(bsp_id, key) IF provider == BSP(bsp_id) ->
                    file_key_states[key] = Add
                BspStorageFailed(bsp_id, key) IF provider == BSP(bsp_id) ->
                    file_key_states[key] = Remove
                MspAcceptedStorageRequest(msp_id, key) IF provider == Bucket(bucket_id) ->
                    file_key_states[key] = Add
                // Other relevant events...

    // Return final state for each file key
    RETURN file_key_states.into_vec()
```

#### 2. Event Tracking Logic

The command processes events chronologically to determine final file key state:

- If a file key has multiple operations, only the last one matters
- Events are processed in block order to maintain consistency
- Only events relevant to the specific provider (BSP/Bucket) are considered

#### 3. Modified Trie Construction Process

```rust
// Pseudo-code for two-phase trie construction
FUNCTION process_deletion_for_target(target: DeletionTarget, file_key: FileKey):
    // Phase 1: Build from finalized data
    finalized_block = blockchain_service.get_finalized_block()

    file_keys = MATCH target:
        BSP(id) -> indexer.get_all_file_keys_for_bsp(id)
        Bucket(id) -> indexer.get_all_file_keys_for_bucket(id)

    ephemeral_forest = InMemoryForestStorage::new()

    FOR EACH key IN file_keys:
        metadata = create_minimal_file_metadata(key)
        ephemeral_forest.insert_file_metadata(metadata)

    // Phase 2: Apply catch-up
    changes = fisherman_service.get_file_key_changes_since_block(
        finalized_block,
        target
    )

    FOR EACH change IN changes:
        MATCH change.operation:
            Add -> ephemeral_forest.insert_file_metadata(
                create_minimal_file_metadata(change.file_key)
            )
            Remove -> ephemeral_forest.remove_file_metadata(change.file_key)

    // Generate proof from updated trie
    proof = ephemeral_forest.generate_proof([file_key])
    RETURN proof
```

### Error Handling

- **Catch-up failures**: If the catch-up command fails, the specific thread processing that BSP/bucket fails while other threads continue
- **Best block changes**: Ignored during processing - we use the best block at catch-up time
- **Reorgs**: Not handled - assumed to be rare enough to retry on failure

### Performance Considerations

- Catch-up typically processes ~10-100 blocks (depending on finalization delay)
- Event filtering reduces the number of operations to apply
- File key state tracking prevents redundant operations

## Alternatives Considered

### 1. Always Query Unfinalized Data

**Approach**: Modify indexer to track unfinalized data
**Rejected Because**: Requires significant indexer changes and fork handling complexity

### 2. Generate Proofs at Finalized Block

**Approach**: Only generate proofs for finalized state
**Rejected Because**: Proofs would be stale and likely rejected on-chain

### 3. Full Trie Rebuild from Genesis

**Approach**: Build trie from all historical events
**Rejected Because**: Prohibitively expensive for large datasets

## Unresolved Questions

1. **Event Types**: Should we track additional events beyond file upload/deletion and BSP storage success/failed?
2. **Optimization**: Could we cache the catch-up results for recently processed blocks?
3. **Monitoring**: What metrics should we expose for catch-up performance?

## Implementation Plan

### Phase 1: Fisherman Command Implementation

- Implement `get_file_key_changes_since_block` command
- Add event filtering logic for specific providers
- Test with various block ranges

### Phase 2: Task Integration

- Modify file deletion task to use two-phase approach
- Update error handling for catch-up failures
- Add logging for debugging

### Phase 3: Testing

- Unit tests for command logic
- Integration tests with real blockchain data
- Performance testing with large block ranges

### Migration

- No migration needed - this enhances the existing ephemeral trie approach
- Existing RFC-00 remains valid as the foundation

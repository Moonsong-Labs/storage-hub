# RFC: Event Metadata Enhancement for Fisherman Service

**Title**: Add File Metadata to Storage Provider Events
**Author**: StorageHub Team
**Date**: 2025-01-29
**Status**: Accepted

## Summary

This RFC proposes enhancing specific pallet-file-system events to include complete file metadata. This enhancement will eliminate the need for the fisherman service to query external sources (indexer or chain storage) during the catch-up phase, making the process more efficient and reliable.

## Motivation

The fisherman service constructs merkle tries on-demand (known as ephemeral merkle forest) to verify file storage across BSPs and buckets stored by MSPs. In this structure:

- **File keys** serve as leaf node keys in the merkle trie
- **File metadata** serves as the corresponding leaf node values

File keys are constructed from complete `FileMetadata` as defined in [ADR-001](../../decisions/001-file-key-construction-using-complete-filemetadata.md). Currently, when the fisherman service processes events during catch-up (as described in RFC-01), certain events lack this complete file metadata:

- `BspConfirmedStoring`: Only contains file keys, not the full metadata needed for trie construction
- `MspAcceptedStorageRequest`: Missing complete file metadata required to generate file keys and populate trie values

This creates a **deterministic requirement** (not just an optimization) because the fisherman service cannot construct proper merkle tries without access to complete FileMetadata. The current approach forces the fisherman service to:

1. Query the indexer (which may not have unfinalized data)
2. Fall back to runtime API calls (but storage requests may no longer be in state at that point in time)

Both approaches are **unreliable** and can cause fisherman service failures during catch-up, preventing proper ephemeral merkle forest construction and verification.

## Detailed Design

### Events to Enhance

#### 1. BspConfirmedStoring Event

Current structure:

```rust
BspConfirmedStoring {
    bsp_id: StorageProviderId,
    confirmed_file_keys: Vec<FileKey>,
    // ... other fields
}
```

Proposed structure:

```rust
BspConfirmedStoring {
    bsp_id: StorageProviderId,
    confirmed_file_keys: Vec<(FileKey, FileMetadata)>,
    // ... other fields
}
```

#### 2. MspAcceptedStorageRequest Event

Current structure:

```rust
MspAcceptedStorageRequest {
    msp_id: StorageProviderId,
    file_key: FileKey,
    // ... other fields
}
```

Proposed structure:

```rust
MspAcceptedStorageRequest {
    msp_id: StorageProviderId,
    file_key: FileKey,
    file_metadata: FileMetadata,
    // ... other fields
}
```

### FileMetadata Structure

The `FileMetadata` should include:

- `owner`: AccountId of the file owner
- `bucket_id`: The bucket containing the file
- `location`: File location/path
- `size`: File size in bytes
- `fingerprint`: File content fingerprint

### Benefits

1. **Efficiency**: No external queries needed during event processing
2. **Reliability**: Eliminates dependency on indexer availability for unfinalized data
3. **Simplicity**: Reduces complexity in the fisherman service implementation
4. **Performance**: Faster catch-up phase without additional network calls

### Impact on Existing Code

This change affects:

- Pallet event definitions
- Event emission logic in the pallet
- Event processing in client code
- Storage requirements (slightly larger events)

## Alternatives Considered

### 1. Runtime API for Batch Metadata Queries

**Approach**: Add a runtime API to efficiently query multiple file metadata at once
**Rejected Because**: Still requires the storage request to exist in state

### 2. Separate Metadata Events

**Approach**: Emit separate events containing just file metadata
**Rejected Because**: Increases event processing complexity and storage overhead

### 3. Keep Current Approach

**Approach**: Continue querying indexer/chain as needed
**Rejected Because**: Maintains current complexity and potential failure points

### Fisherman Service Command Updates

The fisherman service command structure will be updated to always return complete file metadata:

**Current Structure:**

```rust
pub enum FileKeyOperation {
    /// File key was added with optional metadata (Some when available, None when pending)
    Add(Option<shc_common::types::FileMetadata>),
    /// File key was removed
    Remove,
}
```

**Proposed Structure:**

```rust
pub enum FileKeyOperation {
    /// File key was added with complete metadata (always available after this RFC)
    Add(shc_common::types::FileMetadata),
    /// File key was removed
    Remove,
}
```

This change ensures that the fisherman service can guarantee file metadata availability when returning file key changes, eliminating the need to handle `None` cases in the catch-up process.

## Implementation Plan

### Phase 1: Runtime Changes

- Update event structures in `pallet-file-system`:
  - Modify `BspConfirmedStoring` to include `Vec<(FileKey, FileMetadata)>` in confirmed_file_keys
  - Modify `MspAcceptedStorageRequest` to include `file_metadata: FileMetadata`
- Update event emission logic to include complete metadata when events are created
- Update runtime tests to verify new event format and metadata inclusion

### Phase 2: Client Updates

- Update `FileKeyOperation::Add` to require `FileMetadata` (remove `Option<FileMetadata>`)
- Update fisherman service event processing to utilize embedded metadata for ephemeral trie construction
- Remove external metadata query logic from catch-up process
- Simplify fisherman service implementation by eliminating fallback query mechanisms
- Update integration tests to verify end-to-end functionality with new event format

### Validation

- Verify fisherman service can successfully construct ephemeral merkle forest during catch-up using only event data
- Confirm no external queries are required for file key construction and trie population
- Test catch-up performance improvement with embedded metadata

## Dependencies

- **RFC-01**: [Finalized Data with Catch-Up](01-RFC-finalized-data-catch-up.md) - This RFC builds on the catch-up mechanism defined in RFC-01
- **ADR-001**: [File Key Construction using Complete FileMetadata](../../decisions/0001-file-key-construction-using-complete-filemetadata.md) - Defines the file key construction algorithm that requires all FileMetadata fields
- Requires coordination between runtime and client teams for event structure changes

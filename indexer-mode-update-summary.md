# IndexerMode Update Summary

This document summarizes the changes made to update the spawn_indexer_service call to pass the indexer_mode parameter.

## Changes Made

### 1. Added IndexerMode Import in node/src/service.rs
- Initially attempted to import from `crate::cli::IndexerMode` but removed to avoid circular dependency

### 2. Defined IndexerMode in client/indexer-service/src/lib.rs
```rust
/// The mode in which the indexer runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexerMode {
    /// Full indexing mode - indexes all blockchain data
    Full,
    /// Lite indexing mode - indexes only essential data for storage operations
    Lite,
}
```

### 3. Updated spawn_indexer_service Function Signature
- Added `indexer_mode: IndexerMode` parameter in client/indexer-service/src/lib.rs
- Updated the function to pass this parameter to IndexerService::new()

### 4. Updated IndexerService Struct
- Added `indexer_mode: crate::IndexerMode` field to the IndexerService struct in handler.rs
- Updated the `new` method to accept and store the indexer_mode parameter

### 5. Updated All spawn_indexer_service Calls in node/src/service.rs
- Added conversion from cli::IndexerMode to shc_indexer_service::IndexerMode
- Updated all 4 occurrences of spawn_indexer_service calls to include:
```rust
match indexer_config.indexer_mode {
    crate::cli::IndexerMode::Full => shc_indexer_service::IndexerMode::Full,
    crate::cli::IndexerMode::Lite => shc_indexer_service::IndexerMode::Lite,
},
```

### 6. Added Logging for IndexerMode
- Updated the IndexerService startup log message to show which mode is being used:
```rust
info!(target: LOG_TARGET, "IndexerService starting up in {:?} mode!", self.actor.indexer_mode);
```

## Next Steps

The indexer_mode is now being passed through the system, but it's not yet being used to control indexing behavior. The next step would be to implement the conditional indexing logic based on the mode in the various index_*_event methods.

## Testing

To test the changes:
1. Run the node with `--indexer-mode lite` to see the lite mode message
2. Run the node with `--indexer-mode full` (or default) to see the full mode message
3. Verify the indexer starts correctly in both modes
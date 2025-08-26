# Telemetry Error Classification Refactor Plan

## Objective
Replace the unsafe string-based error classification function with a type-safe trait-based approach where each error type self-categorizes for telemetry.

## Current Problem
- `/client/common/src/task_context.rs` contains `classify_error(&anyhow::Error)` function
- Uses string matching on error messages (fragile, not type-safe)
- Global function trying to classify all errors (anti-pattern)
- 13 task files use this function with `classify_error(&e)` calls

## Implementation Steps

### Step 1: Create New Telemetry Error Module
**File**: `/client/common/src/telemetry_error.rs` (NEW FILE)

```rust
use serde::{Deserialize, Serialize};

/// Type-safe error categories for telemetry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    Network,
    Timeout,
    Permission,
    Storage,
    Proof,
    Blockchain,
    Capacity,
    FileOperation,
    ForestOperation,
    Configuration,
}

impl ErrorCategory {
    /// Convert to string for telemetry events
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::Network => "network_error",
            ErrorCategory::Timeout => "timeout_error",
            ErrorCategory::Permission => "permission_error",
            ErrorCategory::Storage => "storage_error",
            ErrorCategory::Proof => "proof_error",
            ErrorCategory::Blockchain => "blockchain_error",
            ErrorCategory::Capacity => "capacity_error",
            ErrorCategory::FileOperation => "file_operation_error",
            ErrorCategory::ForestOperation => "forest_operation_error",
            ErrorCategory::Configuration => "configuration_error",
        }
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trait for errors to self-categorize for telemetry
pub trait TelemetryErrorCategory {
    /// Returns the telemetry category for this error
    fn telemetry_category(&self) -> ErrorCategory;
}
```

### Step 2: Update common/src/lib.rs
**File**: `/client/common/src/lib.rs`

Add:
```rust
pub mod telemetry_error;
```

### Step 3: Implement Trait for File Manager Errors
**File**: `/client/file-manager/src/error.rs`

Add at the end of file:
```rust
use shc_common::telemetry_error::{ErrorCategory, TelemetryErrorCategory};

impl TelemetryErrorCategory for FileStorageError {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::FileAlreadyExists 
            | Self::FileDoesNotExist
            | Self::IncompleteFile
            | Self::FileIsEmpty
            | Self::FingerprintAndStoredFileMismatch
            | Self::FailedToParseKey
            | Self::FailedToParseFileMetadata
            | Self::FailedToParseFingerprint
            | Self::FailedToParseChunkWithId
            | Self::FailedToConstructFileKeyProof => ErrorCategory::FileOperation,
            
            Self::FailedToReadStorage
            | Self::FailedToWriteToStorage
            | Self::FailedToInsertFileChunk
            | Self::FailedToGetFileChunk
            | Self::FailedToDeleteFileChunk
            | Self::FileChunkAlreadyExists
            | Self::FileChunkDoesNotExist
            | Self::FailedToConstructTrieIter
            | Self::FailedToParsePartialRoot
            | Self::FailedToHasherOutput
            | Self::FailedToAddEntityToExcludeList
            | Self::FailedToAddEntityFromExcludeList
            | Self::ErrorParsingExcludeType => ErrorCategory::Storage,
            
            Self::FailedToGenerateCompactProof => ErrorCategory::Proof,
        }
    }
}

impl TelemetryErrorCategory for FileStorageWriteError {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::FileDoesNotExist
            | Self::FileChunkAlreadyExists
            | Self::FingerprintAndStoredFileMismatch
            | Self::FailedToContructFileTrie
            | Self::FailedToParseFileMetadata
            | Self::FailedToParseFingerprint
            | Self::FailedToParsePartialRoot => ErrorCategory::FileOperation,
            
            Self::FailedToInsertFileChunk
            | Self::FailedToGetFileChunk
            | Self::FailedToPersistChanges
            | Self::FailedToDeleteRoot
            | Self::FailedToDeleteChunk
            | Self::FailedToConstructTrieIter
            | Self::FailedToReadStorage
            | Self::FailedToUpdatePartialRoot
            | Self::FailedToGetStoredChunksCount => ErrorCategory::Storage,
            
            Self::ChunkCountOverflow => ErrorCategory::Capacity,
        }
    }
}
```

**File**: `/client/file-manager/src/traits.rs`

Add import at top:
```rust
use shc_common::telemetry_error::{ErrorCategory, TelemetryErrorCategory};
```

Add implementations after the enum definitions:
```rust
impl TelemetryErrorCategory for FileStorageError {
    // Same implementation as above
}

impl TelemetryErrorCategory for FileStorageWriteError {
    // Same implementation as above
}
```

### Step 4: Implement Trait for Forest Manager Errors
**File**: `/client/forest-manager/src/error.rs`

Add at the end of file:
```rust
use shc_common::telemetry_error::{ErrorCategory, TelemetryErrorCategory};

impl<H> TelemetryErrorCategory for ForestStorageError<H> {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::FailedToCreateTrieIterator
            | Self::FailedToSeek(_)
            | Self::FailedToReadLeaf(_)
            | Self::FailedToInsertFileKey(_)
            | Self::FileKeyAlreadyExists(_)
            | Self::FailedToParseKey
            | Self::FailedToDecodeValue
            | Self::FailedToConstructProvenLeaves => ErrorCategory::ForestOperation,
            
            Self::ExpectingRootToBeInStorage
            | Self::FailedToReadStorage
            | Self::FailedToWriteToStorage
            | Self::FailedToCopyRocksDB => ErrorCategory::Storage,
            
            Self::FailedToGenerateCompactProof
            | Self::InvalidProvingScenario => ErrorCategory::Proof,
        }
    }
}
```

### Step 5: Implement Trait for Blockchain Service Errors
**File**: `/client/blockchain-service/src/types.rs`

Add after the `WatchTransactionError` enum definition:
```rust
use shc_common::telemetry_error::{ErrorCategory, TelemetryErrorCategory};

impl TelemetryErrorCategory for WatchTransactionError {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::Timeout => ErrorCategory::Timeout,
            Self::WatcherChannelClosed => ErrorCategory::Network,
            Self::TransactionFailed { .. } => ErrorCategory::Blockchain,
            Self::Internal(_) => ErrorCategory::Blockchain,
        }
    }
}
```

### Step 6: Update Task Context
**File**: `/client/common/src/task_context.rs`

1. Remove the entire `classify_error` function (lines 69-92)
2. Keep the `TaskContext` struct and its methods unchanged
3. Keep the `calculate_transfer_rate_mbps` function unchanged
4. Remove the test `test_error_classification` (lines 142-154)

### Step 7: Update Each Task File
For each of the following 13 files, make these changes:

Files to update:
- `/client/src/tasks/fisherman_process_file_deletion.rs`
- `/client/src/tasks/user_sends_file.rs`
- `/client/src/tasks/msp_stop_storing_insolvent_user.rs`
- `/client/src/tasks/msp_delete_bucket.rs`
- `/client/src/tasks/msp_move_bucket.rs`
- `/client/src/tasks/msp_charge_fees.rs`
- `/client/src/tasks/msp_upload_file.rs`
- `/client/src/tasks/bsp_delete_file.rs`
- `/client/src/tasks/bsp_move_bucket.rs`
- `/client/src/tasks/bsp_charge_fees.rs`
- `/client/src/tasks/bsp_submit_proof.rs`
- `/client/src/tasks/bsp_download_file.rs`
- `/client/src/tasks/bsp_upload_file.rs`

#### Change Pattern:
1. Remove import: `task_context::classify_error`
2. Add import: `shc_common::telemetry_error::TelemetryErrorCategory`
3. Replace every occurrence of:
   ```rust
   error_type: classify_error(&e),
   ```
   With:
   ```rust
   error_type: e.telemetry_category().to_string(),
   ```

#### Example Transformation:

**BEFORE:**
```rust
use shc_common::task_context::{classify_error, TaskContext};

// ... later in code ...
let error_type = classify_error(&e);
let failed_event = SomeFailedEvent {
    // ...
    error_type,
    // ...
};
```

**AFTER:**
```rust
use shc_common::task_context::TaskContext;
use shc_common::telemetry_error::TelemetryErrorCategory;

// ... later in code ...
let failed_event = SomeFailedEvent {
    // ...
    error_type: e.telemetry_category().to_string(),
    // ...
};
```

### Step 8: Handle Complex Error Types
For any task-specific error enums that wrap other errors, implement the trait:

**Example pattern:**
```rust
#[derive(thiserror::Error, Debug)]
enum TaskSpecificError {
    #[error("Storage operation failed")]
    Storage(#[from] FileStorageError),
    
    #[error("Forest operation failed")]
    Forest(#[from] ForestStorageError),
}

impl TelemetryErrorCategory for TaskSpecificError {
    fn telemetry_category(&self) -> ErrorCategory {
        match self {
            Self::Storage(e) => e.telemetry_category(),
            Self::Forest(e) => e.telemetry_category(),
        }
    }
}
```

## Validation Steps

1. **Compile Check**: After all changes, run `cargo check` to ensure no compilation errors
2. **Test Update**: Run `cargo test` and fix any failing tests
3. **Verify Telemetry**: Check that telemetry events still have proper error categorization

## Success Criteria

- [ ] No more `classify_error` function exists
- [ ] All error types used in telemetry implement `TelemetryErrorCategory`
- [ ] All 13 task files updated to use the new trait
- [ ] Code compiles without errors
- [ ] Tests pass (after updating/removing the classify_error test)

## Benefits Achieved

1. **Type Safety**: Compiler enforces error categorization
2. **Maintainability**: Each error type self-categorizes
3. **No String Matching**: Categories are enums, not fragile string comparisons
4. **Extensibility**: New error types can easily implement the trait
5. **Performance**: No runtime string parsing or matching
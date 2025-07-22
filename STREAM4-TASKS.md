# Stream 4: Fallback Removal

**Branch**: stream4-fallback-removal  
**Estimated Time**: 2-3 hours  
**Dependencies**: None

## Tasks

### 1. PostgreSQL Fallback Removal ⏳

**File**: `backend/bin/src/main.rs` (lines 144-159)

**Current State**: 
- Mock fallback is commented out but error handling needs improvement
- Should fail cleanly when mock_mode is false and connection fails

**Required Changes**:
1. Remove commented mock fallback code
2. Ensure proper error propagation
3. Only use mock when explicitly enabled in config

**Look for this pattern around line 144**:
```rust
Err(e) => {
    error!("Failed to connect to PostgreSQL: {}", e);
    
    // WIP: Mock fallback - commented out until diesel traits are fully implemented
    // For now, just return the error
    Err(Box::new(e))
}
```

### 2. RPC Fallback Removal ⏳

**File**: `backend/bin/src/main.rs` (lines 199-212)

**Current State**: 
- Falls back to mock connection on failure
- Should fail when mock_mode is false

**Required Changes**:
Remove this entire block:
```rust
#[cfg(feature = "mocks")]
{
    info!("Falling back to mock RPC connection");
    let mock_conn = AnyRpcConnection::Mock(MockConnection::new());
    let client = StorageHubRpcClient::new(Arc::new(mock_conn));
    Ok(Arc::new(client))
}
```

Replace with proper error propagation:
```rust
#[cfg(not(feature = "mocks"))]
{
    Err(e.into())
}
```

### 3. Query Method Review ⏳

**Files**: 
- `backend/lib/src/data/postgres/client.rs`
- `backend/lib/src/data/postgres/queries.rs`

**Task**:
1. Review all query methods
2. Check if they use shc-indexer-db model methods
3. For any manual/custom queries not available in the models, add:

```rust
todo!("Add to shc-indexer-db: <SQL query description>")
```

**Example**:
```rust
// If you find a custom query like:
pub async fn get_files_by_status(&self, status: &str) -> Result<Vec<File>> {
    // Custom SQL here
}

// Replace with:
pub async fn get_files_by_status(&self, status: &str) -> Result<Vec<File>> {
    todo!("Add to shc-indexer-db: SELECT * FROM files WHERE status = $1")
}
```

## Testing

After implementation:
1. Test PostgreSQL connection failure:
   - With mock_mode = false → should exit with error
   - With mock_mode = true → should use mock (when implemented)
   
2. Test RPC connection failure:
   - With mock_mode = false → should exit with error
   - With mock_mode = true → should use mock

3. Verify all queries either:
   - Use model methods from shc-indexer-db
   - Have clear todo!() messages for missing functionality

## Commit Strategy

Make separate commits:
1. "fix(backend): remove PostgreSQL automatic mock fallback"
2. "fix(backend): remove RPC automatic mock fallback"
3. "refactor(backend): add todo markers for missing query methods"

## Notes

- This improves production reliability by failing fast when connections aren't available
- Makes mock usage explicit and intentional
- Clear separation between production and test behavior
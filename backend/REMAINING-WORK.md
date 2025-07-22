# Backend Remaining Work Summary

Quick reference for engineers picking up remaining tasks after Phase 1.

## High Priority Tasks

### Stream 3: CLI and Environment (3-4 hours)

**Task 1: Environment Filter** (`backend/bin/src/main.rs` lines 31-32)
```rust
// Current:
let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("info,sh_backend=debug,sh_backend_lib=debug"));

// Should be:
let filter = EnvFilter::from_default_env();
```

**Task 2: CLI Implementation** (`backend/bin/src/main.rs`)
- Add clap CLI parsing
- Accept `--config` flag for config file path
- Implement config override logic:
  - Default path: `backend_config.toml`
  - If default path missing, use `Config::default()`
  - If explicit path given and missing, error
  - CLI args override config values

**Task 3: Parking_lot Migration** (`backend/lib/src/data/storage/memory.rs`)
```rust
// Replace:
use std::sync::{Arc, RwLock};

// With:
use parking_lot::RwLock;
use std::sync::Arc;
```

### Stream 4: Fallback Removal (2-3 hours)

**Task 1: Remove PostgreSQL Fallback** (`backend/bin/src/main.rs` lines 144-159)
- Currently commented out but needs proper error handling
- Should fail immediately if mock_mode is false and connection fails

**Task 2: Remove RPC Fallback** (`backend/bin/src/main.rs` lines 199-212)
```rust
// Remove the fallback logic:
#[cfg(feature = "mocks")]
{
    info!("Falling back to mock RPC connection");
    let mock_conn = AnyRpcConnection::Mock(MockConnection::new());
    let client = StorageHubRpcClient::new(Arc::new(mock_conn));
    Ok(Arc::new(client))
}
```

**Task 3: Query Method Review** (`backend/lib/src/data/postgres/queries.rs`)
- Check if queries use shc-indexer-db model methods
- For any manual queries, add:
```rust
todo!("Add method to shc-indexer-db: SELECT * FROM table WHERE condition")
```

## Low Priority Tasks

### Stream 1: Documentation (1-2 hours)

**Task 1: Handler Documentation** (`backend/lib/src/api/handlers.rs`)
- Remove any mentions of specific endpoints/routes
- Focus on what the handler does, not where it's mounted

**Task 2: Constructor Documentation**
- Review all `new()` methods
- Remove verbose examples for simple constructors
- Keep only parameter descriptions

## Architecture Notes

Phase 1 implemented a major architectural change:
- Connection-level mocking (DbConnection/RpcConnection traits)
- Clients receive connections, don't implement mock logic
- Type-safe enum dispatch pattern

This makes the remaining work straightforward - mostly removing code rather than adding complexity.
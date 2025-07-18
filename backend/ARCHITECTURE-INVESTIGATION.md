# Architecture Investigation Report - Stream 6

## Storage Trait Architecture

### Overview
The backend uses a two-level trait system for storage abstraction:

1. **Storage trait** (`traits.rs`): Core abstraction for backend-specific data operations
   - Generic over error types via associated type
   - Provides counter operations (increment, decrement, get, set, delete)
   - All methods are async and trait requires Send + Sync

2. **BoxedStorage** (`boxed.rs`): Type-erased version of Storage trait
   - Uses `Box<dyn Error + Send + Sync>` for error type erasure
   - Enables different storage implementations to be used interchangeably
   - Services store `Arc<dyn BoxedStorage>` for runtime polymorphism

### Current Implementation
- **InMemoryStorage**: Thread-safe in-memory storage using `Arc<RwLock<HashMap>>`
- Wrapped in `BoxedStorageWrapper` for type erasure when used in services

### Purpose and Benefits
1. **Flexibility**: Storage implementations can define their own error types
2. **Interoperability**: Different storage backends can be swapped at runtime
3. **Future extensibility**: Easy to add Redis, database, or other backends
4. **Clean service boundaries**: Services don't need to know implementation details

## Mock Architecture Analysis

### Current State
Mocks are located in `/backend/lib/src/mocks/` containing:
- `MockPostgresClient` implementing `PostgresClientTrait`
- `MockStorageHubRpc` implementing `StorageHubRpcTrait`
- Feature-gated with `#[cfg(feature = "mocks")]`

### Identified Issues

1. **Multiple TestPostgresClient Duplicates**
   - Found 3 identical stub implementations:
     - `/backend/lib/src/lib.rs:83-190`
     - `/backend/lib/src/api/routes.rs` (test module)
     - `/backend/lib/src/api/handlers.rs` (test module)
   - These are minimal stubs that just return `Ok(())` or empty results

2. **Architectural Placement**
   - Mocks are in a separate `mocks` module rather than as alternate implementations
   - Test code creates its own stubs instead of using the provided mocks
   - Creates unnecessary coupling between test code and mock types

3. **Integration Issues**
   - `bin/src/main.rs` falls back to mocks on connection failure (should be explicit)
   - Tests don't consistently use the mock infrastructure

## Recommended Architecture

### Move Mocks to Data Source Level
```
backend/lib/src/data/
├── postgres/
│   ├── mod.rs          # Exports trait and implementations
│   ├── client.rs       # Real PostgreSQL client
│   └── mock.rs         # Mock implementation (moved from mocks/)
└── rpc/
    ├── mod.rs          # Exports trait and implementations
    ├── client.rs       # Real RPC client (to be implemented)
    └── mock.rs         # Mock implementation (moved from mocks/)
```

### Benefits
1. **Locality**: Mocks stay close to their real implementations
2. **Clarity**: Clear that mocks are alternate data source implementations
3. **DRY**: Eliminates duplicate TestPostgresClient implementations
4. **Decoupling**: Tests work with traits, not specific mock types

## Implementation Plan

### Phase 1: Consolidate TestPostgresClient
1. Remove duplicate TestPostgresClient implementations from test modules
2. Update tests to use MockPostgresClient from mocks module
3. Ensure all tests pass with consolidated implementation

### Phase 2: Restructure Mock Architecture
1. Create `backend/lib/src/data/postgres/mock.rs`
   - Move MockPostgresClient implementation
   - Keep feature gating with `#[cfg(feature = "mocks")]`
2. Create `backend/lib/src/data/rpc/mock.rs`
   - Move MockStorageHubRpc implementation
   - Keep feature gating
3. Update module exports in respective `mod.rs` files
4. Remove old mocks directory

### Phase 3: Update Integration Points
1. Fix `bin/src/main.rs` to explicitly choose mock vs real implementation
2. Update example code to use new mock locations
3. Update documentation to reflect new architecture

### Phase 4: Implement Real RPC Client
1. Create `backend/lib/src/data/rpc/client.rs` with real StorageHub RPC implementation
2. Ensure it follows the same pattern as PostgreSQL client
3. Add appropriate error handling and connection management

## Next Steps
1. Get approval on proposed architecture
2. Create feature branch for implementation
3. Execute phases sequentially with tests passing at each stage
4. Update documentation and examples
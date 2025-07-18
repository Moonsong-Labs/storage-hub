# Backend Improvements Plan

This document outlines the improvements to be made to the StorageHub backend component, organized by category and priority.

## Dependency Management

### 1. Workspace Dependencies in Binary Crate
**Requirement**: Use workspace dependencies in backend-bin/Cargo.toml
**Current State**: The binary crate uses explicit version numbers (e.g., `tokio = { version = "1", features = ["full"] }`)
**Target State**: Use workspace references (e.g., `tokio = { workspace = true, features = ["full"] }`)
**Intent**: Ensure consistent dependency versions across the entire project
**Files to Modify**: `backend/bin/Cargo.toml`

### 6. Dependency Organization in Manifests
**Requirement**: Group dependencies by logical units instead of by type
**Current State**: Dependencies grouped as "async trait support", "workspace dependencies"
**Target State**: Group by functionality: "web server", "futures", "config", etc.
**Intent**: Improve readability and maintainability of dependency declarations
**Files to Modify**: `backend/lib/Cargo.toml`, `backend/bin/Cargo.toml`

## Configuration & CLI

### 2. Environment Filter Initialization
**Requirement**: Pass env filter directly to fmt subscriber
**Current State**: Default filter is hardcoded: `EnvFilter::new("info,sh_backend=debug,sh_backend_lib=debug")`
**Target State**: Use `EnvFilter::from_default_env()` and pass directly to subscriber
**Intent**: Simplify env filter usage, no need for manual env variable handling
**Files to Modify**: `backend/bin/src/main.rs`

### 3. CLI Arguments for Configuration
**Requirement**: Add CLI args with default config handling
**Current State**: Config path is hardcoded as `"backend_config.toml"`
**Target State**: 
  - Default config path with default config values
  - If default file not present, use default values
  - If explicit path given and file not present, error
  - CLI options override loaded/default config
**Intent**: Provide flexible configuration with sensible defaults
**Implementation**: Use clap for CLI, apply overrides to loaded/default config
**Files to Modify**: `backend/bin/src/main.rs`, `backend/bin/Cargo.toml` (for clap dependency)

## Error Handling & Fallbacks

### 4. PostgreSQL Connection Fallback Removal
**Requirement**: Remove automatic fallback to mock when DB connection fails
**Current State**: When connection fails, automatically falls back to mock PostgreSQL client
**Target State**: Fail if mock_mode is not set and DB is unavailable
**Intent**: Explicit failure modes for production reliability
**Files to Modify**: `backend/bin/src/main.rs` (lines 127-165)

### 5. StorageHub RPC Connection Initialization
**Requirement**: Initialize StorageHub node RPC connection in binary
**Current State**: No StorageHub RPC client initialization
**Target State**: Binary should initialize RPC connection (real or mock based on config)
**Intent**: Consistency with PostgreSQL client initialization pattern
**Implementation**: Use `shc-rpc` crate (contains both interface and implementation)
**Note**: May split interface/implementation in future
**Files to Create/Modify**: New RPC client integration, update `Services` struct

## Code Quality & Documentation

### 7. Endpoint Documentation
**Requirement**: Avoid mentioning specific endpoints in handler documentation
**Current State**: Documentation may reference specific routes
**Target State**: Document functionality without route specifics
**Intent**: Prevent documentation/implementation mismatches
**Files to Review**: `backend/lib/src/api/handlers.rs`

### 9. Verbose Documentation Examples
**Requirement**: Remove unnecessary examples from simple constructors
**Current State**: Constructor documentation includes examples
**Target State**: Keep parameter descriptions, remove verbose examples for simple cases
**Intent**: Reduce documentation verbosity without losing clarity
**Files to Modify**: Various files with constructor documentation

## Architecture & Design

### 8. Test PostgreSQL Client Redundancy
**Requirement**: Remove redundant TestPostgresClient implementations
**Current State**: Multiple TestPostgresClient definitions
**Target State**: Use mock client or extract to dedicated module
**Intent**: Reduce code duplication
**Questions**: Need to identify all TestPostgresClient implementations

### 10. PostgreSQL Client Query Methods
**Requirement**: Use data model methods instead of manual queries
**Current State**: Some queries may be manually constructed
**Target State**: Leverage methods from shc-indexer-db models
**Implementation**: If query not in db models, use `todo!()` with SQL explanation
**Intent**: Maintain consistency and reduce query duplication
**Files to Review**: `backend/lib/src/data/postgres/client.rs`

### 11. PostgreSQL Client Unit Tests
**Requirement**: Remove unit tests that require real server
**Current State**: Tests marked with `#[ignore]` that need actual database
**Target State**: Remove these tests or implement proper mocking
**Intent**: Tests should run without external dependencies
**Files to Modify**: `backend/lib/src/data/postgres/client.rs`

### 12. Queries Module
**Requirement**: Fix and uncomment queries module
**Current State**: Module is commented out, doesn't compile
**Target State**: Working queries module
**Intent**: Enable custom query functionality
**Files to Fix**: `backend/lib/src/data/postgres/queries.rs`
**Current Issue**: Uses undefined method `get_connection()`

### 14. Storage Trait vs BoxedStorage
**Requirement**: Clarify need for both traits
**Current State**: Both `Storage` trait and `BoxedStorage` exist
**Target State**: Potentially consolidate or clarify separation
**Intent**: Reduce architectural complexity
**Questions**: Is BoxedStorage just for error type erasure?

### 16. Mock PostgreSQL Client Design
**Requirement**: Mock should be at data source level, not client level
**Current State**: Separate MockPostgresClient struct
**Target State**: Real client uses mock connection when in mock mode
**Intent**: Test actual client implementation code paths with mock data
**Architecture**: Mock the connection/data source, not the client
**Complexity**: This requires redesigning the mock architecture

### 17. StorageHub RPC Client Implementation
**Requirement**: Implement production RPC client
**Current State**: Only mock RPC client exists
**Target State**: Real RPC client with mock mode support
**Intent**: Complete the RPC integration
**Architecture**: Mock should spawn listener for realistic testing

## Technical Improvements

### 13. Mutex Implementation
**Requirement**: Use parking_lot instead of std Mutex/RwLock
**Current State**: Using std::sync::Mutex
**Target State**: Use parking_lot::Mutex
**Intent**: Better performance and no poisoning
**Files to Modify**: `backend/lib/src/mocks/postgres_mock.rs`, others using Mutex

### 15. Mock Module Feature Gating
**Requirement**: Remove redundant feature gates inside mocks module
**Current State**: Inner modules have additional feature gates
**Target State**: Only gate at module level
**Intent**: Reduce redundancy since parent module is already gated
**Files to Modify**: `backend/lib/src/mocks/mod.rs`

## CI/CD

### 18. CI Workflow Branch Triggers
**Requirement**: Understand purpose of `perm-*` branch pattern
**Current State**: CI triggers on `main` and `perm-*` branches
**Question**: What is the purpose of `perm-*` branches?
**Files**: `.github/workflows/lint.yml`, `.github/workflows/backend.yml`

## Priority and Complexity Assessment

### High Priority, Low Complexity:
- 1. Workspace dependencies
- 2. Environment filter
- 4. PostgreSQL fallback removal
- 7. Documentation updates
- 13. Parking_lot migration
- 15. Feature gate cleanup

### High Priority, Medium Complexity:
- 3. CLI arguments
- 5. RPC client initialization
- 12. Queries module fix

### Medium Priority, High Complexity:
- 16. Mock client architecture redesign
- 17. Full RPC implementation

### Low Priority:
- 6. Dependency grouping
- 8. Test client consolidation
- 9. Documentation verbosity
- 10. Query method usage
- 11. Unit test removal
- 14. Storage trait clarification
- 18. CI branch pattern clarification

## Updated Architecture Decisions

Based on clarifications:

1. **Mock Architecture**: Mocks should be at the data source level (connections), not client level. This allows testing actual client code paths.

2. **Services Level**: Services receive real clients that use mocked data sources when in mock mode.

3. **RPC Implementation**: Use `shc-rpc` crate for both interface and implementation.

4. **Config Management**: Use clap for CLI, implement override logic without needing figment.

5. **Mock RPC**: Should spawn actual listener for realistic testing.

## Remaining Questions

1. **Storage Traits**: Need to investigate the purpose of BoxedStorage vs Storage trait separation.

2. **CI Branches**: Need to check remote for existing `perm-*` branches to understand the pattern.
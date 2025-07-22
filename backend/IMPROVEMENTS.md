# Backend Improvements Plan

This document outlines the improvements to be made to the StorageHub backend component, organized by category and priority.

**Last Updated**: After Phase 1 completion
**Status**: Major architectural work completed, remaining tasks are straightforward implementations

## Dependency Management

### 1. Workspace Dependencies in Binary Crate ✅
**Status**: COMPLETED
**Implementation**: Binary crate now uses workspace dependencies consistently
**Result**: All dependencies now reference workspace versions

### 6. Dependency Organization in Manifests ✅
**Status**: COMPLETED
**Implementation**: Dependencies reorganized by functionality
**Result**: Clear groupings: "Web framework", "Async runtime", "Serialization", "Database", etc.

## Configuration & CLI

### 2. Environment Filter Initialization ⏳
**Status**: PENDING
**Current State**: Still using default filter with fallback
**Required Change**: Remove the fallback filter, use `EnvFilter::from_default_env()` directly
**Files to Modify**: `backend/bin/src/main.rs` (lines 31-32)

### 3. CLI Arguments for Configuration ⏳
**Status**: PARTIALLY COMPLETE
**Completed**: clap added to dependencies
**Required Implementation**:
  - Parse CLI arguments for config path
  - Implement config override logic
  - Handle explicit vs default path behavior
**Files to Modify**: `backend/bin/src/main.rs`

## Error Handling & Fallbacks

### 4. PostgreSQL Connection Fallback Removal ⏳
**Status**: PENDING
**Current State**: Mock fallback currently commented out but needs proper error handling
**Required Change**: Ensure proper failure when mock_mode is false and connection fails
**Note**: RPC has similar fallback that needs removal (lines 201-206)
**Files to Modify**: `backend/bin/src/main.rs`

### 5. StorageHub RPC Connection Initialization ✅
**Status**: COMPLETED
**Implementation**: Full RPC client initialization with connection abstraction
**Result**: RPC client created in binary, passed to Services, supports both real and mock connections

## Code Quality & Documentation

### 7. Endpoint Documentation ⏳
**Status**: PENDING
**Required Change**: Review and update handler documentation to remove endpoint mentions
**Files to Review**: `backend/lib/src/api/handlers.rs`

### 9. Verbose Documentation Examples ⏳
**Status**: PENDING
**Required Change**: Simplify constructor documentation, remove unnecessary examples
**Note**: Examples were removed from PostgresClient::new but other constructors may need review
**Files to Review**: Various files with constructor documentation

## Architecture & Design

### 8. Test PostgreSQL Client Redundancy ✅
**Status**: COMPLETED
**Implementation**: Removed in favor of connection-based mocking architecture
**Result**: No more duplicate test clients

### 10. PostgreSQL Client Query Methods ⏳
**Status**: PENDING
**Required Change**: Review queries and add `todo!()` for any not available in db models
**Files to Review**: `backend/lib/src/data/postgres/client.rs`, `backend/lib/src/data/postgres/queries.rs`

### 11. PostgreSQL Client Unit Tests ✅
**Status**: COMPLETED
**Implementation**: Test updated to use new connection architecture
**Note**: Test still requires real database but now uses proper connection abstraction

### 12. Queries Module ✅
**Status**: COMPLETED
**Implementation**: Module fixed by using connection abstraction
**Result**: Queries module now compiles and uses `self.conn.get_connection()`

### 14. Storage Trait vs BoxedStorage ✅
**Status**: COMPLETED
**Finding**: BoxedStorage is for error type erasure
**Result**: Architecture understood and documented, separation is justified

### 16. Mock PostgreSQL Client Design ✅
**Status**: COMPLETED (Major Work)
**Implementation**: Complete redesign with connection-level mocking
**Result**: 
  - Created DbConnection trait and AnyDbConnection enum
  - PostgresClient now accepts connections
  - Mock at infrastructure level, not client level
**Note**: MockDbConnection partially implemented (commented due to diesel complexity)

### 17. StorageHub RPC Client Implementation ✅
**Status**: COMPLETED
**Implementation**: Full RPC client with connection abstraction
**Result**:
  - Created RpcConnection trait and AnyRpcConnection enum
  - Implemented WsConnection for real connections
  - MockConnection for testing
  - StorageHubRpcClient uses connection abstraction

## Technical Improvements

### 13. Mutex Implementation ⏳
**Status**: PARTIALLY COMPLETE
**Completed**: parking_lot added to dependencies
**Remaining**: Replace std::sync::RwLock with parking_lot::RwLock in memory.rs
**Files to Modify**: `backend/lib/src/data/storage/memory.rs`

### 15. Mock Module Feature Gating ✅
**Status**: COMPLETED
**Implementation**: Mocks module restructured as part of architecture redesign
**Result**: Clean feature gating structure

## CI/CD

### 18. CI Workflow Branch Triggers ✅
**Status**: COMPLETED
**Implementation**: Removed `perm-*` branches from CI triggers
**Result**: CI now only triggers on main branch and PRs

## Summary of Remaining Work

### High Priority Tasks (Can be done in parallel):

#### Stream 3: CLI and Environment (3-4 hours)
- **2. Environment Filter**: Remove default fallback
- **3. CLI Implementation**: Add argument parsing with config overrides
- **13. Parking_lot Usage**: Replace std::sync in memory.rs

#### Stream 4: Fallback Removal (2-3 hours)
- **4. PostgreSQL Fallback**: Remove automatic mock fallback
- **4b. RPC Fallback**: Remove automatic mock fallback
- **10. Query Methods**: Review and add todo!() for missing methods

### Low Priority Tasks:

#### Stream 1: Documentation (1-2 hours)
- **7. Handler Documentation**: Remove endpoint mentions
- **9. Constructor Documentation**: Simplify verbose examples

### Completed Items (12/18):
✅ 1, 5, 6, 8, 11, 12, 14, 15, 16, 17, 18

### Remaining Items (6/18):
⏳ 2, 3, 4, 7, 9, 10, 13

## Updated Architecture Decisions

Based on clarifications:

1. **Mock Architecture**: Mocks should be at the data source level (connections), not client level. This allows testing actual client code paths.

2. **Services Level**: Services receive real clients that use mocked data sources when in mock mode.

3. **RPC Implementation**: Use `shc-rpc` crate for both interface and implementation.

4. **Config Management**: Use clap for CLI, implement override logic without needing figment.

5. **Mock RPC**: Should spawn actual listener for realistic testing.

## Key Architectural Changes Implemented

The Phase 1 work included a major architectural redesign (Stream 6) that provides:

1. **Connection Abstraction Pattern**: Trait + enum pattern avoiding trait object issues
2. **Infrastructure-Level Mocking**: Mocks at connection level, not client level
3. **Type Safety**: Compile-time type safety with runtime flexibility
4. **Clean Separation**: Connection management separate from business logic

This foundation makes the remaining work straightforward implementation tasks.
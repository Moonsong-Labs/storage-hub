# Backend Refactoring Implementation Plan

## Overview

Comprehensive refactoring of the StorageHub backend to improve code organization, remove unnecessary abstractions, and align with Rust best practices while preserving future-use components like mocks and storage abstractions.

## Prerequisites

- [ ] Nightly Rust toolchain installed (for rustfmt unstable features)
- [ ] All tests passing before starting refactoring
- [ ] Backup of current code state (git commit/branch)
- [ ] Access to full workspace to identify RPC type reuse opportunities

## Phase 1: Code Conformance & Structural Reorganization

### 1. Create rustfmt.toml Configuration

- **File**: `/backend/rustfmt.toml` (create new)
- **Operation**: Add formatting configuration
- **Details**:
  ```toml
  unstable_features = true
  group_imports = "StdExternalCrate"
  imports_granularity = "Module"
  reorder_imports = true
  ```
- **Success**: `cargo fmt -- --check` passes with new import ordering

### 2. Reorganize Storage Module

- **File**: `/backend/lib/src/data/storage/traits.rs`
- **Operation**: Move trait content to parent module
- **Details**:
  - Move `Storage` trait from `traits.rs` to `storage/mod.rs`
  - Delete `traits.rs` file
  - Update imports in `mod.rs` to re-export: `pub use self::Storage;`
- **Success**: All imports resolve correctly

### 3. Reorganize RPC Module Structure

- **Files**: Multiple in `/backend/lib/src/data/rpc/`
- **Operation**: Restructure to trait → utilities → implementations
- **Details**:
  - In `rpc/mod.rs`: Move `RpcConnection` trait from `connection.rs`
  - Keep `RpcConfig` and utilities in `connection.rs`
  - Leave implementations (`ws_connection.rs`, `mock_connection.rs`) as-is
  - Remove `RpcConnectionBuilder` from `connection.rs`
- **Success**: Module compiles with clearer hierarchy

### 4. Standardize All Import Orders

- **Files**: All `.rs` files in `/backend/`
- **Operation**: Run formatter and manually adjust imports
- **Details**:
  ```rust
  // Example pattern:
  use std::sync::Arc;           // 1. std imports
  
  use anyhow::{Context, Result}; // 2. external crates
  use tokio::sync::RwLock;
  
  use crate::error::Error;       // 3. internal crates
  
  mod handlers;                  // 4. module declarations
  mod routes;
  
  use handlers::*;               // 5. module imports
  
  pub use routes::create_routes; // 6. re-exports
  ```
- **Success**: Consistent import ordering across all files

### 5. Consolidate Test Utilities

- **Files**: Various service files with test modules
- **Operation**: Create test-only implementations
- **Details**:
  ```rust
  // In services/mod.rs
  #[cfg(test)]
  impl Services {
      pub fn test() -> Self {
          Self {
              storage: Box::new(crate::data::storage::memory::MemoryStorage::new()),
              rpc: None, // Or mock as needed
          }
      }
  }
  ```
- **Success**: Test setup simplified across all test modules

## Phase 2: Dependency Alignment & Dead Code Removal

### 6. Remove StorageHubRpcTrait

- **File**: `/backend/lib/src/data/rpc/mod.rs`
- **Operation**: Remove trait and implement directly on client
- **Details**:
  - Delete `StorageHubRpcTrait` definition (lines 10-45 approx)
  - In `client.rs`, change from `impl StorageHubRpcTrait for StorageHubClient`
    to direct method implementations
  - Update all usage sites to use client directly
- **Success**: Client methods work without trait indirection

### 7. Reuse RPC Types from Client Crate

- **Files**: `/backend/lib/src/data/rpc/mod.rs`, `/backend/Cargo.toml`
- **Operation**: Import types from client workspace crate
- **Details**:
  - Add to Cargo.toml: `shc-rpc-client = { path = "../../client/rpc" }`
  - Remove duplicate type definitions: `FileMetadata`, `BucketInfo`, etc.
  - Import from client: `use shc_rpc_client::{FileMetadata, BucketInfo, ...};`
- **Success**: No duplicate type definitions, imports resolve

### 8. Remove RpcConnection Builder

- **File**: `/backend/lib/src/data/rpc/connection.rs`
- **Operation**: Remove builder pattern
- **Details**:
  - Delete `RpcConnectionBuilder` struct and impl block
  - Remove associated tests in `tests/rpc_connections_test.rs`
  - Update any usage to direct construction
- **Success**: Connection creation simplified

### 9. Clean Up Mock Connection

- **File**: `/backend/lib/src/data/rpc/mock_connection.rs`
- **Operation**: Remove default responses
- **Details**:
  - Remove all default response implementations
  - Keep structure but require explicit setup in tests
  - Update affected tests to provide responses
- **Success**: Tests explicitly define expected behavior

### 10. Move/Clean Memory Storage Tests

- **File**: `/backend/lib/src/data/storage/memory.rs`
- **Operation**: Move or remove excessive tests
- **Details**:
  - Identify tests that are just testing the placeholder counter functionality
  - Either move them to `counter.rs` service tests or remove entirely
  - Keep only tests that validate the storage contract implementation
- **Success**: Tests focused on actual storage behavior, not placeholder logic

## Phase 3: Code Quality & Integration

### 11. Standardize Error Handling

- **File**: `/backend/lib/src/error.rs`
- **Operation**: Implement thiserror patterns
- **Details**:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum Error {
      #[error("Configuration error: {0}")]
      Config(String),
      
      #[error("RPC error: {0}")]
      Rpc(#[from] jsonrpsee::core::Error),
      
      #[error("Storage error: {0}")]
      Storage(#[from] Box<dyn std::error::Error + Send + Sync>),
  }
  ```
- **Success**: All error handling uses consistent patterns

### 12. Wire Up Health Service

- **Files**: `/backend/lib/src/api/routes.rs`, `/backend/lib/src/api/handlers.rs`
- **Operation**: Add health endpoint
- **Details**:
  - In `routes.rs`: Add `.route("/health", get(handlers::health_check))`
  - In `handlers.rs`: 
    ```rust
    pub async fn health_check(State(services): State<Arc<Services>>) -> impl IntoResponse {
        match services.health.check().await {
            Ok(status) => (StatusCode::OK, Json(status)),
            Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"status": "unhealthy"}))),
        }
    }
    ```
- **Success**: `/health` endpoint returns service status

### 13. Remove Verbose Documentation

- **Files**: All source files
- **Operation**: Clean up excessive comments
- **Details**:
  - Remove line-by-line explanations
  - Keep only "why" comments for non-obvious logic
  - Simplify function docs to one line where appropriate
  - Consolidate repetitive postgres-related comments to one per section
- **Success**: Code is self-documenting, comments add value

### 14. Update Ignored Tests

- **Files**: All test files
- **Operation**: Simplify ignored test patterns
- **Details**:
  ```rust
  #[ignore]
  #[test]
  fn test_postgres_connection() {
      todo!("Implement when postgres mock available")
  }
  ```
- **Success**: Tests clearly indicate what's needed

### 15. Create Test App Configuration

- **File**: `/backend/lib/src/api/mod.rs`
- **Operation**: Add test-specific app creation
- **Details**:
  ```rust
  #[cfg(test)]
  impl App {
      pub fn test() -> Self {
          let services = Arc::new(Services::test());
          create_app(services)
      }
  }
  ```
- **Success**: Test app creation standardized

## Testing Strategy

- [ ] Run `cargo fmt -- --check` after Phase 1
- [ ] Run `cargo clippy --all-targets` after each phase
- [ ] Run `cargo test` to ensure no regressions
- [ ] Manual test of `/health` endpoint
- [ ] Verify RPC client still connects to StorageHub node
- [ ] Verify BoxedStorage continues to work correctly with different implementations

## Rollback Plan

Since this is a large refactoring:
1. Create feature branch before starting
2. Commit after each phase completion
3. If issues arise, revert to last known good commit
4. For partial rollback, use git cherry-pick to keep good changes

## Post-Implementation Checklist

- [ ] All imports follow the standardized pattern
- [ ] No duplicate RPC type definitions
- [ ] Health service integrated and functional
- [ ] Test utilities consolidated in `#[cfg(test)]` blocks
- [ ] Documentation is concise and valuable
- [ ] No compiler warnings or clippy issues
- [ ] BoxedStorage pattern preserved for type erasure needs

## Notes

- **BoxedStorage**: After analysis, this pattern is necessary for type-erasing the Storage trait with its associated Error type. It's not unnecessary abstraction but a solution to Rust's type system requirements when using trait objects with associated types.
- **Mocks**: All mock implementations are preserved for future testing needs
- **Import Order**: The nightly requirement for rustfmt is only for development tooling and doesn't affect the compiled binary
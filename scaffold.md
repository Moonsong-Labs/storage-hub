# StorageHub Backend Scaffold Plan

## Overview

This comprehensive plan creates a production-ready backend scaffold for StorageHub that integrates seamlessly with the existing Cargo workspace. The scaffold provides a REST API server with proper separation of concerns, comprehensive mocking capabilities, and dedicated CI/CD infrastructure.

The backend serves as a REST API layer that reads from the existing StorageHub indexer database and provides useful endpoints for external consumers, while maintaining its own internal state separate from the blockchain indexer.

## Project Structure

```
backend/
├── lib/                    # Library crate (sh-backend-lib)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Main library interface
│       ├── api/            # REST API layer
│       │   ├── mod.rs
│       │   ├── handlers.rs # Request handlers
│       │   └── routes.rs   # Route definitions
│       ├── services/       # Business logic layer
│       │   ├── mod.rs
│       │   ├── counter.rs  # Counter service
│       │   └── health.rs   # Health check service
│       ├── data/           # Data access layer
│       │   ├── mod.rs
│       │   ├── postgres/   # StorageHub indexer DB access
│       │   │   ├── mod.rs
│       │   │   ├── client.rs    # PostgreSQL client wrapper
│       │   │   └── queries.rs   # Custom queries beyond models
│       │   └── storage/    # Backend-specific storage
│       │       ├── mod.rs
│       │       ├── traits.rs    # Storage traits
│       │       └── memory.rs    # In-memory storage
│       ├── mocks/          # Mock implementations (feature-gated)
│       │   ├── mod.rs
│       │   ├── postgres_mock.rs # Mock indexer DB
│       │   ├── storage_mock.rs  # Mock backend storage
│       │   └── rpc_mock.rs      # Mock StorageHub RPC
│       ├── config.rs       # Configuration management
│       └── error.rs        # Error handling
└── bin/                    # Binary crate (sh-backend-bin)
    ├── Cargo.toml
    └── src/
        └── main.rs         # Application entry point
```

## Data Layer Architecture

The backend uses a dual data layer approach:

### 1. PostgreSQL Data Layer (READ-ONLY)
- **Purpose**: Access existing StorageHub indexer database
- **Data**: BSP/MSP information, file metadata, payment streams, blockchain events
- **Models**: Reuses existing shc-indexer-db crate models
- **Usage**: Backend reads this data to present useful information via API

### 2. Backend Storage Layer (READ-WRITE)
- **Purpose**: Store backend-specific data not part of indexer DB
- **Data**: Counters, user sessions, caches, backend configuration
- **Implementation**: In-memory (dev), Redis/separate DB (prod)
- **Usage**: Backend internal state and temporary data

Both layers have feature-gated mock implementations for testing.

### Example Usage

```rust
// PostgreSQL layer - read StorageHub indexer data
let active_bsps = postgres_client.get_active_bsps().await?;
let file_metadata = postgres_client.get_file_by_id(file_id).await?;
let payment_streams = postgres_client.get_payment_streams_for_user(user_id).await?;

// Backend storage layer - backend-specific data
let api_call_count = storage.increment_counter("api_calls").await?;
let user_session = storage.get_user_session(session_id).await?;
let cached_result = storage.get_cache("expensive_query_result").await?;
```

## Implementation Phases

### Phase 1: Foundation Setup

#### 1.1 Directory Structure Creation
- Create backend/lib/ and backend/bin/ directories
- Setup proper Cargo.toml files for both crates
- Configure workspace integration

#### 1.2 Dependency Management
- Library crate uses axum web framework with tokio async runtime
- Leverages existing workspace dependencies (serde, toml, thiserror, etc.)
- Depends on shc-indexer-db crate for PostgreSQL models and queries
- Optional mocking features for testing flexibility (feature-gated)

#### 1.3 Workspace Integration
- Update root Cargo.toml to include new backend crates
- Follow naming convention (sh-* prefix for StorageHub ecosystem)
- Ensure compatibility with existing build systems

### Phase 2: Core Implementation

#### 2.1 API Layer Implementation
- REST endpoints: GET /counter, POST /counter/inc, POST /counter/dec
- Health check endpoint for monitoring
- Proper HTTP error handling and JSON responses
- Request/response serialization with serde

#### 2.2 Service Layer Architecture
- Counter service with increment/decrement/get operations
- Dependency injection pattern for testability
- Async trait-based abstractions
- Clean separation of business logic

#### 2.3 Data Layer Design
- **PostgreSQL data layer**: Read-only access to existing StorageHub indexer DB
  - Uses existing shc-indexer-db crate models (BSP, MSP, files, payment streams)
  - Custom queries for backend-specific data aggregation
  - Feature-gated mocking for testing without real database
- **Backend storage layer**: Backend-specific persistence (counters, sessions, caches)
  - Abstract storage traits for flexibility
  - In-memory storage implementation for development
  - Thread-safe operations with proper locking
  - Separate from indexer DB to avoid coupling

### Phase 3: Binary Crate and Configuration

#### 3.1 Application Bootstrap
- Main binary handles server initialization
- Configuration loading from TOML files
- Logging setup with tracing
- Graceful error handling

#### 3.2 Configuration Management
- Structured configuration with nested sections
- Environment-specific settings
- Mock mode toggles for development/testing
- Default values for rapid setup

#### 3.3 Server Setup
- Axum router configuration
- Service dependency injection
- TCP listener binding
- Production-ready server architecture

### Phase 4: Mock Infrastructure

#### 4.1 StorageHub RPC Mock
- jsonrpsee-based RPC trait implementations
- Configurable mock responses
- File information and listing operations
- Error scenario simulation

#### 4.2 PostgreSQL Database Mock
- Mock implementation of indexer DB queries
- Returns realistic data structures matching shc-indexer-db models
- Simulates BSP/MSP listings, file metadata, payment streams
- Feature-gated for testing only (excluded from production builds)

#### 4.3 Mock Integration
- Feature-gated mock implementations (not available in production builds)
- Feature gates: 
  - Default build: `cargo build --release` (no mocks compiled in)
  - Development: `cargo build --features dev` (mocks compiled in)
  - Testing: `cargo test --features test` (mocks compiled in)
- Configuration toggle between real and mock services (only when mocks are compiled in)
- Production builds cannot access mocks regardless of configuration
- Realistic data structures matching actual services
- Testing-friendly interfaces with configurable behaviors

### Phase 5: CI/CD Integration

#### 5.1 Dedicated GitHub Actions Workflow
- Separate CI pipeline for backend components
- Path-based triggering for efficiency
- Rust toolchain setup with caching
- Dependency installation (libpq-dev)

#### 5.2 Quality Assurance Jobs
- Format checking with rustfmt
- Linting with clippy (zero warnings policy)
- Comprehensive test execution
- Feature-specific testing (mocks enabled)

#### 5.3 Testing Infrastructure
- Integration tests for API endpoints
- Mock service validation
- HTTP request/response testing
- Status code verification

## Key Technical Decisions

### Web Framework: Axum
- Modern async-first design
- Excellent ecosystem integration
- Tower middleware support
- Production-ready performance

### Dependency Injection Pattern
- Trait-based service abstraction
- Easy mocking and testing
- Configurable implementations
- Clean architecture boundaries

### Configuration Strategy
- TOML-based configuration files
- Environment variable support
- Structured configuration types
- Development-friendly defaults

### Error Handling Approach
- Structured error types with thiserror
- Proper HTTP status mapping
- Detailed error information
- Consistent error responses

## Implementation Dependencies

```
DEPENDENCY FLOW
===============
Phase 1 (Foundation)
└── Phase 2 (Core Implementation)
    └── Phase 3 (Binary & Config)
        └── Phase 4 (Mock Infrastructure)
            └── Phase 5 (CI/CD Integration)

CRITICAL PATH ITEMS
==================
1. Workspace integration (blocks everything)
2. Service trait definitions (blocks mocks)
3. Basic API implementation (blocks testing)
4. Mock interfaces (blocks CI testing)
```

## Expected Outcomes

### Deliverables
- Production-ready backend scaffold
- Comprehensive testing infrastructure
- Mock implementations for external dependencies
- Dedicated CI/CD pipeline
- Documentation and configuration examples

### Quality Metrics
- Zero clippy warnings policy
- Comprehensive test coverage
- Clean architecture patterns
- Proper error handling
- Configuration flexibility

### Integration Points
- Seamless workspace integration
- Compatible with existing CI infrastructure
- Extensible for future functionality
- Mock-friendly for testing

## Next Steps

This plan provides a complete roadmap for implementing the backend scaffold. The implementation can proceed sequentially through the phases, with each phase building upon the previous one.
# Database Connection Architecture

## Current Structure

The database connection system supports multiple backends (PostgreSQL and SQLite) through a unified interface. Mock connections are located in `/backend/lib/src/data/postgres/mock_connection.rs` and are designed to simulate database behavior for testing purposes.

### Key Components

#### Connection Abstraction
- **DbConnection** trait - Backend-agnostic database connection interface
- **AnyDbConnection** enum - Dispatches between PostgreSQL, SQLite, and Mock connections
- **AnyAsyncConnection** enum - Dispatches between async connection types

#### Backend Implementations
- **PgConnection** - PostgreSQL connection pool using diesel-async
- **SqliteConnection** - SQLite connection pool using SyncConnectionWrapper
- **MockDbConnection** - In-memory mock implementation for testing

#### Mock Components
- **MockTestData** - In-memory storage for test data (files, buckets, MSPs)
- **MockErrorConfig** - Configuration for simulating various error conditions  
- **MockAsyncConnection** - Mock implementation of diesel's `AsyncConnection` trait
- **MockTransactionManager** - Simulates transaction management

## Database Backend Support

### PostgreSQL
- Full async support via diesel-async's `AsyncPgConnection`
- Connection pooling with bb8
- Native support for all diesel PostgreSQL features

### SQLite
- Async support via diesel-async's `SyncConnectionWrapper`
- Connection pooling with bb8
- Wraps synchronous SQLite connections for async compatibility

### Mock Connections
The mocks are intended to:
- Replace real database connections in tests
- Store test data in memory rather than a database
- Simulate various error conditions (connection failures, timeouts, query errors)
- Support the same interface as the real database clients

## Current Issues

### 1. Incomplete Diesel Trait Implementation

- The mock is currently commented out throughout the codebase (see `/backend/lib/src/data/postgres/mod.rs:14-19`)
- The `MockAsyncConnection` needs to fully implement diesel-async's `AsyncConnection` trait
- Missing proper implementations for `LoadConnection`, query execution, and result handling

### 2. Feature Flag Disabled

- The `mocks` feature exists in `Cargo.toml` but the mock code is commented out
- Files like `connection.rs` have placeholders for mock variants but they're disabled

### 3. Unimplemented Query Execution

- The `LoadConnection` implementation returns empty cursors (`mock_connection.rs:286-288`)
- No actual query processing logic - mocks don't interact with the test data
- The `PostgresClient` methods would fail with mocks as queries aren't properly handled

### 4. Integration Points Missing

- `AnyDbConnection` enum has mock variant commented out (`connection.rs:166-167`)
- `AnyAsyncConnection` enum missing mock variant (`connection.rs:217-218`)
- Test files can't use mocks without these integration points

### 5. Test Data Management

- While `MockTestData` structure exists, it lacks query implementation
- Methods like `get_file_by_key`, `get_files_by_user` in `PostgresClient` have no mock logic
- The mock connection doesn't translate diesel queries to operations on test data

## SQLite Integration

### Implementation Details

SQLite support has been added alongside PostgreSQL using diesel-async's `SyncConnectionWrapper`:

1. **Type Alias**: `AsyncSqliteConnection` wraps `diesel::SqliteConnection` for async compatibility
2. **Connection Pool**: Uses the same bb8 pooling as PostgreSQL
3. **Unified Interface**: Both backends implement the same `DbConnection` trait
4. **Enum Dispatch**: `AnyAsyncConnection` handles routing between PostgreSQL and SQLite

### Configuration

Cargo.toml dependencies have been updated:
```toml
diesel = { version = "2.2.4", features = ["postgres", "sqlite", "chrono", "numeric"] }
diesel-async = { version = "0.5.0", features = ["bb8", "postgres", "sqlite"] }
```

### Limitations

- The `AsyncConnection` implementation for `AnyAsyncConnection` currently only supports PostgreSQL backend due to diesel's type constraints
- SQLite connections will panic if PostgreSQL-specific queries are attempted
- This is a diesel limitation where different backends have incompatible type systems

## Root Cause

The primary issue is that implementing a full mock for diesel-async's `AsyncConnection` trait requires:

- Proper query parsing and execution against in-memory data
- Result set construction matching diesel's expectations
- Transaction state management
- Cursor/stream implementations for query results

This is complex because diesel expects specific low-level database behaviors that are difficult to mock without essentially reimplementing a mini database engine.

## Recommendations

### Short-term Solutions

1. **Use Test Database**: For integration tests, use a real PostgreSQL instance (Docker container) instead of mocks
2. **Unit Test Boundaries**: Test business logic separately from database logic by extracting pure functions
3. **Repository Pattern**: Create a higher-level abstraction layer that's easier to mock than diesel connections

### Long-term Solutions

1. **SQL Parser Integration**: Use a SQL parser library to interpret diesel queries and execute them against in-memory data
2. **Alternative Mocking Strategy**: Consider using libraries like `sqlx` which have better mocking support
3. **Test Fixtures**: Use database fixtures with transaction rollback for integration tests

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| MockTestData | ✅ Partial | Structure exists but lacks query execution |
| MockErrorConfig | ✅ Complete | Error simulation works |
| MockAsyncConnection | ❌ Incomplete | Diesel trait not fully implemented |
| MockDbConnection | ✅ Partial | Wrapper exists but depends on MockAsyncConnection |
| MockTransactionManager | ✅ Basic | Simple implementation exists |
| Feature Flag Integration | ❌ Disabled | Code is commented out |
| Query Execution | ❌ Missing | No translation from SQL to in-memory operations |

## Next Steps

1. **Immediate**: Enable the existing mock infrastructure by uncommenting the feature flag code
2. **Short-term**: Implement basic query execution for common patterns (SELECT, INSERT, UPDATE, DELETE)
3. **Medium-term**: Add support for JOIN queries and complex WHERE clauses
4. **Long-term**: Consider architectural changes to make the system more testable
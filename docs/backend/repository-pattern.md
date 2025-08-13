# Repository Pattern Architecture

## Architecture Overview

The database system uses a clean repository pattern with transparent test transaction management through the SmartPool pattern. This provides:

1. **Production Repository** - Real database operations with connection pooling
2. **Mock Repository** - In-memory implementation for unit testing
3. **SmartPool** - Automatic test transaction management
4. **Common Trait** - Unified interface for both implementations

## Core Components

### SmartPool Pattern

Located in `/backend/lib/src/repository/pool.rs`, the SmartPool provides:

- **Automatic Test Transactions**: In test mode, automatically wraps all operations in rolled-back transactions
- **Single Connection for Tests**: Pool size of 1 ensures transaction persistence across operations
- **Normal Pooling in Production**: Standard connection pooling with configurable size
- **Zero Runtime Overhead**: Test-specific code is compiled out in production builds

### Repository Implementation

The production repository (`/backend/lib/src/repository/postgres.rs`) provides:

- **Direct Database Access**: Uses diesel queries with async support
- **Transparent Test Behavior**: Same code works in test and production
- **Connection Management**: Delegates to SmartPool for connection handling
- **Error Handling**: Proper error conversion and context

### Mock Repository

The mock repository (`/backend/lib/src/repository/mock.rs`) provides:

- **In-Memory Storage**: HashMap-based storage for all entities
- **Atomic ID Generation**: Thread-safe ID generation for new entities
- **Async Interface**: Matches production repository interface exactly
- **No Database Dependency**: Enables fast unit testing

### StorageOperations Trait

The common trait (`/backend/lib/src/repository/traits.rs`) defines:

- **Unified Interface**: Both repositories implement the same operations
- **Async Methods**: All operations are async for consistency
- **Type Safety**: Strong typing for all parameters and returns
- **Extensibility**: Easy to add new operations

## Database Operations

### Supported Operations

| Entity | Operations |
|--------|------------|
| BSP | create, get_by_id, update_capacity, list |
| Bucket | create, get_by_id, get_by_user |
| File | get_by_key, get_by_user, get_by_bucket |

### Error Handling

All operations return `Result<T, RepositoryError>` where:

- **Database Errors**: Diesel errors are wrapped and contextualized
- **Pool Errors**: Connection pool errors are handled gracefully
- **Not Found**: Explicit error for missing entities
- **Validation**: Input validation at repository boundary

## Testing Strategy

### Three-Level Testing

1. **Unit Tests with MockRepository**
   - Fast, in-memory testing
   - No database required
   - Tests business logic in isolation

2. **Integration Tests with Test Database**
   - Uses real PostgreSQL with test transactions
   - Automatically rolled back after each test
   - Tests actual SQL queries and database behavior

3. **End-to-End Tests**
   - Full application stack
   - Real database operations
   - API endpoint testing

### Test Helpers

Located in `/backend/lib/src/test_helpers.rs`:

```rust
// Create repository with test database
let repo = create_test_repository().await;

// Create mock repository
let mock = create_mock_repository();

// Create test application
let app = create_test_app().await;
```

## Configuration

### Database URLs

- **Production**: Set via `DATABASE_URL` environment variable
- **Test**: Set via `TEST_DATABASE_URL` or defaults to local test database
- **Mock**: No database URL required

### Connection Pool Settings

| Setting | Test Value | Production Value |
|---------|------------|------------------|
| Max Size | 1 | 32 |
| Min Idle | 0 | 5 |
| Timeout | 5s | 30s |

## Migration from Previous Architecture

### Removed Components

The following components have been removed in favor of the simpler repository pattern:

- **AnyBackend**: Multi-backend abstraction no longer needed
- **AnyConnection**: Connection switching handled at repository level
- **SQLite Support**: Focus on PostgreSQL with mocks for testing
- **Complex Mock System**: Replaced with simple MockRepository

### New Components

- **SmartPool**: Transparent test transaction management
- **Repository Pattern**: Clean separation of database logic
- **MockRepository**: Simple in-memory implementation
- **DBClient**: Simplified client using repository abstraction

## Implementation Details

### SmartPool Mechanics

1. **Pool Size = 1 in Tests**: Ensures all operations use the same connection
2. **First get() Initializes**: Calls `begin_test_transaction()` once
3. **AtomicBool Tracking**: Prevents multiple initialization (would panic)
4. **Automatic Rollback**: Test transaction never commits

### Repository Pattern Benefits

1. **Testability**: Easy to mock at repository level
2. **Simplicity**: No complex type abstractions
3. **Performance**: Direct database access without overhead
4. **Maintainability**: Clear separation of concerns

## Usage Examples

### Production Usage

```rust
#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").unwrap();
    let repo = Repository::new(&database_url).await.unwrap();
    let db_client = DBClient::new(Arc::new(repo));
    
    // Use db_client in application...
}
```

### Test Usage

```rust
#[tokio::test]
async fn test_bsp_creation() {
    let repo = create_test_repository().await;
    
    let bsp = repo.create_bsp(NewBsp {
        account: "test".to_string(),
        capacity: 1000,
    }).await.unwrap();
    
    assert_eq!(bsp.account, "test");
    // Automatically rolled back after test
}
```

### Mock Usage

```rust
#[test]
fn test_business_logic() {
    let mock = MockRepository::new();
    let service = MyService::new(Arc::new(mock));
    
    // Test without database...
}
```

## Performance Considerations

### Connection Pool Efficiency

- **Production**: 32 connections handle concurrent requests
- **Test**: Single connection reduces overhead
- **Mock**: No database overhead at all

### Query Optimization

- **Prepared Statements**: Diesel uses prepared statements
- **Connection Reuse**: Pool maintains warm connections
- **Batch Operations**: Repository supports batch operations

## Future Enhancements

### Potential Improvements

1. **Caching Layer**: Add Redis caching at repository level
2. **Read Replicas**: Support read/write splitting
3. **Metrics**: Add performance metrics to repository operations
4. **Audit Logging**: Track all database modifications

### Extension Points

- **Repository Middleware**: Add cross-cutting concerns
- **Query Builders**: Complex query composition
- **Migration Tools**: Automated schema migrations
- **Backup Strategy**: Point-in-time recovery support
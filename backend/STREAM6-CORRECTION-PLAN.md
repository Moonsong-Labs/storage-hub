# Stream 6 Correction Plan: Mock Architecture Redesign

## Current State (What Was Done)

### Misunderstood Implementation
I moved mocks to be alternate implementations at the client level:
```rust
// Current structure
data/
├── postgres/
│   ├── client.rs         // PostgresClient (production)
│   └── mock.rs           // MockPostgresClient (completely different implementation)
└── rpc/
    └── mock.rs           // MockStorageHubRpc (no real client exists)

// Usage pattern (WRONG)
let client: Arc<dyn PostgresClientTrait> = if config.mock_mode {
    Arc::new(MockPostgresClient::new())  // Completely different code path
} else {
    Arc::new(PostgresClient::new(...))   // Production code path
};
```

**Problem**: The production `PostgresClient` code is never tested. Tests use a completely different implementation (`MockPostgresClient`).

## Required State (From IMPROVEMENTS.md #16)

### Mock at Data Source Level
> "Mock should be at data source level, not client level... Real client uses mock connection when in mock mode"

The intent is to test the actual client implementation with mocked data sources:

```rust
// Desired structure
data/
├── postgres/
│   ├── client.rs         // PostgresClient (always used)
│   ├── connection.rs     // Connection trait/abstraction
│   ├── pg_connection.rs  // Real PostgreSQL connection
│   └── mock_connection.rs // Mock connection
└── rpc/
    ├── client.rs         // StorageHubRpcClient (always used)
    ├── connection.rs     // RPC connection trait
    ├── ws_connection.rs  // Real WebSocket connection
    └── mock_connection.rs // Mock RPC connection
```

## Work Required

### 1. Design Connection Abstraction for PostgreSQL

Create a trait that abstracts the database connection:

```rust
// data/postgres/connection.rs
#[async_trait]
pub trait DbConnection: Send + Sync {
    async fn execute(&self, query: &str, params: &[&dyn ToSql]) -> Result<Vec<Row>>;
    async fn execute_one(&self, query: &str, params: &[&dyn ToSql]) -> Result<Row>;
    async fn test_connection(&self) -> Result<()>;
}

// Real implementation
pub struct PgConnection {
    pool: PgPool,
}

// Mock implementation  
pub struct MockConnection {
    data: Arc<Mutex<MockDatabase>>,
}
```

### 2. Refactor PostgresClient to Use Connection Abstraction

```rust
// data/postgres/client.rs
pub struct PostgresClient {
    conn: Arc<dyn DbConnection>,  // Uses abstraction
}

impl PostgresClient {
    pub fn new(conn: Arc<dyn DbConnection>) -> Self {
        Self { conn }
    }
}

// Now ALL the PostgresClient logic is tested!
impl PostgresClientTrait for PostgresClient {
    async fn get_file_by_key(&self, file_key: &[u8]) -> Result<File> {
        // This code runs in both prod AND test!
        let row = self.conn.execute_one(
            "SELECT * FROM files WHERE file_key = $1",
            &[&file_key]
        ).await?;
        
        // Conversion logic is tested!
        Ok(File::from_row(row)?)
    }
}
```

### 3. Update Binary to Create Appropriate Connection

```rust
// bin/src/main.rs
let db_connection: Arc<dyn DbConnection> = if config.database.mock_mode {
    Arc::new(MockConnection::new())
} else {
    Arc::new(PgConnection::new(&config.database.url).await?)
};

let postgres_client = Arc::new(PostgresClient::new(db_connection));
```

### 4. Create Mock Connection Implementation

```rust
// data/postgres/mock_connection.rs
pub struct MockConnection {
    data: Arc<Mutex<MockDatabase>>,
}

impl MockConnection {
    pub fn new() -> Self {
        // Initialize with test data
    }
    
    pub fn add_file(&self, file: File) {
        // Allow tests to add data
    }
}

#[async_trait]
impl DbConnection for MockConnection {
    async fn execute_one(&self, query: &str, params: &[&dyn ToSql]) -> Result<Row> {
        // Parse query and return mock data
        if query.contains("SELECT * FROM files") {
            // Return mock file data
        }
    }
}
```

### 5. Implement RPC Client with Same Pattern

Since no real RPC client exists yet, implement it properly:

```rust
// data/rpc/connection.rs
#[async_trait]
pub trait RpcConnection: Send + Sync {
    async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T>;
}

// data/rpc/client.rs
pub struct StorageHubRpcClient {
    conn: Arc<dyn RpcConnection>,
}

// Real and mock connections follow same pattern as PostgreSQL
```

### 6. Delete Current Mock Clients

Remove:
- `data/postgres/mock.rs` (MockPostgresClient)
- `data/rpc/mock.rs` (MockStorageHubRpc)

These will be replaced by mock connections.

## Benefits of Correct Implementation

1. **Production Code Gets Tested**: The actual `PostgresClient` implementation runs in tests
2. **Bug Detection**: Logic errors in query building, parameter binding, and result parsing are caught
3. **Realistic Testing**: Mock connections can simulate errors, delays, and edge cases
4. **Single Implementation**: No duplicate client logic to maintain
5. **Better Coverage**: All code paths in the client are exercised

## Migration Strategy

1. **Phase 1**: Create connection abstractions without breaking existing code
2. **Phase 2**: Refactor PostgresClient to use connection abstraction
3. **Phase 3**: Update all usages to create connections instead of clients
4. **Phase 4**: Remove old mock clients
5. **Phase 5**: Implement RPC client with same pattern

## Testing Example

```rust
#[test]
async fn test_postgres_client_error_handling() {
    // Create mock that simulates connection failure
    let mock_conn = Arc::new(MockConnection::new());
    mock_conn.set_error_mode(true);
    
    // Test REAL PostgresClient with mock connection
    let client = PostgresClient::new(mock_conn);
    
    // This tests the actual error handling in PostgresClient!
    let result = client.get_file_by_key(&[1, 2, 3]).await;
    assert!(matches!(result, Err(Error::Database(_))));
}
```

## Estimated Effort

- Design connection abstractions: 2-3 hours
- Refactor PostgresClient: 3-4 hours  
- Implement mock connections: 2-3 hours
- Update all usages: 1-2 hours
- Implement RPC client properly: 4-6 hours
- Testing and documentation: 2-3 hours

**Total**: 14-21 hours (vs original estimate of 4-6 hours for implementation)

## Success Criteria

1. Only one PostgresClient implementation exists
2. All tests use the real PostgresClient with mock connections
3. Production code paths are fully tested
4. Mock connections can simulate various scenarios (errors, delays, data conditions)
5. Same pattern applied to RPC client
# Comprehensive Testing Strategy for Diesel + PostgreSQL

## The Problem

You're right - mocking repositories doesn't test if your SQL queries are correct. You need to test against a real database, but also need tests to be fast and maintainable.

## Recommended Strategy: Three-Layer Testing Pyramid

### Layer 1: Transaction Rollback Tests (80% of tests)
**Fast, accurate, everyday workhorse**

```rust
// backend/lib/src/test_helpers.rs
use diesel_async::{AsyncPgConnection, AsyncConnection};
use once_cell::sync::Lazy;
use deadpool_diesel::postgres::{Pool, Manager};

// Single shared test database pool
pub static TEST_POOL: Lazy<Pool> = Lazy::new(|| {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test:test@localhost/storage_hub_test".to_string());
    
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager)
        .max_size(1) // Single connection for serial tests
        .build()
        .expect("Failed to create test pool");
    
    // Run migrations once
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut conn = pool.get().await.unwrap();
        run_migrations(&mut conn).await;
    });
    
    pool
});

/// Run a test in a transaction that's automatically rolled back
pub async fn test_transaction<F, R>(test: F) -> R
where
    F: FnOnce(AsyncPgConnection) -> BoxFuture<'static, R>,
{
    let mut conn = TEST_POOL.get().await.unwrap();
    
    // Start a transaction
    let mut transaction = conn.begin().await.unwrap();
    
    // Run the test
    let result = test(transaction).await;
    
    // Transaction automatically rolls back when dropped
    // No commit = automatic rollback
    
    result
}
```

**Usage:**
```rust
#[tokio::test]
async fn test_bsp_creation() {
    test_transaction(|mut conn| async move {
        // This is REAL Diesel code against REAL PostgreSQL
        let new_bsp = NewBsp {
            account: "test-account".to_string(),
            capacity: 1000,
        };
        
        let created = diesel::insert_into(bsp::table)
            .values(&new_bsp)
            .get_result::<Bsp>(&mut conn)
            .await
            .unwrap();
        
        assert_eq!(created.account, "test-account");
        
        // Verify it's actually in the database
        let found = bsp::table
            .find(created.id)
            .first::<Bsp>(&mut conn)
            .await
            .unwrap();
        
        assert_eq!(found.id, created.id);
        
        // When this function ends, transaction rolls back
        // Database is clean for next test
    }).await;
}
```

**Pros:**
- ✅ Tests real SQL queries
- ✅ Very fast (milliseconds per test)
- ✅ No cleanup needed
- ✅ Same database as production

**Cons:**
- ❌ Tests must run serially (`cargo test -- --test-threads=1`)
- ❌ Can't test transaction-level code

### Layer 2: Testcontainers for Isolation (15% of tests)
**When you need parallel execution or transaction testing**

```rust
// backend/lib/src/test_helpers/containers.rs
use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage};
use testcontainers_modules::postgres::Postgres;

pub async fn setup_test_container() -> (String, ContainerAsync<Postgres>) {
    let container = Postgres::default()
        .with_db_name("test_db")
        .with_user("test_user")
        .with_password("test_pass")
        .start()
        .await
        .expect("Failed to start PostgreSQL container");
    
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let connection_string = format!(
        "postgres://test_user:test_pass@localhost:{}/test_db",
        port
    );
    
    // Run migrations
    let pool = create_pool(&connection_string).await.unwrap();
    run_migrations(&pool).await.unwrap();
    
    (connection_string, container)
}

/// Pre-seeded container for complex scenarios
pub async fn setup_seeded_container() -> (String, ContainerAsync<Postgres>) {
    let (url, container) = setup_test_container().await;
    
    // Seed with test data
    let pool = create_pool(&url).await.unwrap();
    let mut conn = pool.get().await.unwrap();
    
    // Insert test data
    diesel::sql_query("
        INSERT INTO bsp (account, capacity) VALUES 
        ('alice', 1000),
        ('bob', 2000),
        ('charlie', 3000)
    ")
    .execute(&mut conn)
    .await
    .unwrap();
    
    (url, container)
}
```

**Usage:**
```rust
#[tokio::test]
async fn test_concurrent_operations() {
    let (db_url, _container) = setup_test_container().await;
    let pool = create_pool(&db_url).await.unwrap();
    
    // Test parallel operations
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let mut conn = pool.get().await.unwrap();
                // Each task gets its own connection
                create_bsp(&mut conn, &format!("account-{}", i)).await
            })
        })
        .collect();
    
    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 10);
}

#[tokio::test]
async fn test_transaction_commit() {
    let (db_url, _container) = setup_test_container().await;
    let mut conn = establish_connection(&db_url).await.unwrap();
    
    // Test actual transaction behavior
    conn.transaction(|conn| async move {
        create_bsp(conn, "test").await?;
        // Actually commits
        Ok::<_, diesel::result::Error>(())
    }).await.unwrap();
    
    // Verify commit worked
    let count = bsp::table.count().get_result(&mut conn).await.unwrap();
    assert_eq!(count, 1);
}
```

### Layer 3: Database Fixtures & Snapshots (5% of tests)
**For complex scenarios with lots of test data**

```rust
// backend/lib/src/test_helpers/fixtures.rs

pub struct DatabaseFixture {
    pool: Pool,
    snapshot_id: Option<String>,
}

impl DatabaseFixture {
    /// Create a fixture with pre-loaded data
    pub async fn new(scenario: &str) -> Self {
        let pool = TEST_POOL.clone();
        let mut conn = pool.get().await.unwrap();
        
        // Load fixture data based on scenario
        match scenario {
            "complex_bsp_network" => {
                diesel::sql_query(include_str!("../fixtures/complex_bsp_network.sql"))
                    .execute(&mut conn)
                    .await
                    .unwrap();
            },
            "payment_test_data" => {
                diesel::sql_query(include_str!("../fixtures/payment_test_data.sql"))
                    .execute(&mut conn)
                    .await
                    .unwrap();
            },
            _ => panic!("Unknown fixture scenario"),
        }
        
        Self { pool, snapshot_id: None }
    }
    
    /// Create a database snapshot for quick restore
    pub async fn snapshot(&mut self) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        
        // PostgreSQL-specific: Create a savepoint
        diesel::sql_query(&format!("SAVEPOINT {}", snapshot_id))
            .execute(&mut conn)
            .await?;
        
        self.snapshot_id = Some(snapshot_id);
        Ok(())
    }
    
    /// Restore to snapshot point
    pub async fn restore(&self) -> Result<()> {
        if let Some(ref snapshot_id) = self.snapshot_id {
            let mut conn = self.pool.get().await?;
            diesel::sql_query(&format!("ROLLBACK TO SAVEPOINT {}", snapshot_id))
                .execute(&mut conn)
                .await?;
        }
        Ok(())
    }
}
```

## Choosing the Right Strategy

### Use Transaction Rollback When:
- Testing simple CRUD operations
- Testing query correctness
- Testing business logic with database
- You want fastest possible tests

### Use Testcontainers When:
- Testing concurrent operations
- Testing transaction behavior
- Need complete isolation
- Testing database-level features (indexes, constraints)
- Running tests in parallel

### Use Fixtures/Snapshots When:
- Complex test scenarios with lots of data
- Performance testing
- Testing migrations
- Testing complex reports/analytics

## Performance Comparison

| Strategy | Setup Time | Test Time | Cleanup | Isolation |
|----------|------------|-----------|---------|-----------|
| Transaction Rollback | 0ms (shared) | 5-50ms | 0ms | Transaction |
| Testcontainers | 3-5s | 10-100ms | 1s | Complete |
| Fixtures | 100-500ms | 10-100ms | 100ms | Partial |
| Mock Repository | 0ms | 1-5ms | 0ms | Complete |

## Implementation Plan

### Phase 1: Basic Setup (Week 1)
```rust
// 1. Create test database
createdb storage_hub_test

// 2. Add test helpers module
mod test_helpers {
    pub mod transactions;
    pub mod containers;
    pub mod fixtures;
}

// 3. Configure test environment
// .env.test
TEST_DATABASE_URL=postgres://localhost/storage_hub_test
```

### Phase 2: Migrate Existing Tests (Week 2)
```rust
// Before: Mock repository
#[test]
fn test_something() {
    let mock = MockRepository::new();
    // ...
}

// After: Real database
#[tokio::test]
async fn test_something() {
    test_transaction(|conn| async move {
        // Real Diesel queries
    }).await;
}
```

### Phase 3: Add Integration Tests (Week 3)
```rust
// tests/integration/concurrent_operations.rs
#[tokio::test]
async fn test_concurrent_bsp_creation() {
    let (url, _container) = setup_test_container().await;
    // Test with real isolation
}
```

## CI/CD Configuration

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: test
          POSTGRES_DB: storage_hub_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run transaction tests
        env:
          TEST_DATABASE_URL: postgres://postgres:test@localhost/storage_hub_test
        run: cargo test --lib -- --test-threads=1
  
  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run integration tests
        run: cargo test --test '*' 
```

## Best Practices

### 1. Test Organization
```
tests/
├── unit/           # Transaction rollback tests
├── integration/    # Testcontainer tests  
└── fixtures/       # SQL fixture files
```

### 2. Test Naming
```rust
#[tokio::test]
async fn test_bsp_creation_with_valid_data() { }

#[tokio::test]
async fn test_bsp_creation_fails_with_duplicate_account() { }
```

### 3. Assertion Helpers
```rust
// Custom assertions for common patterns
async fn assert_bsp_exists(conn: &mut AsyncPgConnection, id: i64) {
    let exists = bsp::table
        .find(id)
        .select(bsp::id)
        .first::<i64>(conn)
        .await
        .is_ok();
    assert!(exists, "BSP with id {} should exist", id);
}
```

## Summary

This strategy gives you:
- ✅ **Real SQL testing** - No mocking, actual queries
- ✅ **Fast feedback** - Transaction tests run in milliseconds
- ✅ **Production parity** - Same PostgreSQL version
- ✅ **Flexibility** - Different strategies for different needs
- ✅ **Maintainability** - Clear test organization

The key insight: **Don't try to mock Diesel or PostgreSQL**. Instead, use transactions for speed and containers for isolation. This gives you confidence that your queries actually work while keeping tests fast and maintainable.
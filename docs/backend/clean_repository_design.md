# Clean Repository Design with SmartPool Pattern

## Core Design: SmartPool with Automatic Test Transactions

The repository pattern uses a SmartPool that transparently handles different behaviors for test and production environments:

```rust
// backend/lib/src/repository/pool.rs

use deadpool_diesel::postgres::{Pool, Manager, Object};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A smart connection pool that handles test transactions transparently
pub struct SmartPool {
    inner: Arc<Pool>,
    #[cfg(test)]
    test_tx_initialized: AtomicBool,  // No Arc needed - AtomicBool is already thread-safe!
}

impl SmartPool {
    pub async fn new(database_url: &str) -> Result<Self> {
        let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
        
        #[cfg(test)]
        let pool = Pool::builder(manager)
            .max_size(1)  // Single connection for tests - ensures persistence
            .build()?;
        
        #[cfg(not(test))]
        let pool = Pool::builder(manager)
            .max_size(32)  // Normal pool size for production
            .build()?;
            
        Ok(Self { 
            inner: Arc::new(pool),
            #[cfg(test)]
            test_tx_initialized: AtomicBool::new(false),
        })
    }
    
    pub async fn get(&self) -> Result<Object> {
        let mut conn = self.inner.get().await?;
        
        #[cfg(test)]
        {
            // Pool size is 1, so this is always the same connection
            // Only initialize test transaction once to avoid panic
            if !self.test_tx_initialized.load(Ordering::Acquire) {
                conn.begin_test_transaction().await?;
                self.test_tx_initialized.store(true, Ordering::Release);
            }
        }
        
        Ok(conn)
    }
}
```

## Repository Implementation - Clean and Simple

```rust
// backend/lib/src/repository/mod.rs

/// Repository now only needs a SmartPool
pub struct Repository {
    pool: SmartPool,
}

impl Repository {
    pub async fn new(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: SmartPool::new(database_url).await?,
        })
    }
    
    pub async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp> {
        let mut conn = self.pool.get().await?;
        
        // Identical code for test and production!
        use crate::schema::bsp::dsl::*;
        diesel::insert_into(bsp)
            .values(&new_bsp)
            .get_result(&mut *conn)
            .await
    }
    
    pub async fn get_bsp_by_id(&self, id: i64) -> Result<Option<Bsp>> {
        let mut conn = self.pool.get().await?;
        
        use crate::schema::bsp::dsl;
        dsl::bsp
            .find(id)
            .first(&mut *conn)
            .await
            .optional()
    }
    
    pub async fn update_bsp_capacity(&self, id: i64, new_capacity: i64) -> Result<Bsp> {
        let mut conn = self.pool.get().await?;
        
        use crate::schema::bsp::dsl::*;
        diesel::update(bsp.find(id))
            .set(capacity.eq(new_capacity))
            .get_result(&mut *conn)
            .await
    }
    
    // All methods are identical for test and production
    // The SmartPool handles the difference transparently
}
```

## Separate Mock Repository

```rust
// backend/lib/src/repository/mock.rs

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::RwLock;

/// Completely separate mock implementation
pub struct MockRepository {
    bsps: Arc<RwLock<HashMap<i64, Bsp>>>,
    buckets: Arc<RwLock<HashMap<i64, Bucket>>>,
    next_id: Arc<AtomicI64>,
}

impl MockRepository {
    pub fn new() -> Self {
        Self {
            bsps: Arc::new(RwLock::new(HashMap::new())),
            buckets: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicI64::new(1)),
        }
    }
    
    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
    
    pub async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp> {
        let mut bsps = self.bsps.write().await;
        let id = self.next_id();
        
        let bsp = Bsp {
            id,
            account: new_bsp.account,
            capacity: new_bsp.capacity,
            multiaddresses: new_bsp.multiaddresses,
        };
        
        bsps.insert(id, bsp.clone());
        Ok(bsp)
    }
    
    pub async fn get_bsp_by_id(&self, id: i64) -> Result<Option<Bsp>> {
        let bsps = self.bsps.read().await;
        Ok(bsps.get(&id).cloned())
    }
    
    // ... other mock implementations
}
```

## Common Trait for Both

```rust
// backend/lib/src/repository/trait.rs

#[async_trait]
pub trait StorageOperations: Send + Sync {
    async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp>;
    async fn get_bsp_by_id(&self, id: i64) -> Result<Option<Bsp>>;
    async fn update_bsp_capacity(&self, id: i64, capacity: i64) -> Result<Bsp>;
    async fn list_bsps(&self, limit: i64, offset: i64) -> Result<Vec<Bsp>>;
    // ... all other operations
}

// Real database implementation
#[async_trait]
impl StorageOperations for Repository {
    async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp> {
        self.create_bsp(new_bsp).await
    }
    
    async fn get_bsp_by_id(&self, id: i64) -> Result<Option<Bsp>> {
        self.get_bsp_by_id(id).await
    }
    
    // ... delegate to actual methods
}

// Mock implementation
#[async_trait]
impl StorageOperations for MockRepository {
    async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp> {
        self.create_bsp(new_bsp).await
    }
    
    async fn get_bsp_by_id(&self, id: i64) -> Result<Option<Bsp>> {
        self.get_bsp_by_id(id).await
    }
    
    // ... delegate to mock methods
}
```

## Why This Design Works

### Key Insights

1. **Single Connection in Tests**: Pool size of 1 ensures all operations use the same connection
2. **Automatic Test Transactions**: First `get()` initializes test transaction, subsequent calls reuse it
3. **No Panic on Reuse**: The `test_tx_initialized` flag prevents calling `begin_test_transaction` twice
4. **Zero Runtime Overhead**: `cfg(test)` ensures test code doesn't exist in production builds
5. **Same Repository Code**: Repository doesn't know or care about test vs production

### The Magic of `begin_test_transaction`

- Sets the connection to never commit (rollback-only mode)
- Panics if called on a connection already in a transaction
- That's why we track initialization with `AtomicBool`
- Once set, the connection stays in test mode until dropped

## Test Helpers

```rust
// backend/lib/src/test_helpers.rs

/// Create a test app with automatic test transactions
#[cfg(test)]
pub async fn create_test_app() -> Router {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test:test@localhost/test_db".to_string());
    
    let repo = Repository::new(&database_url).await
        .expect("Failed to create repository");
    
    create_app(repo)
}

/// Create a production app
pub async fn create_production_app(database_url: &str) -> Router {
    let repo = Repository::new(database_url).await?;
    create_app(repo)
}
```

## Usage in Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_repository_directly() {
        let repo = Repository::new(&test_database_url()).await.unwrap();
        
        // Create data - automatically in test transaction
        let bsp = repo.create_bsp(NewBsp {
            account: "test".to_string(),
            capacity: 1000,
        }).await.unwrap();
        
        // Verify it's there
        let found = repo.get_bsp_by_id(bsp.id).await.unwrap();
        assert!(found.is_some());
        
        // Everything rolls back when test ends
    }
    
    #[tokio::test]
    async fn test_endpoint() {
        let app = create_test_app().await;
        let client = TestClient::new(app);
        
        // Test the actual production endpoint
        let response = client
            .post("/bsp")
            .json(&json!({
                "account": "test",
                "capacity": 1000
            }))
            .send()
            .await;
        
        assert_eq!(response.status(), 200);
        
        // Verify it exists (within same test transaction)
        let response = client.get("/bsp/1").send().await;
        assert_eq!(response.status(), 200);
        
        // Everything rolls back automatically
    }
    
    #[tokio::test]
    async fn test_with_mock() {
        let repo: Arc<dyn StorageOperations> = Arc::new(MockRepository::new());
        
        // Test without any database
        let bsp = repo.create_bsp(NewBsp {
            account: "test".to_string(),
            capacity: 1000,
        }).await.unwrap();
        
        assert_eq!(bsp.account, "test");
    }
}
```

## Application Structure

```rust
// backend/bin/src/main.rs

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").unwrap();
    
    // Production: SmartPool automatically uses normal connections
    let repo = Repository::new(&database_url).await.unwrap();
    
    let app = create_app(repo);
    // Run app...
}

// backend/tests/integration.rs

#[tokio::test]
async fn test_full_application() {
    // Tests: SmartPool automatically uses test transactions
    let app = create_test_app().await;
    let client = TestClient::new(app);
    
    // Test endpoints with automatic rollback
    let response = client.post("/bsp").json(&data).send().await;
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_business_logic() {
    // Unit tests use MockRepository for speed
    let repo: Arc<dyn StorageOperations> = Arc::new(
        MockRepository::new()
    );
    
    // Test without database...
}
```

## Summary

This design provides a complete testing strategy:

### Capabilities
1. **Mock data for business logic tests** - Separate `MockRepository` with in-memory storage
2. **Testing queries against real database** - Test transactions ensure rollback
3. **Production database support** - Normal connections without overhead
4. **Shared client logic** - Same Repository code for all environments
5. **Endpoint testing** - Can test production handlers with automatic test transactions

### Key Design Elements
- **SmartPool Pattern**: Handles test vs production transparently
- **Pool Size 1 for Tests**: Ensures connection persistence
- **AtomicBool Tracking**: Prevents panic from multiple `begin_test_transaction` calls
- **Compile-Time Gating**: `cfg(test)` eliminates runtime overhead
- **No Wrapper Types**: Returns raw connections, keeping it simple

The design is clean, simple, and provides exactly what's needed for comprehensive testing.
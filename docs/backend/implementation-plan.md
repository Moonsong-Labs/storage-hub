# Database Architecture Implementation Plan

## Overview

This plan details the complete refactoring of the database backend from a complex multi-backend system to a clean repository pattern with SmartPool for transparent test transaction management.

## Phase 1: Remove Outdated Components

### Objective
Remove all outdated database abstraction layers and multi-backend support infrastructure to prepare for the simplified repository pattern implementation.

### Steps

1. **Remove Multi-Backend Abstractions**
   - File: `backend/lib/src/data/any_backend.rs`
   - Operation: Delete file
   - Details: The AnyBackend abstraction is no longer needed with repository pattern
   - Success: File deleted, imports updated

2. **Remove AnyConnection Abstraction**
   - File: `backend/lib/src/data/any_connection.rs`
   - Operation: Delete file
   - Details: Multi-connection abstraction replaced by SmartPool
   - Success: File deleted, references removed

3. **Remove SQLite Implementation**
   - File: `backend/lib/src/data/sqlite/` entire directory
   - Operation: Delete directory and all contents
   - Details: SQLite support removed in favor of PostgreSQL-only with mock repository
   - Success: Directory removed, module declarations updated

4. **Remove Connection Abstractions**
   - Files:
     - `backend/lib/src/data/postgres/connection.rs` - Contains AnyDbConnection and AnyAsyncConnection enums
     - `backend/lib/src/data/postgres/pg_connection.rs` - PgConnection wrapper
     - `backend/lib/src/data/postgres/mock_connection.rs` - Old mock connection system
   - Operation: Delete files
   - Details: These abstractions will be replaced by SmartPool and repository pattern
   - Success: Files deleted, module exports updated

5. **Clean Up Dependencies**
   - File: Root `Cargo.toml`
   - Operation: Remove SQLite features from diesel dependencies
   - Details: Remove "sqlite" from diesel and diesel-async feature lists
   - Success: Dependencies updated to PostgreSQL-only

6. **Update Module Declarations**
   - Files:
     - `backend/lib/src/data/mod.rs` - Remove deleted module declarations
     - `backend/lib/src/data/postgres/mod.rs` - Remove connection module exports
   - Operation: Update module declarations
   - Details: Clean up references to removed modules
   - Success: Module structure updated

7. **Stub PostgresClient**
   - File: `backend/lib/src/data/postgres/client.rs`
   - Operation: Replace implementation with stubs returning NotImplemented errors
   - Details: Maintain method signatures as guide for repository implementation
   - Success: Client stubbed with all method signatures preserved

8. **Update Main Binary**
   - File: `backend/bin/src/main.rs`
   - Operation: Remove old database initialization code
   - Details: Remove references to AnyDbConnection and connection initialization
   - Success: Binary updated to compile without connection abstractions

### Testing Strategy
- Verify compilation with `SKIP_WASM_BUILD=1 cargo check`
- Ensure no dangling references to removed types
- Document any compilation issues and resolutions

### Rollback Plan
Git revert to restore deleted files if critical functionality is discovered

---

## Phase 2: Implement SmartPool

### Prerequisites
- [ ] Phase 1 completed successfully
- [ ] Deadpool-diesel dependency added
- [ ] Test database available for integration tests

### Steps

1. **Create SmartPool Module**
   - File: `backend/lib/src/repository/pool.rs` (create new)
   - Operation: Implement SmartPool with automatic test transaction support
   - Details:
     ```rust
     use deadpool_diesel::postgres::{Pool, Manager, Object};
     use std::sync::Arc;
     use std::sync::atomic::{AtomicBool, Ordering};
     
     pub struct SmartPool {
         inner: Arc<Pool>,
         #[cfg(test)]
         test_tx_initialized: AtomicBool,
     }
     
     impl SmartPool {
         pub async fn new(database_url: &str) -> Result<Self, Error> {
             let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
             
             #[cfg(test)]
             let pool = Pool::builder(manager)
                 .max_size(1)  // Single connection for tests
                 .build()?;
             
             #[cfg(not(test))]
             let pool = Pool::builder(manager)
                 .max_size(32)  // Normal pool size
                 .build()?;
                 
             Ok(Self { 
                 inner: Arc::new(pool),
                 #[cfg(test)]
                 test_tx_initialized: AtomicBool::new(false),
             })
         }
         
         pub async fn get(&self) -> Result<Object, Error> {
             let mut conn = self.inner.get().await?;
             
             #[cfg(test)]
             {
                 if !self.test_tx_initialized.load(Ordering::Acquire) {
                     conn.begin_test_transaction().await?;
                     self.test_tx_initialized.store(true, Ordering::Release);
                 }
             }
             
             Ok(conn)
         }
     }
     ```
   - Success: SmartPool compiles and can create connections

2. **Create Error Types**
   - File: `backend/lib/src/repository/error.rs` (create new)
   - Operation: Define repository-specific error types
   - Details:
     ```rust
     use thiserror::Error;
     
     #[derive(Debug, Error)]
     pub enum RepositoryError {
         #[error("Database error: {0}")]
         Database(#[from] diesel::result::Error),
         #[error("Pool error: {0}")]
         Pool(#[from] deadpool_diesel::PoolError),
         #[error("Not found")]
         NotFound,
     }
     ```
   - Success: Error types defined and integrated

3. **Update Module Structure**
   - File: `backend/lib/src/repository/mod.rs` (create new)
   - Operation: Export pool and error modules
   - Details:
     ```rust
     pub mod pool;
     pub mod error;
     
     pub use pool::SmartPool;
     pub use error::RepositoryError;
     ```
   - Success: Repository module properly structured

### Testing Strategy
- [ ] Unit test SmartPool creation with valid/invalid URLs
- [ ] Integration test verifying test transaction behavior
- [ ] Load test confirming production pool sizing

### Rollback Plan
Remove repository module, restore previous connection logic

---

## Phase 3: Implement Repository Pattern

### Prerequisites
- [ ] Phase 2 completed successfully
- [ ] SmartPool working in test and production modes
- [ ] Schema definitions available

### Steps

1. **Define Repository Trait**
   - File: `backend/lib/src/repository/traits.rs` (create new)
   - Operation: Define common interface for real and mock repositories
   - Details:
     ```rust
     use async_trait::async_trait;
     
     #[async_trait]
     pub trait StorageOperations: Send + Sync {
         async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp, RepositoryError>;
         async fn get_bsp_by_id(&self, id: i64) -> Result<Option<Bsp>, RepositoryError>;
         async fn update_bsp_capacity(&self, id: i64, capacity: i64) -> Result<Bsp, RepositoryError>;
         async fn list_bsps(&self, limit: i64, offset: i64) -> Result<Vec<Bsp>, RepositoryError>;
         
         async fn create_bucket(&self, new_bucket: NewBucket) -> Result<Bucket, RepositoryError>;
         async fn get_bucket_by_id(&self, id: i64) -> Result<Option<Bucket>, RepositoryError>;
         async fn get_buckets_by_user(&self, user_id: &str) -> Result<Vec<Bucket>, RepositoryError>;
         
         async fn get_file_by_key(&self, key: &str) -> Result<Option<File>, RepositoryError>;
         async fn get_files_by_user(&self, user_id: &str) -> Result<Vec<File>, RepositoryError>;
         async fn get_files_by_bucket(&self, bucket_id: i64) -> Result<Vec<File>, RepositoryError>;
     }
     ```
   - Success: Trait defined with all necessary operations

2. **Implement Production Repository**
   - File: `backend/lib/src/repository/postgres.rs` (create new)
   - Operation: Implement repository using SmartPool
   - Details:
     ```rust
     pub struct Repository {
         pool: SmartPool,
     }
     
     impl Repository {
         pub async fn new(database_url: &str) -> Result<Self, RepositoryError> {
             Ok(Self {
                 pool: SmartPool::new(database_url).await?,
             })
         }
         
         pub async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp, RepositoryError> {
             let mut conn = self.pool.get().await?;
             
             use crate::schema::bsp::dsl::*;
             diesel::insert_into(bsp)
                 .values(&new_bsp)
                 .get_result(&mut *conn)
                 .await
                 .map_err(Into::into)
         }
         
         // Implement all other methods similarly
     }
     ```
   - Success: All database operations implemented

3. **Implement Mock Repository**
   - File: `backend/lib/src/repository/mock.rs` (create new)
   - Operation: Create in-memory mock implementation
   - Details:
     ```rust
     use std::collections::HashMap;
     use std::sync::atomic::{AtomicI64, Ordering};
     use tokio::sync::RwLock;
     
     pub struct MockRepository {
         bsps: Arc<RwLock<HashMap<i64, Bsp>>>,
         buckets: Arc<RwLock<HashMap<i64, Bucket>>>,
         files: Arc<RwLock<HashMap<String, File>>>,
         next_id: Arc<AtomicI64>,
     }
     
     impl MockRepository {
         pub fn new() -> Self {
             Self {
                 bsps: Arc::new(RwLock::new(HashMap::new())),
                 buckets: Arc::new(RwLock::new(HashMap::new())),
                 files: Arc::new(RwLock::new(HashMap::new())),
                 next_id: Arc::new(AtomicI64::new(1)),
             }
         }
         
         fn next_id(&self) -> i64 {
             self.next_id.fetch_add(1, Ordering::SeqCst)
         }
         
         pub async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp, RepositoryError> {
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
         
         // Implement all other methods with in-memory storage
     }
     ```
   - Success: Mock repository fully functional

4. **Implement Trait for Both Repositories**
   - File: `backend/lib/src/repository/traits.rs` (append)
   - Operation: Add trait implementations
   - Details:
     ```rust
     #[async_trait]
     impl StorageOperations for Repository {
         async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp, RepositoryError> {
             self.create_bsp(new_bsp).await
         }
         // Delegate all methods
     }
     
     #[async_trait]
     impl StorageOperations for MockRepository {
         async fn create_bsp(&self, new_bsp: NewBsp) -> Result<Bsp, RepositoryError> {
             self.create_bsp(new_bsp).await
         }
         // Delegate all methods
     }
     ```
   - Success: Both repositories implement common trait

### Testing Strategy
- [ ] Unit tests for MockRepository with all CRUD operations
- [ ] Integration tests for Repository with real database
- [ ] Verify test transactions rollback properly
- [ ] Performance tests comparing mock vs real repository

### Rollback Plan
Keep old PostgresClient temporarily, switch back if issues arise

---

## Phase 4: Implement DBClient

### Prerequisites
- [ ] Phase 3 completed successfully
- [ ] Repository pattern working with both implementations
- [ ] API handlers ready for integration

### Steps

1. **Rename and Refactor PostgresClient to DBClient**
   - File: `backend/lib/src/data/postgres/client.rs` → `backend/lib/src/data/db_client.rs`
   - Operation: Refactor to use repository pattern
   - Details:
     ```rust
     use std::sync::Arc;
     use crate::repository::{StorageOperations, Repository, MockRepository};
     
     pub struct DBClient {
         repo: Arc<dyn StorageOperations>,
     }
     
     impl DBClient {
         pub async fn new(database_url: &str) -> Result<Self, Error> {
             let repo = Repository::new(database_url).await?;
             Ok(Self {
                 repo: Arc::new(repo),
             })
         }
         
         #[cfg(test)]
         pub fn new_mock() -> Self {
             Self {
                 repo: Arc::new(MockRepository::new()),
             }
         }
         
         pub async fn get_file_by_key(&self, key: &str) -> Result<Option<File>, Error> {
             self.repo.get_file_by_key(key).await
                 .map_err(|e| Error::Database(e.to_string()))
         }
         
         // Delegate all methods to repository
     }
     ```
   - Success: DBClient uses repository abstraction

2. **Update Services Module**
   - File: `backend/lib/src/services/mod.rs`
   - Operation: Replace PostgresClient with DBClient
   - Details:
     ```rust
     pub struct Services {
         pub db: Arc<DBClient>,
         pub rpc: Arc<RpcClient>,
         pub storage: Arc<dyn Storage>,
     }
     
     impl Services {
         pub async fn new(config: Config) -> Result<Self, Error> {
             let db = Arc::new(DBClient::new(&config.database_url).await?);
             // ... rest of initialization
         }
         
         #[cfg(test)]
         pub fn test() -> Self {
             Self {
                 db: Arc::new(DBClient::new_mock()),
                 // ... test implementations
             }
         }
     }
     ```
   - Success: Services integrated with new DBClient

3. **Update Main Application**
   - File: `backend/bin/src/main.rs`
   - Operation: Remove old connection logic, use new DBClient
   - Details:
     ```rust
     #[tokio::main]
     async fn main() -> Result<()> {
         let config = Config::from_env()?;
         let services = Services::new(config).await?;
         
         let app = create_app(services);
         // ... start server
     }
     ```
   - Success: Application uses new architecture

4. **Update API Handlers**
   - File: `backend/lib/src/api/handlers.rs`
   - Operation: Ensure handlers work with new DBClient
   - Details: No changes needed if DBClient maintains same interface
   - Success: All endpoints functional

### Testing Strategy
- [ ] End-to-end tests with test database
- [ ] Unit tests using MockRepository
- [ ] Integration tests verifying rollback behavior
- [ ] Performance benchmarks

### Rollback Plan
Keep old PostgresClient code commented until new system proven stable

---

## Phase 5: Testing and Documentation

### Prerequisites
- [ ] All previous phases completed
- [ ] New architecture fully implemented
- [ ] CI/CD pipeline ready for updates

### Steps

1. **Create Test Helpers**
   - File: `backend/lib/src/test_helpers.rs` (create new)
   - Operation: Helper functions for test setup
   - Details:
     ```rust
     #[cfg(test)]
     pub async fn create_test_repository() -> Repository {
         let database_url = std::env::var("TEST_DATABASE_URL")
             .unwrap_or_else(|_| "postgres://test:test@localhost/test_db".to_string());
         
         Repository::new(&database_url).await
             .expect("Failed to create test repository")
     }
     
     #[cfg(test)]
     pub fn create_mock_repository() -> MockRepository {
         MockRepository::new()
     }
     ```
   - Success: Test helpers available

2. **Write Comprehensive Tests**
   - File: `backend/tests/repository_tests.rs` (create new)
   - Operation: Test all repository operations
   - Details:
     - Test CRUD operations
     - Test transaction rollback
     - Test concurrent access
     - Test error conditions
   - Success: All tests passing

3. **Update Documentation**
   - Files:
     - `docs/backend/clean_repository_design.md` (already updated)
     - `docs/backend/postgresql-mocks.md` → `docs/backend/repository-pattern.md`
     - `README.md` (update database section)
   - Operation: Document new architecture
   - Success: Documentation reflects current implementation

4. **Migration Guide**
   - File: `docs/backend/migration-guide.md` (create new)
   - Operation: Document migration from old to new system
   - Details:
     - Breaking changes
     - Update instructions
     - Code examples
   - Success: Migration path documented

### Testing Strategy
- [ ] Run full test suite
- [ ] Manual testing of all endpoints
- [ ] Load testing
- [ ] Database migration testing

### Rollback Plan
Full git revert if critical issues discovered

---

## Success Criteria

### Phase 1 Success
- [ ] All outdated code removed
- [ ] No compilation errors
- [ ] Git history preserved

### Phase 2 Success
- [ ] SmartPool working in test and production
- [ ] Test transactions automatic
- [ ] Connection pooling functional

### Phase 3 Success
- [ ] Repository pattern implemented
- [ ] Mock repository functional
- [ ] Trait abstraction working

### Phase 4 Success
- [ ] DBClient integrated
- [ ] All endpoints functional
- [ ] Tests passing

### Phase 5 Success
- [ ] Full test coverage
- [ ] Documentation complete
- [ ] Performance acceptable

## Risk Mitigation

### High Risk Areas
1. **Test Transaction Behavior**: Thoroughly test begin_test_transaction behavior
2. **Connection Pool Size**: Monitor connection usage in production
3. **Mock Repository Performance**: Ensure adequate for large test datasets
4. **Migration Complexity**: Keep old code available during transition

### Contingency Plans
1. **Partial Rollback**: Can keep MockRepository while using old PostgresClient
2. **Gradual Migration**: Implement repository methods incrementally
3. **Feature Flags**: Use feature flags to switch between old/new systems
4. **Database Backup**: Ensure database backups before major changes

## Timeline Estimate

- Phase 1: 2-3 hours (removal and cleanup)
- Phase 2: 3-4 hours (SmartPool implementation)
- Phase 3: 4-6 hours (Repository pattern)
- Phase 4: 2-3 hours (DBClient integration)
- Phase 5: 3-4 hours (Testing and documentation)

**Total: 14-20 hours of development time**

## Notes

- The SmartPool pattern elegantly solves the test transaction problem
- Repository pattern provides clean separation of concerns
- Mock repository enables fast unit testing
- The architecture supports future extensions (e.g., caching layer)
- Consider adding metrics and logging to repository operations
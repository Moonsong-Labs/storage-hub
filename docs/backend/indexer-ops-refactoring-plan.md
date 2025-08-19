# Implementation Plan: Refactor Storage Traits to Separate Read and Write Operations

## Overview

Refactor the repository pattern to separate read-only operations (`IndexerOps`) from mutable operations (`IndexerOpsMut`), ensuring write operations are only available in test environments while maintaining type safety and avoiding runtime downcasting.

## Prerequisites

- [ ] Existing `StorageOperations` trait in `backend/lib/src/repository/mod.rs`
- [ ] MockRepository implementation in `backend/lib/src/repository/mock.rs`
- [ ] PostgreSQL Repository in `backend/lib/src/repository/postgres.rs`
- [ ] DBClient in `backend/lib/src/data/postgres/db_client.rs`
- [ ] Test files using the repository pattern

## Steps

### 1. **Rename and Split StorageOperations Trait**

- File: `backend/lib/src/repository/mod.rs`
- Operation: Replace `StorageOperations` trait (lines 93-192) with two new traits
- Details:
  ```rust
  // Line 93: Replace entire StorageOperations trait with:
  
  /// Read-only operations for indexer data access.
  #[async_trait]
  pub trait IndexerOps: Send + Sync {
      // Move all read-only methods here:
      async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>>;
      async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>>;
      async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>>;
      async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>>;
      async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>>;
      async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>>;
      async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>>;
  }
  
  /// Mutable operations for test environments.
  /// This trait always exists but implementations are conditional.
  #[async_trait]
  pub trait IndexerOpsMut: IndexerOps {
      async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp>;
      async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp>;
      async fn delete_bsp(&self, account: &str) -> RepositoryResult<()>;
      async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket>;
      async fn create_file(&self, new_file: NewFile) -> RepositoryResult<File>;
      async fn update_file_step(&self, file_key: &[u8], step: i32) -> RepositoryResult<()>;
      async fn delete_file(&self, file_key: &[u8]) -> RepositoryResult<()>;
      async fn clear_all(&self);
  }
  ```
- Success: Traits compile without errors

### 2. **Add Trait Aliases for Backward Compatibility**

- File: `backend/lib/src/repository/mod.rs`
- Operation: Add trait aliases after the trait definitions (around line 193)
- Details:
  ```rust
  // Production and mocks-only alias - read-only
  #[cfg(not(test))]
  pub trait StorageOperations: IndexerOps {}
  
  #[cfg(not(test))]
  impl<T: IndexerOps> StorageOperations for T {}
  
  // Test alias - read and write
  #[cfg(test)]
  pub trait StorageOperations: IndexerOps + IndexerOpsMut {}
  
  #[cfg(test)]
  impl<T: IndexerOps + IndexerOpsMut> StorageOperations for T {}
  ```
- Success: `StorageOperations` includes write operations only in tests, not with mocks feature alone

### 3. **Update MockRepository Implementation**

- File: `backend/lib/src/repository/mock.rs`
- Operation: Split trait implementations (lines 53-182)
- Details:
  ```rust
  // Line 53: Implement IndexerOps
  #[async_trait]
  impl IndexerOps for MockRepository {
      // Move read-only methods here:
      async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>> { /* existing */ }
      async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> { /* existing */ }
      async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>> { /* existing */ }
      async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>> { /* existing */ }
      async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>> { /* existing */ }
      async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>> { /* existing */ }
      async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>> { /* existing */ }
  }
  
  // MockRepository always implements IndexerOpsMut
  #[async_trait]
  impl IndexerOpsMut for MockRepository {
      // Move mutable methods here from both trait impl and standalone methods:
      async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp> { /* existing */ }
      async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp> { /* existing */ }
      async fn delete_bsp(&self, account: &str) -> RepositoryResult<()> { /* from line 223 */ }
      async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket> { /* existing */ }
      async fn create_file(&self, new_file: NewFile) -> RepositoryResult<File> { /* from line 188 */ }
      async fn update_file_step(&self, file_key: &[u8], step: i32) -> RepositoryResult<()> { /* from line 236 */ }
      async fn delete_file(&self, file_key: &[u8]) -> RepositoryResult<()> { /* from line 248 */ }
      async fn clear_all(&self) { /* from line 258 */ }
  }
  ```
- Success: MockRepository always has write operations available

### 4. **Update PostgreSQL Repository for Production**

- File: `backend/lib/src/repository/postgres.rs`
- Operation: Change trait implementation (line 56)
- Details:
  ```rust
  // Line 56: Replace StorageOperations with IndexerOps
  #[async_trait]
  impl IndexerOps for Repository {
      // Keep only read methods:
      async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>> { /* existing */ }
      async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> { /* existing */ }
      async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>> { /* existing */ }
      async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>> { /* existing */ }
      async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>> { /* existing */ }
      async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>> { /* existing */ }
      async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>> { /* existing */ }
  }
  ```
- Success: Production builds cannot access write operations

### 5. **Add Test-Only Write Implementation for PostgreSQL**

- File: `backend/lib/src/repository/postgres.rs`
- Operation: Add conditional implementation after IndexerOps impl (around line 200)
- Details:
  ```rust
  // Add test-only implementation of IndexerOpsMut
  #[cfg(test)]
  #[async_trait]
  impl IndexerOpsMut for Repository {
      async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp> { 
          // Move existing create_bsp implementation here
      }
      
      async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp> {
          // Move existing update_bsp_capacity implementation here
      }
      
      async fn delete_bsp(&self, account: &str) -> RepositoryResult<()> {
          let mut conn = self.pool.get().await?;
          diesel::delete(bsp::table.filter(bsp::account.eq(account)))
              .execute(&mut *conn)
              .await?;
          Ok(())
      }
      
      async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket> {
          // Move existing create_bucket implementation here
      }
      
      async fn create_file(&self, new_file: NewFile) -> RepositoryResult<File> {
          let mut conn = self.pool.get().await?;
          let now = Utc::now().naive_utc();
          
          let result = diesel::insert_into(file::table)
              .values((
                  file::account.eq(&new_file.account),
                  file::file_key.eq(&new_file.file_key),
                  file::bucket_id.eq(&new_file.bucket_id),
                  file::location.eq(&new_file.location),
                  file::fingerprint.eq(&new_file.fingerprint),
                  file::size.eq(&new_file.size),
                  file::step.eq(&new_file.step),
                  file::created_at.eq(&now),
                  file::updated_at.eq(&now),
              ))
              .get_result(&mut *conn)
              .await?;
          Ok(result)
      }
      
      async fn update_file_step(&self, file_key: &[u8], step: i32) -> RepositoryResult<()> {
          let mut conn = self.pool.get().await?;
          diesel::update(file::table.filter(file::file_key.eq(file_key)))
              .set((
                  file::step.eq(step),
                  file::updated_at.eq(Utc::now().naive_utc()),
              ))
              .execute(&mut *conn)
              .await?;
          Ok(())
      }
      
      async fn delete_file(&self, file_key: &[u8]) -> RepositoryResult<()> {
          let mut conn = self.pool.get().await?;
          diesel::delete(file::table.filter(file::file_key.eq(file_key)))
              .execute(&mut *conn)
              .await?;
          Ok(())
      }
      
      async fn clear_all(&self) {
          // For PostgreSQL in tests, rely on transaction rollback instead
          // This method becomes a no-op as test transactions handle cleanup
      }
  }
  ```
- Success: PostgreSQL repository has write operations only in test builds

### 6. **Update DBClient to Use StorageOperations**

- File: `backend/lib/src/data/postgres/db_client.rs`
- Operation: Keep using StorageOperations (no changes needed)
- Details:
  ```rust
  // Line 9: Keep using StorageOperations
  use crate::repository::StorageOperations;
  
  // Line 33: Keep using StorageOperations
  pub struct DBClient {
      repository: Arc<dyn StorageOperations>,
  }
  
  // Line 41: Keep using StorageOperations
  pub fn new(repository: Arc<dyn StorageOperations>) -> Self {
      Self { repository }
  }
  ```
- Success: DBClient automatically gets IndexerOpsMut in tests, IndexerOps only in production/mocks

### 7. **Add Test-Only Methods to DBClient**

- File: `backend/lib/src/data/postgres/db_client.rs`
- Operation: Add test-only impl block after main impl (around line 150)
- Details:
  ```rust
  #[cfg(test)]
  impl DBClient {
      /// Create a new BSP (test only)
      pub async fn create_bsp(&self, new_bsp: crate::repository::NewBsp) -> crate::error::Result<shc_indexer_db::models::Bsp> {
          // In tests, StorageOperations includes IndexerOpsMut
          let bsp = self.repository.create_bsp(new_bsp)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))?;
          Ok(bsp)
      }
      
      /// Create a new file (test only)
      pub async fn create_file(&self, new_file: crate::repository::NewFile) -> crate::error::Result<shc_indexer_db::models::File> {
          let file = self.repository.create_file(new_file)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))?;
          Ok(file)
      }
      
      /// Update BSP capacity (test only)
      pub async fn update_bsp_capacity(&self, id: i64, capacity: bigdecimal::BigDecimal) -> crate::error::Result<shc_indexer_db::models::Bsp> {
          let bsp = self.repository.update_bsp_capacity(id, capacity)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))?;
          Ok(bsp)
      }
      
      /// Delete a BSP (test only)
      pub async fn delete_bsp(&self, account: &str) -> crate::error::Result<()> {
          self.repository.delete_bsp(account)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))
      }
      
      /// Create a bucket (test only)
      pub async fn create_bucket(&self, new_bucket: crate::repository::NewBucket) -> crate::error::Result<shc_indexer_db::models::Bucket> {
          let bucket = self.repository.create_bucket(new_bucket)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))?;
          Ok(bucket)
      }
      
      /// Update file step (test only)
      pub async fn update_file_step(&self, file_key: &[u8], step: i32) -> crate::error::Result<()> {
          self.repository.update_file_step(file_key, step)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))
      }
      
      /// Delete a file (test only)
      pub async fn delete_file(&self, file_key: &[u8]) -> crate::error::Result<()> {
          self.repository.delete_file(file_key)
              .await
              .map_err(|e| crate::error::Error::Database(e.to_string()))
      }
      
      /// Clear all data (test only)
      pub async fn clear_all(&self) -> crate::error::Result<()> {
          self.repository.clear_all().await;
          Ok(())
      }
  }
  ```
- Success: Test code can use mutable operations through DBClient

### 8. **Handle Mocks-Only Feature Case**

- File: `backend/lib/src/repository/mock.rs`
- Operation: Remove standalone helper methods or add documentation (line 185)
- Details:
  ```rust
  // Remove the standalone impl block with helper methods (lines 187-270)
  // These are now part of the IndexerOpsMut trait implementation
  // When using feature = "mocks" without cfg(test), users must access
  // the MockRepository directly and cast it to use IndexerOpsMut methods
  ```
- Success: Clean separation between trait methods and direct access

### 9. **Update Test Imports**

- File: `backend/lib/src/repository/mock.rs` (test module, line 272+)
- Operation: Update imports in tests
- Details:
  ```rust
  // Line 278: Update use statement
  use crate::repository::{IndexerOps, IndexerOpsMut};
  ```
- Success: Tests compile and run

### 10. **Update Integration Test Imports**

- File: `backend/lib/src/data/postgres/queries.rs` (test module, line 110+)
- Operation: Update imports where MockRepository is used
- Details:
  ```rust
  // Line 114: Update use statement
  use crate::repository::{MockRepository, NewBsp, StorageOperations};
  // StorageOperations in tests includes IndexerOpsMut automatically
  ```
- Success: Integration tests compile and run

## Testing Strategy

- [ ] Run `cargo build --release` - verify production build has no write operations
- [ ] Run `cargo build --features mocks` - verify MockRepository has write methods but DBClient doesn't
- [ ] Run `cargo test` in backend/lib - all existing tests should pass with write operations available
- [ ] Verify MockRepository tests work with both read and write operations
- [ ] Verify PostgreSQL repository tests can use write operations with test transactions
- [ ] Check that with `--features mocks` (no test), write operations must be accessed directly on MockRepository
- [ ] Check that in tests, DBClient has write operation methods available

## Rollback Plan

1. Revert all changes to repository/mod.rs to restore original `StorageOperations` trait
2. Revert MockRepository to implement single `StorageOperations` trait
3. Revert PostgreSQL Repository to implement single `StorageOperations` trait
4. Revert DBClient changes (no changes needed if we keep StorageOperations)
5. No database migrations or data changes to rollback

## Key Design Decisions

1. **IndexerOpsMut always exists**: This simplifies the code and avoids conditional compilation complexity at the trait definition level
2. **MockRepository always implements IndexerOpsMut**: Provides consistency and allows mock usage in non-test scenarios
3. **StorageOperations trait alias changes based on cfg(test)**: Maintains backward compatibility while providing test-only access to mutations
4. **DBClient uses StorageOperations**: Automatically gains write operations in tests without code duplication
5. **No downcasting**: Type safety is maintained throughout with compile-time guarantees
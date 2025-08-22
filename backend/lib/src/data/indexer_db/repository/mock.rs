//! Mock repository implementation for unit testing.
//!
//! Provides an in-memory implementation of the repository pattern that mimics
//! database operations without requiring a real database connection.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::Utc;
use tokio::sync::RwLock;

use super::{Bsp, Bucket, File, IndexerOps, IndexerOpsMut, NewBsp, NewBucket, NewFile};
use crate::repository::error::{RepositoryError, RepositoryResult};

/// Mock repository implementation using in-memory storage
pub struct MockRepository {
    bsps: Arc<RwLock<HashMap<i64, Bsp>>>,
    buckets: Arc<RwLock<HashMap<i64, Bucket>>>,
    files: Arc<RwLock<HashMap<Vec<u8>, File>>>,
    next_id: Arc<AtomicI64>,
}

impl MockRepository {
    /// Create a new mock repository
    pub fn new() -> Self {
        Self {
            bsps: Arc::new(RwLock::new(HashMap::new())),
            buckets: Arc::new(RwLock::new(HashMap::new())),
            files: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicI64::new(1)),
        }
    }

    /// Generate next unique ID
    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for MockRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IndexerOps for MockRepository {
    // ============ BSP Read Operations ============

    async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>> {
        let bsps = self.bsps.read().await;
        Ok(bsps.get(&id).cloned())
    }

    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> {
        let bsps = self.bsps.read().await;
        let mut all_bsps: Vec<Bsp> = bsps.values().cloned().collect();
        all_bsps.sort_by_key(|b| b.id);

        Ok(all_bsps
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }

    // ============ Bucket Read Operations ============

    async fn get_bucket_by_id(&self, id: i64) -> RepositoryResult<Option<Bucket>> {
        let buckets = self.buckets.read().await;
        Ok(buckets.get(&id).cloned())
    }

    async fn get_buckets_by_user(&self, user_account: &str) -> RepositoryResult<Vec<Bucket>> {
        let buckets = self.buckets.read().await;
        Ok(buckets
            .values()
            .filter(|b| b.account == user_account)
            .cloned()
            .collect())
    }

    // ============ File Read Operations ============

    async fn get_file_by_key(&self, key: &[u8]) -> RepositoryResult<Option<File>> {
        let files = self.files.read().await;
        Ok(files.get(key).cloned())
    }

    async fn get_files_by_user(&self, user_account: &[u8]) -> RepositoryResult<Vec<File>> {
        let files = self.files.read().await;
        Ok(files
            .values()
            .filter(|f| f.account == user_account)
            .cloned()
            .collect())
    }

    async fn get_files_by_bucket(&self, bucket_id: i64) -> RepositoryResult<Vec<File>> {
        let files = self.files.read().await;
        Ok(files
            .values()
            .filter(|f| f.bucket_id == bucket_id)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl IndexerOpsMut for MockRepository {
    // ============ BSP Write Operations ============

    async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp> {
        let mut bsps = self.bsps.write().await;

        // Check for duplicate accounts
        for bsp in bsps.values() {
            if bsp.account == new_bsp.account {
                return Err(RepositoryError::Database(
                    diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        Box::new("BSP with this account already exists".to_string()),
                    ),
                ));
            }
        }

        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let bsp = Bsp {
            id,
            account: new_bsp.account,
            capacity: new_bsp.capacity,
            stake: new_bsp.stake,
            onchain_bsp_id: new_bsp.onchain_bsp_id,
            merkle_root: new_bsp.merkle_root,
            last_tick_proven: 0,
            created_at: now,
            updated_at: now,
        };

        bsps.insert(id, bsp.clone());
        Ok(bsp)
    }

    async fn update_bsp_capacity(&self, id: i64, capacity: BigDecimal) -> RepositoryResult<Bsp> {
        let mut bsps = self.bsps.write().await;
        match bsps.get_mut(&id) {
            Some(bsp) => {
                bsp.capacity = capacity;
                bsp.updated_at = Utc::now().naive_utc();
                Ok(bsp.clone())
            }
            None => Err(RepositoryError::not_found("BSP")),
        }
    }

    async fn delete_bsp(&self, account: &str) -> RepositoryResult<()> {
        let mut bsps = self.bsps.write().await;
        let id_to_remove = bsps.values().find(|b| b.account == account).map(|b| b.id);

        if let Some(id) = id_to_remove {
            bsps.remove(&id);
            Ok(())
        } else {
            Err(RepositoryError::not_found("BSP"))
        }
    }

    // ============ Bucket Write Operations ============

    async fn create_bucket(&self, new_bucket: NewBucket) -> RepositoryResult<Bucket> {
        let mut buckets = self.buckets.write().await;
        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let bucket = Bucket {
            id,
            msp_id: new_bucket.msp_id,
            account: new_bucket.account,
            onchain_bucket_id: new_bucket.onchain_bucket_id,
            name: new_bucket.name,
            collection_id: new_bucket.collection_id,
            private: new_bucket.private,
            merkle_root: new_bucket.merkle_root,
            created_at: now,
            updated_at: now,
        };

        buckets.insert(id, bucket.clone());
        Ok(bucket)
    }

    // ============ File Write Operations ============

    async fn create_file(&self, new_file: NewFile) -> RepositoryResult<File> {
        let mut files = self.files.write().await;

        // Check for duplicate key
        if files.contains_key(&new_file.file_key) {
            return Err(RepositoryError::Database(
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    Box::new("File with this key already exists".to_string()),
                ),
            ));
        }

        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let file = File {
            id,
            account: new_file.account,
            file_key: new_file.file_key.clone(),
            bucket_id: new_file.bucket_id,
            location: new_file.location,
            fingerprint: new_file.fingerprint,
            size: new_file.size,
            step: new_file.step,
            deletion_status: None,
            created_at: now,
            updated_at: now,
        };

        files.insert(file.file_key.clone(), file.clone());
        Ok(file)
    }

    async fn update_file_step(&self, file_key: &[u8], step: i32) -> RepositoryResult<()> {
        let mut files = self.files.write().await;
        match files.get_mut(file_key) {
            Some(file) => {
                file.step = step;
                file.updated_at = Utc::now().naive_utc();
                Ok(())
            }
            None => Err(RepositoryError::not_found("File")),
        }
    }

    async fn delete_file(&self, file_key: &[u8]) -> RepositoryResult<()> {
        let mut files = self.files.write().await;
        match files.remove(file_key) {
            Some(_) => Ok(()),
            None => Err(RepositoryError::not_found("File")),
        }
    }

    async fn clear_all(&self) {
        let mut bsps = self.bsps.write().await;
        let mut buckets = self.buckets.write().await;
        let mut files = self.files.write().await;

        bsps.clear();
        buckets.clear();
        files.clear();

        // Reset ID counter
        self.next_id.store(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    //! Scaffolding tests for MockRepository - basic coverage only
    //!
    //! Missing coverage:
    //! - Bucket update/delete operations and cascade behavior
    //! - Edge cases: empty collections, max values, unicode/binary data limits
    //! - Relationship integrity: orphaned files, MSP relationships, ownership validation
    //! - Concurrent updates to same entity and read-modify-write races
    //! - Data validation: negative values, size limits, timestamp handling
    //! - Complex queries: multi-field filtering, sorting, counts, aggregations
    //! - Pool exhaustion and initialization failures
    //! - Business logic: capacity consistency, merkle validation, state transitions

    use std::sync::Arc;

    use bigdecimal::FromPrimitive;

    use super::*;
    use crate::constants::test::{
        accounts::*, bsp::*, buckets::*, error_cases::*, file_metadata::*, merkle::*, msp::*,
        pagination::*,
    };

    #[tokio::test]
    async fn test_mock_repository_bsp_operations() {
        let repo = MockRepository::new();

        // Create BSP
        let new_bsp = NewBsp {
            account: TEST_BSP_ACCOUNT_STR.to_string(),
            capacity: BigDecimal::from_i64(DEFAULT_CAPACITY).unwrap(),
            stake: BigDecimal::from_i64(DEFAULT_STAKE).unwrap(),
            onchain_bsp_id: TEST_BSP_ONCHAIN_ID_STR.to_string(),
            merkle_root: BSP_MERKLE_ROOT.to_vec(),
        };

        let created_bsp = repo.create_bsp(new_bsp.clone()).await.unwrap();
        assert_eq!(created_bsp.account, TEST_BSP_ACCOUNT_STR);
        assert_eq!(created_bsp.id, DEFAULT_BSP_ID);

        // Get by ID
        let found = repo.get_bsp_by_id(DEFAULT_BSP_ID).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().account, TEST_BSP_ACCOUNT_STR);

        // Update capacity
        let updated = repo
            .update_bsp_capacity(
                DEFAULT_BSP_ID,
                BigDecimal::from_i64(UPDATED_CAPACITY).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            updated.capacity,
            BigDecimal::from_i64(UPDATED_CAPACITY).unwrap()
        );

        // List BSPs
        let list = repo
            .list_bsps(DEFAULT_PAGE_SIZE as i64, DEFAULT_OFFSET as i64)
            .await
            .unwrap();
        assert_eq!(list.len(), 1);

        // Delete BSP (using helper method)
        repo.delete_bsp(TEST_BSP_ACCOUNT_STR).await.unwrap();
        let found = repo.get_bsp_by_id(DEFAULT_BSP_ID).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_bucket_operations() {
        let repo = MockRepository::new();

        // Create bucket
        let new_bucket = NewBucket {
            msp_id: Some(DEFAULT_MSP_ID),
            account: TEST_USER_ACCOUNT_STR.to_string(),
            onchain_bucket_id: TEST_ONCHAIN_BUCKET_ID.to_vec(),
            name: TEST_BUCKET_NAME_STR.to_vec(),
            collection_id: None,
            private: false,
            merkle_root: BUCKET_MERKLE_ROOT.to_vec(),
        };

        let created = repo.create_bucket(new_bucket).await.unwrap();
        assert_eq!(created.account, TEST_USER_ACCOUNT_STR);

        // Get by ID
        let found = repo.get_bucket_by_id(created.id).await.unwrap();
        assert!(found.is_some());

        // Get by user
        let buckets = repo
            .get_buckets_by_user(TEST_USER_ACCOUNT_STR)
            .await
            .unwrap();
        assert_eq!(buckets.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_repository_file_operations() {
        let repo = MockRepository::new();

        // Create file (using helper method)
        let new_file = NewFile {
            account: ALTERNATIVE_USER_ACCOUNT_STR.as_bytes().to_vec(),
            file_key: TEST_FILE_KEY_STR.to_vec(),
            bucket_id: TEST_BUCKET_ID_INT,
            location: TEST_LOCATION_STR.to_vec(),
            fingerprint: TEST_FINGERPRINT.to_vec(),
            size: TEST_FILE_SIZE as i64,
            step: INITIAL_STEP as i32,
        };

        let created = repo.create_file(new_file.clone()).await.unwrap();
        assert_eq!(created.file_key, TEST_FILE_KEY_STR);

        // Get by key
        let found = repo.get_file_by_key(TEST_FILE_KEY_STR).await.unwrap();
        assert!(found.is_some());

        // Get by user
        let files = repo
            .get_files_by_user(ALTERNATIVE_USER_ACCOUNT_STR.as_bytes())
            .await
            .unwrap();
        assert_eq!(files.len(), 1);

        // Get by bucket
        let files = repo.get_files_by_bucket(TEST_BUCKET_ID_INT).await.unwrap();
        assert_eq!(files.len(), 1);

        // Update step (using helper method)
        repo.update_file_step(TEST_FILE_KEY_STR, UPDATED_STEP as i32)
            .await
            .unwrap();
        let updated = repo
            .get_file_by_key(TEST_FILE_KEY_STR)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.step, UPDATED_STEP as i32);
    }

    #[tokio::test]
    async fn test_mock_repository_concurrent_access() {
        let repo = Arc::new(MockRepository::new());
        let mut handles = vec![];

        // Spawn multiple tasks creating BSPs concurrently
        for i in 0..CONCURRENT_TEST_COUNT {
            let repo_clone = repo.clone();
            let handle = tokio::spawn(async move {
                let new_bsp = NewBsp {
                    account: format!("{}_{}", TEST_BSP_ACCOUNT_STR, i),
                    capacity: BigDecimal::from_i64(DEFAULT_CAPACITY).unwrap(),
                    stake: BigDecimal::from_i64(DEFAULT_STAKE).unwrap(),
                    onchain_bsp_id: format!("{}{}", TEST_BSP_ONCHAIN_ID_PREFIX, i),
                    merkle_root: vec![i as u8],
                };
                repo_clone.create_bsp(new_bsp).await
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all BSPs were created
        let bsps = repo
            .list_bsps(LARGE_PAGE_SIZE as i64, DEFAULT_OFFSET as i64)
            .await
            .unwrap();
        assert_eq!(bsps.len(), CONCURRENT_TEST_COUNT);
    }

    // Test error conditions
    #[tokio::test]
    async fn test_mock_repository_error_conditions() {
        let repo = MockRepository::new();

        // Test getting non-existent entities
        assert!(repo.get_bsp_by_id(NON_EXISTENT_ID).await.unwrap().is_none());
        assert!(repo
            .get_bucket_by_id(NON_EXISTENT_ID)
            .await
            .unwrap()
            .is_none());
        assert!(repo
            .get_file_by_key(NON_EXISTENT_FILE_KEY)
            .await
            .unwrap()
            .is_none());

        // Test updating non-existent BSP
        let result = repo
            .update_bsp_capacity(
                NON_EXISTENT_ID,
                BigDecimal::from_i64(UPDATED_CAPACITY).unwrap(),
            )
            .await;
        assert!(result.is_err());

        // Test updating non-existent file
        let result = repo
            .update_file_step(NON_EXISTENT_FILE_KEY, UPDATED_STEP as i32)
            .await;
        assert!(result.is_err());

        // Test deleting non-existent BSP
        let result = repo.delete_bsp(NON_EXISTENT_ACCOUNT).await;
        assert!(result.is_err());

        // Test deleting non-existent file
        let result = repo.delete_file(NON_EXISTENT_FILE_KEY).await;
        assert!(result.is_err());

        // Test duplicate BSP account
        let bsp = NewBsp {
            account: TEST_BSP_ACCOUNT_STR.to_string(),
            capacity: BigDecimal::from_i64(DEFAULT_CAPACITY).unwrap(),
            stake: BigDecimal::from_i64(DEFAULT_STAKE).unwrap(),
            onchain_bsp_id: TEST_BSP_ONCHAIN_ID_STR.to_string(),
            merkle_root: BSP_MERKLE_ROOT.to_vec(),
        };
        repo.create_bsp(bsp.clone()).await.unwrap();
        let result = repo.create_bsp(bsp).await;
        assert!(result.is_err());

        // Test duplicate file key
        let bucket = repo
            .create_bucket(NewBucket {
                msp_id: Some(DEFAULT_MSP_ID),
                account: TEST_USER_ACCOUNT_STR.to_string(),
                onchain_bucket_id: TEST_ONCHAIN_BUCKET_ID.to_vec(),
                name: TEST_BUCKET_NAME_STR.to_vec(),
                collection_id: None,
                private: false,
                merkle_root: BUCKET_MERKLE_ROOT.to_vec(),
            })
            .await
            .unwrap();

        let file = NewFile {
            account: TEST_USER_ACCOUNT.to_vec(),
            file_key: TEST_FILE_KEY_STR.to_vec(),
            bucket_id: bucket.id,
            location: TEST_LOCATION_STR.to_vec(),
            fingerprint: TEST_FINGERPRINT.to_vec(),
            size: TEST_FILE_SIZE as i64,
            step: INITIAL_STEP as i32,
        };
        repo.create_file(file.clone()).await.unwrap();
        let result = repo.create_file(file).await;
        assert!(result.is_err());
    }

    // Test pagination
    #[tokio::test]
    async fn test_mock_repository_pagination() {
        let repo = MockRepository::new();

        // Create multiple BSPs
        for i in 0..TOTAL_BSPS_FOR_PAGINATION {
            let bsp = NewBsp {
                account: format!("{}_{}", TEST_BSP_ACCOUNT_STR, i),
                capacity: BigDecimal::from_i64(DEFAULT_CAPACITY).unwrap(),
                stake: BigDecimal::from_i64(DEFAULT_STAKE).unwrap(),
                onchain_bsp_id: format!("{}{}", TEST_BSP_ONCHAIN_ID_PREFIX, i),
                merkle_root: vec![i as u8],
            };
            repo.create_bsp(bsp).await.unwrap();
        }

        // Test first page
        let page1 = repo.list_bsps(DEFAULT_PAGE_SIZE as i64, 0).await.unwrap();
        assert_eq!(page1.len(), DEFAULT_PAGE_SIZE);

        // Test second page
        let page2 = repo
            .list_bsps(DEFAULT_PAGE_SIZE as i64, DEFAULT_PAGE_SIZE as i64)
            .await
            .unwrap();
        assert_eq!(page2.len(), DEFAULT_PAGE_SIZE);

        // Test third page (partial)
        let page3 = repo
            .list_bsps(DEFAULT_PAGE_SIZE as i64, (DEFAULT_PAGE_SIZE * 2) as i64)
            .await
            .unwrap();
        assert_eq!(
            page3.len(),
            TOTAL_BSPS_FOR_PAGINATION - (DEFAULT_PAGE_SIZE * 2)
        );

        // Test beyond available data
        let page4 = repo
            .list_bsps(DEFAULT_PAGE_SIZE as i64, TOTAL_BSPS_FOR_PAGINATION as i64)
            .await
            .unwrap();
        assert_eq!(page4.len(), 0);

        // Verify no duplicates and correct ordering
        let all = repo.list_bsps(LARGE_PAGE_SIZE as i64, 0).await.unwrap();
        assert_eq!(all.len(), TOTAL_BSPS_FOR_PAGINATION);
        for i in 1..all.len() {
            assert!(all[i].id > all[i - 1].id);
        }
    }

    #[tokio::test]
    async fn test_mock_repository_clear_all() {
        let repo = MockRepository::new();

        // Add some data
        let new_bsp = NewBsp {
            account: TEST_BSP_ACCOUNT_STR.to_string(),
            capacity: BigDecimal::from_i64(DEFAULT_CAPACITY).unwrap(),
            stake: BigDecimal::from_i64(DEFAULT_STAKE).unwrap(),
            onchain_bsp_id: TEST_BSP_ONCHAIN_ID_STR.to_string(),
            merkle_root: BSP_MERKLE_ROOT.to_vec(),
        };
        repo.create_bsp(new_bsp).await.unwrap();

        let new_bucket = NewBucket {
            msp_id: Some(DEFAULT_MSP_ID),
            account: TEST_USER_ACCOUNT_STR.to_string(),
            onchain_bucket_id: TEST_ONCHAIN_BUCKET_ID.to_vec(),
            name: TEST_BUCKET_NAME_STR.to_vec(),
            collection_id: None,
            private: false,
            merkle_root: BUCKET_MERKLE_ROOT.to_vec(),
        };
        repo.create_bucket(new_bucket).await.unwrap();

        // Verify data exists
        assert_eq!(
            repo.list_bsps(DEFAULT_PAGE_SIZE as i64, DEFAULT_OFFSET as i64)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repo.get_buckets_by_user(TEST_USER_ACCOUNT_STR)
                .await
                .unwrap()
                .len(),
            1
        );

        // Clear all
        repo.clear_all().await;

        // Verify data is gone
        assert_eq!(
            repo.list_bsps(DEFAULT_PAGE_SIZE as i64, DEFAULT_OFFSET as i64)
                .await
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            repo.get_buckets_by_user(TEST_USER_ACCOUNT_STR)
                .await
                .unwrap()
                .len(),
            0
        );
    }
}

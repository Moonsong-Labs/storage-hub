//! Mock repository implementation for unit testing.
//!
//! Provides an in-memory implementation of the repository pattern that mimics
//! database operations without requiring a real database connection.

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{
    Bsp, Bucket, File, NewBsp, NewBucket, NewFile, StorageOperations,
};
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
impl StorageOperations for MockRepository {
    // ============ BSP Operations ============
    
    async fn create_bsp(&self, new_bsp: NewBsp) -> RepositoryResult<Bsp> {
        let mut bsps = self.bsps.write().await;
        
        // Check for duplicate accounts
        for bsp in bsps.values() {
            if bsp.account == new_bsp.account {
                return Err(RepositoryError::Database(
                    diesel::result::Error::DatabaseError(
                        diesel::result::DatabaseErrorKind::UniqueViolation,
                        Box::new("BSP with this account already exists".to_string()),
                    )
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
    
    async fn get_bsp_by_id(&self, id: i64) -> RepositoryResult<Option<Bsp>> {
        let bsps = self.bsps.read().await;
        Ok(bsps.get(&id).cloned())
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
    
    // ============ Bucket Operations ============
    
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
    
    // ============ File Operations ============
    
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

// ============ Extended Mock Repository Methods ============
// These methods are specific to the mock and useful for testing

impl MockRepository {
    /// Create a file (mock-specific helper method for testing)
    pub async fn create_file(&self, new_file: NewFile) -> RepositoryResult<File> {
        let mut files = self.files.write().await;
        
        // Check for duplicate key
        if files.contains_key(&new_file.file_key) {
            return Err(RepositoryError::Database(
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    Box::new("File with this key already exists".to_string()),
                )
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
            created_at: now,
            updated_at: now,
        };
        
        files.insert(file.file_key.clone(), file.clone());
        Ok(file)
    }
    
    /// Delete a BSP by account (mock-specific helper)
    pub async fn delete_bsp(&self, account: &str) -> RepositoryResult<()> {
        let mut bsps = self.bsps.write().await;
        let id_to_remove = bsps.values()
            .find(|b| b.account == account)
            .map(|b| b.id);
        
        if let Some(id) = id_to_remove {
            bsps.remove(&id);
            Ok(())
        } else {
            Err(RepositoryError::not_found("BSP"))
        }
    }
    
    /// Update file step (mock-specific helper)
    pub async fn update_file_step(&self, file_key: &[u8], step: i32) -> RepositoryResult<()> {
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
    
    /// Delete a file by its key (mock-specific helper)
    pub async fn delete_file(&self, file_key: &[u8]) -> RepositoryResult<()> {
        let mut files = self.files.write().await;
        match files.remove(file_key) {
            Some(_) => Ok(()),
            None => Err(RepositoryError::not_found("File")),
        }
    }
    
    /// Clear all data (useful for test cleanup)
    pub async fn clear_all(&self) {
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
    use super::*;
    use bigdecimal::FromPrimitive;
    
    #[tokio::test]
    async fn test_mock_repository_bsp_operations() {
        let repo = MockRepository::new();
        
        // Create BSP
        let new_bsp = NewBsp {
            account: "test_account".to_string(),
            capacity: BigDecimal::from_i64(1000).unwrap(),
            stake: BigDecimal::from_i64(100).unwrap(),
            onchain_bsp_id: "onchain_123".to_string(),
            merkle_root: vec![1, 2, 3],
            multiaddresses: vec![vec![4, 5, 6]],
        };
        
        let created_bsp = repo.create_bsp(new_bsp.clone()).await.unwrap();
        assert_eq!(created_bsp.account, "test_account");
        assert_eq!(created_bsp.id, 1);
        
        // Get by ID
        let found = repo.get_bsp_by_id(1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().account, "test_account");
        
        // Update capacity
        let updated = repo.update_bsp_capacity(1, BigDecimal::from_i64(2000).unwrap()).await.unwrap();
        assert_eq!(updated.capacity, BigDecimal::from_i64(2000).unwrap());
        
        // List BSPs
        let list = repo.list_bsps(10, 0).await.unwrap();
        assert_eq!(list.len(), 1);
        
        // Delete BSP (using helper method)
        repo.delete_bsp("test_account").await.unwrap();
        let found = repo.get_bsp_by_id(1).await.unwrap();
        assert!(found.is_none());
    }
    
    #[tokio::test]
    async fn test_mock_repository_bucket_operations() {
        let repo = MockRepository::new();
        
        // Create bucket
        let new_bucket = NewBucket {
            msp_id: Some(1),
            account: "user123".to_string(),
            onchain_bucket_id: vec![4, 5, 6],
            name: b"test_bucket".to_vec(),
            collection_id: None,
            private: false,
            merkle_root: vec![7, 8, 9],
        };
        
        let created = repo.create_bucket(new_bucket).await.unwrap();
        assert_eq!(created.account, "user123");
        
        // Get by ID
        let found = repo.get_bucket_by_id(created.id).await.unwrap();
        assert!(found.is_some());
        
        // Get by user
        let buckets = repo.get_buckets_by_user("user123").await.unwrap();
        assert_eq!(buckets.len(), 1);
    }
    
    #[tokio::test]
    async fn test_mock_repository_file_operations() {
        let repo = MockRepository::new();
        
        // Create file (using helper method)
        let new_file = NewFile {
            account: b"user456".to_vec(),
            file_key: b"file_key_123".to_vec(),
            bucket_id: 1,
            location: b"ipfs://hash".to_vec(),
            fingerprint: vec![10, 11, 12],
            size: 1024,
            step: 0,
        };
        
        let created = repo.create_file(new_file.clone()).await.unwrap();
        assert_eq!(created.file_key, b"file_key_123");
        
        // Get by key
        let found = repo.get_file_by_key(b"file_key_123").await.unwrap();
        assert!(found.is_some());
        
        // Get by user
        let files = repo.get_files_by_user(b"user456").await.unwrap();
        assert_eq!(files.len(), 1);
        
        // Get by bucket
        let files = repo.get_files_by_bucket(1).await.unwrap();
        assert_eq!(files.len(), 1);
        
        // Update step (using helper method)
        repo.update_file_step(b"file_key_123", 1).await.unwrap();
        let updated = repo.get_file_by_key(b"file_key_123").await.unwrap().unwrap();
        assert_eq!(updated.step, 1);
    }
    
    #[tokio::test]
    async fn test_mock_repository_concurrent_access() {
        let repo = Arc::new(MockRepository::new());
        let mut handles = vec![];
        
        // Spawn multiple tasks creating BSPs concurrently
        for i in 0..10 {
            let repo_clone = repo.clone();
            let handle = tokio::spawn(async move {
                let new_bsp = NewBsp {
                    account: format!("account_{}", i),
                    capacity: BigDecimal::from_i64(1000).unwrap(),
                    stake: BigDecimal::from_i64(100).unwrap(),
                    onchain_bsp_id: format!("onchain_{}", i),
                    merkle_root: vec![i as u8],
                    multiaddresses: vec![],
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
        let bsps = repo.list_bsps(100, 0).await.unwrap();
        assert_eq!(bsps.len(), 10);
    }
    
    #[tokio::test]
    async fn test_mock_repository_clear_all() {
        let repo = MockRepository::new();
        
        // Add some data
        let new_bsp = NewBsp {
            account: "test".to_string(),
            capacity: BigDecimal::from_i64(100).unwrap(),
            stake: BigDecimal::from_i64(10).unwrap(),
            onchain_bsp_id: "test".to_string(),
            merkle_root: vec![],
            multiaddresses: vec![],
        };
        repo.create_bsp(new_bsp).await.unwrap();
        
        let new_bucket = NewBucket {
            msp_id: None,
            account: "test".to_string(),
            onchain_bucket_id: vec![1],
            name: vec![],
            collection_id: None,
            private: false,
            merkle_root: vec![],
        };
        repo.create_bucket(new_bucket).await.unwrap();
        
        // Verify data exists
        assert_eq!(repo.list_bsps(10, 0).await.unwrap().len(), 1);
        assert_eq!(repo.get_buckets_by_user("test").await.unwrap().len(), 1);
        
        // Clear all
        repo.clear_all().await;
        
        // Verify data is gone
        assert_eq!(repo.list_bsps(10, 0).await.unwrap().len(), 0);
        assert_eq!(repo.get_buckets_by_user("test").await.unwrap().len(), 0);
    }
}
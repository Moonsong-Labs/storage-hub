//! Database client wrapper using repository pattern abstraction
//!
//! This module provides a database client that delegates all operations
//! to an underlying repository implementation, allowing for both production
//! PostgreSQL and mock implementations for testing.

use std::sync::Arc;

#[cfg(all(test, feature = "mocks"))]
use crate::repository::MockRepository;
use crate::repository::StorageOperations;

/// Database client that delegates to a repository implementation
///
/// This client provides a clean abstraction over database operations,
/// delegating all actual work to an underlying repository that implements
/// the `StorageOperations` trait. This allows for easy swapping between
/// production PostgreSQL and mock implementations for testing.
///
/// ## Usage Example
/// ```ignore
/// use repository::{Repository, StorageOperations};
/// use data::postgres::DBClient;
///
/// // Production usage with PostgreSQL
/// let repo = Repository::new(&database_url).await?;
/// let client = DBClient::new(Arc::new(repo));
///
/// // Test usage with mock (when available)
/// let mock_repo = MockRepository::new();
/// let client = DBClient::new(Arc::new(mock_repo));
/// ```
#[derive(Clone)]
pub struct DBClient {
    repository: Arc<dyn StorageOperations>,
}

impl DBClient {
    /// Create a new database client with the given repository
    ///
    /// # Arguments
    /// * `repository` - Repository implementation to use for database operations
    pub fn new(repository: Arc<dyn StorageOperations>) -> Self {
        Self { repository }
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> crate::error::Result<()> {
        // Try to list BSPs with a limit of 1 to test the connection
        self.repository
            .list_bsps(1, 0)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Get a file by its key
    pub async fn get_file_by_key(
        &self,
        file_key: &[u8],
    ) -> crate::error::Result<shc_indexer_db::models::File> {
        self.repository
            .get_file_by_key(file_key)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?
            .ok_or_else(|| crate::error::Error::NotFound("File not found".to_string()))
    }

    /// Get all files for a user
    pub async fn get_files_by_user(
        &self,
        user_account: &[u8],
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        self.repository
            .get_files_by_user(user_account)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Get files for a user stored by a specific MSP
    pub async fn get_files_by_user_and_msp(
        &self,
        user_account: &[u8],
        _msp_id: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        // For now, just return files by user since MSP filtering isn't in the trait yet
        // TODO: Add MSP filtering to repository trait if needed
        self.get_files_by_user(user_account, limit, offset).await
    }

    /// Get a BSP by its ID
    pub async fn get_bsp_by_id(
        &self,
        bsp_id: i64,
    ) -> crate::error::Result<Option<shc_indexer_db::models::Bsp>> {
        self.repository
            .get_bsp_by_id(bsp_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Get all BSPs with optional pagination
    pub async fn get_all_bsps(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::Bsp>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        self.repository
            .list_bsps(limit, offset)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Get an MSP by its ID
    pub async fn get_msp_by_id(
        &self,
        _msp_id: i64,
    ) -> crate::error::Result<Option<shc_indexer_db::models::Msp>> {
        // MSP operations not yet in repository trait
        // TODO: Add MSP operations to repository trait if needed
        Err(crate::error::Error::Database(
            "MSP operations not yet implemented in repository".to_string(),
        ))
    }

    /// Get all MSPs with optional pagination
    pub async fn get_all_msps(
        &self,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::Msp>> {
        // MSP operations not yet in repository trait
        // TODO: Add MSP operations to repository trait if needed
        Err(crate::error::Error::Database(
            "MSP operations not yet implemented in repository".to_string(),
        ))
    }

    /// Get all files in a bucket
    pub async fn get_files_by_bucket_id(
        &self,
        bucket_id: i64,
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        self.repository
            .get_files_by_bucket(bucket_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Get a bucket by its ID
    pub async fn get_bucket_by_id(
        &self,
        bucket_id: i64,
    ) -> crate::error::Result<shc_indexer_db::models::Bucket> {
        self.repository
            .get_bucket_by_id(bucket_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?
            .ok_or_else(|| crate::error::Error::NotFound("Bucket not found".to_string()))
    }

    /// Get all buckets for a user
    pub async fn get_buckets_by_user(
        &self,
        user_account: &[u8],
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::Bucket>> {
        // Convert user_account bytes to string
        let user_str = String::from_utf8_lossy(user_account);

        self.repository
            .get_buckets_by_user(&user_str)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }
}

// Test-only mutable operations
#[cfg(test)]
impl DBClient {
    /// Create a new BSP (test only)
    pub async fn create_bsp(
        &self,
        new_bsp: crate::repository::NewBsp,
    ) -> crate::error::Result<shc_indexer_db::models::Bsp> {
        // In tests, StorageOperations includes IndexerOpsMut
        self.repository
            .create_bsp(new_bsp)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Update BSP capacity (test only)
    pub async fn update_bsp_capacity(
        &self,
        id: i64,
        capacity: bigdecimal::BigDecimal,
    ) -> crate::error::Result<shc_indexer_db::models::Bsp> {
        self.repository
            .update_bsp_capacity(id, capacity)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Delete a BSP (test only)
    pub async fn delete_bsp(&self, account: &str) -> crate::error::Result<()> {
        self.repository
            .delete_bsp(account)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Create a new bucket (test only)
    pub async fn create_bucket(
        &self,
        new_bucket: crate::repository::NewBucket,
    ) -> crate::error::Result<shc_indexer_db::models::Bucket> {
        self.repository
            .create_bucket(new_bucket)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Create a new file (test only)
    pub async fn create_file(
        &self,
        new_file: crate::repository::NewFile,
    ) -> crate::error::Result<shc_indexer_db::models::File> {
        self.repository
            .create_file(new_file)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Update file step (test only)
    pub async fn update_file_step(&self, file_key: &[u8], step: i32) -> crate::error::Result<()> {
        self.repository
            .update_file_step(file_key, step)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Delete a file (test only)
    pub async fn delete_file(&self, file_key: &[u8]) -> crate::error::Result<()> {
        self.repository
            .delete_file(file_key)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Clear all data (test only)
    pub async fn clear_all(&self) -> crate::error::Result<()> {
        self.repository.clear_all().await;
        Ok(())
    }
}

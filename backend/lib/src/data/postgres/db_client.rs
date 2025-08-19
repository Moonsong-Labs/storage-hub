//! Database client wrapper using repository pattern abstraction
//!
//! This module provides a database client that delegates all operations
//! to an underlying repository implementation, allowing for both production
//! PostgreSQL and mock implementations for testing.

use std::sync::Arc;

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
        let file = self
            .repository
            .get_file_by_key(file_key)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?
            .ok_or_else(|| crate::error::Error::NotFound("File not found".to_string()))?;

        // Convert from repository File to shc_indexer_db::models::File
        Ok(shc_indexer_db::models::File {
            id: file.id,
            account: file.account,
            file_key: file.file_key,
            bucket_id: file.bucket_id,
            location: file.location,
            fingerprint: file.fingerprint,
            size: file.size,
            step: file.step,
            created_at: file.created_at,
            updated_at: file.updated_at,
        })
    }

    /// Get all files for a user
    pub async fn get_files_by_user(
        &self,
        user_account: &[u8],
        _limit: Option<i64>,
        _offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::File>> {
        let files = self
            .repository
            .get_files_by_user(user_account)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        // Convert from repository Files to shc_indexer_db::models::File
        Ok(files
            .into_iter()
            .map(|f| shc_indexer_db::models::File {
                id: f.id,
                account: f.account,
                file_key: f.file_key,
                bucket_id: f.bucket_id,
                location: f.location,
                fingerprint: f.fingerprint,
                size: f.size,
                step: f.step,
                created_at: f.created_at,
                updated_at: f.updated_at,
            })
            .collect())
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
        let bsp = self
            .repository
            .get_bsp_by_id(bsp_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        // Convert from repository Bsp to shc_indexer_db::models::Bsp
        Ok(bsp.map(|b| shc_indexer_db::models::Bsp {
            id: b.id,
            account: b.account,
            capacity: b.capacity,
            stake: b.stake,
            last_tick_proven: b.last_tick_proven,
            onchain_bsp_id: b.onchain_bsp_id,
            merkle_root: b.merkle_root,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }))
    }

    /// Get all BSPs with optional pagination
    pub async fn get_all_bsps(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> crate::error::Result<Vec<shc_indexer_db::models::Bsp>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let bsps = self
            .repository
            .list_bsps(limit, offset)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        // Convert from repository Bsps to shc_indexer_db::models::Bsp
        Ok(bsps
            .into_iter()
            .map(|b| shc_indexer_db::models::Bsp {
                id: b.id,
                account: b.account,
                capacity: b.capacity,
                stake: b.stake,
                last_tick_proven: b.last_tick_proven,
                onchain_bsp_id: b.onchain_bsp_id,
                merkle_root: b.merkle_root,
                created_at: b.created_at,
                updated_at: b.updated_at,
            })
            .collect())
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
        let files = self
            .repository
            .get_files_by_bucket(bucket_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        // Convert from repository Files to shc_indexer_db::models::File
        Ok(files
            .into_iter()
            .map(|f| shc_indexer_db::models::File {
                id: f.id,
                account: f.account,
                file_key: f.file_key,
                bucket_id: f.bucket_id,
                location: f.location,
                fingerprint: f.fingerprint,
                size: f.size,
                step: f.step,
                created_at: f.created_at,
                updated_at: f.updated_at,
            })
            .collect())
    }

    /// Get a bucket by its ID
    pub async fn get_bucket_by_id(
        &self,
        bucket_id: i64,
    ) -> crate::error::Result<shc_indexer_db::models::Bucket> {
        let bucket = self
            .repository
            .get_bucket_by_id(bucket_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?
            .ok_or_else(|| crate::error::Error::NotFound("Bucket not found".to_string()))?;

        // Convert from repository Bucket to shc_indexer_db::models::Bucket
        Ok(shc_indexer_db::models::Bucket {
            id: bucket.id,
            msp_id: bucket.msp_id,
            account: bucket.account,
            onchain_bucket_id: bucket.onchain_bucket_id,
            name: bucket.name,
            collection_id: bucket.collection_id,
            private: bucket.private,
            merkle_root: bucket.merkle_root,
            created_at: bucket.created_at,
            updated_at: bucket.updated_at,
        })
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

        let buckets = self
            .repository
            .get_buckets_by_user(&user_str)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;

        // Convert from repository Buckets to shc_indexer_db::models::Bucket
        Ok(buckets
            .into_iter()
            .map(|b| shc_indexer_db::models::Bucket {
                id: b.id,
                msp_id: b.msp_id,
                account: b.account,
                onchain_bucket_id: b.onchain_bucket_id,
                name: b.name,
                collection_id: b.collection_id,
                private: b.private,
                merkle_root: b.merkle_root,
                created_at: b.created_at,
                updated_at: b.updated_at,
            })
            .collect())
    }
}

#[cfg(all(test, feature = "mocks"))]
impl DBClient {
    /// Create a test database client with mock repository
    pub fn test() -> Self {
        use crate::repository::MockRepository;

        let mock_repo = MockRepository::new();
        Self::new(Arc::new(mock_repo))
    }
}

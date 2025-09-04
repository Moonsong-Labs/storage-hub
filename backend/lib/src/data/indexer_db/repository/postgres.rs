//! PostgreSQL repository implementation.
//!
//! This module provides the production repository implementation using
//! PostgreSQL as the backing database through diesel-async.
//!
//! ## Key Components
//! - [`Repository`] - PostgreSQL implementation of StorageOperations
//!
//! ## Features
//! - Connection pooling through SmartPool
//! - Automatic test transactions in test mode
//! - Type-safe queries through diesel
//! - Comprehensive error handling

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[cfg(test)]
use shc_indexer_db::OnchainBspId;
use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    schema::{bsp, bucket, file},
    OnchainMspId,
};

#[cfg(test)]
use crate::data::indexer_db::repository::IndexerOpsMut;
use crate::data::indexer_db::repository::{
    error::RepositoryResult, pool::SmartPool, BucketId, FileKey, IndexerOps,
};

/// PostgreSQL repository implementation.
///
/// Provides all database operations using a connection pool
/// with automatic test transaction management.
pub struct Repository {
    pool: SmartPool,
}

impl Repository {
    /// Create a new Repository with the given database URL.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Result<Self, RepositoryError>` - Repository instance or error
    pub async fn new(database_url: &str) -> RepositoryResult<Self> {
        Ok(Self {
            pool: SmartPool::new(database_url).await?,
        })
    }
}

#[async_trait]
impl IndexerOps for Repository {
    // ============ BSP Read Operations ============
    async fn list_bsps(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<Bsp>> {
        let mut conn = self.pool.get().await?;

        let results: Vec<Bsp> = bsp::table
            .order(bsp::id.asc())
            .limit(limit)
            .offset(offset)
            .load(&mut *conn)
            .await?;

        Ok(results)
    }

    // ============ MSP Read Operations ============
    async fn get_msp_by_onchain_id(&self, msp: &OnchainMspId) -> RepositoryResult<Msp> {
        let mut conn = self.pool.get().await?;

        Msp::get_by_onchain_msp_id(&mut conn, msp.clone())
            .await
            .map_err(Into::into)
    }

    // ============ Bucket Read Operations ============
    async fn get_bucket_by_onchain_id(&self, bid: BucketId<'_>) -> RepositoryResult<Bucket> {
        let mut conn = self.pool.get().await?;

        Bucket::get_by_onchain_bucket_id(&mut conn, bid.0.to_owned())
            .await
            .map_err(Into::into)
    }

    async fn get_buckets_by_user_and_msp(
        &self,
        msp: i64,
        account: &str,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<Bucket>> {
        let mut conn = self.pool.get().await?;

        // Same as Bucket::get_user_buckets_by_msp but with pagination
        let buckets = bucket::table
            .order(bucket::id.asc())
            .filter(bucket::account.eq(account))
            .filter(bucket::msp_id.eq(msp))
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .await?;

        Ok(buckets)
    }

    async fn get_files_by_bucket(
        &self,
        bucket: i64,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<File>> {
        let mut conn = self.pool.get().await?;

        // Same as File::get_by_bucket_id but with pagination
        let files = file::table
            .filter(file::bucket_id.eq(bucket))
            .limit(limit)
            .offset(offset)
            .load(&mut conn)
            .await?;

        Ok(files)
    }

    // ============ File Read Operations ============
    async fn get_file_by_file_key(&self, key: FileKey<'_>) -> RepositoryResult<File> {
        let mut conn = self.pool.get().await?;

        File::get_by_file_key(&mut conn, key.0)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
#[async_trait]
impl IndexerOpsMut for Repository {
    // ============ BSP Write Operations ============
    async fn delete_bsp(&self, onchain_bsp_id: &OnchainBspId) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        diesel::delete(bsp::table.filter(bsp::onchain_bsp_id.eq(onchain_bsp_id)))
            .execute(&mut *conn)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use shc_indexer_db::{OnchainBspId, OnchainMspId};
    use shp_types::Hash;

    use super::*;
    use crate::data::indexer_db::{
        repository::{error::RepositoryError, postgres::Repository},
        test_helpers::{
            setup_test_db,
            snapshot_move_bucket::{
                BSP_NUM, BSP_ONE_ACCOUNT, BSP_ONE_ONCHAIN_ID, BUCKET_ACCOUNT, BUCKET_FILES,
                BUCKET_ID, BUCKET_NAME, BUCKET_ONCHAIN_ID, BUCKET_PRIVATE, FILE_ONE_FILE_KEY,
                FILE_ONE_LOCATION, MSP_ONE_ACCOUNT, MSP_ONE_ID, MSP_ONE_ONCHAIN_ID, MSP_TWO_ID,
                SNAPSHOT_SQL,
            },
        },
    };

    // TODO: replace with IndexOpsMut methods to use diesel directly
    const EXTRA_BUCKETS: &str = r#"INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root) \
            VALUES \
                (1, '0xuser123', 1, 'user123-bucket1', 'b1', false, '\x0102'), \
                (2, '0xuser123', 1, 'user123-bucket2', 'b2', true, '\x0304'), \
                (3, '0xuser123', 2, 'user123-bucket3', 'b3', false, '\x0506'), \
                (4, '0xuser456', 1, 'user456-bucket1', 'b4', true, '\x0708'), \
                (5, '0xuser123', 1, 'user123-bucket4', 'b5', false, '\x0910');"#;

    #[tokio::test]
    async fn list_bsps() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting all BSPs
        let bsps = repo
            .list_bsps(100, 0)
            .await
            .expect("Failed to get all BSPs");
        assert_eq!(
            bsps.len(),
            BSP_NUM,
            "Should have {BSP_NUM} BSPs without pagination"
        );

        // Test with limit
        let limited_bsps = repo
            .list_bsps(2, 0)
            .await
            .expect("Failed to get limited BSPs");
        assert!(
            limited_bsps.len() <= 2,
            "Should return at most 2 BSPs with limit"
        );

        // Test with offset
        let offset_bsps = repo
            .list_bsps(100, 1)
            .await
            .expect("Failed to get offset BSPs");
        assert_ne!(
            limited_bsps[0].id, offset_bsps[0].id,
            "First BSP should be skipped with offset 1"
        );
    }

    #[tokio::test]
    async fn get_msp_by_onchain_id() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting MSP by onchain ID
        let msp_id = OnchainMspId::new(Hash::from(MSP_ONE_ONCHAIN_ID));
        let msp = repo
            .get_msp_by_onchain_id(&msp_id)
            .await
            .expect("Failed to get MSP");

        assert_eq!(msp.account, MSP_ONE_ACCOUNT);
    }

    #[tokio::test]
    async fn get_bucket_by_onchain_id() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting bucket by onchain ID
        let bucket = repo
            .get_bucket_by_onchain_id(BUCKET_ONCHAIN_ID.as_slice().into())
            .await
            .expect("Failed to get bucket");

        assert_eq!(bucket.name, BUCKET_NAME.as_bytes());
        assert_eq!(bucket.account, BUCKET_ACCOUNT);
        assert_eq!(bucket.private, BUCKET_PRIVATE);
    }

    #[tokio::test]
    async fn get_bucket_by_onchain_id_not_found() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let result = repo
            .get_bucket_by_onchain_id(b"nonexistent_bucket_id".as_slice().into())
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn get_files_by_bucket_filters_correctly() {
        // TODO: add a different bucket (with a file) to check filtering
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting all files in bucket
        let files = repo
            .get_files_by_bucket(BUCKET_ID, 100, 0)
            .await
            .expect("Failed to get bucket files");

        assert_eq!(
            files.len(),
            BUCKET_FILES,
            "Should have {BUCKET_FILES} files in bucket"
        );
    }

    #[tokio::test]
    async fn get_files_by_bucket_pagination() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test with limit
        let limited_files = repo
            .get_files_by_bucket(BUCKET_ID, 2, 0)
            .await
            .expect("Failed to get limited files");
        assert!(
            limited_files.len() <= 2,
            "Should return at most 2 files with limit"
        );

        // Test with offset
        let offset_files = repo
            .get_files_by_bucket(BUCKET_ID, 100, 1)
            .await
            .expect("Failed to get offset files");
        assert_ne!(
            limited_files[0].id, offset_files[0].id,
            "First File should be skipped with offset 1"
        );

        // Test limit and offset combined
        let paginated_files = repo
            .get_files_by_bucket(BUCKET_ID, 1, 1)
            .await
            .expect("Failed to get offset files");
        assert!(
            paginated_files.len() <= 1,
            "Should return at most 1 file with limit"
        );
        assert_ne!(
            limited_files[0].id, paginated_files[0].id,
            "First File should be skipped with offset 1"
        );
    }

    #[tokio::test]
    #[ignore = "TODO: setup empty bucket"]
    async fn get_files_by_bucket_empty_bucket() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let empty_bucket_id = 9999;

        let files = repo
            .get_files_by_bucket(empty_bucket_id, 100, 0)
            .await
            .expect("should handle empty bucket");

        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn get_files_by_bucket_nonexistent_bucket() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting all files in bucket
        let files = repo
            .get_files_by_bucket(123456, 100, 0)
            .await
            .expect("should handle non-existent bucket");

        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp() {
        let (_container, database_url) = setup_test_db(
            vec![SNAPSHOT_SQL.to_string()],
            vec![EXTRA_BUCKETS.to_string()],
        )
        .await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let buckets = repo
            .get_buckets_by_user_and_msp(MSP_ONE_ID, "0xuser123", 100, 0)
            .await
            .expect("Failed to get user buckets");

        assert_eq!(
            buckets.len(),
            3,
            "Should have 3 buckets for 0xuser123 with MSP #1"
        );
    }

    //TODO: add the following tests
    // * `get_buckets_by_user_and_msp_filters_other_users`: trim down extra buckets for the test above, since otherwise the test above _also_ filters
    // * `get_buckets_by_user_and_msp_filters_other_msps`: same here
    // * `get_buckets_by_user_and_msp_filters_no_msp`: for buckets without msp

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_pagination() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test with limit
        let limited_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 2, 0)
            .await
            .expect("Failed to get limited buckets");
        assert!(
            limited_buckets.len() <= 2,
            "Should return at most 2 buckets with limit"
        );

        // Test with offset
        let offset_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 100, 1)
            .await
            .expect("Failed to get offset buckets");
        assert_ne!(
            limited_buckets[0].name, offset_buckets[0].name,
            "Should skip first bucket"
        );

        // Test limit and offset combined
        let paginated_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 2, 1)
            .await
            .expect("Failed to get offset buckets");
        assert!(
            paginated_buckets.len() <= 2,
            "Should return at most 2 buckets with limit"
        );
        assert_ne!(
            limited_buckets[0].name, paginated_buckets[0].name,
            "Should skip first bucket"
        );
    }

    #[tokio::test]
    async fn get_file_by_file_key() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let file = repo
            .get_file_by_file_key(FILE_ONE_FILE_KEY.as_slice().into())
            .await
            .expect("Failed to get file info");

        assert_eq!(file.file_key, FILE_ONE_FILE_KEY);
        assert_eq!(file.onchain_bucket_id, BUCKET_ONCHAIN_ID);
        assert_eq!(file.location, FILE_ONE_LOCATION.as_bytes());
    }

    #[tokio::test]
    async fn get_file_by_file_key_not_found() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let result = repo
            .get_file_by_file_key(b"non-existing-file-key".as_slice().into())
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn delete_bsp() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Verify initial count
        let initial_bsps = repo
            .list_bsps(100, 0)
            .await
            .expect("Failed to get initial BSPs");
        assert_eq!(
            initial_bsps.len(),
            BSP_NUM,
            "Should start with {BSP_NUM} BSPs"
        );

        // Delete a BSP
        let bsp_id = OnchainBspId::new(Hash::from(BSP_ONE_ONCHAIN_ID));
        repo.delete_bsp(&bsp_id)
            .await
            .expect("Failed to delete BSP");

        // Verify deletion
        let remaining_bsps = repo
            .list_bsps(100, 0)
            .await
            .expect("Failed to get remaining BSPs");
        assert_eq!(
            remaining_bsps.len(),
            BSP_NUM - 1,
            "Should have 1 less BSP after deletion"
        );

        // Verify the correct BSP was deleted
        for bsp in &remaining_bsps {
            assert_ne!(
                bsp.account, BSP_ONE_ACCOUNT,
                "Deleted BSP should not be present"
            );
        }
    }
}

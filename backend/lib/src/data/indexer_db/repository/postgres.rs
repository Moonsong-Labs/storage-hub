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
// FIXME: all these tests fail locally due to some testcontainers setup issue
mod tests {
    use hex::ToHex;
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
        // TODO: replace with IndexerOpsMut methods to use diesel directly
        // Add a second bucket with one file to verify filtering
        let additional_bucket_id = 123;
        let additional_data = format!(r#"
            -- Add a second bucket
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES ({bucket_id}, '5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o', 2, 'other-bucket', {onchain_bucket_id}', false, '\x0102');

            -- Add a file to the second bucket
            INSERT INTO file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step, deletion_status)
            VALUES (4, '\x20d81e86ed5b986d1d6ddbe416627f96f740252c4a80ab8ed91db58f7ecf9657',
                    '\xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890', {bucket_id},
                    '{onchain_bucket_id}', 'file.txt',
                    '\x0000000000000000000000000000000000000000000000000000000000000002', 12345, 1, NULL);
        "#, bucket_id = additional_bucket_id, onchain_bucket_id = "additional-bucket");

        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![additional_data]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Test getting all files in bucket #1
        let bucket1_files = repo
            .get_files_by_bucket(BUCKET_ID, 100, 0)
            .await
            .expect("Failed to get bucket #1 files");

        assert_eq!(
            bucket1_files.len(),
            BUCKET_FILES,
            "Should have {BUCKET_FILES} files in bucket #1"
        );

        // Verify all files belong to bucket #1
        for file in &bucket1_files {
            assert_eq!(
                file.bucket_id, BUCKET_ID,
                "All files should belong to bucket #1"
            );
        }

        // Test getting files in other bucket
        let bucket2_files = repo
            .get_files_by_bucket(additional_bucket_id, 100, 0)
            .await
            .expect("Failed to get other bucket files");

        assert_eq!(bucket2_files.len(), 1, "Should have 1 file in other bucket");

        // Verify the file belongs to bucket #2
        for file in &bucket2_files {
            assert_eq!(
                file.bucket_id, additional_bucket_id,
                "File should belong to other bucket"
            );
        }
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
    async fn get_files_by_bucket_empty_bucket() {
        let empty_bucket_id = 9999;

        // TODO: replace with IndexerOpsMut methods to use diesel directly
        // Create an empty bucket (no files associated with it)
        // Also need to create the MSP that the bucket references
        let setup_sql = format!(
            r#"
            INSERT INTO msp (id, account, onchain_msp_id)
            VALUES
                ({msp_id}, '5CMDKyadzWu6MUwCzBB93u32Z1PPPsV8A1qAy4ydyVWuRzWR', '\x0000000000000000000000000000000000000000000000000000000000000301');

            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES
                ({bucket_id}, '0xemptybucketuser', {msp_id}, 'empty-bucket', 'empty-bucket-id', false, '\x0000');
        "#,
            msp_id = 123,
            bucket_id = empty_bucket_id,
        );

        let (_container, database_url) = setup_test_db(vec![], vec![setup_sql.to_string()]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let files = repo
            .get_files_by_bucket(empty_bucket_id, 100, 0)
            .await
            .expect("should handle empty bucket");

        assert!(files.is_empty(), "Empty bucket should return no files");

        // Verify that other buckets still have files
        let bucket1_files = repo
            .get_files_by_bucket(BUCKET_ID, 100, 0)
            .await
            .expect("Failed to get bucket 1 files");

        assert_eq!(
            bucket1_files.len(),
            BUCKET_FILES,
            "Bucket #1 should still have its files"
        );
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
        // TODO: replace with IndexerOpsMut methods to use diesel directly
        // Add 2 more buckets for BUCKET_ACCOUNT with MSP #2
        let extra_buckets = format!(
            r#"INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES
                (2, '{account}', {msp_id}, 'user-bucket2', 'b2', true, '\x0304'),
                (3, '{account}', {msp_id}, 'user-bucket3', 'b3', false, '\x0506');"#,
            account = BUCKET_ACCOUNT,
            msp_id = MSP_TWO_ID
        );

        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![extra_buckets]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // SNAPSHOT_SQL already has 1 bucket for BUCKET_ACCOUNT with MSP #2
        // We added 2 more, so should have 3 total
        let buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 100, 0)
            .await
            .expect("Failed to get user buckets");

        assert_eq!(
            buckets.len(),
            3,
            "Should have 3 buckets for BUCKET_ACCOUNT with MSP #2"
        );
    }

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
    async fn get_buckets_by_user_and_msp_filters_other_users() {
        // TODO: replace with IndexerOpsMut methods to use diesel directly
        // Add one bucket for a different user with MSP #2 to test filtering
        let user_filter_test_data = format!(
            r#"INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES
                (2, '0xotheruser', {msp_id}, 'other-user-bucket', 'oub1', false, '\x0506');"#,
            msp_id = MSP_TWO_ID
        );

        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![user_filter_test_data]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Should only return BUCKET_ACCOUNT's bucket (from snapshot), not other user's bucket
        let target_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 100, 0)
            .await
            .expect("Failed to get target user buckets");

        assert_eq!(
            target_buckets.len(),
            1,
            "Should return exactly 1 bucket for BUCKET_ACCOUNT (from snapshot)"
        );

        // Verify the returned bucket belongs to BUCKET_ACCOUNT
        assert_eq!(
            target_buckets[0].account, BUCKET_ACCOUNT,
            "Bucket should belong to BUCKET_ACCOUNT"
        );
        assert_eq!(
            target_buckets[0].msp_id,
            Some(MSP_TWO_ID),
            "Bucket should have MSP #2"
        );

        // Verify other user's bucket is not included
        let other_user_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, "0xotheruser", 100, 0)
            .await
            .expect("Failed to get other user buckets");

        assert_eq!(
            other_user_buckets.len(),
            1,
            "Other user should have their own bucket"
        );
        assert_ne!(
            other_user_buckets[0].id, target_buckets[0].id,
            "Should be different buckets"
        );
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_filters_other_msps() {
        // TODO: replace with IndexerOpsMut methods to use diesel directly
        // Add bucket for BUCKET_ACCOUNT with MSP #1 to test MSP filtering
        let msp_filter_test_data = format!(
            r#"INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES
                (2, '{account}', {msp_id}, 'user-msp1-bucket', 'mb1', false, '\x0102');"#,
            account = BUCKET_ACCOUNT,
            msp_id = MSP_ONE_ID
        );

        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![msp_filter_test_data]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Should only return buckets for MSP #1
        let msp1_buckets = repo
            .get_buckets_by_user_and_msp(MSP_ONE_ID, BUCKET_ACCOUNT, 100, 0)
            .await
            .expect("Failed to get MSP #1 buckets");

        assert_eq!(
            msp1_buckets.len(),
            1,
            "Should return exactly 1 bucket for MSP #1 (from our insert)"
        );

        // Verify the returned bucket has MSP #1
        assert_eq!(
            msp1_buckets[0].msp_id,
            Some(MSP_ONE_ID),
            "Bucket should have MSP #1"
        );
        assert_eq!(
            msp1_buckets[0].name, b"user-msp1-bucket",
            "Should be our MSP1 bucket"
        );

        // Should only return buckets for MSP #2
        let msp2_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 100, 0)
            .await
            .expect("Failed to get MSP #2 buckets");

        assert_eq!(
            msp2_buckets.len(),
            1,
            "Should return exactly 1 bucket for MSP #2 (from snapshot)"
        );

        // Verify the returned bucket has MSP #2
        assert_eq!(
            msp2_buckets[0].msp_id,
            Some(MSP_TWO_ID),
            "Bucket should have MSP #2"
        );
        assert_eq!(
            msp2_buckets[0].name,
            BUCKET_NAME.as_bytes(),
            "Should be the snapshot bucket"
        );
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_filters_no_msp() {
        // TODO: replace with IndexerOpsMut methods to use diesel directly
        // Add bucket with NULL MSP to test filtering
        let null_msp_test_data = format!(
            r#"INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES
                (2, '{account}', NULL, 'no-msp-bucket', 'nmb1', false, '\x0506');"#,
            account = BUCKET_ACCOUNT
        );

        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![null_msp_test_data]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Should only return buckets with the specified MSP, not those with NULL MSP
        let msp_buckets = repo
            .get_buckets_by_user_and_msp(MSP_TWO_ID, BUCKET_ACCOUNT, 100, 0)
            .await
            .expect("Failed to get MSP #2 buckets");

        assert_eq!(
            msp_buckets.len(),
            1,
            "Should return exactly 1 bucket with MSP #2 (excluding NULL MSP bucket)"
        );

        // Verify the returned bucket has MSP #2, not NULL
        assert_eq!(
            msp_buckets[0].msp_id,
            Some(MSP_TWO_ID),
            "Bucket should have MSP #2"
        );
        assert_eq!(
            msp_buckets[0].name,
            BUCKET_NAME.as_bytes(),
            "Should be the snapshot bucket with MSP"
        );

        // Verify NULL MSP bucket is not included
        assert_ne!(
            msp_buckets[0].name, b"no-msp-bucket",
            "Should not include the NULL MSP bucket"
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

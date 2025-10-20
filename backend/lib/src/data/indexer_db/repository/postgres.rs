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
#[cfg(test)]
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
#[cfg(test)]
use shc_indexer_db::{models::FileStorageRequestStep, OnchainBspId};
use shc_indexer_db::{
    models::{payment_stream::PaymentStream, Bsp, Bucket, File, Msp},
    schema::{bsp, bucket, file},
    OnchainMspId,
};
use shp_types::Hash;

#[cfg(test)]
use crate::constants::test;
#[cfg(test)]
use crate::data::indexer_db::repository::IndexerOpsMut;
use crate::data::indexer_db::repository::{
    error::{RepositoryError, RepositoryResult},
    pool::SmartPool,
    IndexerOps, PaymentStreamData, PaymentStreamKind,
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
    async fn get_msp_by_onchain_id(&self, onchain_msp_id: &OnchainMspId) -> RepositoryResult<Msp> {
        let mut conn = self.pool.get().await?;

        Msp::get_by_onchain_msp_id(&mut conn, onchain_msp_id.clone())
            .await
            .map_err(Into::into)
    }

    // ============ Bucket Read Operations ============
    async fn get_bucket_by_onchain_id(&self, onchain_bucket_id: &Hash) -> RepositoryResult<Bucket> {
        let mut conn = self.pool.get().await?;

        Bucket::get_by_onchain_bucket_id(&mut conn, onchain_bucket_id.as_bytes().to_vec())
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
    async fn get_file_by_file_key(&self, file_key: &Hash) -> RepositoryResult<File> {
        let mut conn = self.pool.get().await?;

        File::get_by_file_key(&mut conn, file_key.as_bytes())
            .await
            .map_err(Into::into)
    }

    // ============ Payment Stream Operations ============
    async fn get_payment_streams_for_user(
        &self,
        user_account: &str,
    ) -> RepositoryResult<Vec<PaymentStreamData>> {
        let mut conn = self.pool.get().await?;

        // Get all payment streams for the user from the database
        let streams = PaymentStream::get_all_by_user(&mut conn, user_account.to_string()).await?;

        // Convert to our PaymentStreamData format, preserving BigDecimal types
        streams
            .into_iter()
            .map(|stream| {
                let kind = match (stream.rate, stream.amount_provided) {
                    (Some(rate), None) => Ok(PaymentStreamKind::Fixed { rate }),
                    (None, Some(amount_provided)) => {
                        Ok(PaymentStreamKind::Dynamic { amount_provided })
                    }
                    _ => Err(RepositoryError::configuration(
                        "payment stream must be either fixed or dynamic",
                    )),
                }?;

                Ok(PaymentStreamData {
                    provider: stream.provider,
                    total_amount_paid: stream.total_amount_paid,
                    kind,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

#[cfg(test)]
#[async_trait]
impl IndexerOpsMut for Repository {
    async fn create_msp(
        &self,
        account: &str,
        onchain_msp_id: OnchainMspId,
    ) -> RepositoryResult<Msp> {
        let mut conn = self.pool.get().await?;

        let msp = Msp::create(
            &mut conn,
            account.to_string(),
            BigDecimal::from(test::msp::DEFAULT_CAPACITY),
            test::msp::DEFAULT_VALUE_PROP.to_string(),
            vec![], // No multiaddresses for test data
            onchain_msp_id,
        )
        .await?;

        Ok(msp)
    }

    async fn delete_msp(&self, onchain_msp_id: &OnchainMspId) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        Msp::delete(&mut conn, onchain_msp_id.clone()).await?;
        Ok(())
    }

    async fn create_bsp(
        &self,
        account: &str,
        onchain_bsp_id: OnchainBspId,
        capacity: BigDecimal,
        stake: BigDecimal,
    ) -> RepositoryResult<Bsp> {
        let mut conn = self.pool.get().await?;

        let bsp = Bsp::create(
            &mut conn,
            account.to_string(),
            capacity,
            test::bsp::DEFAULT_MERKLE_ROOT.to_vec(),
            vec![], // No multiaddresses for test data
            onchain_bsp_id,
            stake,
        )
        .await?;

        Ok(bsp)
    }

    async fn delete_bsp(&self, onchain_bsp_id: &OnchainBspId) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        // TODO: also clear related associations, like bsp_file
        Bsp::delete(&mut conn, onchain_bsp_id.clone()).await?;
        Ok(())
    }

    async fn create_bucket(
        &self,
        account: &str,
        msp_id: Option<i64>,
        name: &[u8],
        onchain_bucket_id: &Hash,
        private: bool,
    ) -> RepositoryResult<Bucket> {
        let mut conn = self.pool.get().await?;

        let bucket = Bucket::create(
            &mut conn,
            msp_id,
            account.to_string(),
            onchain_bucket_id.as_bytes().to_vec(),
            name.to_vec(),
            None, // No collection_id
            private,
            test::bucket::DEFAULT_MERKLE_ROOT.to_vec(),
            format!("{:#?}", test::bucket::DEFAULT_VALUE_PROP_ID),
        )
        .await?;

        Ok(bucket)
    }

    async fn delete_bucket(&self, onchain_bucket_id: &Hash) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        // TODO: also clear related associations
        Bucket::delete(&mut conn, onchain_bucket_id.as_bytes().to_vec()).await?;
        Ok(())
    }

    async fn create_file(
        &self,
        account: &[u8],
        file_key: &Hash,
        bucket_id: i64,
        onchain_bucket_id: &Hash,
        location: &[u8],
        fingerprint: &[u8],
        size: i64,
    ) -> RepositoryResult<File> {
        let mut conn = self.pool.get().await?;

        let file = File::create(
            &mut conn,
            account.to_vec(),
            file_key.as_bytes().to_vec(),
            bucket_id,
            onchain_bucket_id.as_bytes().to_vec(),
            location.to_vec(),
            fingerprint.to_vec(),
            size,
            FileStorageRequestStep::Requested,
            vec![], // No peer_ids for simple test data
        )
        .await?;

        Ok(file)
    }

    async fn delete_file(&self, file_key: &Hash) -> RepositoryResult<()> {
        let mut conn = self.pool.get().await?;

        // TODO: also clear related associations, like bsp_file
        File::delete(&mut conn, file_key.as_bytes()).await?;
        Ok(())
    }

    async fn create_payment_stream(
        &self,
        user_account: &str,
        provider: &str,
        total_amount_paid: BigDecimal,
        kind: PaymentStreamKind,
    ) -> RepositoryResult<PaymentStreamData> {
        let mut conn = self.pool.get().await?;

        let payment_stream = match &kind {
            PaymentStreamKind::Fixed { rate } => {
                PaymentStream::create_fixed_rate(
                    &mut conn,
                    user_account.to_string(),
                    provider.to_string(),
                    rate.clone(),
                )
                .await?
            }
            PaymentStreamKind::Dynamic { amount_provided } => {
                PaymentStream::create_dynamic_rate(
                    &mut conn,
                    user_account.to_string(),
                    provider.to_string(),
                    amount_provided.clone(),
                )
                .await?
            }
        };

        // Update the total amount paid if provided
        if total_amount_paid > BigDecimal::from(0) {
            PaymentStream::update_total_amount(
                &mut conn,
                payment_stream.id,
                total_amount_paid.clone(),
                payment_stream.last_tick_charged,
                payment_stream.charged_at_tick,
            )
            .await?;
        }

        Ok(PaymentStreamData {
            provider: provider.to_string(),
            total_amount_paid,
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;
    use shc_indexer_db::{OnchainBspId, OnchainMspId};
    use shp_types::Hash;

    use super::*;
    use crate::{
        data::indexer_db::{
            repository::{error::RepositoryError, postgres::Repository, IndexerOpsMut},
            test_helpers::{
                setup_test_db,
                snapshot_move_bucket::{
                    BSP_NUM, BSP_ONE_ONCHAIN_ID, BUCKET_ACCOUNT, BUCKET_FILES, BUCKET_ID,
                    BUCKET_NAME, BUCKET_ONCHAIN_ID, BUCKET_PRIVATE, FILE_ONE_FILE_KEY,
                    FILE_ONE_LOCATION, MSP_ONE_ACCOUNT, MSP_ONE_ID, MSP_ONE_ONCHAIN_ID, MSP_TWO_ID,
                    MSP_TWO_ONCHAIN_ID, SNAPSHOT_SQL,
                },
            },
        },
        test_utils::{random_bytes_32, random_hash},
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
            .get_bucket_by_onchain_id(&Hash::from_slice(BUCKET_ONCHAIN_ID.as_slice()))
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

        // Any non-existent bucket ID will work for this test
        let result = repo.get_bucket_by_onchain_id(&random_hash()).await;

        assert!(
            result.is_err(),
            "Should return an error for non-existent bucket"
        );

        // Check for specific "not found" database error
        let err = result.unwrap_err();
        match err {
            RepositoryError::Database(db_err) => {
                // Check that the error message indicates the item was not found
                let error_string = db_err.to_string();
                assert!(
                    error_string.contains("not found") || error_string.contains("No rows returned"),
                    "Expected 'not found' error, got: {}",
                    error_string
                );
            }
            _ => panic!("Expected Database error for not found, got: {:?}", err),
        }
    }

    #[tokio::test]
    async fn get_files_by_bucket_filters_correctly() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Add a second bucket with one file to verify filtering
        // These values are arbitrary placeholders - the exact values don't matter for this test
        let additional_bucket_id = random_hash();
        let additional_bucket = repo
            .create_bucket(
                "5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o",
                Some(MSP_TWO_ID),
                b"other-bucket",
                &additional_bucket_id,
                false,
            )
            .await
            .expect("Failed to create additional bucket");

        // Add a file to the second bucket
        repo.create_file(
            &random_bytes_32(),
            &random_hash(),
            additional_bucket.id,
            &additional_bucket_id,
            b"file.txt",
            &random_bytes_32(),
            12345,
        )
        .await
        .expect("Failed to create file in additional bucket");

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
            .get_files_by_bucket(additional_bucket.id, 100, 0)
            .await
            .expect("Failed to get other bucket files");

        assert_eq!(bucket2_files.len(), 1, "Should have 1 file in other bucket");

        // Verify the file belongs to bucket #2
        for file in &bucket2_files {
            assert_eq!(
                file.bucket_id, additional_bucket.id,
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
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Create an empty bucket (no files associated with it)
        // First create a new MSP that the bucket references
        // These values are arbitrary placeholders - they just need to be unique
        let empty_msp_onchain_id = OnchainMspId::new(Hash::from(hex!(
            "0000000000000000000000000000000000000000000000000000000000000999"
        )));
        repo.create_msp(
            "5EmptyMspAccountAddressForTestingPurpose",
            empty_msp_onchain_id.clone(),
        )
        .await
        .expect("Failed to create MSP");

        let empty_msp = repo
            .get_msp_by_onchain_id(&empty_msp_onchain_id)
            .await
            .expect("Failed to get MSP");

        let empty_bucket = repo
            .create_bucket(
                "0xemptybucketuser",
                Some(empty_msp.id),
                b"empty-bucket",
                &random_hash(),
                false,
            )
            .await
            .expect("Failed to create empty bucket");

        let files = repo
            .get_files_by_bucket(empty_bucket.id, 100, 0)
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
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // SNAPSHOT_SQL already has 1 bucket for BUCKET_ACCOUNT with MSP #2
        // Add 2 more buckets for BUCKET_ACCOUNT with MSP #2
        // These bucket names and IDs are arbitrary placeholders
        repo.create_bucket(
            BUCKET_ACCOUNT,
            Some(MSP_TWO_ID),
            b"user-bucket2",
            &random_hash(),
            true,
        )
        .await
        .expect("Failed to create bucket 2");

        repo.create_bucket(
            BUCKET_ACCOUNT,
            Some(MSP_TWO_ID),
            b"user-bucket3",
            &random_hash(),
            false,
        )
        .await
        .expect("Failed to create bucket 3");

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

        // Add more buckets for pagination testing
        // SNAPSHOT_SQL already has 1 bucket for BUCKET_ACCOUNT with MSP #2
        // These bucket names and IDs are arbitrary placeholders
        repo.create_bucket(
            BUCKET_ACCOUNT,
            Some(MSP_TWO_ID),
            b"pagination-bucket-2",
            &random_hash(),
            false,
        )
        .await
        .expect("Failed to create pagination bucket 2");

        repo.create_bucket(
            BUCKET_ACCOUNT,
            Some(MSP_TWO_ID),
            b"pagination-bucket-3",
            &random_hash(),
            false,
        )
        .await
        .expect("Failed to create pagination bucket 3");

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
        assert!(
            !offset_buckets.is_empty(),
            "Should have buckets after offset"
        );
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
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Add one bucket for a different user with MSP #2 to test filtering
        // The exact user account doesn't matter, just needs to be different from BUCKET_ACCOUNT
        let other_user = "0xotheruser";
        repo.create_bucket(
            other_user,
            Some(MSP_TWO_ID),
            b"other-user-bucket",
            &random_hash(),
            false,
        )
        .await
        .expect("Failed to create other user bucket");

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
            .get_buckets_by_user_and_msp(MSP_TWO_ID, other_user, 100, 0)
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
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Add bucket for BUCKET_ACCOUNT with MSP #1 to test MSP filtering
        let msp1_bucket_name = b"user-msp1-bucket"; // saved for assertion
        repo.create_bucket(
            BUCKET_ACCOUNT,
            Some(MSP_ONE_ID),
            msp1_bucket_name,
            &random_hash(),
            false,
        )
        .await
        .expect("Failed to create MSP1 bucket");

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
            msp1_buckets[0].name, msp1_bucket_name,
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
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Add bucket with NULL MSP to test filtering
        let no_msp_bucket_name = b"no-msp-bucket"; // saved for assertion
        repo.create_bucket(
            BUCKET_ACCOUNT,
            None, // NULL MSP
            no_msp_bucket_name,
            &random_hash(),
            false,
        )
        .await
        .expect("Failed to create bucket with no MSP");

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
            msp_buckets[0].name, no_msp_bucket_name,
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
            .get_file_by_file_key(&Hash::from_slice(FILE_ONE_FILE_KEY.as_slice()))
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

        // Any non-existent file key will work for this test
        let result = repo.get_file_by_file_key(&random_hash()).await;

        assert!(
            result.is_err(),
            "Should return an error for non-existent file"
        );

        // Check for specific "not found" database error
        let err = result.unwrap_err();
        match err {
            RepositoryError::Database(db_err) => {
                // Check that the error message indicates the item was not found
                let error_string = db_err.to_string();
                assert!(
                    error_string.contains("not found") || error_string.contains("No rows returned"),
                    "Expected 'not found' error, got: {}",
                    error_string
                );
            }
            _ => panic!("Expected Database error for not found, got: {:?}", err),
        }
    }

    #[tokio::test]
    async fn get_payment_streams_filters_by_user() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let provider = format!("0x{}", hex::encode(MSP_TWO_ONCHAIN_ID));

        // inject additional payment stream to filter out
        let additional_user = hex::encode(random_bytes_32());
        repo.create_payment_stream(
            &additional_user,
            &provider,
            BigDecimal::from(1234),
            PaymentStreamKind::Fixed {
                rate: BigDecimal::from(42),
            },
        )
        .await
        .expect("able to inject payment stream");

        let streams = repo
            .get_payment_streams_for_user(BUCKET_ACCOUNT)
            .await
            .expect("able to retrieve payment streams");

        assert!(streams.len() > 0, "should have at least 1 payment stream");
        dbg!(&streams);

        let msp_stream = streams
            .iter()
            .find(|stream| stream.provider.as_str() == &provider)
            .expect("should have a payment stream with MSP holding the bucket");

        assert!(
            matches!(msp_stream.kind, PaymentStreamKind::Fixed { .. }),
            "msp stream should always be fixed"
        );

        let bsp_streams = streams
            .iter()
            .filter(|stream| matches!(stream.kind, PaymentStreamKind::Dynamic { .. }))
            .collect::<Vec<_>>();

        assert_eq!(
            bsp_streams.len(),
            3,
            "should have 3 BSPs storing files for given user"
        );

        let bsp_one = format!("0x{}", hex::encode(BSP_ONE_ONCHAIN_ID));
        bsp_streams
            .iter()
            .find(|stream| stream.provider.as_str() == &bsp_one)
            .expect("should have a payment stream with BSP #1");
    }

    #[tokio::test]
    async fn delete_bsp() {
        let (_container, database_url) =
            setup_test_db(vec![SNAPSHOT_SQL.to_string()], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        // Create a new BSP without any files for deletion testing
        // All these values are arbitrary placeholders for test data
        let test_bsp_id = OnchainBspId::new(Hash::from(hex!(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        )));

        let created_bsp = repo
            .create_bsp(
                "5TestBspAccountAddressForDeletionTesting",
                test_bsp_id.clone(),
                BigDecimal::from(1000000_i64),
                BigDecimal::from(50000_i64),
            )
            .await
            .expect("Failed to create test BSP");

        // Verify the BSP was created
        let initial_bsps = repo
            .list_bsps(100, 0)
            .await
            .expect("Failed to get initial BSPs");
        assert_eq!(
            initial_bsps.len(),
            BSP_NUM + 1,
            "Should have one additional BSP after creation"
        );

        // Delete the newly created BSP (which has no files)
        repo.delete_bsp(&test_bsp_id)
            .await
            .expect("Failed to delete BSP");

        // Verify deletion
        let remaining_bsps = repo
            .list_bsps(100, 0)
            .await
            .expect("Failed to get remaining BSPs");
        assert_eq!(
            remaining_bsps.len(),
            BSP_NUM,
            "Should be back to original BSP count after deletion"
        );

        // Verify the correct BSP was deleted
        for bsp in &remaining_bsps {
            assert_ne!(bsp.id, created_bsp.id, "Deleted BSP should not be present");
        }
    }
}

//! Database client wrapper using repository pattern abstraction
//!
//! This module provides a database client that delegates all operations
//! to an underlying repository implementation, allowing for both production
//! PostgreSQL and mock implementations for testing.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

#[cfg(test)]
use bigdecimal::BigDecimal;

#[cfg(test)]
use shc_indexer_db::OnchainBspId;
use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    OnchainMspId,
};
use tracing::debug;

use crate::{
    constants::database::{DEFAULT_PAGE_LIMIT, MSP_CACHE_TTL_SECS},
    data::indexer_db::repository::{PaymentStreamData, StorageOperations},
    error::Result,
};

#[cfg(test)]
use crate::data::indexer_db::repository::PaymentStreamKind;

/// Cache entry for MSP data
struct MspCacheEntry {
    msp: Msp,
    last_refreshed: Instant,
}

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
    msp_cache: Arc<RwLock<Option<MspCacheEntry>>>,
}

impl DBClient {
    /// Create a new database client with the given repository
    ///
    /// # Arguments
    /// * `repository` - Repository implementation to use for database operations
    pub fn new(repository: Arc<dyn StorageOperations>) -> Self {
        Self {
            repository,
            msp_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Test the database connection
    pub async fn test_connection(&self) -> Result<()> {
        debug!(target: "indexer_db::client::test_connection", "Testing database connection");

        // Try to list BSPs with a limit of 1 to test the connection
        self.repository.list_bsps(1, 0).await?;
        Ok(())
    }

    /// Get all BSPs with pagination
    pub async fn get_all_bsps(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Bsp>> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        let offset = offset.unwrap_or(0);
        debug!(target: "indexer_db::client::get_all_bsps", limit = limit, offset = offset, "Fetching BSPs");

        self.repository
            .list_bsps(limit, offset)
            .await
            .map_err(Into::into)
    }

    /// Retrieve a given MSP's entry by its onchain ID
    ///
    /// This method caches the MSP data to avoid repeated database hits.
    /// The cache is automatically refreshed after the configured TTL expires.
    pub async fn get_msp(&self, msp_onchain_id: &OnchainMspId) -> Result<Msp> {
        debug!(target: "indexer_db::client::get_msp", onchain_id = %msp_onchain_id, "Fetching MSP");

        // Check if we have a valid cached entry
        {
            let cache = self.msp_cache.read().await;
            if let Some(entry) = &*cache {
                // Check if the cache entry matches the requested MSP and is still valid
                if entry.msp.onchain_msp_id == *msp_onchain_id
                    && entry.last_refreshed.elapsed() < Duration::from_secs(MSP_CACHE_TTL_SECS)
                {
                    return Ok(entry.msp.clone());
                }
            }
        }

        // Cache miss or expired, fetch from database
        let msp = self
            .repository
            .get_msp_by_onchain_id(msp_onchain_id)
            .await
            .map_err(Into::<crate::error::Error>::into)?;

        // Update cache with the new value
        {
            let mut cache = self.msp_cache.write().await;
            *cache = Some(MspCacheEntry {
                msp: msp.clone(),
                last_refreshed: Instant::now(),
            });
        }

        Ok(msp)
    }

    /// Invalidate the MSP cache if it matches the given MSP
    ///
    /// # Arguments
    /// * expected_id: the MSP for which the cache should be invalidated
    ///
    /// If no MSP was specified the cache is always invalidated.
    pub async fn invalidate_msp_cache(&self, expected_id: Option<&OnchainMspId>) {
        let mut cache = self.msp_cache.write().await;

        match expected_id {
            None => *cache = None,
            Some(id)
                if cache
                    .as_ref()
                    .map(|cache| &cache.msp.onchain_msp_id == id)
                    .unwrap_or_default() =>
            {
                *cache = None
            }
            _ => {}
        }
    }

    /// Retrieve info on a specific bucket given its onchain ID
    pub async fn get_bucket(&self, bucket_onchain_id: &[u8]) -> Result<Bucket> {
        let hash = shp_types::Hash::from_slice(bucket_onchain_id);
        debug!(target: "indexer_db::client::get_bucket", onchain_id = %hash, "Fetching bucket");

        self.repository
            .get_bucket_by_onchain_id(&hash)
            .await
            .map_err(Into::into)
    }

    /// Get the files of the given bucket with pagination
    pub async fn get_bucket_files(
        &self,
        bucket: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<File>> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        let offset = offset.unwrap_or(0);
        debug!(
            target: "indexer_db::client::get_bucket_files",
            bucket_id = bucket,
            limit = limit,
            offset = offset,
            "Fetching bucket files"
        );

        self.repository
            .get_files_by_bucket(bucket, limit, offset)
            .await
            .map_err(Into::into)
    }

    /// Get all the `user`'s buckets with the given MSP
    pub async fn get_user_buckets(
        &self,
        msp: &OnchainMspId,
        user: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Bucket>> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        let offset = offset.unwrap_or(0);
        debug!(
            target: "indexer_db::client::get_user_buckets",
            msp = %msp,
            user = %user,
            limit = limit,
            offset = offset,
            "Fetching user buckets"
        );

        let msp = self.get_msp(msp).await?;

        self.repository
            .get_buckets_by_user_and_msp(msp.id, user, limit, offset)
            .await
            .map_err(Into::into)
    }

    /// Count all of the `user`'s buckets with the given MSP
    pub async fn get_user_buckets_count(&self, msp: &OnchainMspId, user: &str) -> Result<u64> {
        debug!(
            target: "indexer_db::client::get_user_buckets_count",
            msp = %msp,
            user = %user,
            "Counting user buckets"
        );

        let msp = self.get_msp(msp).await?;

        self.repository
            .get_buckets_count_by_user_and_msp(msp.id, user)
            .await
            .map_err(Into::into)
    }

    pub async fn get_file_info(&self, file_key: &[u8]) -> Result<File> {
        let hash = shp_types::Hash::from_slice(file_key);
        debug!(target: "indexer_db::client::get_file_info", file_key = %hash, "Fetching file info");

        self.repository
            .get_file_by_file_key(&hash)
            .await
            .map_err(Into::into)
    }

    /// Get all payment streams for a user
    pub async fn get_payment_streams_for_user(
        &self,
        user_account: &str,
    ) -> Result<Vec<PaymentStreamData>> {
        debug!(
            target: "indexer_db::client::get_payment_streams_for_user",
            user_account = %user_account,
            "Fetching payment streams for user"
        );

        self.repository
            .get_payment_streams_for_user(user_account)
            .await
            .map_err(Into::into)
    }

    /// Get the number of files stored by the given MSP
    pub async fn get_number_of_files_stored_by_msp(&self, msp: &OnchainMspId) -> Result<u64> {
        debug!(target: "indexer_db::client::get_number_of_files_stored_by_msp", msp = %msp, "Fetching number of files stored by MSP");

        self.repository
            .get_number_of_files_stored_by_msp(msp)
            .await
            .map_err(Into::into)
    }
}

// Test-only mutable operations
#[cfg(test)]
impl DBClient {
    /// Create a new MSP
    pub async fn create_msp(&self, account: &str, onchain_msp_id: OnchainMspId) -> Result<Msp> {
        let msp = self.repository.create_msp(account, onchain_msp_id).await?;

        // Invalidate cache after creating the cached MSP
        // NOTE: the operation described above is technically incorrect (creating an existing msp)
        self.invalidate_msp_cache(Some(&onchain_msp_id)).await;

        Ok(msp)
    }

    /// Delete an MSP
    pub async fn delete_msp(&self, onchain_msp_id: &OnchainMspId) -> Result<()> {
        let result = self.repository.delete_msp(onchain_msp_id).await?;

        // Invalidate cache after deleting the cached MSP
        self.invalidate_msp_cache(Some(&onchain_msp_id)).await;

        Ok(result)
    }

    /// Create a new BSP
    pub async fn create_bsp(
        &self,
        account: &str,
        onchain_bsp_id: OnchainBspId,
        capacity: BigDecimal,
        stake: BigDecimal,
    ) -> Result<Bsp> {
        self.repository
            .create_bsp(account, onchain_bsp_id, capacity, stake)
            .await
            .map_err(Into::into)
    }

    /// Delete a BSP
    pub async fn delete_bsp(&self, onchain_bsp_id: &OnchainBspId) -> Result<()> {
        self.repository
            .delete_bsp(onchain_bsp_id)
            .await
            .map_err(Into::into)
    }

    /// Create a new bucket
    pub async fn create_bucket(
        &self,
        account: &str,
        msp_id: Option<i64>,
        name: &[u8],
        onchain_bucket_id: &[u8],
        private: bool,
    ) -> Result<Bucket> {
        let hash = shp_types::Hash::from_slice(onchain_bucket_id);
        self.repository
            .create_bucket(account, msp_id, name, &hash, private)
            .await
            .map_err(Into::into)
    }

    /// Delete a bucket
    pub async fn delete_bucket(&self, onchain_bucket_id: &[u8]) -> Result<()> {
        let hash = shp_types::Hash::from_slice(onchain_bucket_id);
        self.repository
            .delete_bucket(&hash)
            .await
            .map_err(Into::into)
    }

    /// Create a new file
    pub async fn create_file(
        &self,
        account: &[u8],
        file_key: &[u8],
        bucket_id: i64,
        onchain_bucket_id: &[u8],
        location: &[u8],
        fingerprint: &[u8],
        size: i64,
    ) -> Result<File> {
        let file_hash = shp_types::Hash::from_slice(file_key);
        let bucket_hash = shp_types::Hash::from_slice(onchain_bucket_id);
        self.repository
            .create_file(
                account,
                &file_hash,
                bucket_id,
                &bucket_hash,
                location,
                fingerprint,
                size,
            )
            .await
            .map_err(Into::into)
    }

    /// Delete a file
    pub async fn delete_file(&self, file_key: &[u8]) -> Result<()> {
        let hash = shp_types::Hash::from_slice(file_key);
        self.repository.delete_file(&hash).await.map_err(Into::into)
    }

    /// Create a payment stream
    pub async fn create_payment_stream(
        &self,
        user_account: &str,
        provider: &str,
        total_amount_paid: BigDecimal,
        kind: PaymentStreamKind,
    ) -> Result<PaymentStreamData> {
        self.repository
            .create_payment_stream(user_account, provider, total_amount_paid, kind)
            .await
            .map_err(Into::into)
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use std::sync::Arc;

    use bigdecimal::FromPrimitive;
    use hex_literal::hex;

    use shp_types::Hash;

    use super::*;
    use crate::{
        constants::test::bsp::DEFAULT_BSP_ID,
        data::indexer_db::{
            mock_repository::MockRepository, repository::postgres::Repository,
            test_helpers::setup_test_db,
        },
    };

    async fn delete_bsp(client: DBClient, id: OnchainBspId) {
        let bsps = client
            // ensure we get as many as possible
            .get_all_bsps(Some(i64::MAX), Some(0))
            .await
            .expect("able to retrieve all bsps");

        let amount_of_bsps = bsps.len();
        assert!(amount_of_bsps > 0);

        client.delete_bsp(&id).await.expect("able to delete bsp");

        let bsps = client
            .get_all_bsps(Some(i64::MAX), Some(0))
            .await
            .expect("able to retrieve all bsps");

        assert_eq!(bsps.len(), amount_of_bsps - 1);
    }

    #[tokio::test]
    async fn delete_bsp_with_mock_repo() {
        // Create mock repository and add test data
        let repo = Arc::new(MockRepository::new());
        let client = DBClient::new(repo);

        client
            .create_bsp(
                "test_bsp_account",
                DEFAULT_BSP_ID,
                BigDecimal::from_i64(1000).unwrap(),
                BigDecimal::from_i64(100).unwrap(),
            )
            .await
            .expect("should create BSP");

        delete_bsp(client, DEFAULT_BSP_ID).await;
    }

    #[tokio::test]
    async fn delete_bsp_with_repo() {
        let (_container, database_url) = setup_test_db(vec![], vec![]).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("able to connect to db");
        let client = DBClient::new(Arc::new(repo));

        // Create a new BSP without any files for deletion testing
        // to avoid violating constraint on bsp_file table

        // All these values are arbitrary placeholders for test data
        let test_bsp_id = OnchainBspId::new(Hash::from(hex!(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        )));

        let _ = client
            .create_bsp(
                "5TestBspAccountAddressForDeletionTesting",
                test_bsp_id.clone(),
                BigDecimal::from(1000000_i64),
                BigDecimal::from(50000_i64),
            )
            .await
            .expect("Failed to create test BSP");

        delete_bsp(client, test_bsp_id).await;
    }

    //TODO: reuse tests from repository/postgres.rs
    // and setup mock repository the same way
}

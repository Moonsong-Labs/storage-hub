//! Database client wrapper using repository pattern abstraction
//!
//! This module provides a database client that delegates all operations
//! to an underlying repository implementation, allowing for both production
//! PostgreSQL and mock implementations for testing.

use std::sync::Arc;

#[cfg(test)]
use shc_indexer_db::OnchainBspId;
use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    OnchainMspId,
};

use crate::{
    constants::database::DEFAULT_PAGE_LIMIT, data::indexer_db::repository::StorageOperations,
    error::Result,
};

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
    pub async fn test_connection(&self) -> Result<()> {
        // Try to list BSPs with a limit of 1 to test the connection
        self.repository
            .list_bsps(1, 0)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Get all BSPs with pagination
    pub async fn get_all_bsps(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Bsp>> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        let offset = offset.unwrap_or(0);

        self.repository
            .list_bsps(limit, offset)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Retrieve a given MSP's entry by its onchain ID
    pub async fn get_msp(&self, msp_onchain_id: &OnchainMspId) -> Result<Msp> {
        // TODO: should we cache this?
        // since we always reference the same msp
        self.repository
            .get_msp_by_onchain_id(msp_onchain_id)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Retrieve info on a specific bucket given its onchain ID
    pub async fn get_bucket(&self, bucket_onchain_id: &[u8]) -> Result<Bucket> {
        self.repository
            .get_bucket_by_onchain_id(bucket_onchain_id.into())
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
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

        self.repository
            .get_files_by_bucket(bucket, limit, offset)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    /// Get all the `user`'s buckets with the given MSP
    pub async fn get_user_buckets(
        &self,
        msp: &OnchainMspId,
        user: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Bucket>> {
        let msp = self.get_msp(msp).await?;

        self.repository
            .get_buckets_by_user_and_msp(
                msp.id,
                user,
                limit.unwrap_or(DEFAULT_PAGE_LIMIT),
                offset.unwrap_or(0),
            )
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }

    pub async fn get_file_info(&self, file_key: &[u8]) -> Result<File> {
        self.repository
            .get_file_by_file_key(file_key.into())
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }
}

// Test-only mutable operations
#[cfg(test)]
impl DBClient {
    /// Delete a BSP
    pub async fn delete_bsp(&self, account: &OnchainBspId) -> crate::error::Result<()> {
        self.repository
            .delete_bsp(account)
            .await
            .map_err(|e| crate::error::Error::Database(e.to_string()))
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        constants::{database::DEFAULT_DATABASE_URL, test::bsp::DEFAULT_BSP_ID},
        data::indexer_db::{
            mock_repository::{tests::inject_sample_bsp, MockRepository},
            repository::postgres::Repository,
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
        let repo = MockRepository::new();
        let _id = inject_sample_bsp(&repo).await;

        // initialize client
        let client = DBClient::new(Arc::new(repo));
        delete_bsp(client, DEFAULT_BSP_ID).await;
    }

    #[tokio::test]
    // TODO: should NOT panic when we add testcontainers
    #[should_panic]
    async fn delete_bsp_with_repo() {
        // TODO: seed db with bsp

        let repo = Repository::new(DEFAULT_DATABASE_URL)
            .await
            .expect("able to connect to db");

        let client = DBClient::new(Arc::new(repo));
        delete_bsp(client, DEFAULT_BSP_ID).await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use shc_indexer_db::{OnchainBspId, OnchainMspId};

    use super::*;
    use crate::data::indexer_db::test_helpers::setup_test_db;
    use crate::data::indexer_db::repository::postgres::Repository;
    use crate::data::indexer_db::repository::BucketId;

    #[tokio::test]
    async fn test_test_connection() {
        // Initialize container with minimal data
        let init_sql = r#"
            INSERT INTO bsp (id, account, capacity, stake, onchain_bsp_id, merkle_root)
            VALUES (1, '0xtest', 1000, 100, '0x0000000000000000000000000000000000000000000000000000000000000001', '\x0102');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test connection should succeed
        client
            .test_connection()
            .await
            .expect("Connection test failed");
    }

    #[tokio::test]
    async fn test_get_all_bsps() {
        // Initialize container with BSP test data
        let init_sql = r#"
            INSERT INTO bsp (id, account, capacity, stake, onchain_bsp_id, merkle_root)
            VALUES 
                (1, '0xbsp1', 1000, 100, '0x0000000000000000000000000000000000000000000000000000000000000001', '\x0102'),
                (2, '0xbsp2', 2000, 200, '0x0000000000000000000000000000000000000000000000000000000000000002', '\x0304'),
                (3, '0xbsp3', 3000, 300, '0x0000000000000000000000000000000000000000000000000000000000000003', '\x0506');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test getting all BSPs
        let bsps = client
            .get_all_bsps(None, None)
            .await
            .expect("Failed to get all BSPs");
        assert_eq!(bsps.len(), 3, "Should have 3 BSPs without pagination");

        // Test with limit
        let limited_bsps = client
            .get_all_bsps(Some(2), None)
            .await
            .expect("Failed to get limited BSPs");
        assert_eq!(limited_bsps.len(), 2, "Should return 2 BSPs with limit");

        // Test with offset
        let offset_bsps = client
            .get_all_bsps(Some(2), Some(1))
            .await
            .expect("Failed to get offset BSPs");
        assert_eq!(offset_bsps.len(), 2, "Should return 2 BSPs with offset");
        assert_eq!(
            offset_bsps[0].account, "0xbsp2",
            "First BSP should be skipped"
        );
    }

    #[tokio::test]
    async fn test_get_msp() {
        // Initialize container with MSP test data
        let init_sql = r#"
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1', 5000, 'Fast storage', '0x0000000000000000000000000000000000000000000000000000000000000001'),
                (2, '0xmsp2', 8000, 'Reliable storage', '0x0000000000000000000000000000000000000000000000000000000000000002');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test getting MSP by onchain ID
        let msp_id = OnchainMspId::new(shp_types::Hash::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ]));
        let msp = client.get_msp(&msp_id).await.expect("Failed to get MSP");

        assert_eq!(msp.account, "0xmsp1");
        assert_eq!(msp.value_prop, "Fast storage");
    }

    #[tokio::test]
    async fn test_get_bucket() {
        // Initialize container with bucket test data
        let init_sql = r#"
            -- Insert MSP first (required for foreign key)
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1', 5000, 'Storage provider', '0x0000000000000000000000000000000000000000000000000000000000000001');

            -- Insert Buckets
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser1', 1, 'my-bucket', 'bucket123', false, '\x0102'),
                (2, '0xuser2', 1, 'private-bucket', 'bucket456', true, '\x0304');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test getting bucket by onchain ID
        let bucket = client
            .get_bucket(b"bucket123")
            .await
            .expect("Failed to get bucket");

        assert_eq!(bucket.name, b"my-bucket");
        assert_eq!(bucket.account, "0xuser1");
        assert_eq!(bucket.private, false);
    }

    #[tokio::test]
    async fn test_get_bucket_files() {
        // Initialize container with file test data
        let init_sql = r#"
            -- Insert MSP first
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1', 5000, 'Storage provider', '0x0000000000000000000000000000000000000000000000000000000000000001');

            -- Insert Bucket
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser1', 1, 'test-bucket', 'bucket123', false, '\x0102');

            -- Insert Files
            INSERT INTO file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step)
            VALUES 
                (1, '0xuser1', 'file1.txt', 1, 'bucket123', '/data/file1.txt', '\x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20', 1024, 1),
                (2, '0xuser1', 'file2.txt', 1, 'bucket123', '/data/file2.txt', '\x2021222324252627282930313233343536373839404142434445464748495051', 2048, 1),
                (3, '0xuser1', 'file3.txt', 1, 'bucket123', '/data/file3.txt', '\x5253545556575859606162636465666768697071727374757677787980818283', 512, 0),
                (4, '0xuser1', 'file4.txt', 1, 'bucket123', '/data/file4.txt', '\x8485868788899091929394959697989900010203040506070809101112131415', 4096, 1);
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test getting all files in bucket
        let files = client
            .get_bucket_files(1, None, None)
            .await
            .expect("Failed to get bucket files");
        assert_eq!(files.len(), 4, "Should have 4 files in bucket");

        // Test with limit
        let limited_files = client
            .get_bucket_files(1, Some(2), None)
            .await
            .expect("Failed to get limited files");
        assert_eq!(limited_files.len(), 2, "Should return 2 files with limit");

        // Test with offset
        let offset_files = client
            .get_bucket_files(1, Some(2), Some(2))
            .await
            .expect("Failed to get offset files");
        assert_eq!(
            offset_files.len(),
            2,
            "Should return remaining 2 files with offset"
        );
        assert_eq!(
            offset_files[0].file_key, b"file3.txt",
            "Should skip first 2 files"
        );
    }

    #[tokio::test]
    async fn test_get_user_buckets() {
        // Initialize container with multiple buckets for different users
        let init_sql = r#"
            -- Insert MSPs
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1', 5000, 'MSP One', '0x0000000000000000000000000000000000000000000000000000000000000001'),
                (2, '0xmsp2', 8000, 'MSP Two', '0x0000000000000000000000000000000000000000000000000000000000000002');

            -- Insert Buckets for different users and MSPs
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser123', 1, 'user123-bucket1', 'b1', false, '\x0102'),
                (2, '0xuser123', 1, 'user123-bucket2', 'b2', true, '\x0304'),
                (3, '0xuser123', 2, 'user123-bucket3', 'b3', false, '\x0506'),
                (4, '0xuser456', 1, 'user456-bucket1', 'b4', true, '\x0708'),
                (5, '0xuser123', 1, 'user123-bucket4', 'b5', false, '\x0910');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test getting user buckets for specific MSP
        let msp_id = OnchainMspId::new(shp_types::Hash::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1,
        ]));
        let buckets = client
            .get_user_buckets(&msp_id, "0xuser123", None, None)
            .await
            .expect("Failed to get user buckets");
        assert_eq!(
            buckets.len(),
            3,
            "Should have 3 buckets for user123 with MSP1"
        );

        // Test with limit
        let limited_buckets = client
            .get_user_buckets(&msp_id, "0xuser123", Some(2), None)
            .await
            .expect("Failed to get limited buckets");
        assert_eq!(
            limited_buckets.len(),
            2,
            "Should return 2 buckets with limit"
        );

        // Test with offset
        let offset_buckets = client
            .get_user_buckets(&msp_id, "0xuser123", Some(2), Some(1))
            .await
            .expect("Failed to get offset buckets");
        assert_eq!(
            offset_buckets.len(),
            2,
            "Should return 2 buckets with offset"
        );
        assert_eq!(
            offset_buckets[0].name, b"user123-bucket2",
            "Should skip first bucket"
        );
    }

    #[tokio::test]
    async fn test_get_file_info() {
        // Initialize container with file test data
        let init_sql = r#"
            -- Insert MSP first
            INSERT INTO msp (id, account, capacity, value_prop, onchain_msp_id)
            VALUES 
                (1, '0xmsp1', 5000, 'Storage provider', '0x0000000000000000000000000000000000000000000000000000000000000001');

            -- Insert Bucket
            INSERT INTO bucket (id, account, msp_id, name, onchain_bucket_id, private, merkle_root)
            VALUES 
                (1, '0xuser1', 1, 'test-bucket', 'bucket123', false, '\x0102');

            -- Insert Files with unique file keys
            INSERT INTO file (id, account, file_key, bucket_id, onchain_bucket_id, location, fingerprint, size, step)
            VALUES 
                (1, '0xuser1', 'unique-file-key-123', 1, 'bucket123', '/data/special.txt', '\x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20', 8192, 1),
                (2, '0xuser1', 'another-file-key', 1, 'bucket123', '/data/other.txt', '\x2021222324252627282930313233343536373839404142434445464748495051', 4096, 0);
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Test getting file by file key
        let file = client
            .get_file_info(b"unique-file-key-123")
            .await
            .expect("Failed to get file info");

        assert_eq!(file.file_key, b"unique-file-key-123");
        assert_eq!(file.size, 8192);
        assert_eq!(file.location.as_deref(), Some("/data/special.txt"));
        assert_eq!(file.step.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_delete_bsp() {
        // Initialize container with BSP test data
        let init_sql = r#"
            INSERT INTO bsp (id, account, capacity, stake, onchain_bsp_id, merkle_root)
            VALUES 
                (1, '0xbsp1', 1000, 100, '0x0000000000000000000000000000000000000000000000000000000000000001', '\x0102'),
                (2, '0xbsp2', 2000, 200, '0x0000000000000000000000000000000000000000000000000000000000000002', '\x0304'),
                (3, '0xbsp3', 3000, 300, '0x0000000000000000000000000000000000000000000000000000000000000003', '\x0506');
        "#;

        let (_container, database_url) = setup_test_db(Some(init_sql)).await;

        let repo = Repository::new(&database_url)
            .await
            .expect("Failed to create repository");

        let client = DBClient::new(Arc::new(repo));

        // Verify initial count
        let initial_bsps = client
            .get_all_bsps(None, None)
            .await
            .expect("Failed to get initial BSPs");
        assert_eq!(initial_bsps.len(), 3, "Should start with 3 BSPs");

        // Delete a BSP
        let bsp_id = OnchainBspId::new(shp_types::Hash::from([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 2,
        ]));
        client
            .delete_bsp(&bsp_id)
            .await
            .expect("Failed to delete BSP");

        // Verify deletion
        let remaining_bsps = client
            .get_all_bsps(None, None)
            .await
            .expect("Failed to get remaining BSPs");
        assert_eq!(remaining_bsps.len(), 2, "Should have 2 BSPs after deletion");

        // Verify the correct BSP was deleted
        for bsp in &remaining_bsps {
            assert_ne!(bsp.account, "0xbsp2", "Deleted BSP should not be present");
        }
    }
}

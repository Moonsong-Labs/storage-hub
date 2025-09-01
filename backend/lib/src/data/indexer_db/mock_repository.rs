//! Mock repository implementation for unit testing.
//!
//! Provides an in-memory implementation of the repository pattern that mimics
//! database operations without requiring a real database connection.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use tokio::sync::RwLock;

use shc_indexer_db::models::{Bsp, Bucket, File, Msp};

use crate::data::indexer_db::repository::{
    error::{RepositoryError, RepositoryResult},
    BucketId, IndexerOps, IndexerOpsMut, ProviderId,
};

/// Mock repository implementation using in-memory storage
// TODO: add failure-injection mechanism (similar to RPC mocks)
pub struct MockRepository {
    bsps: Arc<RwLock<HashMap<i64, Bsp>>>,
    msps: Arc<RwLock<HashMap<i64, Msp>>>,
    buckets: Arc<RwLock<HashMap<i64, Bucket>>>,
    /// Files stored in BTreeMap to maintain natural ordering by ID
    files: Arc<RwLock<BTreeMap<i64, File>>>,
    next_id: Arc<AtomicI64>,
}

impl MockRepository {
    /// Create a new mock repository
    pub fn new() -> Self {
        Self {
            bsps: Arc::new(RwLock::new(HashMap::new())),
            msps: Arc::new(RwLock::new(HashMap::new())),
            buckets: Arc::new(RwLock::new(HashMap::new())),
            files: Arc::new(RwLock::new(BTreeMap::new())),
            next_id: Arc::new(AtomicI64::new(1)),
        }
    }

    /// Generate next unique ID
    pub fn next_id(&self) -> i64 {
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

    // ============ MSP Read Operations ============
    async fn get_msp_by_onchain_id(&self, msp: ProviderId<'_>) -> RepositoryResult<Msp> {
        let msps = self.msps.read().await;
        msps.values()
            .find(|m| m.onchain_msp_id == msp.0)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("MSP"))
    }

    async fn list_user_buckets_by_msp(
        &self,
        msp: i64,
        account: &str,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<Bucket>> {
        todo!()
    }

    // ============ Bucket Read Operations ============
    async fn get_bucket_by_onchain_id(&self, bid: BucketId<'_>) -> RepositoryResult<Bucket> {
        let buckets = self.buckets.read().await;
        buckets
            .values()
            .find(|b| b.onchain_bucket_id == bid.0)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("Bucket"))
    }

    async fn get_files_by_bucket(
        &self,
        bucket: i64,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<File>> {
        let files = self.files.read().await;

        Ok(files
            .values()
            .filter(|f| f.bucket_id == bucket)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl IndexerOpsMut for MockRepository {
    // ============ BSP Write Operations ============
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
}

#[cfg(test)]
pub mod tests {
    use bigdecimal::{BigDecimal, FromPrimitive};
    use chrono::Utc;

    use super::*;
    use crate::constants::rpc::DUMMY_MSP_ID;
    use crate::constants::test::{accounts::*, bsp, bucket, file, merkle::*, msp};

    pub async fn inject_sample_bsp(repo: &MockRepository) -> i64 {
        let id = repo.next_id();
        let now = Utc::now().naive_utc();

        // fixture
        repo.bsps.write().await.insert(
            id,
            Bsp {
                id,
                account: TEST_BSP_ACCOUNT_STR.to_string(),
                capacity: BigDecimal::from_i64(bsp::DEFAULT_CAPACITY).unwrap(),
                stake: BigDecimal::from_i64(bsp::DEFAULT_STAKE).unwrap(),
                last_tick_proven: 0,
                created_at: now,
                updated_at: now,
                onchain_bsp_id: bsp::DEFAULT_BSP_ID.to_string(),
                merkle_root: BSP_MERKLE_ROOT.to_vec(),
            },
        );

        id
    }

    pub async fn inject_sample_msp(repo: &MockRepository) -> i64 {
        let id = repo.next_id();
        let now = Utc::now().naive_utc();

        // fixture
        repo.msps.write().await.insert(
            id,
            Msp {
                id,
                account: TEST_MSP_ACCOUNT_STR.to_string(),
                capacity: BigDecimal::from_i64(msp::DEFAULT_CAPACITY).unwrap(),
                value_prop: msp::DEFAULT_VALUE_PROP.to_string(),
                created_at: now,
                updated_at: now,
                onchain_msp_id: DUMMY_MSP_ID.to_string(),
            },
        );

        id
    }

    pub async fn inject_sample_bucket(repo: &MockRepository, msp_id: Option<i64>) -> i64 {
        let id = repo.next_id();
        let now = Utc::now().naive_utc();

        repo.buckets.write().await.insert(
            id,
            Bucket {
                id,
                msp_id,
                account: TEST_BSP_ACCOUNT_STR.to_string(),
                onchain_bucket_id: bucket::DEFAULT_BUCKET_ID.as_bytes().to_vec(),
                name: bucket::DEFAULT_BUCKET_NAME.as_bytes().to_vec(),
                collection_id: None,
                private: !bucket::DEFAULT_IS_PUBLIC,
                merkle_root: vec![],
                created_at: now,
                updated_at: now,
            },
        );

        id
    }

    pub async fn inject_sample_file(
        repo: &MockRepository,
        bucket_id: i64,
        file_key: Option<&str>,
    ) -> i64 {
        let id = repo.next_id();
        let now = Utc::now().naive_utc();
        let key = file_key.unwrap_or(file::DEFAULT_FILE_KEY);

        repo.files.write().await.insert(
            id,
            File {
                id,
                account: TEST_BSP_ACCOUNT_STR.as_bytes().to_vec(),
                file_key: key.as_bytes().to_vec(),
                bucket_id,
                location: file::DEFAULT_LOCATION.as_bytes().to_vec(),
                fingerprint: file::DEFAULT_FINGERPRINT.to_vec(),
                size: file::DEFAULT_SIZE,
                step: file::DEFAULT_STEP,
                deletion_status: None,
                created_at: now,
                updated_at: now,
            },
        );

        id
    }

    #[tokio::test]
    async fn test_mock_repo_read() {
        let repo = MockRepository::new();
        let id = inject_sample_bsp(&repo).await;

        let bsps = repo.list_bsps(1, 0).await.expect("able to list bsps");
        let bsp = &bsps[0];

        assert_eq!(bsps.len(), 1);
        assert_eq!(bsp.id, id);
    }

    #[tokio::test]
    async fn test_mock_repo_write() {
        let repo = MockRepository::new();
        _ = inject_sample_bsp(&repo).await;

        let bsps = repo.list_bsps(1, 0).await.expect("able to list bsps");
        let bsp = &bsps[0];

        // Delete BSP
        repo.delete_bsp(&bsp.account).await.unwrap();

        let found = repo.list_bsps(1, 0).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn test_get_msp_by_onchain_id() {
        let repo = MockRepository::new();
        let id = inject_sample_msp(&repo).await;

        // Test successful retrieval
        let msp = repo
            .get_msp_by_onchain_id(ProviderId(DUMMY_MSP_ID))
            .await
            .expect("should find MSP by onchain ID");

        assert_eq!(msp.id, id);
        assert_eq!(msp.onchain_msp_id, DUMMY_MSP_ID);
        assert_eq!(msp.account, TEST_MSP_ACCOUNT_STR);
        assert_eq!(msp.value_prop, msp::DEFAULT_VALUE_PROP);

        // Test not found case
        let result = repo
            .get_msp_by_onchain_id(ProviderId("0xnonexistent"))
            .await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, RepositoryError::NotFound { entity: _ }));
        }
    }

    #[tokio::test]
    async fn test_get_files_by_bucket_filters_correctly() {
        let repo = MockRepository::new();

        // Create a bucket with files
        let bucket_id = inject_sample_bucket(&repo, None).await;
        let file1_id = inject_sample_file(&repo, bucket_id, Some("file1.txt")).await;
        let file2_id = inject_sample_file(&repo, bucket_id, Some("file2.txt")).await;
        let file3_id = inject_sample_file(&repo, bucket_id, Some("file3.txt")).await;

        // Create another bucket with a file
        let other_bucket_id = inject_sample_bucket(&repo, None).await;
        let _other_file_id = inject_sample_file(&repo, other_bucket_id, Some("other.txt")).await;

        // Retrieve files from the first bucket only
        let files = repo
            .get_files_by_bucket(bucket_id, 10, 0)
            .await
            .expect("should retrieve files by bucket");

        assert_eq!(files.len(), 3);
        assert_eq!(files[0].id, file1_id);
        assert_eq!(files[1].id, file2_id);
        assert_eq!(files[2].id, file3_id);
        assert_eq!(files[0].file_key, b"file1.txt");
        assert_eq!(files[1].file_key, b"file2.txt");
        assert_eq!(files[2].file_key, b"file3.txt");

        // Verify the other bucket's file is not included
        for file in &files {
            assert_eq!(file.bucket_id, bucket_id);
        }
    }

    #[tokio::test]
    async fn test_get_files_by_bucket_pagination() {
        let repo = MockRepository::new();

        let bucket_id = inject_sample_bucket(&repo, None).await;
        let file1_id = inject_sample_file(&repo, bucket_id, Some("file1.txt")).await;
        let file2_id = inject_sample_file(&repo, bucket_id, Some("file2.txt")).await;
        let file3_id = inject_sample_file(&repo, bucket_id, Some("file3.txt")).await;

        // Test limit
        let limited_files = repo
            .get_files_by_bucket(bucket_id, 2, 0)
            .await
            .expect("should retrieve limited files");

        assert_eq!(limited_files.len(), 2);
        assert_eq!(limited_files[0].id, file1_id);
        assert_eq!(limited_files[1].id, file2_id);

        // Test offset
        let offset_files = repo
            .get_files_by_bucket(bucket_id, 10, 1)
            .await
            .expect("should retrieve files with offset");

        assert_eq!(offset_files.len(), 2);
        assert_eq!(offset_files[0].id, file2_id);
        assert_eq!(offset_files[1].id, file3_id);

        // Test limit and offset combined
        let paginated_files = repo
            .get_files_by_bucket(bucket_id, 1, 1)
            .await
            .expect("should retrieve paginated files");

        assert_eq!(paginated_files.len(), 1);
        assert_eq!(paginated_files[0].id, file2_id);
    }

    #[tokio::test]
    async fn test_get_files_by_bucket_empty_bucket() {
        let repo = MockRepository::new();

        let empty_bucket_id = inject_sample_bucket(&repo, None).await;

        let empty_files = repo
            .get_files_by_bucket(empty_bucket_id, 10, 0)
            .await
            .expect("should handle empty bucket");

        assert!(empty_files.is_empty());
    }

    #[tokio::test]
    async fn test_get_files_by_bucket_nonexistent_bucket() {
        let repo = MockRepository::new();

        // Use a bucket ID that doesn't exist
        let non_existent_files = repo
            .get_files_by_bucket(999999, 10, 0)
            .await
            .expect("should handle non-existent bucket");

        assert!(non_existent_files.is_empty());
    }

    #[tokio::test]
    async fn test_get_bucket_by_onchain_id() {
        let repo = MockRepository::new();
        let bucket_id = inject_sample_bucket(&repo, Some(1)).await;

        let bucket = repo
            .get_bucket_by_onchain_id(BucketId(bucket::DEFAULT_BUCKET_ID.as_bytes()))
            .await
            .expect("should find bucket by onchain ID");

        assert_eq!(bucket.id, bucket_id);
        assert_eq!(
            bucket.onchain_bucket_id,
            bucket::DEFAULT_BUCKET_ID.as_bytes()
        );
    }

    #[tokio::test]
    async fn test_get_bucket_by_onchain_id_not_found() {
        let repo = MockRepository::new();
        inject_sample_bucket(&repo, None).await;

        let result = repo
            .get_bucket_by_onchain_id(BucketId(b"nonexistent_bucket_id"))
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepositoryError::NotFound(_)));
    }
}

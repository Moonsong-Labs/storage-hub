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

use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    OnchainBspId, OnchainMspId,
};

use crate::data::indexer_db::repository::{
    error::{RepositoryError, RepositoryResult},
    BucketId, FileKey, IndexerOps, IndexerOpsMut,
};

/// Mock repository implementation using in-memory storage
// TODO: add failure-injection mechanism (similar to RPC mocks)
pub struct MockRepository {
    bsps: Arc<RwLock<HashMap<i64, Bsp>>>,
    msps: Arc<RwLock<HashMap<i64, Msp>>>,
    /// Buckets stored in BTreeMap to maintain natural ordering by ID
    buckets: Arc<RwLock<BTreeMap<i64, Bucket>>>,
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
            buckets: Arc::new(RwLock::new(BTreeMap::new())),
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
    async fn get_msp_by_onchain_id(&self, msp: &OnchainMspId) -> RepositoryResult<Msp> {
        let msps = self.msps.read().await;
        msps.values()
            .find(|m| &m.onchain_msp_id == msp)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("MSP"))
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

    async fn get_buckets_by_user_and_msp(
        &self,
        msp: i64,
        account: &str,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<Bucket>> {
        let buckets = self.buckets.read().await;

        Ok(buckets
            .values()
            .filter(|b| b.msp_id == Some(msp) && b.account == account)
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
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

    // ============ File Read Operations ============
    async fn get_file_by_file_key(&self, key: FileKey<'_>) -> RepositoryResult<File> {
        let files = self.files.read().await;

        files
            .values()
            .find(|f| f.file_key.as_slice() == key.0)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("File"))
    }
}

#[async_trait]
impl IndexerOpsMut for MockRepository {
    // ============ BSP Write Operations ============
    async fn delete_bsp(&self, account: &OnchainBspId) -> RepositoryResult<()> {
        let mut bsps = self.bsps.write().await;
        let id_to_remove = bsps
            .values()
            .find(|b| &b.onchain_bsp_id == account)
            .map(|b| b.id);

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

    use shp_types::Hash;

    use super::*;
    use crate::constants::{
        rpc::DUMMY_MSP_ID,
        test::{accounts::*, bsp, bucket, file, merkle::*, msp},
    };

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
                onchain_bsp_id: bsp::DEFAULT_BSP_ID.clone(),
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
                onchain_msp_id: OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            },
        );

        id
    }

    pub async fn inject_sample_bucket(repo: &MockRepository, msp_id: Option<i64>) -> i64 {
        inject_bucket_with_account(repo, msp_id, TEST_BSP_ACCOUNT_STR, None).await
    }

    pub async fn inject_bucket_with_account(
        repo: &MockRepository,
        msp_id: Option<i64>,
        account: &str,
        name: Option<&str>,
    ) -> i64 {
        let id = repo.next_id();
        let now = Utc::now().naive_utc();
        let bucket_name = name.unwrap_or(bucket::DEFAULT_BUCKET_NAME);

        repo.buckets.write().await.insert(
            id,
            Bucket {
                id,
                msp_id,
                account: account.to_string(),
                onchain_bucket_id: bucket::DEFAULT_BUCKET_ID.as_bytes().to_vec(),
                name: bucket_name.as_bytes().to_vec(),
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
                onchain_bucket_id: bucket::DEFAULT_BUCKET_ID.as_bytes().to_vec(),
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
    async fn mock_repo_read() {
        let repo = MockRepository::new();
        let id = inject_sample_bsp(&repo).await;

        let bsps = repo.list_bsps(1, 0).await.expect("able to list bsps");
        let bsp = &bsps[0];

        assert_eq!(bsps.len(), 1);
        assert_eq!(bsp.id, id);
    }

    #[tokio::test]
    async fn mock_repo_write() {
        let repo = MockRepository::new();
        _ = inject_sample_bsp(&repo).await;

        let bsps = repo.list_bsps(1, 0).await.expect("able to list bsps");
        let bsp = &bsps[0];

        // Delete BSP
        repo.delete_bsp(&bsp.onchain_bsp_id).await.unwrap();

        let found = repo.list_bsps(1, 0).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn get_msp_by_onchain_id() {
        let repo = MockRepository::new();
        let id = inject_sample_msp(&repo).await;

        // Test successful retrieval
        let msp = repo
            .get_msp_by_onchain_id(&OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)))
            .await
            .expect("should find MSP by onchain ID");

        assert_eq!(msp.id, id);
        assert_eq!(msp.onchain_msp_id.as_bytes(), &DUMMY_MSP_ID);

        // Test not found case
        let result = repo
            .get_msp_by_onchain_id(&OnchainMspId::new(Hash::zero()))
            .await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, RepositoryError::NotFound { entity: _ }));
        }
    }

    #[tokio::test]
    async fn get_bucket_by_onchain_id() {
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
    async fn get_bucket_by_onchain_id_not_found() {
        let repo = MockRepository::new();
        inject_sample_bucket(&repo, None).await;

        let result = repo
            .get_bucket_by_onchain_id(BucketId(b"nonexistent_bucket_id"))
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn get_files_by_bucket_filters_correctly() {
        let repo = MockRepository::new();

        // Create a bucket with files
        let bucket_id = inject_sample_bucket(&repo, None).await;
        let _file1_id = inject_sample_file(&repo, bucket_id, Some("file1.txt")).await;
        let _file2_id = inject_sample_file(&repo, bucket_id, Some("file2.txt")).await;
        let _file3_id = inject_sample_file(&repo, bucket_id, Some("file3.txt")).await;

        // Create another bucket with a file
        let other_bucket_id = inject_sample_bucket(&repo, None).await;
        let _other_file_id = inject_sample_file(&repo, other_bucket_id, Some("other.txt")).await;

        // Retrieve files from the first bucket only
        let files = repo
            .get_files_by_bucket(bucket_id, 10, 0)
            .await
            .expect("should retrieve files by bucket");

        assert_eq!(files.len(), 3);

        // Verify the other bucket's file is not included
        for file in &files {
            assert_eq!(file.bucket_id, bucket_id);
        }
    }

    #[tokio::test]
    async fn get_files_by_bucket_pagination() {
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
    async fn get_files_by_bucket_empty_bucket() {
        let repo = MockRepository::new();

        let empty_bucket_id = inject_sample_bucket(&repo, None).await;

        let empty_files = repo
            .get_files_by_bucket(empty_bucket_id, 10, 0)
            .await
            .expect("should handle empty bucket");

        assert!(empty_files.is_empty());
    }

    #[tokio::test]
    async fn get_files_by_bucket_nonexistent_bucket() {
        let repo = MockRepository::new();

        // Use a bucket ID that doesn't exist
        let non_existent_files = repo
            .get_files_by_bucket(999999, 10, 0)
            .await
            .expect("should handle non-existent bucket");

        assert!(non_existent_files.is_empty());
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp() {
        let repo = MockRepository::new();
        let msp_id = inject_sample_msp(&repo).await;
        let user_account = "test_user";

        // Create buckets for the same user with the same MSP
        let bucket1_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("bucket1")).await;
        let bucket2_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("bucket2")).await;
        let bucket3_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("bucket3")).await;

        // Test fetching user buckets by MSP
        let buckets = repo
            .get_buckets_by_user_and_msp(msp_id, user_account, 10, 0)
            .await
            .expect("should list user buckets by MSP");

        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].id, bucket1_id);
        assert_eq!(buckets[1].id, bucket2_id);
        assert_eq!(buckets[2].id, bucket3_id);

        // Verify all buckets belong to the correct user and MSP
        for bucket in &buckets {
            assert_eq!(bucket.account, user_account);
            assert_eq!(bucket.msp_id, Some(msp_id));
        }
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_filters_other_users() {
        let repo = MockRepository::new();
        let msp_id = inject_sample_msp(&repo).await;
        let user_account = "test_user";

        // Create bucket for target user
        let user_bucket_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("user_bucket"))
                .await;

        // Create buckets for different users with the same MSP
        let _other_user1 =
            inject_bucket_with_account(&repo, Some(msp_id), "other_user1", Some("other1")).await;
        let _other_user2 =
            inject_bucket_with_account(&repo, Some(msp_id), "other_user2", Some("other2")).await;

        // Should only return the target user's bucket
        let buckets = repo
            .get_buckets_by_user_and_msp(msp_id, user_account, 10, 0)
            .await
            .expect("should filter by user");

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].id, user_bucket_id);
        assert_eq!(buckets[0].account, user_account);
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_filters_other_msps() {
        let repo = MockRepository::new();
        let msp1_id = inject_sample_msp(&repo).await;
        let msp2_id = inject_sample_msp(&repo).await;
        let user_account = "test_user";

        // Create buckets for the same user with different MSPs
        let msp1_bucket_id =
            inject_bucket_with_account(&repo, Some(msp1_id), user_account, Some("msp1_bucket"))
                .await;
        let _msp2_bucket =
            inject_bucket_with_account(&repo, Some(msp2_id), user_account, Some("msp2_bucket"))
                .await;

        // Should only return buckets for MSP1
        let buckets = repo
            .get_buckets_by_user_and_msp(msp1_id, user_account, 10, 0)
            .await
            .expect("should filter by MSP");

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].id, msp1_bucket_id);
        assert_eq!(buckets[0].msp_id, Some(msp1_id));
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_filters_no_msp() {
        let repo = MockRepository::new();
        let msp_id = inject_sample_msp(&repo).await;
        let user_account = "test_user";

        // Create bucket with MSP
        let msp_bucket_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("with_msp")).await;

        // Create bucket without MSP (None)
        let _no_msp_bucket =
            inject_bucket_with_account(&repo, None, user_account, Some("no_msp")).await;

        // Should only return bucket with the specified MSP
        let buckets = repo
            .get_buckets_by_user_and_msp(msp_id, user_account, 10, 0)
            .await
            .expect("should filter out buckets without MSP");

        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].id, msp_bucket_id);
        assert_eq!(buckets[0].msp_id, Some(msp_id));
    }

    #[tokio::test]
    async fn get_buckets_by_user_and_msp_pagination() {
        let repo = MockRepository::new();
        let msp_id = inject_sample_msp(&repo).await;
        let user_account = "test_user";

        // Create multiple buckets
        let bucket1_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("bucket1")).await;
        let bucket2_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("bucket2")).await;
        let bucket3_id =
            inject_bucket_with_account(&repo, Some(msp_id), user_account, Some("bucket3")).await;

        // Test limit
        let limited_buckets = repo
            .get_buckets_by_user_and_msp(msp_id, user_account, 2, 0)
            .await
            .expect("should retrieve limited buckets");

        assert_eq!(limited_buckets.len(), 2);
        assert_eq!(limited_buckets[0].id, bucket1_id);
        assert_eq!(limited_buckets[1].id, bucket2_id);

        // Test offset
        let offset_buckets = repo
            .get_buckets_by_user_and_msp(msp_id, user_account, 10, 1)
            .await
            .expect("should retrieve buckets with offset");

        assert_eq!(offset_buckets.len(), 2);
        assert_eq!(offset_buckets[0].id, bucket2_id);
        assert_eq!(offset_buckets[1].id, bucket3_id);

        // Test limit and offset combined
        let paginated_buckets = repo
            .get_buckets_by_user_and_msp(msp_id, user_account, 1, 1)
            .await
            .expect("should retrieve paginated buckets");

        assert_eq!(paginated_buckets.len(), 1);
        assert_eq!(paginated_buckets[0].id, bucket2_id);
    }

    #[tokio::test]
    async fn get_file_by_file_key() {
        let repo = MockRepository::new();
        let bucket_id = inject_sample_bucket(&repo, None).await;
        let file_key = "test_file.txt";
        let file_id = inject_sample_file(&repo, bucket_id, Some(file_key)).await;

        let file = repo
            .get_file_by_file_key(file_key.as_bytes().into())
            .await
            .expect("should find file by file key");

        assert_eq!(file.id, file_id);
        assert_eq!(file.file_key, file_key.as_bytes());
        assert_eq!(file.bucket_id, bucket_id);
    }

    #[tokio::test]
    async fn get_file_by_file_key_not_found() {
        let repo = MockRepository::new();
        let bucket_id = inject_sample_bucket(&repo, None).await;
        inject_sample_file(&repo, bucket_id, Some("existing_file.txt")).await;

        let result = repo
            .get_file_by_file_key(b"nonexistent_file.txt".as_slice().into())
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::NotFound { .. }
        ));
    }
}

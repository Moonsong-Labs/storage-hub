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
use bigdecimal::BigDecimal;
use chrono::Utc;
use hex_literal::hex;
use tokio::sync::RwLock;

use shc_indexer_db::{
    models::{Bsp, Bucket, File, Msp},
    OnchainBspId, OnchainMspId,
};
use shp_types::Hash;

use crate::{
    constants::{mocks::MOCK_ADDRESS, rpc::DUMMY_MSP_ID, test},
    data::indexer_db::repository::{
        error::{RepositoryError, RepositoryResult},
        IndexerOps, IndexerOpsMut, PaymentStreamData, PaymentStreamKind,
    },
    test_utils::{random_bytes_32, random_hash},
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
    /// Payment streams stored by ID with (user_account, PaymentStreamData) tuple
    payment_streams: Arc<RwLock<HashMap<i64, (String, PaymentStreamData)>>>,
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
            payment_streams: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicI64::new(1)),
        }
    }

    /// Create a new mock repository with some sample data loaded in
    // TODO: take advantage of this in the tests below instead of doing separate (duplicated) setups
    // see: https://github.com/Moonsong-Labs/storage-hub/pull/459/files#r2369522861
    pub async fn sample() -> Self {
        let this = Self::new();

        // Create 3 MSPs
        // MSP 1: The main MSP with DUMMY_MSP_ID
        let msp1 = this
            .create_msp(
                &hex::encode(random_bytes_32()),
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP 1");

        // MSP 2: Another MSP
        let msp2 = this
            .create_msp(
                &hex::encode(random_bytes_32()),
                OnchainMspId::new(random_hash()),
            )
            .await
            .expect("should create MSP 2");

        // MSP 3: Third MSP
        let msp3 = this
            .create_msp(
                &hex::encode(random_bytes_32()),
                OnchainMspId::new(random_hash()),
            )
            .await
            .expect("should create MSP 3");

        // Create 3 buckets, one per MSP
        // Bucket 1: For MOCK_ADDRESS and DUMMY_MSP_ID, as expected by SDK tests
        // same hash as what the SDK test excepts
        let bucket1_hash = Hash::from_slice(&hex!(
            "d8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692"
        ));
        let bucket1 = this
            .create_bucket(
                MOCK_ADDRESS,
                Some(msp1.id),
                b"Documents",
                &bucket1_hash,
                true, // private
            )
            .await
            .expect("should create bucket 1");

        // Bucket 2: For MSP 2
        let bucket2_user = random_bytes_32();
        let bucket2_hash = random_hash();
        let bucket2 = this
            .create_bucket(
                &hex::encode(bucket2_user),
                Some(msp2.id),
                b"Photos",
                &bucket2_hash,
                false, // public
            )
            .await
            .expect("should create bucket 2");

        // Bucket 3: For MSP 3
        let bucket3_user = random_bytes_32();
        let bucket3_hash = random_hash();
        let bucket3 = this
            .create_bucket(
                &hex::encode(bucket3_user),
                Some(msp3.id),
                b"Projects",
                &bucket3_hash,
                true, // private
            )
            .await
            .expect("should create bucket 3");

        // Create 3 files, one per bucket
        // but bucket 1 should have 2 files

        // File 1: /Reports/Q1-2024.pdf
        // same hash as what the SDK test excepts
        let bucket1_file1_key = Hash::from_slice(&hex!(
            "e901c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f7"
        ));
        this.create_file(
            MOCK_ADDRESS.as_bytes(),
            &bucket1_file1_key,
            bucket1.id,
            &bucket1_hash,
            b"/Reports/Q1-2024.pdf", // expected by the SDK tests
            &random_bytes_32(),
            1048576, // 1MB
        )
        .await
        .expect("should create file 1");

        // File 2: /Thesis/chapter1.pdf
        let bucket1_file2_key = random_hash();
        this.create_file(
            MOCK_ADDRESS.as_bytes(),
            &bucket1_file2_key,
            bucket1.id,
            &bucket1_hash,
            b"/Thesis/chapter1.pdf", // expected by the SDK tests
            &random_bytes_32(),
            1048576, // 1MB
        )
        .await
        .expect("should create file 1");

        // File 2: In bucket 2
        let file2_key = random_hash();
        this.create_file(
            &bucket2_user,
            &file2_key,
            bucket2.id,
            &bucket2_hash,
            b"vacation/beach.jpg",
            &random_bytes_32(),
            5242880, // 5MB
        )
        .await
        .expect("should create file 2");

        // File 3: In bucket 3
        let file3_key = random_hash();
        this.create_file(
            &bucket3_user,
            &file3_key,
            bucket3.id,
            &bucket3_hash,
            b"code/src/main.rs",
            &random_bytes_32(),
            65536, // 64KB
        )
        .await
        .expect("should create file 3");

        // Create some sample payment streams
        this.create_payment_stream(
            MOCK_ADDRESS,
            &hex::encode(DUMMY_MSP_ID),
            BigDecimal::from(500000),
            PaymentStreamKind::Fixed {
                rate: BigDecimal::from(5),
            },
        )
        .await
        .expect("should create fixed payment stream");

        this.create_payment_stream(
            MOCK_ADDRESS,
            &hex::encode(random_bytes_32()),
            BigDecimal::from(200000),
            PaymentStreamKind::Dynamic {
                amount_provided: BigDecimal::from(10),
            },
        )
        .await
        .expect("should create dynamic payment stream");

        this
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
    async fn get_msp_by_onchain_id(&self, onchain_msp_id: &OnchainMspId) -> RepositoryResult<Msp> {
        let msps = self.msps.read().await;
        msps.values()
            .find(|m| &m.onchain_msp_id == onchain_msp_id)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("MSP"))
    }

    // ============ Bucket Read Operations ============
    async fn get_bucket_by_onchain_id(&self, onchain_bucket_id: &Hash) -> RepositoryResult<Bucket> {
        let buckets = self.buckets.read().await;
        buckets
            .values()
            .find(|b| b.onchain_bucket_id == onchain_bucket_id.as_bytes())
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
    async fn get_file_by_file_key(&self, file_key: &Hash) -> RepositoryResult<File> {
        let files = self.files.read().await;

        files
            .values()
            .find(|f| f.file_key.as_slice() == file_key.as_bytes())
            .cloned()
            .ok_or_else(|| RepositoryError::not_found("File"))
    }

    // ============ Payment Stream Operations ============
    async fn get_payment_streams_for_user(
        &self,
        user_account: &str,
    ) -> RepositoryResult<Vec<PaymentStreamData>> {
        let streams = self.payment_streams.read().await;

        Ok(streams
            .values()
            .filter(|(account, _)| account == user_account)
            .map(|(_, data)| data.clone())
            .collect())
    }
}

#[async_trait]
impl IndexerOpsMut for MockRepository {
    // ============ MSP Write Operations ============
    async fn create_msp(
        &self,
        account: &str,
        onchain_msp_id: OnchainMspId,
    ) -> RepositoryResult<Msp> {
        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let msp = Msp {
            id,
            account: account.to_string(),
            capacity: BigDecimal::from(test::msp::DEFAULT_CAPACITY),
            value_prop: test::msp::DEFAULT_VALUE_PROP.to_string(),
            created_at: now,
            updated_at: now,
            onchain_msp_id,
        };

        self.msps.write().await.insert(id, msp.clone());
        Ok(msp)
    }

    async fn delete_msp(&self, onchain_msp_id: &OnchainMspId) -> RepositoryResult<()> {
        let mut msps = self.msps.write().await;
        let id_to_remove = msps
            .values()
            .find(|m| &m.onchain_msp_id == onchain_msp_id)
            .map(|m| m.id);

        if let Some(id) = id_to_remove {
            msps.remove(&id);
            Ok(())
        } else {
            Err(RepositoryError::not_found("MSP"))
        }
    }

    // ============ BSP Write Operations ============
    async fn create_bsp(
        &self,
        account: &str,
        onchain_bsp_id: OnchainBspId,
        capacity: BigDecimal,
        stake: BigDecimal,
    ) -> RepositoryResult<Bsp> {
        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let bsp = Bsp {
            id,
            account: account.to_string(),
            capacity,
            stake,
            last_tick_proven: test::bsp::DEFAULT_LAST_TICK_PROVEN,
            created_at: now,
            updated_at: now,
            onchain_bsp_id,
            merkle_root: test::bsp::DEFAULT_MERKLE_ROOT.to_vec(),
        };

        self.bsps.write().await.insert(id, bsp.clone());
        Ok(bsp)
    }

    async fn delete_bsp(&self, onchain_bsp_id: &OnchainBspId) -> RepositoryResult<()> {
        let mut bsps = self.bsps.write().await;
        let id_to_remove = bsps
            .values()
            .find(|b| &b.onchain_bsp_id == onchain_bsp_id)
            .map(|b| b.id);

        if let Some(id) = id_to_remove {
            bsps.remove(&id);
            Ok(())
        } else {
            Err(RepositoryError::not_found("BSP"))
        }
    }

    // ============ Bucket Write Operations ============
    async fn create_bucket(
        &self,
        account: &str,
        msp_id: Option<i64>,
        name: &[u8],
        onchain_bucket_id: &Hash,
        private: bool,
    ) -> RepositoryResult<Bucket> {
        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let bucket = Bucket {
            id,
            msp_id,
            account: account.to_string(),
            onchain_bucket_id: onchain_bucket_id.as_bytes().to_vec(),
            name: name.to_vec(),
            collection_id: None,
            private,
            merkle_root: test::bucket::DEFAULT_MERKLE_ROOT.to_vec(),
            created_at: now,
            updated_at: now,
        };

        self.buckets.write().await.insert(id, bucket.clone());
        Ok(bucket)
    }

    async fn delete_bucket(&self, onchain_bucket_id: &Hash) -> RepositoryResult<()> {
        let mut buckets = self.buckets.write().await;
        let id_to_remove = buckets
            .values()
            .find(|b| b.onchain_bucket_id == onchain_bucket_id.as_bytes())
            .map(|b| b.id);

        if let Some(id) = id_to_remove {
            buckets.remove(&id);
            Ok(())
        } else {
            Err(RepositoryError::not_found("Bucket"))
        }
    }

    // ============ File Write Operations ============
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
        let id = self.next_id();
        let now = Utc::now().naive_utc();

        let file = File {
            id,
            account: account.to_vec(),
            file_key: file_key.as_bytes().to_vec(),
            bucket_id,
            onchain_bucket_id: onchain_bucket_id.as_bytes().to_vec(),
            location: location.to_vec(),
            fingerprint: fingerprint.to_vec(),
            size,
            step: test::file::DEFAULT_STEP,
            deletion_status: None,
            created_at: now,
            updated_at: now,
        };

        self.files.write().await.insert(id, file.clone());
        Ok(file)
    }

    async fn delete_file(&self, file_key: &Hash) -> RepositoryResult<()> {
        let mut files = self.files.write().await;
        let id_to_remove = files
            .values()
            .find(|f| f.file_key == file_key.as_bytes())
            .map(|f| f.id);

        if let Some(id) = id_to_remove {
            files.remove(&id);
            Ok(())
        } else {
            Err(RepositoryError::not_found("File"))
        }
    }

    // ============ Payment Stream Write Operations ============
    async fn create_payment_stream(
        &self,
        user_account: &str,
        provider: &str,
        total_amount_paid: BigDecimal,
        kind: PaymentStreamKind,
    ) -> RepositoryResult<PaymentStreamData> {
        let id = self.next_id();

        let stream_data = PaymentStreamData {
            provider: provider.to_string(),
            total_amount_paid,
            kind,
        };

        self.payment_streams
            .write()
            .await
            .insert(id, (user_account.to_string(), stream_data.clone()));

        Ok(stream_data)
    }
}

#[cfg(test)]
pub mod tests {
    use bigdecimal::FromPrimitive;
    use shp_types::Hash;

    use super::*;
    use crate::{
        constants::{
            rpc::DUMMY_MSP_ID,
            test::{accounts::*, bucket, file},
        },
        test_utils::random_hash,
    };

    #[tokio::test]
    async fn get_msp_by_onchain_id() {
        let repo = MockRepository::new();
        let created_msp = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP");

        // Test successful retrieval
        let msp = repo
            .get_msp_by_onchain_id(&OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)))
            .await
            .expect("should find MSP by onchain ID");

        assert_eq!(msp.id, created_msp.id);
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
        let bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let created_bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                Some(1),
                bucket::DEFAULT_BUCKET_NAME.as_bytes(),
                &bucket_hash,
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket");

        let bucket = repo
            .get_bucket_by_onchain_id(&bucket_hash)
            .await
            .expect("should find bucket by onchain ID");

        assert_eq!(bucket.id, created_bucket.id);
        assert_eq!(
            bucket.onchain_bucket_id,
            bucket::DEFAULT_BUCKET_ID.as_slice()
        );
    }

    #[tokio::test]
    async fn get_bucket_by_onchain_id_not_found() {
        let repo = MockRepository::new();
        let bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        repo.create_bucket(
            TEST_BSP_ACCOUNT_STR,
            None,
            bucket::DEFAULT_BUCKET_NAME.as_bytes(),
            &bucket_hash,
            !bucket::DEFAULT_IS_PUBLIC,
        )
        .await
        .expect("should create bucket");

        let result = repo.get_bucket_by_onchain_id(&random_hash()).await;

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
        let bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                None,
                bucket::DEFAULT_BUCKET_NAME.as_bytes(),
                &bucket_hash,
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket");

        repo.create_file(
            TEST_BSP_ACCOUNT_STR.as_bytes(),
            &random_hash(),
            bucket.id,
            &bucket_hash,
            file::DEFAULT_LOCATION.as_bytes(),
            file::DEFAULT_FINGERPRINT,
            file::DEFAULT_SIZE,
        )
        .await
        .expect("should create file");

        repo.create_file(
            TEST_BSP_ACCOUNT_STR.as_bytes(),
            &random_hash(),
            bucket.id,
            &bucket_hash,
            file::DEFAULT_LOCATION.as_bytes(),
            file::DEFAULT_FINGERPRINT,
            file::DEFAULT_SIZE,
        )
        .await
        .expect("should create file");

        repo.create_file(
            TEST_BSP_ACCOUNT_STR.as_bytes(),
            &random_hash(),
            bucket.id,
            &bucket_hash,
            file::DEFAULT_LOCATION.as_bytes(),
            file::DEFAULT_FINGERPRINT,
            file::DEFAULT_SIZE,
        )
        .await
        .expect("should create file");

        // Create another bucket with a file
        let other_bucket_hash = random_hash();
        let other_bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                None,
                "other_bucket".as_bytes(),
                &other_bucket_hash,
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create another bucket");

        repo.create_file(
            TEST_BSP_ACCOUNT_STR.as_bytes(),
            &random_hash(),
            other_bucket.id,
            &other_bucket_hash,
            file::DEFAULT_LOCATION.as_bytes(),
            file::DEFAULT_FINGERPRINT,
            file::DEFAULT_SIZE,
        )
        .await
        .expect("should create file");

        // Retrieve files from the first bucket only
        let files = repo
            .get_files_by_bucket(bucket.id, 10, 0)
            .await
            .expect("should retrieve files by bucket");

        assert_eq!(files.len(), 3);

        // Verify the other bucket's file is not included
        for file in &files {
            assert_eq!(file.bucket_id, bucket.id);
        }
    }

    #[tokio::test]
    async fn get_files_by_bucket_pagination() {
        let repo = MockRepository::new();

        let bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                None,
                bucket::DEFAULT_BUCKET_NAME.as_bytes(),
                &bucket_hash,
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket");

        let file1_bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let file1 = repo
            .create_file(
                TEST_BSP_ACCOUNT_STR.as_bytes(),
                &random_hash(),
                bucket.id,
                &file1_bucket_hash,
                file::DEFAULT_LOCATION.as_bytes(),
                file::DEFAULT_FINGERPRINT,
                file::DEFAULT_SIZE,
            )
            .await
            .expect("should create file1");

        let file2_bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let file2 = repo
            .create_file(
                TEST_BSP_ACCOUNT_STR.as_bytes(),
                &random_hash(),
                bucket.id,
                &file2_bucket_hash,
                file::DEFAULT_LOCATION.as_bytes(),
                file::DEFAULT_FINGERPRINT,
                file::DEFAULT_SIZE,
            )
            .await
            .expect("should create file2");

        let file3_bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let file3 = repo
            .create_file(
                TEST_BSP_ACCOUNT_STR.as_bytes(),
                &random_hash(),
                bucket.id,
                &file3_bucket_hash,
                file::DEFAULT_LOCATION.as_bytes(),
                file::DEFAULT_FINGERPRINT,
                file::DEFAULT_SIZE,
            )
            .await
            .expect("should create file3");

        // Test limit
        let limited_files = repo
            .get_files_by_bucket(bucket.id, 2, 0)
            .await
            .expect("should retrieve limited files");

        assert_eq!(limited_files.len(), 2);
        assert_eq!(limited_files[0].id, file1.id);
        assert_eq!(limited_files[1].id, file2.id);

        // Test offset
        let offset_files = repo
            .get_files_by_bucket(bucket.id, 10, 1)
            .await
            .expect("should retrieve files with offset");

        assert_eq!(offset_files.len(), 2);
        assert_eq!(offset_files[0].id, file2.id);
        assert_eq!(offset_files[1].id, file3.id);

        // Test limit and offset combined
        let paginated_files = repo
            .get_files_by_bucket(bucket.id, 1, 1)
            .await
            .expect("should retrieve paginated files");

        assert_eq!(paginated_files.len(), 1);
        assert_eq!(paginated_files[0].id, file2.id);
    }

    #[tokio::test]
    async fn get_files_by_bucket_empty_bucket() {
        let repo = MockRepository::new();

        let bucket_hash = Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice());
        let empty_bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                None,
                bucket::DEFAULT_BUCKET_NAME.as_bytes(),
                &bucket_hash,
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket");

        let empty_files = repo
            .get_files_by_bucket(empty_bucket.id, 10, 0)
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
        let msp = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP");
        let msp_id = msp.id;
        let user_account = "test_user";

        // Create buckets for the same user with the same MSP
        let bucket1_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "bucket1".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;
        let bucket2_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "bucket2".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;
        let bucket3_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "bucket3".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;

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
        let msp = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP");
        let msp_id = msp.id;
        let user_account = "test_user";

        // Create bucket for target user
        let user_bucket_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "user_bucket".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;

        // Create buckets for different users with the same MSP
        repo.create_bucket(
            "other_user1",
            Some(msp_id),
            "other1".as_bytes(),
            &random_hash(),
            !bucket::DEFAULT_IS_PUBLIC,
        )
        .await
        .expect("should create bucket");

        repo.create_bucket(
            "other_user2",
            Some(msp_id),
            "other2".as_bytes(),
            &random_hash(),
            !bucket::DEFAULT_IS_PUBLIC,
        )
        .await
        .expect("should create bucket");

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
        let msp1 = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP1");
        let msp1_id = msp1.id;
        let msp2 = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP2");
        let msp2_id = msp2.id;
        let user_account = "test_user";

        // Create buckets for the same user with different MSPs
        let msp1_bucket_id = repo
            .create_bucket(
                user_account,
                Some(msp1_id),
                "msp1_bucket".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;
        let _msp2_bucket = repo
            .create_bucket(
                user_account,
                Some(msp2_id),
                "msp2_bucket".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;

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
        let msp = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP");
        let msp_id = msp.id;
        let user_account = "test_user";

        // Create bucket with MSP
        let msp_bucket_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "with_msp".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;

        // Create bucket without MSP (None)
        let _no_msp_bucket = repo
            .create_bucket(
                user_account,
                None,
                "no_msp".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;

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
        let msp = repo
            .create_msp(
                TEST_MSP_ACCOUNT_STR,
                OnchainMspId::new(Hash::from_slice(&DUMMY_MSP_ID)),
            )
            .await
            .expect("should create MSP");
        let msp_id = msp.id;
        let user_account = "test_user";

        // Create multiple buckets
        let bucket1_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "bucket1".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;
        let bucket2_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "bucket2".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;
        let bucket3_id = repo
            .create_bucket(
                user_account,
                Some(msp_id),
                "bucket3".as_bytes(),
                &random_hash(),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket")
            .id;

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
        let bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                None,
                bucket::DEFAULT_BUCKET_NAME.as_bytes(),
                &Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice()),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket");

        let file_key = random_hash();
        let created_file = repo
            .create_file(
                TEST_BSP_ACCOUNT_STR.as_bytes(),
                &file_key,
                bucket.id,
                &Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice()),
                file::DEFAULT_LOCATION.as_bytes(),
                file::DEFAULT_FINGERPRINT,
                file::DEFAULT_SIZE,
            )
            .await
            .expect("should create file");

        let file = repo
            .get_file_by_file_key(&file_key)
            .await
            .expect("should find file by file key");

        assert_eq!(file.id, created_file.id);
        assert_eq!(file.file_key, file_key.as_bytes());
        assert_eq!(file.bucket_id, bucket.id);
    }

    #[tokio::test]
    async fn get_file_by_file_key_not_found() {
        let repo = MockRepository::new();
        let bucket = repo
            .create_bucket(
                TEST_BSP_ACCOUNT_STR,
                None,
                bucket::DEFAULT_BUCKET_NAME.as_bytes(),
                &Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice()),
                !bucket::DEFAULT_IS_PUBLIC,
            )
            .await
            .expect("should create bucket");
        repo.create_file(
            TEST_BSP_ACCOUNT_STR.as_bytes(),
            &random_hash(),
            bucket.id,
            &Hash::from_slice(bucket::DEFAULT_BUCKET_ID.as_slice()),
            file::DEFAULT_LOCATION.as_bytes(),
            file::DEFAULT_FINGERPRINT,
            file::DEFAULT_SIZE,
        )
        .await
        .expect("should create file");

        let result = repo.get_file_by_file_key(&random_hash()).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RepositoryError::NotFound { .. }
        ));
    }

    #[tokio::test]
    async fn get_payment_streams_filters_by_user() {
        let repo = MockRepository::new();
        let user1 = "user_1";
        let user2 = "user_2";
        let provider = hex::encode(random_bytes_32());

        // Create payment stream for user1
        repo.create_payment_stream(
            user1,
            &provider,
            BigDecimal::from_i64(100000).unwrap(),
            PaymentStreamKind::Fixed {
                rate: BigDecimal::from_i64(3).unwrap(),
            },
        )
        .await
        .expect("should create payment stream for user1");

        // Create another payment stream for user1
        repo.create_payment_stream(
            user1,
            &hex::encode(random_bytes_32()),
            BigDecimal::from_i64(100000).unwrap(),
            PaymentStreamKind::Fixed {
                rate: BigDecimal::from_i64(3).unwrap(),
            },
        )
        .await
        .expect("should create payment stream for user1");

        // Create payment stream for user2
        repo.create_payment_stream(
            user2,
            &provider,
            BigDecimal::from_i64(200000).unwrap(),
            PaymentStreamKind::Dynamic {
                amount_provided: BigDecimal::from_i64(7).unwrap(),
            },
        )
        .await
        .expect("should create payment stream for user2");

        // Retrieve payment streams for user1
        let user1_streams = repo
            .get_payment_streams_for_user(user1)
            .await
            .expect("should retrieve user1 streams");

        // Should only get user1's stream
        assert_eq!(user1_streams.len(), 2);

        // Retrieve payment streams for user2
        let user2_streams = repo
            .get_payment_streams_for_user(user2)
            .await
            .expect("should retrieve user2 streams");

        // Should only get user2's stream
        assert_eq!(user2_streams.len(), 1);
        assert_eq!(user2_streams[0].provider, provider);
    }
}

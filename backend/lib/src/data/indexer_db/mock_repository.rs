//! Mock repository implementation for unit testing.
//!
//! Provides an in-memory implementation of the repository pattern that mimics
//! database operations without requiring a real database connection.

use std::{
    collections::HashMap,
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
    next_id: Arc<AtomicI64>,
}

impl MockRepository {
    /// Create a new mock repository
    pub fn new() -> Self {
        Self {
            bsps: Arc::new(RwLock::new(HashMap::new())),
            msps: Arc::new(RwLock::new(HashMap::new())),
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
        todo!()
    }

    async fn get_files_by_bucket(
        &self,
        bucket: i64,
        limit: i64,
        offset: i64,
    ) -> RepositoryResult<Vec<File>> {
        todo!()
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
    use crate::constants::test::{accounts::*, bsp, merkle::*, msp};

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
            assert!(matches!(e, RepositoryError::NotFound(_)));
        }
    }
}

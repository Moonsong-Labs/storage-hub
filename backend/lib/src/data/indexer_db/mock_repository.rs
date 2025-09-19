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
use bigdecimal::{BigDecimal, FromPrimitive};
use tokio::sync::RwLock;

use shc_indexer_db::{models::Bsp, OnchainBspId};

use crate::data::indexer_db::repository::{
    error::{RepositoryError, RepositoryResult},
    IndexerOps, IndexerOpsMut, PaymentStreamData, PaymentStreamKind,
};

/// Mock repository implementation using in-memory storage
// TODO: add failure-injection mechanism (similar to RPC mocks)
pub struct MockRepository {
    bsps: Arc<RwLock<HashMap<i64, Bsp>>>,
    next_id: Arc<AtomicI64>,
}

impl MockRepository {
    /// Create a new mock repository
    pub fn new() -> Self {
        Self {
            bsps: Arc::new(RwLock::new(HashMap::new())),
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

    // ============ Payment Stream Operations ============
    async fn get_payment_streams_for_user(
        &self,
        _user_account: &str,
    ) -> RepositoryResult<Vec<PaymentStreamData>> {
        // TODO(MOCK): add mechanism to create this data in the mock repository and retrieve it here
        Ok(vec![
            PaymentStreamData {
                provider: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
                total_amount_paid: BigDecimal::from_i64(500000).unwrap(),
                kind: PaymentStreamKind::Fixed {
                    rate: BigDecimal::from_i64(5).unwrap(),
                },
            },
            PaymentStreamData {
                provider: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
                total_amount_paid: BigDecimal::from_i64(200000).unwrap(),
                kind: PaymentStreamKind::Dynamic {
                    amount_provided: BigDecimal::from_i64(10).unwrap(),
                },
            },
        ])
    }

    async fn calculate_msp_storage_for_user(
        &self,
        _msp_id: i64,
        _user_account: &str,
    ) -> RepositoryResult<BigDecimal> {
        // TODO(MOCK): add mechanism to create this data in the mock repository and retrieve it here
        Ok(BigDecimal::from(30))
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
    use chrono::Utc;

    use super::*;
    use crate::constants::test::{accounts::*, bsp::*, merkle::*};

    pub async fn inject_sample_bsp(repo: &MockRepository) -> i64 {
        let id = repo.next_id();
        let now = Utc::now().naive_utc();

        // fixture
        repo.bsps.write().await.insert(
            id,
            Bsp {
                id,
                account: TEST_BSP_ACCOUNT_STR.to_string(),
                capacity: BigDecimal::from_i64(DEFAULT_CAPACITY).unwrap(),
                stake: BigDecimal::from_i64(DEFAULT_STAKE).unwrap(),
                last_tick_proven: 0,
                created_at: now,
                updated_at: now,
                onchain_bsp_id: OnchainBspId::new(DEFAULT_BSP_ID),
                merkle_root: BSP_MERKLE_ROOT.to_vec(),
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
        repo.delete_bsp(&bsp.onchain_bsp_id).await.unwrap();

        let found = repo.list_bsps(1, 0).await.unwrap();
        assert!(found.is_empty());
    }
}

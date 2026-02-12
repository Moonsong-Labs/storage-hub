//! Node health service implementation
//!
//! Checks whether the MSP node is functioning correctly by evaluating
//! three signals: indexer health, storage request acceptance, and
//! transaction nonce liveness.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

use shc_indexer_db::OnchainMspId;

use crate::{
    config::NodeHealthConfig,
    data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient},
    models::node_health::{
        IndexerSignal, NodeHealthResponse, NodeHealthSignals, RequestAcceptanceSignal,
        SignalStatus, TxNonceSignal,
    },
};

/// In-memory state for nonce tracking across health check calls
struct NonceState {
    /// The last observed on-chain nonce
    last_nonce: u64,
    /// When we first observed this nonce value
    first_seen_at: Instant,
}

/// Service for checking MSP node operational health
pub struct NodeHealthService {
    db: Arc<DBClient>,
    rpc: Arc<StorageHubRpcClient>,
    msp_id: OnchainMspId,
    config: NodeHealthConfig,
    /// Cached MSP database ID (resolved once, cached forever)
    msp_db_id: RwLock<Option<i64>>,
    /// Cached MSP account address (resolved once, cached forever)
    msp_account: RwLock<Option<String>>,
    /// In-memory nonce tracking state
    nonce_state: Arc<RwLock<NonceState>>,
}

impl NodeHealthService {
    /// Create a new node health service
    pub fn new(
        db: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        msp_id: OnchainMspId,
        config: NodeHealthConfig,
    ) -> Self {
        Self {
            db,
            rpc,
            msp_id,
            config,
            msp_db_id: RwLock::new(None),
            msp_account: RwLock::new(None),
            nonce_state: Arc::new(RwLock::new(NonceState {
                last_nonce: 0,
                first_seen_at: Instant::now(),
            })),
        }
    }

    /// Resolve and cache the MSP's database ID
    async fn get_msp_db_id(&self) -> Result<i64, String> {
        // Check cache first
        {
            let cached = self.msp_db_id.read().await;
            if let Some(id) = *cached {
                return Ok(id);
            }
        }

        // Fetch from DB
        let msp = self
            .db
            .get_msp(&self.msp_id)
            .await
            .map_err(|e| format!("Failed to get MSP from DB: {}", e))?;

        let id = msp.id;

        // Cache it
        {
            let mut cached = self.msp_db_id.write().await;
            *cached = Some(id);
        }

        Ok(id)
    }

    /// Resolve and cache the MSP's signing account
    async fn get_msp_account(&self) -> Result<String, String> {
        // Check cache first
        {
            let cached = self.msp_account.read().await;
            if let Some(ref account) = *cached {
                return Ok(account.clone());
            }
        }

        // Fetch from DB
        let msp = self
            .db
            .get_msp(&self.msp_id)
            .await
            .map_err(|e| format!("Failed to get MSP from DB: {}", e))?;

        let account = msp.account.clone();

        // Cache it
        {
            let mut cached = self.msp_account.write().await;
            *cached = Some(account.clone());
        }

        Ok(account)
    }

    /// Run all health checks and produce the response
    pub async fn check_node_health(&self) -> NodeHealthResponse {
        debug!(target: "node_health_service::check_node_health", "Node health check initiated");

        let indexer = self.check_indexer().await;
        let request_acceptance = self.check_request_acceptance(&indexer).await;
        let tx_nonce = self.check_tx_nonce().await;

        // Overall status is the worst of all signals
        let overall = indexer
            .status
            .worst(request_acceptance.status)
            .worst(tx_nonce.status);

        NodeHealthResponse {
            status: overall,
            checked_at: Utc::now().to_rfc3339(),
            signals: NodeHealthSignals {
                indexer,
                request_acceptance,
                tx_nonce,
            },
        }
    }

    /// Check indexer health signal
    async fn check_indexer(&self) -> IndexerSignal {
        // Get service state from DB
        let service_state = match self.db.get_service_state().await {
            Ok(state) => state,
            Err(e) => {
                error!(target: "node_health_service::check_indexer", error = %e, "Failed to get service state");
                return IndexerSignal {
                    status: SignalStatus::Unknown,
                    last_indexed_block: 0,
                    finalized_block: 0,
                    lag_blocks: 0,
                    last_updated_secs_ago: 0,
                    message: Some(format!("Failed to get service state: {}", e)),
                };
            }
        };

        // Get finalized block from RPC
        let finalized_block = match self.rpc.get_finalized_block_number().await {
            Ok(block) => block,
            Err(e) => {
                error!(target: "node_health_service::check_indexer", error = %e, "Failed to get finalized block number");
                return IndexerSignal {
                    status: SignalStatus::Unknown,
                    last_indexed_block: service_state.last_indexed_finalized_block,
                    finalized_block: 0,
                    lag_blocks: 0,
                    last_updated_secs_ago: 0,
                    message: Some(format!("Failed to get finalized block from RPC: {}", e)),
                };
            }
        };

        let now = Utc::now().naive_utc();
        let updated_secs_ago = (now - service_state.updated_at).num_seconds().max(0) as u64;

        let last_indexed = service_state.last_indexed_finalized_block;
        let lag_blocks = if finalized_block > last_indexed as u64 {
            finalized_block - last_indexed as u64
        } else {
            0
        };

        let is_stale = updated_secs_ago >= self.config.indexer_stale_threshold_secs;
        let is_lagging = lag_blocks >= self.config.indexer_lag_blocks_threshold;

        let status = if is_stale {
            SignalStatus::Unhealthy
        } else if is_lagging {
            SignalStatus::Degraded
        } else {
            SignalStatus::Healthy
        };

        let message = match status {
            SignalStatus::Unhealthy => Some(format!(
                "Indexer stuck: last updated {}s ago (threshold: {}s)",
                updated_secs_ago, self.config.indexer_stale_threshold_secs
            )),
            SignalStatus::Degraded => Some(format!(
                "Indexer lagging: {} blocks behind (threshold: {})",
                lag_blocks, self.config.indexer_lag_blocks_threshold
            )),
            _ => None,
        };

        IndexerSignal {
            status,
            last_indexed_block: last_indexed,
            finalized_block,
            lag_blocks,
            last_updated_secs_ago: updated_secs_ago,
            message,
        }
    }

    /// Check storage request acceptance signal
    ///
    /// If the indexer is unhealthy, mark this signal as unknown (can't trust stale data).
    async fn check_request_acceptance(&self, indexer: &IndexerSignal) -> RequestAcceptanceSignal {
        // If indexer is unhealthy, we can't trust the DB data
        if indexer.status == SignalStatus::Unhealthy {
            return RequestAcceptanceSignal {
                status: SignalStatus::Unknown,
                recent_requests_total: 0,
                recent_requests_accepted: 0,
                acceptance_ratio: None,
                last_accepted_secs_ago: None,
                message: Some(
                    "Cannot evaluate: indexer is unhealthy, DB data may be stale".to_string(),
                ),
            };
        }

        let msp_db_id = match self.get_msp_db_id().await {
            Ok(id) => id,
            Err(e) => {
                warn!(target: "node_health_service::check_request_acceptance", error = %e, "Failed to resolve MSP DB ID");
                return RequestAcceptanceSignal {
                    status: SignalStatus::Unknown,
                    recent_requests_total: 0,
                    recent_requests_accepted: 0,
                    acceptance_ratio: None,
                    last_accepted_secs_ago: None,
                    message: Some(format!("Failed to resolve MSP: {}", e)),
                };
            }
        };

        // Count total recent requests
        let total = match self
            .db
            .count_recent_requests_for_msp(msp_db_id, self.config.request_window_secs)
            .await
        {
            Ok(count) => count,
            Err(e) => {
                error!(target: "node_health_service::check_request_acceptance", error = %e, "Failed to count recent requests");
                return RequestAcceptanceSignal {
                    status: SignalStatus::Unknown,
                    recent_requests_total: 0,
                    recent_requests_accepted: 0,
                    acceptance_ratio: None,
                    last_accepted_secs_ago: None,
                    message: Some(format!("DB query failed: {}", e)),
                };
            }
        };

        // Count accepted recent requests
        let accepted = match self
            .db
            .count_recent_accepted_requests_for_msp(msp_db_id, self.config.request_window_secs)
            .await
        {
            Ok(count) => count,
            Err(e) => {
                error!(target: "node_health_service::check_request_acceptance", error = %e, "Failed to count accepted requests");
                return RequestAcceptanceSignal {
                    status: SignalStatus::Unknown,
                    recent_requests_total: total,
                    recent_requests_accepted: 0,
                    acceptance_ratio: None,
                    last_accepted_secs_ago: None,
                    message: Some(format!("DB query failed: {}", e)),
                };
            }
        };

        // Get last accepted time (informational)
        let last_accepted_secs_ago = match self
            .db
            .get_last_accepted_request_time_for_msp(msp_db_id)
            .await
        {
            Ok(Some(time)) => {
                let now = Utc::now().naive_utc();
                Some((now - time).num_seconds().max(0) as u64)
            }
            Ok(None) => None,
            Err(e) => {
                warn!(target: "node_health_service::check_request_acceptance", error = %e, "Failed to get last accepted time");
                None
            }
        };

        let acceptance_ratio = if total > 0 {
            Some(accepted as f64 / total as f64)
        } else {
            None
        };

        // Derive status
        let status = if total >= self.config.request_min_threshold as i64 && accepted == 0 {
            SignalStatus::Unhealthy
        } else {
            SignalStatus::Healthy
        };

        let message = match status {
            SignalStatus::Unhealthy => Some(format!(
                "MSP not accepting files: 0/{} requests accepted in the last {}s window",
                total, self.config.request_window_secs
            )),
            _ => None,
        };

        RequestAcceptanceSignal {
            status,
            recent_requests_total: total,
            recent_requests_accepted: accepted,
            acceptance_ratio,
            last_accepted_secs_ago,
            message,
        }
    }

    /// Check transaction nonce liveness signal
    async fn check_tx_nonce(&self) -> TxNonceSignal {
        let account = match self.get_msp_account().await {
            Ok(acc) => acc,
            Err(e) => {
                warn!(target: "node_health_service::check_tx_nonce", error = %e, "Failed to resolve MSP account");
                return TxNonceSignal {
                    status: SignalStatus::Unknown,
                    current_nonce: 0,
                    pending_extrinsics: 0,
                    nonce_unchanged_for_secs: 0,
                    message: Some(format!("Failed to resolve MSP account: {}", e)),
                };
            }
        };

        // Get current on-chain nonce
        let current_nonce = match self.rpc.get_account_nonce(&account).await {
            Ok(nonce) => nonce,
            Err(e) => {
                error!(target: "node_health_service::check_tx_nonce", error = %e, "Failed to get account nonce");
                return TxNonceSignal {
                    status: SignalStatus::Unknown,
                    current_nonce: 0,
                    pending_extrinsics: 0,
                    nonce_unchanged_for_secs: 0,
                    message: Some(format!("RPC call failed: {}", e)),
                };
            }
        };

        // Get pending extrinsics count
        let pending = match self.rpc.get_pending_extrinsics_count().await {
            Ok(count) => count,
            Err(e) => {
                error!(target: "node_health_service::check_tx_nonce", error = %e, "Failed to get pending extrinsics");
                return TxNonceSignal {
                    status: SignalStatus::Unknown,
                    current_nonce,
                    pending_extrinsics: 0,
                    nonce_unchanged_for_secs: 0,
                    message: Some(format!("RPC call failed: {}", e)),
                };
            }
        };

        // Update in-memory nonce state
        let nonce_unchanged_for_secs = {
            let mut state = self.nonce_state.write().await;

            if current_nonce != state.last_nonce {
                // Nonce advanced -- reset tracking
                state.last_nonce = current_nonce;
                state.first_seen_at = Instant::now();
                0
            } else {
                // Nonce unchanged -- compute duration
                state.first_seen_at.elapsed().as_secs()
            }
        };

        // Derive status
        let is_stuck =
            nonce_unchanged_for_secs >= self.config.nonce_stuck_threshold_secs && pending > 0;

        let status = if is_stuck {
            SignalStatus::Unhealthy
        } else {
            SignalStatus::Healthy
        };

        let message = if is_stuck {
            Some(format!(
                "Nonce stuck at {} for {}s with {} pending extrinsics (threshold: {}s)",
                current_nonce,
                nonce_unchanged_for_secs,
                pending,
                self.config.nonce_stuck_threshold_secs
            ))
        } else {
            None
        };

        TxNonceSignal {
            status,
            current_nonce,
            pending_extrinsics: pending,
            nonce_unchanged_for_secs,
            message,
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::{
        config::NodeHealthConfig,
        data::{
            indexer_db::mock_repository::MockRepository,
            rpc::{AnyRpcConnection, MockConnection},
        },
    };

    async fn mock_node_health_service() -> NodeHealthService {
        let repo = MockRepository::sample().await;
        let db = Arc::new(DBClient::new(Arc::new(repo)));
        let mock_conn = MockConnection::new();
        let rpc_conn = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let rpc = Arc::new(StorageHubRpcClient::new(rpc_conn));
        let msp_id = OnchainMspId::new(shp_types::Hash::from_slice(
            &crate::constants::rpc::DUMMY_MSP_ID,
        ));
        let config = NodeHealthConfig::default();

        NodeHealthService::new(db, rpc, msp_id, config)
    }

    #[test]
    fn signal_status_worst() {
        assert_eq!(
            SignalStatus::Healthy.worst(SignalStatus::Healthy),
            SignalStatus::Healthy
        );
        assert_eq!(
            SignalStatus::Healthy.worst(SignalStatus::Degraded),
            SignalStatus::Degraded
        );
        assert_eq!(
            SignalStatus::Degraded.worst(SignalStatus::Unhealthy),
            SignalStatus::Unhealthy
        );
        assert_eq!(
            SignalStatus::Healthy.worst(SignalStatus::Unknown),
            SignalStatus::Unknown
        );
        assert_eq!(
            SignalStatus::Unknown.worst(SignalStatus::Unhealthy),
            SignalStatus::Unhealthy
        );
    }

    #[tokio::test]
    async fn check_node_health_returns_response() {
        let service = mock_node_health_service().await;
        let response = service.check_node_health().await;

        // With mock data (finalized block 100, indexer at 100, no requests, nonce 42, no pending),
        // everything should be healthy
        assert_eq!(response.status, SignalStatus::Healthy);
        assert_eq!(response.signals.indexer.status, SignalStatus::Healthy);
        assert_eq!(
            response.signals.request_acceptance.status,
            SignalStatus::Healthy
        );
        assert_eq!(response.signals.tx_nonce.status, SignalStatus::Healthy);
    }

    #[tokio::test]
    async fn nonce_tracking_works() {
        let service = mock_node_health_service().await;

        // First call: nonce is 42 (mock default), should be set as first_seen
        let result = service.check_tx_nonce().await;
        assert_eq!(result.current_nonce, 42);
        assert_eq!(result.nonce_unchanged_for_secs, 0);
        assert_eq!(result.status, SignalStatus::Healthy);

        // Second call: nonce is still 42 (same mock), should track duration but still healthy
        // (because no pending extrinsics in mock)
        let result = service.check_tx_nonce().await;
        assert_eq!(result.current_nonce, 42);
        assert_eq!(result.status, SignalStatus::Healthy);
    }

    #[tokio::test]
    async fn acceptance_healthy_when_below_threshold() {
        let service = mock_node_health_service().await;

        // Mock returns 0 total requests and 0 accepted, which is below the threshold
        // so it should be healthy (not enough data to evaluate)
        let indexer = IndexerSignal {
            status: SignalStatus::Healthy,
            last_indexed_block: 100,
            finalized_block: 100,
            lag_blocks: 0,
            last_updated_secs_ago: 0,
            message: None,
        };

        let result = service.check_request_acceptance(&indexer).await;
        assert_eq!(result.status, SignalStatus::Healthy);
        assert_eq!(result.recent_requests_total, 0);
    }

    #[tokio::test]
    async fn acceptance_unknown_when_indexer_unhealthy() {
        let service = mock_node_health_service().await;

        let indexer = IndexerSignal {
            status: SignalStatus::Unhealthy,
            last_indexed_block: 50,
            finalized_block: 100,
            lag_blocks: 50,
            last_updated_secs_ago: 300,
            message: Some("stuck".to_string()),
        };

        let result = service.check_request_acceptance(&indexer).await;
        assert_eq!(result.status, SignalStatus::Unknown);
    }
}

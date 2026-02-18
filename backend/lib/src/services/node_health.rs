//! Node health service -- checks whether the MSP node is functioning correctly
//! by evaluating three signals: indexer health, storage request acceptance,
//! and transaction nonce liveness.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Utc;
use tracing::{debug, error};

use shc_indexer_db::OnchainMspId;

use crate::{
    config::NodeHealthConfig,
    data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient},
    models::node_health::{
        IndexerSignal, NodeHealthResponse, NodeHealthSignals, RequestAcceptanceSignal,
        SignalStatus, TxNonceSignal,
    },
};

/// Tracks the last observed nonce and when it was first seen at that value.
struct NonceState {
    nonce: u64,
    first_seen: Instant,
}

/// Cached MSP identity resolved lazily from the DB on first health check.
/// Lazy resolution avoids panicking at startup if the indexer hasn't
/// indexed the MSP yet (race condition in Docker-based integration tests).
struct MspIdentity {
    db_id: i64,
    account: String,
}

pub struct NodeHealthService {
    db: Arc<DBClient>,
    rpc: Arc<StorageHubRpcClient>,
    config: NodeHealthConfig,
    msp_id: OnchainMspId,
    msp_identity: Mutex<Option<MspIdentity>>,
    /// Tracks nonce over time to detect stuck transactions.
    nonce_state: Mutex<Option<NonceState>>,
}

impl NodeHealthService {
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
            msp_identity: Mutex::new(None),
            nonce_state: Mutex::new(None),
        }
    }

    /// Resolve and cache the MSP's DB id and signing account.
    /// Returns cached values on subsequent calls.
    async fn get_msp_identity(&self) -> Result<(i64, String), String> {
        {
            let cached = self.msp_identity.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref id) = *cached {
                return Ok((id.db_id, id.account.clone()));
            }
        }

        let msp = self
            .db
            .get_msp(&self.msp_id)
            .await
            .map_err(|e| format!("Failed to get MSP from DB: {}", e))?;

        let result = (msp.id, msp.account.clone());
        *self.msp_identity.lock().unwrap_or_else(|e| e.into_inner()) = Some(MspIdentity {
            db_id: msp.id,
            account: msp.account,
        });
        Ok(result)
    }

    pub async fn check_node_health(&self) -> NodeHealthResponse {
        debug!(target: "node_health_service", "Node health check initiated");

        // Run independent signals concurrently
        let (indexer, tx_nonce) = tokio::join!(self.check_indexer(), self.check_tx_nonce());

        // Request acceptance depends on the indexer result
        let request_acceptance = self.check_request_acceptance(&indexer).await;

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

    async fn check_indexer(&self) -> IndexerSignal {
        let service_state = match self.db.get_service_state().await {
            Ok(state) => state,
            Err(e) => {
                error!(target: "node_health_service", error = %e, "Failed to get service state");
                return IndexerSignal::unknown(format!("Failed to get service state: {}", e));
            }
        };

        let finalized_block = match self.rpc.get_finalized_block_number().await {
            Ok(block) => block,
            Err(e) => {
                error!(target: "node_health_service", error = %e, "Failed to get finalized block");
                return IndexerSignal::unknown(format!(
                    "Failed to get finalized block from RPC: {}",
                    e
                ));
            }
        };

        let now = Utc::now().naive_utc();
        let updated_secs_ago = (now - service_state.updated_at).num_seconds().max(0) as u64;
        let last_indexed = service_state.last_indexed_finalized_block as u64;
        let lag_blocks = finalized_block.saturating_sub(last_indexed);

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

    /// If the indexer is unhealthy, marks this signal as unknown (can't trust stale data).
    async fn check_request_acceptance(&self, indexer: &IndexerSignal) -> RequestAcceptanceSignal {
        if indexer.status == SignalStatus::Unhealthy {
            return RequestAcceptanceSignal::unknown(
                "Cannot evaluate: indexer is unhealthy, DB data may be stale",
            );
        }

        let (msp_db_id, _) = match self.get_msp_identity().await {
            Ok(id) => id,
            Err(e) => {
                error!(target: "node_health_service", error = %e, "Failed to resolve MSP");
                return RequestAcceptanceSignal::unknown(format!("Failed to resolve MSP: {}", e));
            }
        };

        let stats = match self
            .db
            .get_request_acceptance_stats(msp_db_id, self.config.request_window_secs)
            .await
        {
            Ok(stats) => stats,
            Err(e) => {
                error!(target: "node_health_service", error = %e, "Failed to get request stats");
                return RequestAcceptanceSignal::unknown(format!("DB query failed: {}", e));
            }
        };

        let last_accepted_secs_ago = stats.last_accepted_at.map(|time| {
            let now = Utc::now().naive_utc();
            (now - time).num_seconds().max(0) as u64
        });

        let acceptance_ratio = if stats.total > 0 {
            Some(stats.accepted as f64 / stats.total as f64)
        } else {
            None
        };

        let status =
            if stats.total >= self.config.request_min_threshold as i64 && stats.accepted == 0 {
                SignalStatus::Unhealthy
            } else {
                SignalStatus::Healthy
            };

        let message = match status {
            SignalStatus::Unhealthy => Some(format!(
                "MSP not accepting files: 0/{} requests accepted in the last {}s window",
                stats.total, self.config.request_window_secs
            )),
            _ => None,
        };

        RequestAcceptanceSignal {
            status,
            recent_requests_total: stats.total,
            recent_requests_accepted: stats.accepted,
            acceptance_ratio,
            last_accepted_secs_ago,
            message,
        }
    }

    /// Tracks the MSP's on-chain nonce over time. If the nonce hasn't changed
    /// for longer than `nonce_stuck_threshold_secs`, flags as unhealthy.
    ///
    /// On the first call after startup there is no baseline, so the signal
    /// reports Healthy with `nonce_unchanged_for_secs: None`.
    async fn check_tx_nonce(&self) -> TxNonceSignal {
        let (_, account) = match self.get_msp_identity().await {
            Ok(id) => id,
            Err(e) => {
                error!(target: "node_health_service", error = %e, "Failed to resolve MSP account");
                return TxNonceSignal::unknown(format!("Failed to resolve MSP account: {}", e));
            }
        };

        let current_nonce = match self.rpc.get_account_nonce(&account).await {
            Ok(nonce) => nonce,
            Err(e) => {
                error!(target: "node_health_service", error = %e, "Failed to get account nonce");
                return TxNonceSignal::unknown(format!("RPC call failed: {}", e));
            }
        };

        let mut state = self.nonce_state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        let unchanged_secs = match state.as_ref() {
            Some(prev) if prev.nonce == current_nonce => Some(prev.first_seen.elapsed().as_secs()),
            Some(_) => {
                // Nonce changed, reset
                *state = Some(NonceState {
                    nonce: current_nonce,
                    first_seen: now,
                });
                Some(0)
            }
            None => {
                // First call, establish baseline
                *state = Some(NonceState {
                    nonce: current_nonce,
                    first_seen: now,
                });
                None
            }
        };

        let threshold = self.config.nonce_stuck_threshold_secs;
        let is_stuck = unchanged_secs.map_or(false, |s| s >= threshold);

        let status = if is_stuck {
            SignalStatus::Unhealthy
        } else {
            SignalStatus::Healthy
        };

        let message = if is_stuck {
            Some(format!(
                "Nonce unchanged for {}s (threshold: {}s)",
                unchanged_secs.unwrap_or(0),
                threshold
            ))
        } else {
            None
        };

        TxNonceSignal {
            status,
            current_nonce,
            nonce_unchanged_for_secs: unchanged_secs,
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
        mock_node_health_service_with_config(NodeHealthConfig::default()).await
    }

    async fn mock_node_health_service_with_config(config: NodeHealthConfig) -> NodeHealthService {
        let repo = MockRepository::sample().await;
        let db = Arc::new(DBClient::new(Arc::new(repo)));
        let mock_conn = MockConnection::new();
        let rpc_conn = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let rpc = Arc::new(StorageHubRpcClient::new(rpc_conn));
        let msp_id = OnchainMspId::new(shp_types::Hash::from_slice(
            &crate::constants::rpc::DUMMY_MSP_ID,
        ));
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

        // Mock data: finalized block 100, indexer at 100, no requests, nonce 42
        assert_eq!(response.status, SignalStatus::Healthy);
        assert_eq!(response.signals.indexer.status, SignalStatus::Healthy);
        assert_eq!(
            response.signals.request_acceptance.status,
            SignalStatus::Healthy
        );
        assert_eq!(response.signals.tx_nonce.status, SignalStatus::Healthy);
    }

    #[tokio::test]
    async fn nonce_first_check_has_no_baseline() {
        let service = mock_node_health_service().await;

        let result = service.check_tx_nonce().await;
        assert_eq!(result.current_nonce, 42);
        // First call — no baseline yet
        assert_eq!(result.nonce_unchanged_for_secs, None);
        assert_eq!(result.status, SignalStatus::Healthy);
    }

    #[tokio::test]
    async fn nonce_subsequent_check_tracks_duration() {
        let service = mock_node_health_service().await;

        // First call establishes the baseline
        let _ = service.check_tx_nonce().await;

        // Second call should report the duration since the baseline
        let result = service.check_tx_nonce().await;
        assert_eq!(result.current_nonce, 42);
        assert!(result.nonce_unchanged_for_secs.is_some());
        assert_eq!(result.status, SignalStatus::Healthy);
    }

    #[tokio::test]
    async fn nonce_unhealthy_when_stuck() {
        // Use a threshold of 0 so the nonce is immediately "stuck"
        let config = NodeHealthConfig {
            nonce_stuck_threshold_secs: 0,
            ..NodeHealthConfig::default()
        };
        let service = mock_node_health_service_with_config(config).await;

        // First call should establish the baseline
        let _ = service.check_tx_nonce().await;

        // Second call should report the duration since the baseline
        let result = service.check_tx_nonce().await;
        assert_eq!(result.status, SignalStatus::Unhealthy);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn acceptance_healthy_when_below_threshold() {
        let service = mock_node_health_service().await;

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

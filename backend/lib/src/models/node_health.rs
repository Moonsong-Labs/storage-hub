use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl SignalStatus {
    /// Returns the worse of two statuses (unhealthy > degraded > unknown > healthy)
    pub fn worst(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unhealthy, _) | (_, Self::Unhealthy) => Self::Unhealthy,
            (Self::Degraded, _) | (_, Self::Degraded) => Self::Degraded,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            _ => Self::Healthy,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeHealthResponse {
    pub status: SignalStatus,
    pub checked_at: String,
    pub signals: NodeHealthSignals,
}

impl IntoResponse for NodeHealthResponse {
    fn into_response(self) -> Response<Body> {
        let status_code = match self.status {
            SignalStatus::Healthy => StatusCode::OK,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        };
        (status_code, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeHealthSignals {
    pub indexer: IndexerSignal,
    pub request_acceptance: RequestAcceptanceSignal,
    pub tx_nonce: TxNonceSignal,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexerSignal {
    pub status: SignalStatus,
    pub last_indexed_block: u64,
    pub finalized_block: u64,
    pub lag_blocks: u64,
    pub last_updated_secs_ago: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl IndexerSignal {
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self {
            status: SignalStatus::Unknown,
            last_indexed_block: 0,
            finalized_block: 0,
            lag_blocks: 0,
            last_updated_secs_ago: 0,
            message: Some(msg.into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestAcceptanceSignal {
    pub status: SignalStatus,
    pub recent_requests_total: i64,
    pub recent_requests_accepted: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_accepted_secs_ago: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl RequestAcceptanceSignal {
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self {
            status: SignalStatus::Unknown,
            recent_requests_total: 0,
            recent_requests_accepted: 0,
            acceptance_ratio: None,
            last_accepted_secs_ago: None,
            message: Some(msg.into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxNonceSignal {
    pub status: SignalStatus,
    pub current_nonce: u64,
    pub pending_extrinsics: usize,
    pub nonce_unchanged_for_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl TxNonceSignal {
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self {
            status: SignalStatus::Unknown,
            current_nonce: 0,
            pending_extrinsics: 0,
            nonce_unchanged_for_secs: 0,
            message: Some(msg.into()),
        }
    }
}

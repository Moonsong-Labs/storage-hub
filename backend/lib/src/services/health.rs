use std::sync::Arc;

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::{debug, error};

use crate::data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage};

#[derive(Serialize)]
pub struct DetailedHealthStatus {
    pub status: String,
    pub version: String,
    pub service: String,
    pub components: HealthComponents,
}

impl DetailedHealthStatus {
    pub fn is_healthy(&self) -> bool {
        self.status == HealthService::HEALTHY
    }
}

impl IntoResponse for DetailedHealthStatus {
    fn into_response(self) -> Response<Body> {
        let status = if self.is_healthy() {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        (status, Json(self)).into_response()
    }
}

#[derive(Serialize)]
pub struct HealthComponents {
    pub storage: ComponentHealth,
    pub postgres: ComponentHealth,
    pub rpc: ComponentHealth,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub struct HealthService {
    storage: Arc<dyn BoxedStorage>,
    db: Arc<DBClient>,
    rpc: Arc<StorageHubRpcClient>,
}

impl HealthService {
    pub const HEALTHY: &str = "healthy";
    pub const UNHEALTHY: &str = "unhealthy";

    /// Creates a new health service instance
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        db: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    ) -> Self {
        Self { storage, db, rpc }
    }

    pub async fn check_health(&self) -> DetailedHealthStatus {
        debug!(target: "health_service::check_health", "Health check initiated");

        let storage_health = self.check_storage().await;
        let postgres_health = self.check_postgres().await;
        let rpc_health = self.check_rpc().await;

        let overall_status = if storage_health.status == Self::HEALTHY
            && postgres_health.status == Self::HEALTHY
            && rpc_health.status == Self::HEALTHY
        {
            Self::HEALTHY
        } else {
            error!(
                target: "health_service::check_health",
                storage_status = %storage_health.status,
                postgres_status = %postgres_health.status,
                rpc_status = %rpc_health.status,
                "Health check FAILED",
            );
            Self::UNHEALTHY
        };

        DetailedHealthStatus {
            status: overall_status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            service: "storagehub-backend".to_string(),
            components: HealthComponents {
                storage: storage_health,
                postgres: postgres_health,
                rpc: rpc_health,
            },
        }
    }

    async fn check_storage(&self) -> ComponentHealth {
        debug!(target: "health_service::check_storage", "Checking storage health");

        let (status, message) = match self.storage.health_check().await {
            Ok(true) => (Self::HEALTHY, None),
            Ok(false) => (Self::UNHEALTHY, Some("Storage is not healthy".to_string())),
            Err(e) => (
                Self::UNHEALTHY,
                Some(format!("Storage health check failed: {e}")),
            ),
        };

        ComponentHealth {
            status: status.to_string(),
            message,
        }
    }

    async fn check_postgres(&self) -> ComponentHealth {
        debug!(target: "health_service::check_postgres", "Checking PostgreSQL health");

        let (status, message) = match self.db.test_connection().await {
            Ok(_) => (Self::HEALTHY, None),
            Err(e) => (
                Self::UNHEALTHY,
                Some(format!("Database connection failed: {e}")),
            ),
        };

        ComponentHealth {
            status: status.to_string(),
            message,
        }
    }

    async fn check_rpc(&self) -> ComponentHealth {
        debug!(target: "health_service::check_rpc", "Checking RPC health");

        // Verify the RPC connection is alive
        let (status, message) = match self.rpc.is_connected().await {
            true => (Self::HEALTHY, None),
            false => (
                Self::UNHEALTHY,
                Some("RPC connection not established".to_string()),
            ),
        };

        ComponentHealth {
            status: status.to_string(),
            message,
        }
    }
}

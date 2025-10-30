//! TODO(MOCK): this service returns pretty rough health status of the underlying services
//! it doesn't check ALL services in use by the backend, nor does an accurate analysis
//! of all the parts that it does check

use std::sync::Arc;

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use shc_rpc::RpcProviderId;
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

        (status, Json(self))
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

// TODO(SCAFFOLDING): This health service is a stub and should be replaced with
// logic more appropriate to the final usecase
pub struct HealthService {
    storage: Arc<dyn BoxedStorage>,
    db: Arc<DBClient>,
    rpc: Arc<StorageHubRpcClient>,
}

impl HealthService {
    pub const HEALTHY: &str = "healthy";
    pub const UNHEALTHY: &str = "unhealthy";

    /// Instantiate a new [`HealthService`]
    ///
    /// This service uses the following services:
    /// * storage: determine if storage is healthy
    /// * db: determine if the db connection is healthy
    /// * rpc: determine if the rpc connection is healthy
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
            Ok(false) => (Self::UNHEALTHY, None),
            Err(e) => (Self::UNHEALTHY, Some(format!("Storage error: {e}"))),
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
            Err(e) => (Self::UNHEALTHY, Some(format!("Database error: {e}"))),
        };

        ComponentHealth {
            status: status.to_string(),
            message,
        }
    }

    async fn check_rpc(&self) -> ComponentHealth {
        debug!(target: "health_service::check_rpc", "Checking RPC health");

        // First check if the connection to the RPC is established
        if !self.rpc.is_connected().await {
            error!(target: "health_service::check_rpc", "RPC health check failed - connection not established");
            return ComponentHealth {
                status: Self::UNHEALTHY.to_string(),
                message: Some("RPC connection not established".to_string()),
            };
        }

        // Then to make sure everything works test actual RPC functionality
        // by getting the provider ID of the connected node.
        let (status, message) = match self.rpc.get_provider_id().await {
            Ok(RpcProviderId::Msp(_)) => (Self::HEALTHY, None),
            Ok(RpcProviderId::Bsp(_)) => {
                error!(target: "health_service::check_rpc", "RPC health check failed - connected to BSP instead of MSP");
                (
                    Self::UNHEALTHY,
                    Some("The node that we are connected to is a BSP, expected an MSP".to_string()),
                )
            }
            Ok(RpcProviderId::NotAProvider) => {
                error!(target: "health_service::check_rpc", "RPC health check failed - node is not a storage provider");
                (
                    Self::UNHEALTHY,
                    Some("The node that we are connected to is not a storage provider".to_string()),
                )
            }
            Err(e) => {
                error!(target: "health_service::check_rpc", error = %e, "RPC health check failed - RPC call error");
                (Self::UNHEALTHY, Some(format!("RPC call failed: {}", e)))
            }
        };

        ComponentHealth {
            status: status.to_string(),
            message,
        }
    }
}

use std::sync::Arc;

use serde::Serialize;

use crate::data::{indexer_db::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage};

#[derive(Serialize)]
pub struct DetailedHealthStatus {
    pub status: String,
    pub version: String,
    pub service: String,
    pub components: HealthComponents,
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
        let storage_health = self.check_storage().await;
        let postgres_health = self.check_postgres().await;
        let rpc_health = self.check_rpc().await;

        let overall_status = if storage_health.status == "healthy"
            && postgres_health.status == "healthy"
            && rpc_health.status == "healthy"
        {
            "healthy"
        } else {
            "unhealthy"
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
        let (status, message) = match self.storage.health_check().await {
            Ok(true) => ("healthy", None),
            Ok(false) => ("unhealthy", None),
            Err(e) => ("unhealthy", Some(format!("Storage error: {e}"))),
        };

        ComponentHealth {
            status: status.to_string(),
            message,
        }
    }

    async fn check_postgres(&self) -> ComponentHealth {
        match self.db.test_connection().await {
            Ok(_) => ComponentHealth {
                status: "healthy".to_string(),
                message: None,
            },
            Err(e) => ComponentHealth {
                status: "unhealthy".to_string(),
                message: Some(format!("Database error: {e}")),
            },
        }
    }

    async fn check_rpc(&self) -> ComponentHealth {
        match self.rpc.is_connected().await {
            true => ComponentHealth {
                status: "healthy".to_string(),
                message: None,
            },
            false => ComponentHealth {
                status: "unhealthy".to_string(),
                message: Some("RPC connection not established".to_string()),
            },
        }
    }
}

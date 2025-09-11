//! Services module for StorageHub backend

use std::sync::Arc;

#[cfg(all(test, feature = "mocks"))]
use crate::data::{
    indexer_db::mock_repository::MockRepository,
    rpc::{AnyRpcConnection, MockConnection},
    storage::{BoxedStorageWrapper, InMemoryStorage},
};
use crate::{
    config::Config,
    data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage},
};

pub mod auth;
pub mod health;
pub mod msp;

use auth::AuthService;
use axum::extract::FromRef;
use axum_jwt::Decoder;
use health::HealthService;
use msp::MspService;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    pub auth: Arc<AuthService>,
    pub health: Arc<HealthService>,
    pub msp: Arc<MspService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<DBClient>,
    pub rpc: Arc<StorageHubRpcClient>,
}

impl Services {
    /// Create a new services struct
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        config: Config,
    ) -> Self {
        let jwt_secret = hex::decode(config.auth.jwt_secret.trim_start_matches("0x"))
            .expect("valid JWT secret hex string");

        let auth = Arc::new(AuthService::new(jwt_secret.as_slice()));
        let health = Arc::new(HealthService::new(
            storage.clone(),
            postgres.clone(),
            rpc.clone(),
        ));
        let msp = Arc::new(MspService::new(
            storage.clone(),
            postgres.clone(),
            rpc.clone(),
            config.storage_hub.msp_callback_url.clone(),
        ));
        Self {
            auth,
            health,
            msp,
            storage,
            postgres,
            rpc,
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
impl Services {
    /// Create a test services struct with in-memory storage and mocks
    pub fn mocks() -> Self {
        // Create in-memory storage
        let memory_storage = InMemoryStorage::new();
        let storage = Arc::new(BoxedStorageWrapper::new(memory_storage));
        let mut cfg = crate::config::Config::default();
        cfg.storage_hub.msp_callback_url = String::from("http://localhost:8080");

        // Create mock database client
        let repo = MockRepository::new();
        let postgres = Arc::new(DBClient::new(Arc::new(repo)));

        // Create mock RPC client
        let mock_conn = MockConnection::new();
        let rpc_conn = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let rpc = Arc::new(StorageHubRpcClient::new(rpc_conn));

        // Use default config for mocks
        let config = Config::default();

        Self::new(storage, postgres, rpc, config)
    }
}

// axum_jwt extractors require the app state to implement this
// to be able to extract the token/claims in the request
impl FromRef<Services> for Decoder {
    fn from_ref(services: &Services) -> Decoder {
        services.auth.jwt_decoder().clone()
    }
}

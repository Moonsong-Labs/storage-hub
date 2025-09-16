//! Services module for StorageHub backend

use std::sync::Arc;

use auth::AuthService;
use axum::extract::FromRef;
use axum_jwt::Decoder;

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
        let jwt_secret = config
            .auth
            .jwt_secret
            .as_ref()
            .ok_or_else(|| {
                tracing::error!("JWT_SECRET is not set. Please set it in the config file or as an environment variable.");
                "JWT_SECRET is not configured"
            })
            .and_then(|secret| {
                hex::decode(secret.trim_start_matches("0x"))
                    .map_err(|e| {
                        tracing::error!("Invalid JWT_SECRET format. Must be a valid hex string: {}", e);
                        "Invalid JWT_SECRET format"
                    })
            })
            .and_then(|decoded| {
                if decoded.len() < 32 {
                    tracing::error!("JWT_SECRET is too short. Must be at least 32 bytes (64 hex characters), got {} bytes", decoded.len());
                    Err("JWT_SECRET must be at least 32 bytes")
                } else {
                    Ok(decoded)
                }
            })
            .expect("valid JWT secret configuration");

        #[cfg(feature = "mocks")]
        let jwt_validate = !config.auth.mock_mode;
        #[cfg(not(feature = "mocks"))]
        let jwt_validate = true;

        let auth = Arc::new(AuthService::new(
            jwt_secret.as_slice(),
            jwt_validate,
            storage.clone(),
        ));
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
        // Use default config for mocks
        let config = Config::default();

        Self::mocks_with_config(config)
    }

    /// Create a test services struct with in-memory storage and mocks and custom config
    pub fn mocks_with_config(config: Config) -> Self {
        // Create in-memory storage
        let memory_storage = InMemoryStorage::new();
        let storage = Arc::new(BoxedStorageWrapper::new(memory_storage));

        // Create mock database client
        let repo = MockRepository::new();
        let postgres = Arc::new(DBClient::new(Arc::new(repo)));

        // Create mock RPC client
        let mock_conn = MockConnection::new();
        let rpc_conn = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let rpc = Arc::new(StorageHubRpcClient::new(rpc_conn));

        Self::new(storage, postgres, rpc, config)
    }
}

// axum_jwt extractors require the app state to implement this
// to be able to extract the token/claims in the request
impl FromRef<Services> for Decoder {
    fn from_ref(services: &Services) -> Decoder {
        Decoder::new(
            services.auth.jwt_decoding_key().clone(),
            services.auth.jwt_validation().clone(),
        )
    }
}

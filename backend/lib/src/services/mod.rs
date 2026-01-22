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
pub mod download_session;
pub mod health;
pub mod msp;
pub mod upload_session;

use download_session::DownloadSessionManager;
use health::HealthService;
use msp::MspService;
use upload_session::UploadSessionManager;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    pub config: Config,
    pub auth: Arc<AuthService>,
    pub health: Arc<HealthService>,
    pub msp: Arc<MspService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<DBClient>,
    pub rpc: Arc<StorageHubRpcClient>,
    pub download_sessions: Arc<DownloadSessionManager>,
    pub upload_sessions: Arc<UploadSessionManager>,
}

impl Services {
    /// Create a new services struct
    pub async fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
        config: Config,
    ) -> Self {
        let auth = Arc::new(AuthService::from_config(&config.auth, storage.clone()));

        let health = Arc::new(HealthService::new(
            storage.clone(),
            postgres.clone(),
            rpc.clone(),
        ));

        let msp =
            Arc::new(MspService::new(postgres.clone(), rpc.clone(), config.msp.clone()).await);

        let download_sessions = Arc::new(DownloadSessionManager::new(
            config.file_transfer.max_download_sessions,
        ));
        let upload_sessions = Arc::new(UploadSessionManager::new(
            config.file_transfer.max_upload_sessions,
        ));

        Self {
            config,
            auth,
            health,
            msp,
            storage,
            postgres,
            rpc,
            download_sessions,
            upload_sessions,
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
impl Services {
    /// Create a test services struct with in-memory storage and mocks
    pub async fn mocks() -> Self {
        // Use default config for mocks
        let config = Config::default();

        Self::mocks_with_config(config).await
    }

    /// Create a test services struct with in-memory storage and mocks and custom config
    pub async fn mocks_with_config(config: Config) -> Self {
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

        Self::new(storage, postgres, rpc, config).await
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

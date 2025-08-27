//! Services module for StorageHub backend

use std::sync::Arc;

use crate::data::{indexer_db::client::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage};
#[cfg(all(test, feature = "mocks"))]
use crate::data::{
    indexer_db::mock_repository::MockRepository,
    rpc::{AnyRpcConnection, MockConnection},
    storage::{BoxedStorageWrapper, InMemoryStorage},
};

pub mod health;

use health::HealthService;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    // TODO(SCAFFOLDING): Health service is for demostration only
    // Will be replaced with a proper health service is needed when we implement the backend proper
    pub health: Arc<HealthService>,
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
    ) -> Self {
        let health = Arc::new(HealthService::new(
            storage.clone(),
            postgres.clone(),
            rpc.clone(),
        ));
        Self {
            health,
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

        // Create mock database client
        let repo = MockRepository::new();
        let postgres = Arc::new(DBClient::new(Arc::new(repo)));

        // Create mock RPC client
        let mock_conn = MockConnection::new();
        let rpc_conn = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let rpc = Arc::new(StorageHubRpcClient::new(rpc_conn));

        Self::new(storage, postgres, rpc)
    }
}

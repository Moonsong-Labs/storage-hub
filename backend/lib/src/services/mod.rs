//! Services module for StorageHub backend

use std::sync::Arc;

#[cfg(all(test, feature = "mocks"))]
use crate::data::{
    rpc::{AnyRpcConnection, MockConnection},
    storage::{BoxedStorageWrapper, InMemoryStorage},
};
use crate::{
    data::{postgres::DBClient, rpc::StorageHubRpcClient, storage::BoxedStorage},
    repository::MockRepository,
};

// TODO(SCAFFOLDING): Counter module is for demonstration only
// Remove when implementing real MSP services
pub mod counter;
pub mod health;

use counter::CounterService;
use health::HealthService;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    // TODO(SCAFFOLDING): Counter service field is for demonstration only
    // Remove when implementing real MSP services
    pub counter: Arc<CounterService>,
    pub health: Arc<HealthService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<DBClient>,
    pub rpc: Arc<StorageHubRpcClient>,
}

impl Services {
    /// Create a new services container
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<DBClient>,
        rpc: Arc<StorageHubRpcClient>,
    ) -> Self {
        let counter = Arc::new(CounterService::new(storage.clone()));
        let health = Arc::new(HealthService::new(
            storage.clone(),
            postgres.clone(),
            rpc.clone(),
        ));
        Self {
            counter,
            health,
            storage,
            postgres,
            rpc,
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
impl Services {
    /// Create a test services container with in-memory storage and mocks
    pub fn test() -> Self {
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

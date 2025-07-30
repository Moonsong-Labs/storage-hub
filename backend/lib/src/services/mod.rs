//! Services module for StorageHub backend
use std::sync::Arc;

pub mod counter;
pub mod health;

use counter::CounterService;
use health::HealthService;

use crate::data::postgres::PostgresClientTrait;
use crate::data::rpc::StorageHubRpcClient;
use crate::data::storage::BoxedStorage;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    pub counter: Arc<CounterService>,
    pub health: Arc<HealthService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<dyn PostgresClientTrait>,
    pub rpc: Arc<StorageHubRpcClient>,
}

impl Services {
    /// Create a new services container
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<dyn PostgresClientTrait>,
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

#[cfg(test)]
impl Services {
    /// Create a test services container with in-memory storage
    pub fn test() -> Self {
        todo!("Test services not yet implemented - requires mock implementations for PostgresClientTrait and StorageHubRpcClient")
    }
}

//! Services module for StorageHub backend
use std::sync::Arc;

pub mod counter;
pub mod health;

use counter::CounterService;

use crate::data::postgres::PostgresClientTrait;
use crate::data::rpc::StorageHubRpcTrait;
use crate::data::storage::BoxedStorage;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    pub counter: Arc<CounterService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<dyn PostgresClientTrait>,
    pub rpc: Arc<dyn StorageHubRpcTrait>,
}

impl Services {
    /// Create a new services container
    ///
    /// # Arguments
    /// * `storage` - Storage backend for counters and temporary data
    /// * `postgres` - PostgreSQL client for accessing indexer database
    /// * `rpc` - RPC client for accessing StorageHub blockchain
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<dyn PostgresClientTrait>,
        rpc: Arc<dyn StorageHubRpcTrait>,
    ) -> Self {
        let counter = Arc::new(CounterService::new(storage.clone()));
        Self {
            counter,
            storage,
            postgres,
            rpc,
        }
    }
}

#[cfg(test)]
impl Services {
    /// Create a test services container with in-memory storage
    ///
    /// Note: This currently panics as it requires PostgreSQL and RPC mocks
    /// which are not yet implemented. Use only for tests that don't require
    /// these services.
    pub fn test() -> Self {
        use crate::data::storage::{BoxedStorageWrapper, InMemoryStorage};
        
        let memory_storage = InMemoryStorage::new();
        let boxed_storage = BoxedStorageWrapper::new(memory_storage);
        let storage: Arc<dyn BoxedStorage> = Arc::new(boxed_storage);
        
        // TODO: Once mock implementations are complete, use them here
        panic!("Test services require PostgreSQL and RPC mock implementations - currently WIP")
    }
    
    /// Create a test services container with only storage (for counter tests)
    pub fn test_with_storage_only(storage: Arc<dyn BoxedStorage>) -> Self {
        // This is a workaround for tests that only need storage
        // Once full mocks are available, this can be removed
        panic!("Test services require PostgreSQL and RPC mock implementations - currently WIP")
    }
}

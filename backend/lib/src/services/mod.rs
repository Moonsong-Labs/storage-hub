//! Services module for StorageHub backend
use std::sync::Arc;

pub mod counter;
pub mod health;

use crate::data::postgres::PostgresClientTrait;
use crate::data::rpc::StorageHubRpcTrait;
use crate::data::storage::BoxedStorage;
use counter::CounterService;

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

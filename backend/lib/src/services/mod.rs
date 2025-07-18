//! Services module for StorageHub backend

pub mod counter;
pub mod health;

use std::sync::Arc;
use crate::data::storage::BoxedStorage;
use crate::data::postgres::PostgresClientTrait;

/// Container for all backend services
#[derive(Clone)]
pub struct Services {
    pub counter: Arc<counter::CounterService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<dyn PostgresClientTrait>,
}

impl Services {
    /// Create a new services container
    ///
    /// # Arguments
    /// * `storage` - Storage backend for counters and temporary data
    /// * `postgres` - PostgreSQL client for accessing indexer database
    pub fn new(
        storage: Arc<dyn BoxedStorage>,
        postgres: Arc<dyn PostgresClientTrait>,
    ) -> Self {
        let counter = Arc::new(counter::CounterService::new(storage.clone()));
        Self { counter, storage, postgres }
    }
}

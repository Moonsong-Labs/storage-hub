//! Services module for StorageHub backend

pub mod counter;
pub mod health;

use std::sync::Arc;
use crate::data::storage::BoxedStorage;
use crate::data::postgres::PostgresClient;

#[derive(Clone)]
pub struct Services {
    pub counter: Arc<counter::CounterService>,
    pub storage: Arc<dyn BoxedStorage>,
    pub postgres: Arc<PostgresClient>,
}

impl Services {
    pub fn new(storage: Arc<dyn BoxedStorage>, postgres: Arc<PostgresClient>) -> Self {
        let counter = Arc::new(counter::CounterService::new(storage.clone()));
        Self { counter, storage, postgres }
    }
}

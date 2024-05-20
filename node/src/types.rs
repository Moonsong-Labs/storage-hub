use file_manager::in_memory::InMemoryFileStorage;
use forest_manager::in_memory::InMemoryForestStorage;
use reference_trie::RefHasher;
use sp_trie::LayoutV1;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::handler::StorageHubHandlerConfig;

#[derive(Clone)]
pub struct StorageHubBackend {
    pub file_storage: Arc<RwLock<InMemoryFileStorage<LayoutV1<RefHasher>>>>,
    pub forest_storage: Arc<RwLock<InMemoryForestStorage<LayoutV1<RefHasher>>>>,
}

impl StorageHubHandlerConfig for StorageHubBackend {
    type FileStorage = InMemoryFileStorage<LayoutV1<RefHasher>>;
    type ForestStorage = InMemoryForestStorage<LayoutV1<RefHasher>>;
}

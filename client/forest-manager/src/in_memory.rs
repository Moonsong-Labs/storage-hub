use storage_hub_infra::types::Key;

use crate::traits::{ForestProof, ForestStorage};
pub struct InMemoryForestStorage {}

impl InMemoryForestStorage {
    pub fn new() -> Self {
        Self {}
    }
}

impl ForestStorage for InMemoryForestStorage {
    fn generate_proof(&self, _challenged_file_key: &Key) -> ForestProof {
        unimplemented!()
    }

    fn delete_file_key(&self, _file_key: &Key) -> ForestProof {
        unimplemented!()
    }

    fn insert_file_key(&self, _file_key: &Key) -> ForestProof {
        unimplemented!()
    }
}

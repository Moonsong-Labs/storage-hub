pub struct InMemoryForestStorage {}

impl InMemoryForestStorage {
    pub fn new() -> Self {
        Self {}
    }
}

impl storage_hub_infra::storage::ForestStorage for InMemoryForestStorage {
    fn generate_proof(
        &self,
        _challenged_file_key: &storage_hub_infra::types::Key,
    ) -> storage_hub_infra::storage::ForestProof {
        unimplemented!()
    }

    fn delete_file_key(
        &self,
        _file_key: &storage_hub_infra::types::Key,
    ) -> storage_hub_infra::storage::ForestProof {
        unimplemented!()
    }

    fn insert_file_key(
        &self,
        _file_key: &storage_hub_infra::types::Key,
    ) -> storage_hub_infra::storage::ForestProof {
        unimplemented!()
    }
}

pub struct InMemoryFileStorage {}

impl InMemoryFileStorage {
    pub fn new() -> Self {
        Self {}
    }
}

impl storage_hub_infra::storage::FileStorage for InMemoryFileStorage {
    fn generate_proof(
        &self,
        _challenged_key: &storage_hub_infra::types::Key,
    ) -> storage_hub_infra::storage::FileProof {
        unimplemented!()
    }

    fn delete_file(&self, _key: &storage_hub_infra::types::Key) {
        unimplemented!()
    }

    fn get_metadata(
        &self,
        _key: &storage_hub_infra::types::Key,
    ) -> Option<storage_hub_infra::types::Metadata> {
        unimplemented!()
    }

    fn set_metadata(
        &self,
        _key: &storage_hub_infra::types::Key,
        _metadata: &storage_hub_infra::types::Metadata,
    ) {
        unimplemented!()
    }

    fn get_chunk(
        &self,
        _key: &storage_hub_infra::types::Key,
        _chunk: u64,
    ) -> Option<storage_hub_infra::types::Chunk> {
        unimplemented!()
    }

    fn write_chunk(&self, _key: &str, _chunk: u64, _data: &storage_hub_infra::types::Chunk) {
        unimplemented!()
    }
}

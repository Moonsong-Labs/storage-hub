use storage_hub_infra::types::{Chunk, FileProof, Key, Metadata};

use crate::traits::FileStorage;

pub struct InMemoryFileStorage {}

impl InMemoryFileStorage {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileStorage for InMemoryFileStorage {
    type Key = Key;
    type Value = Chunk;

    fn generate_proof(&self, _challenged_key: &Self::Key) -> FileProof<Self::Key> {
        unimplemented!()
    }

    fn delete_file(&self, _key: &Self::Key) {
        unimplemented!()
    }

    fn get_metadata(&self, _key: &Self::Key) -> Option<Metadata> {
        unimplemented!()
    }

    fn set_metadata(&self, _key: &Self::Key, _metadata: &Metadata) {
        unimplemented!()
    }

    fn get_chunk(&self, _key: &Self::Key) -> Option<Self::Value> {
        unimplemented!()
    }

    fn write_chunk(&self, _key: &str, _data: &Self::Value) {
        unimplemented!()
    }
}

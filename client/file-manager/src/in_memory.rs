use storage_hub_infra::types::{Chunk, Key, Metadata};

use crate::traits::{FileProof, FileStorage};

pub struct InMemoryFileStorage {}

impl InMemoryFileStorage {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileStorage for InMemoryFileStorage {
    fn generate_proof(&self, _challenged_key: &Key) -> FileProof {
        unimplemented!()
    }

    fn delete_file(&self, _key: &Key) {
        unimplemented!()
    }

    fn get_metadata(&self, _key: &Key) -> Option<Metadata> {
        unimplemented!()
    }

    fn set_metadata(&self, _key: &Key, _metadata: &Metadata) {
        unimplemented!()
    }

    fn get_chunk(&self, _key: &Key, _chunk: u64) -> Option<Chunk> {
        unimplemented!()
    }

    fn write_chunk(&self, _key: &str, _chunk: u64, _data: &Chunk) {
        unimplemented!()
    }
}

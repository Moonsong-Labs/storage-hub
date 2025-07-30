use wasm_bindgen::prelude::*;

use parity_scale_codec::Encode;
use shp_constants::FILE_CHUNK_SIZE;
use shp_file_metadata::{Chunk, ChunkId, ChunkWithId};
use sp_core::{hashing::blake2_256, Hasher};
use sp_trie::{LayoutV1, MemoryDB, TrieDBMutBuilder, TrieMut};
use std::collections::hash_map::DefaultHasher;

// ────────────────────────────────────────────────────────────────────────────
//  NOTE:
//  The full StorageHub `file-manager` crate (`client/file-manager`) depends on a large
//  graph of Substrate-runtime crates that pull in features which cannot be
//  compiled to `wasm32-unknown-unknown` (e.g. `std` I/O, rocksdb, proc-macro
//  helpers, etc.).
//
//  For the SDK we only need the minimal in-memory Merkle-Patricia trie logic
//  (`InMemoryFileDataTrie`) to compute Merkle roots client-side.  Therefore we
//  copy the few small types instead of depending on the heavy client crate.
//  This keeps the WASM package tiny and avoids compilation failures
//  in browser/Node environments.
//
//  TODO: We need to refactor `client/file-manager` in order to remove all the duplicated code
//  and keep this file as a wrapper for Rust <--> TS
// ────────────────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct Blake2b256Hasher;

impl Hasher for Blake2b256Hasher {
    type Out = [u8; 32];
    type StdHasher = DefaultHasher;

    const LENGTH: usize = 32;

    fn hash(data: &[u8]) -> Self::Out {
        blake2_256(data)
    }
}

// Shortcut for the layout we use.
type TrieLayout = LayoutV1<Blake2b256Hasher>;

type HashOut = <Blake2b256Hasher as Hasher>::Out;
struct InMemoryFileDataTrie {
    root: HashOut,
    memdb: MemoryDB<Blake2b256Hasher>,
}

impl InMemoryFileDataTrie {
    fn new() -> Self {
        let (memdb, root) = MemoryDB::<Blake2b256Hasher>::default_with_root();
        Self { root, memdb }
    }

    fn get_root(&self) -> &HashOut {
        &self.root
    }

    fn write_chunk(&mut self, chunk_id: ChunkId, data: &Chunk) {
        let builder = if self.memdb.keys().is_empty() {
            TrieDBMutBuilder::<TrieLayout>::new(&mut self.memdb, &mut self.root)
        } else {
            TrieDBMutBuilder::<TrieLayout>::from_existing(&mut self.memdb, &mut self.root)
        };

        let mut trie = builder.build();
        let value = ChunkWithId {
            chunk_id,
            data: data.clone(),
        }
        .encode();
        trie.insert(&chunk_id.as_trie_key(), &value)
            .expect("in-memory insert cannot fail");
        // committing happens on drop
    }
}

// ────────────────────────────────────────────────────────────────────────────
// WASM‐exposed wrapper for FileTrie
// ────────────────────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub struct FileTrie {
    inner: InMemoryFileDataTrie,
    next_id: u64,
}

#[wasm_bindgen]
impl FileTrie {
    #[wasm_bindgen(constructor)]
    pub fn new() -> FileTrie {
        FileTrie {
            inner: InMemoryFileDataTrie::new(),
            next_id: 0,
        }
    }

    #[wasm_bindgen]
    pub fn push_chunk(&mut self, bytes: &[u8]) {
        let cid = ChunkId::new(self.next_id);
        self.inner.write_chunk(cid, &bytes.to_vec());
        self.next_id += 1;
    }

    /// Current Merkle root as a hex string.
    #[wasm_bindgen(js_name = get_root)]
    pub fn get_root(&self) -> Vec<u8> {
        self.inner.get_root().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! artifact_test {
        ($name:ident, $file:literal, $expected:literal) => {
            #[test]
            fn $name() {
                use std::{fs::File, io::Read, path::Path};
                const EXPECT: &str = $expected;
                let base = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../docker/resource")
                    .join($file);
                let mut trie = FileTrie::new();
                let mut f = File::open(base).unwrap();
                loop {
                    let mut buf = vec![0u8; FILE_CHUNK_SIZE as usize];
                    let n = f.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    trie.push_chunk(&buf[..n]);
                }
                let root_bytes = trie.get_root();
                assert_eq!(hex::encode(root_bytes), EXPECT);
            }
        };
    }

    // NOTE: The following values were taken from test/util/bspNet/consts.ts
    // In order to verify that we have the same result as original Rust code, we add these as unit tests

    artifact_test!(
        merkle_root_adolphus,
        "adolphus.jpg",
        "34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970"
    );
    artifact_test!(
        merkle_root_smile,
        "smile.jpg",
        "535dd863026735ffe0919cc0fc3d8e5da45b9203f01fbf014dbe98005bd8d2fe"
    );
    artifact_test!(
        merkle_root_whatsup,
        "whatsup.jpg",
        "2b83b972e63f52abc0d4146c4aee1f1ec8aa8e274d2ad1b626529446da93736c"
    );
    artifact_test!(
        merkle_root_cloud,
        "cloud.jpg",
        "5559299bc73782b5ad7e9dd57ba01bb06b8c44f5cab8d7afab5e1db2ea93da4c"
    );
    artifact_test!(
        merkle_root_empty,
        "empty-file",
        "03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
    );
    artifact_test!(
        merkle_root_half_chunk,
        "half-chunk-file",
        "ade3ca4ff2151a2533e816eb9402ae17e21160c6c52b1855ecff29faea8880b5"
    );
    artifact_test!(
        merkle_root_one_chunk,
        "one-chunk-file",
        "0904317e4977ad6f872cd9672d2733da9a628fda86ee9add68623a66918cbd8c"
    );
}

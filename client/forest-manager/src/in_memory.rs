use hash_db::Hasher;
use shc_common::types::{FileMetadata, HasherOutT};
use sp_trie::{recorder::Recorder, MemoryDB, TrieDBBuilder, TrieLayout, TrieMut};
use trie_db::{Trie, TrieDBMutBuilder};

use shc_common::types::ForestProof;

use crate::{
    error::{ErrorT, ForestStorageError},
    prove::prove,
    traits::ForestStorage,
};

pub struct InMemoryForestStorage<T: TrieLayout + 'static> {
    pub root: HasherOutT<T>,
    pub memdb: MemoryDB<T::Hash>,
}

impl<T: TrieLayout> InMemoryForestStorage<T> {
    pub fn new() -> Self {
        Self {
            root: Default::default(),
            memdb: MemoryDB::default(),
        }
    }
}

impl<T: TrieLayout> ForestStorage<T> for InMemoryForestStorage<T>
where
    <T::Hash as Hasher>::Out: TryFrom<[u8; 32]>,
{
    fn contains_file_key(&self, file_key: &HasherOutT<T>) -> Result<bool, ErrorT<T>> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();
        Ok(trie.contains(file_key.as_ref())?)
    }

    fn generate_proof(
        &self,
        challenged_file_keys: Vec<HasherOutT<T>>,
    ) -> Result<ForestProof<T>, ErrorT<T>> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Get the proven leaves or leaf
        let proven = challenged_file_keys
            .iter()
            .map(|file_key| prove::<T>(&trie, file_key))
            .collect::<Result<Vec<_>, _>>()?;

        // Drop the `trie_recorder` to release the `self` and `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)?;

        Ok(ForestProof {
            proven,
            proof,
            root: self.root,
        })
    }

    fn insert_metadata(&mut self, metadata: &FileMetadata) -> Result<HasherOutT<T>, ErrorT<T>> where
    {
        let file_key = metadata.key::<T::Hash>();
        if self.contains_file_key(&file_key)? {
            return Err(ForestStorageError::FileKeyAlreadyExists(file_key).into());
        }

        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Insert the file key and metadata into the trie.
        trie.insert(file_key.as_ref(), b"")?;

        Ok(file_key)
    }

    fn delete_file_key(&mut self, file_key: &HasherOutT<T>) -> Result<(), ErrorT<T>> {
        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Remove the file key from the trie.
        let _ = trie.remove(file_key.as_ref())?;

        Ok(())
    }
}

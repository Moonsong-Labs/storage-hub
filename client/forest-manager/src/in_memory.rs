use hash_db::Hasher;
use log::warn;
use sp_trie::{recorder::Recorder, MemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut};
use trie_db::TrieDBMutBuilder;

use common::types::HashT;
use storage_hub_infra::types::{ForestProof, Metadata};

use crate::{
    prove::prove,
    traits::ForestStorage,
    types::{ForestStorageErrors, RawKey},
    utils::{deserialize_value, serialize_value},
};

pub struct InMemoryForestStorage<T: TrieLayout + 'static> {
    pub root: HashT<T>,
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

impl<T: TrieLayout> ForestStorage for InMemoryForestStorage<T> {
    type LookupKey = HashT<T>;
    type RawKey = RawKey<T>;
    type Value = Metadata;

    fn get_file_key(
        &self,
        file_key: &Self::LookupKey,
    ) -> Result<Option<Self::Value>, ForestStorageErrors> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();

        let maybe_metadata = trie
            .get(file_key.as_ref())
            .map_err(|e| {
                warn!(target: "trie", "Failed to get file key: {:?}", e);
                ForestStorageErrors::FailedToGetFileKey
            })?
            .map(|raw_metadata| deserialize_value(&raw_metadata))
            .transpose()?;

        Ok(maybe_metadata)
    }

    fn generate_proof(
        &self,
        challenged_file_keys: &Vec<Self::LookupKey>,
    ) -> Result<ForestProof<Self::RawKey>, ForestStorageErrors> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Get the proven leaves or leaf
        let proven = challenged_file_keys
            .iter()
            .map(|file_key| prove::<T, Self>(&trie, file_key))
            .collect::<Result<Vec<_>, _>>()?;

        // Drop the `trie_recorder` to release the `self` and `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| ForestStorageErrors::FailedToGenerateCompactProof)?;

        Ok(ForestProof {
            proven,
            proof,
            root: self.root.as_ref().to_vec().into(),
        })
    }

    fn insert_file_key(
        &mut self,
        raw_file_key: &Self::RawKey,
        metadata: &Self::Value,
    ) -> Result<Self::LookupKey, ForestStorageErrors> {
        let file_key = <T::Hash as Hasher>::hash(&raw_file_key.key);

        if self.get_file_key(&file_key)?.is_some() {
            return Err(ForestStorageErrors::FileKeyAlreadyExists);
        }

        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Serialize the metadata.
        let raw_metadata = serialize_value(metadata)?;

        // Insert the file key and metadata into the trie.
        trie.insert(file_key.as_ref(), &raw_metadata)
            .map_err(|_| ForestStorageErrors::FailedToInsertFileKey)?;

        Ok(file_key)
    }

    fn delete_file_key(&mut self, file_key: &Self::LookupKey) -> Result<(), ForestStorageErrors> {
        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Remove the file key from the trie.
        let _ = trie
            .remove(file_key.as_ref())
            .map_err(|_| ForestStorageErrors::FailedToRemoveFileKey)?;

        Ok(())
    }
}

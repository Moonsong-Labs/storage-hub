use sp_trie::{recorder::Recorder, MemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut};
use storage_hub_infra::types::{ForestProof, Metadata};
use trie_db::TrieDBMutBuilder;

use crate::{
    prove::prove,
    traits::ForestStorage,
    types::{Errors, HashT},
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

pub struct RawKey<T> {
    key: Vec<u8>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Clone for RawKey<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<T> From<Vec<u8>> for RawKey<T> {
    fn from(key: Vec<u8>) -> Self {
        Self {
            key,
            _phantom: Default::default(),
        }
    }
}

impl<T> AsRef<[u8]> for RawKey<T> {
    fn as_ref(&self) -> &[u8] {
        &self.key
    }
}

impl<T: TrieLayout> ForestStorage for InMemoryForestStorage<T> {
    type LookupKey = HashT<T>;
    type RawKey = RawKey<T>;
    type Value = Metadata;

    fn get_value(&self, file_key: &Self::LookupKey) -> Option<Self::Value> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();

        trie.get(file_key.as_ref())
            .expect("Failed to read storage")
            .map(|raw_metadata| {
                bincode::deserialize(&raw_metadata).expect("Failed to deserialize metadata")
            })
    }

    fn generate_proof(
        &self,
        challenged_file_key: &Self::LookupKey,
    ) -> Result<ForestProof<Self::RawKey>, Errors> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Get the proven leaves or leaf
        let proven = prove::<T, Self>(trie, challenged_file_key)?;

        // Drop the `trie_recorder` to release the `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| Errors::FailedToGenerateCompactProof)?;

        let proven = proven.ok_or_else(|| Errors::FailedToGetLeafOrLeavesToProve)?;

        Ok(ForestProof {
            proven,
            proof,
            root: self
                .root
                .as_ref()
                .try_into()
                .expect("Failed to convert root hash"),
        })
    }

    fn insert_file_key(
        &mut self,
        file_key: &Self::LookupKey,
        metadata: &Self::Value,
    ) -> Result<ForestProof<Self::RawKey>, Errors> {
        if self.get_value(file_key).is_some() {
            return Err(Errors::FileKeyAlreadyExists);
        }

        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Serialize the metadata.
        let raw_metadata = bincode::serialize(metadata).expect("Failed to serialize metadata");

        // Insert the file key and metadata into the trie.
        trie.insert(file_key.as_ref(), &raw_metadata)
            .expect("Failed to write storage");

        // Drop the `trie` to release the self
        drop(trie);

        // Generate a proof that the file key is in the forest.
        self.generate_proof(file_key)
    }

    fn delete_file_key(
        &mut self,
        file_key: &Self::LookupKey,
    ) -> Result<ForestProof<Self::RawKey>, Errors> {
        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Remove the file key from the trie.
        let _ = trie
            .remove(file_key.as_ref())
            .map_err(|_| Errors::FailedToRemoveFileKey)?;

        // Drop the `trie` to release the self
        drop(trie);

        // Generate a proof that the file key is no longer in the forest.
        self.generate_proof(file_key)
    }
}

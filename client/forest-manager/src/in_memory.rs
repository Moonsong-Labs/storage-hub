use sp_core::H256;
use sp_trie::{recorder::Recorder, MemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut};
use trie_db::TrieDBMutBuilder;

use common::types::HashT;
use storage_hub_infra::types::{ForestProof, Metadata};

use crate::{
    prove::prove, traits::ForestStorage, types::ForestStorageErrors, utils::serialize_value,
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

    fn get_value(
        &self,
        file_key: &Self::LookupKey,
    ) -> Result<Option<Self::Value>, ForestStorageErrors> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();

        let maybe_raw_metadata = trie
            .get(file_key.as_ref())
            .map_err(|_| ForestStorageErrors::FailedToGetFileKey)?;
        match maybe_raw_metadata {
            Some(raw_metadata) => {
                let metadata: Self::Value = bincode::deserialize(&raw_metadata)
                    .map_err(|_| ForestStorageErrors::FailedToDeserializeValue)?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
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

        // Drop the `trie_recorder` to release the `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| ForestStorageErrors::FailedToGenerateCompactProof)?;

        Ok(ForestProof {
            proven,
            proof,
            root: H256::from_slice(
                self.root
                    .as_ref()
                    .try_into()
                    .map_err(|_| ForestStorageErrors::FailedToParseRoot)?,
            ),
        })
    }

    fn insert_file_key(
        &mut self,
        file_key: &Self::LookupKey,
        metadata: &Self::Value,
    ) -> Result<(), ForestStorageErrors> {
        if self.get_value(file_key)?.is_some() {
            return Err(ForestStorageErrors::FileKeyAlreadyExists);
        }

        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Serialize the metadata.
        let raw_metadata = serialize_value(metadata)?;

        // Insert the file key and metadata into the trie.
        trie.insert(file_key.as_ref(), &raw_metadata)
            .map_err(|_| ForestStorageErrors::FailedToInsertFileKey)?;

        Ok(())
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

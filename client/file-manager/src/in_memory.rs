use sp_trie::{recorder::Recorder, MemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut};
use std::collections::HashMap;
use trie_db::TrieDBMutBuilder;

use shc_common::types::{
    Chunk, ChunkId, FileKeyProof, FileMetadata, FileProof, HasherOutT, H_LENGTH,
};

use crate::traits::{
    FileDataTrie, FileStorage, FileStorageError, FileStorageWriteError, FileStorageWriteOutcome,
};

pub struct InMemoryFileDataTrie<T: TrieLayout + 'static> {
    root: HasherOutT<T>,
    memdb: MemoryDB<T::Hash>,
}

impl<T: TrieLayout> Default for InMemoryFileDataTrie<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TrieLayout + 'static> InMemoryFileDataTrie<T> {
    fn new() -> Self {
        Self {
            root: Default::default(),
            memdb: MemoryDB::default(),
        }
    }
}

impl<T: TrieLayout> FileDataTrie<T> for InMemoryFileDataTrie<T> {
    fn get_root(&self) -> &HasherOutT<T> {
        &self.root
    }

    fn stored_chunks_count(&self) -> Result<u64, FileStorageError> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();
        let trie_iter = trie
            .iter()
            .map_err(|_| FileStorageError::FailedToConstructTrieIter)?;
        let stored_chunks = trie_iter.count() as u64;
        Ok(stored_chunks)
    }

    fn generate_proof(&self, chunk_ids: &Vec<ChunkId>) -> Result<FileProof, FileStorageError> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Read all the chunks to prove from the trie.
        let mut chunks = Vec::new();
        for chunk_id in chunk_ids {
            let chunk: Option<Vec<u8>> = trie
                .get(&chunk_id.as_trie_key())
                .map_err(|_| FileStorageError::FailedToGetFileChunk)?;

            let chunk = chunk.ok_or(FileStorageError::FileChunkDoesNotExist)?;
            chunks.push((*chunk_id, chunk));
        }

        // Drop the `trie_recorder` to release the `recorder`
        drop(trie_recorder);

        // Generate proof
        let proof = recorder
            .drain_storage_proof()
            .to_compact_proof::<T::Hash>(self.root)
            .map_err(|_| FileStorageError::FailedToGenerateCompactProof)?;

        Ok(FileProof {
            proof: proof.into(),
            fingerprint: self.get_root().as_ref().into(),
        })
    }

    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();

        trie.get(&chunk_id.as_trie_key())
            .map_err(|_| FileStorageError::FailedToGetFileChunk)?
            .ok_or(FileStorageError::FileChunkDoesNotExist)
    }

    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError> {
        let mut trie = if self.memdb.keys().is_empty() {
            // If the database is empty, create a new trie.
            TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build()
        } else {
            // If the database is not empty, build the trie from an existing root and memdb.
            TrieDBMutBuilder::<T>::from_existing(&mut self.memdb, &mut self.root).build()
        };

        // Check that we don't have a chunk already stored.
        if trie
            .contains(&chunk_id.as_trie_key())
            .map_err(|_| FileStorageWriteError::FailedToGetFileChunk)?
        {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the chunk into the file trie.
        trie.insert(&chunk_id.as_trie_key(), &data)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;

        // dropping the trie automatically commits changes to the underlying db
        drop(trie);

        Ok(())
    }

    fn delete(&mut self) -> Result<(), FileStorageError> {
        self.root = HasherOutT::<T>::default();
        self.memdb.clear();

        Ok(())
    }
}

pub struct InMemoryFileStorage<T: TrieLayout + 'static>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub metadata: HashMap<HasherOutT<T>, FileMetadata>,
    pub file_data: HashMap<HasherOutT<T>, InMemoryFileDataTrie<T>>,
}

impl<T: TrieLayout> InMemoryFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            file_data: HashMap::new(),
        }
    }
}

impl<T: TrieLayout + 'static> FileStorage<T> for InMemoryFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    type FileDataTrie = InMemoryFileDataTrie<T>;

    fn generate_proof(
        &self,
        file_key: &HasherOutT<T>,
        chunk_id: &Vec<ChunkId>,
    ) -> Result<FileKeyProof, FileStorageError> {
        let metadata = self
            .metadata
            .get(file_key)
            .ok_or(FileStorageError::FileDoesNotExist)?;

        let file_data = self.file_data.get(file_key).expect(
            format!(
                "Invariant broken! Metadata for file key {:?} found but no associated trie",
                file_key
            )
            .as_str(),
        );

        let stored_chunks = file_data.stored_chunks_count()?;
        if metadata.chunks_count() != stored_chunks {
            return Err(FileStorageError::IncompleteFile);
        }

        if metadata.fingerprint
            != file_data
                .root
                .as_ref()
                .try_into()
                .expect("Hasher output mismatch!")
        {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        Ok(file_data
            .generate_proof(chunk_id)?
            .to_file_key_proof(metadata.clone()))
    }

    fn delete_file(&mut self, file_key: &HasherOutT<T>) -> Result<(), FileStorageError> {
        // No need to return any errors here since
        // removing from internal HashMap shouldn't fail.
        self.metadata.remove(file_key);
        self.file_data.remove(file_key);

        Ok(())
    }

    fn get_metadata(&self, file_key: &HasherOutT<T>) -> Result<FileMetadata, FileStorageError> {
        self.metadata
            .get(file_key)
            .cloned()
            .ok_or(FileStorageError::FileDoesNotExist)
    }

    fn insert_file(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
    ) -> Result<(), FileStorageError> {
        if self.metadata.contains_key(&key) {
            return Err(FileStorageError::FileAlreadyExists);
        }
        self.metadata.insert(key, metadata);

        let previous = self.file_data.insert(key, InMemoryFileDataTrie::default());
        if previous.is_some() {
            panic!("Key already associated with File Data, but not with File Metadata. Possible inconsistency between them.");
        }

        Ok(())
    }

    fn insert_file_with_data(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
        file_data: Self::FileDataTrie,
    ) -> Result<(), FileStorageError> {
        if self.metadata.contains_key(&key) {
            return Err(FileStorageError::FileAlreadyExists);
        }
        self.metadata.insert(key, metadata);

        let previous = self.file_data.insert(key, file_data);
        if previous.is_some() {
            panic!("Key already associated with File Data, but not with File Metadata. Possible inconsistency between them.");
        }

        Ok(())
    }

    fn get_chunk(
        &self,
        file_key: &HasherOutT<T>,
        chunk_id: &ChunkId,
    ) -> Result<Chunk, FileStorageError> {
        let file_data = self.file_data.get(file_key);
        let file_data = file_data.ok_or(FileStorageError::FileDoesNotExist)?;

        file_data.get_chunk(chunk_id)
    }

    fn write_chunk(
        &mut self,
        file_key: &HasherOutT<T>,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<FileStorageWriteOutcome, FileStorageWriteError> {
        let file_data = self
            .file_data
            .get_mut(file_key)
            .ok_or(FileStorageWriteError::FileDoesNotExist)?;

        file_data.write_chunk(chunk_id, data)?;

        let metadata = self.metadata.get(file_key).expect(
            format!("Key {:?} already associated with File Trie, but no File Metadata. Possible inconsistency between them.",
            file_key
        )
            .as_str(),
        );

        // Check if we have all the chunks for the file.
        let stored_chunks = file_data
            .stored_chunks_count()
            .map_err(|_| FileStorageWriteError::FailedToConstructTrieIter)?;
        if metadata.chunks_count() != stored_chunks {
            return Ok(FileStorageWriteOutcome::FileIncomplete);
        }

        // If we have all the chunks, check if the file metadata fingerprint and the file trie
        // root matches.
        if metadata.fingerprint != file_data.root.as_ref().into() {
            return Err(FileStorageWriteError::FingerprintAndStoredFileMismatch);
        }

        Ok(FileStorageWriteOutcome::FileComplete)
    }
}

mod tests {
    use shc_common::types::FILE_CHUNK_SIZE;
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_trie::LayoutV1;
    use trie_db::TrieHash;

    use super::*;

    #[test]
    #[ignore]
    fn file_trie_creating_empty_works() {
        let file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        // expected hash is the root hash of an empty tree.
        let expected_hash = TrieHash::<LayoutV1<BlakeTwo256>>::try_from([
            3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231,
            135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
        ])
        .unwrap();

        assert_eq!(
            H256::from(*file_trie.get_root()),
            expected_hash,
            "Root should be initialized to default."
        );
    }

    #[test]
    #[ignore]
    fn file_trie_writing_chunk_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        let old_root = file_trie.get_root().clone();
        file_trie
            .write_chunk(&ChunkId::from(0u64), &Chunk::from([1u8; 1024]))
            .unwrap();
        let new_root = file_trie.get_root();
        assert_ne!(&old_root, new_root);

        let chunk = file_trie.get_chunk(&ChunkId::from(0u64)).unwrap();
        assert_eq!(chunk.as_slice(), [1u8; 1024]);
    }

    #[test]
    #[ignore]
    fn file_trie_getting_chunk_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        let chunk = Chunk::from([3u8; 1024]);
        let chunk_id: ChunkId = 3;
        file_trie.write_chunk(&chunk_id, &chunk).unwrap();
        let chunk = file_trie.get_chunk(&chunk_id).unwrap();
        assert_eq!(chunk.as_slice(), [3u8; 1024]);
    }

    #[test]
    #[ignore]
    fn file_trie_getting_stored_chunks_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        let chunk_ids = vec![ChunkId::from(0u64), ChunkId::from(1u64)];
        let chunks = vec![Chunk::from([0u8; 1024]), Chunk::from([1u8; 1024])];

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());
    }

    #[test]
    #[ignore]
    fn file_trie_generating_proof_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        let chunk_ids = vec![
            ChunkId::from(0u64),
            ChunkId::from(1u64),
            ChunkId::from(2u64),
        ];

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_proof = file_trie.generate_proof(&chunk_ids).unwrap();
        let proven_leaves = file_proof.proven;
        for (id, leaf) in proven_leaves.iter().enumerate() {
            assert_eq!(chunk_ids[id], leaf.key);
            assert_eq!(chunks[id], leaf.data);
        }
    }

    #[test]
    #[ignore]
    fn file_trie_deleting_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        let chunk_ids = vec![
            ChunkId::from(0u64),
            ChunkId::from(1u64),
            ChunkId::from(2u64),
        ];

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(file_trie.stored_chunks_count(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        file_trie.delete().unwrap();

        assert_eq!(file_trie.stored_chunks_count(), 0);
    }

    #[test]
    fn inserting_whole_file_works() {}

    #[test]
    fn deleting_whole_file_works() {}

    #[test]
    fn proof_generation_works() {}
}

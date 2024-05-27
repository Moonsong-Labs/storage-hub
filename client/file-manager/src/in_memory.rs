use std::collections::HashMap;

use shc_common::types::{Chunk, ChunkId, FileMetadata, FileProof, HasherOutT, Leaf};

use sp_trie::{recorder::Recorder, MemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut};
use trie_db::TrieDBMutBuilder;

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

    fn stored_chunks_count(&self) -> u64 {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();
        let stored_chunks = trie.key_iter().iter().count() as u64;
        stored_chunks
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
                .get(&chunk_id.to_be_bytes())
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

        // Convert the chunks to prove into `Leaf`s.
        let leaves = chunks
            .into_iter()
            .map(|(id, chunk)| Leaf {
                key: id,
                data: chunk,
            })
            .collect();

        Ok(FileProof {
            proven: leaves,
            proof: proof.into(),
            root: self.get_root().as_ref().into(),
        })
    }

    fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Chunk, FileStorageError> {
        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root).build();

        trie.get(&chunk_id.to_be_bytes())
            .map_err(|_| FileStorageError::FailedToGetFileChunk)?
            .ok_or(FileStorageError::FileChunkDoesNotExist)
    }

    fn write_chunk(
        &mut self,
        chunk_id: &ChunkId,
        data: &Chunk,
    ) -> Result<(), FileStorageWriteError> {
        let mut trie = TrieDBMutBuilder::<T>::new(&mut self.memdb, &mut self.root).build();

        // Check that we don't have a chunk already stored.
        if trie
            .contains(&chunk_id.to_be_bytes())
            .map_err(|_| FileStorageWriteError::FailedToGetFileChunk)?
        {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the chunk into the file trie.
        trie.insert(&chunk_id.to_be_bytes(), &data)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;

        drop(trie);

        Ok(())
    }
}

pub struct InMemoryFileStorage<T: TrieLayout + 'static> {
    pub metadata: HashMap<HasherOutT<T>, FileMetadata>,
    pub file_data: HashMap<HasherOutT<T>, InMemoryFileDataTrie<T>>,
}

impl<T: TrieLayout> InMemoryFileStorage<T> {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            file_data: HashMap::new(),
        }
    }
}

impl<T: TrieLayout + 'static> FileStorage<T> for InMemoryFileStorage<T> {
    type FileDataTrie = InMemoryFileDataTrie<T>;

    fn generate_proof(
        &self,
        file_key: &HasherOutT<T>,
        chunk_id: &Vec<ChunkId>,
    ) -> Result<FileProof, FileStorageError> {
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

        if metadata.chunk_count() != file_data.stored_chunks_count() {
            return Err(FileStorageError::IncompleteFile);
        }

        if metadata.fingerprint != file_data.root.as_ref().into() {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        file_data.generate_proof(chunk_id)
    }

    fn delete_file(&mut self, file_key: &HasherOutT<T>) {
        self.metadata.remove(file_key);
        self.file_data.remove(file_key);
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
            panic!("Invariant broken! Inconsistent metadata and file data storage.");
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
            panic!("Invariant broken! Inconsistent metadata and file data storage.");
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

        let mut trie =
            TrieDBMutBuilder::<T>::new(&mut file_data.memdb, &mut file_data.root).build();

        // Check that we don't have a chunk already stored.
        if trie
            .contains(&chunk_id.to_be_bytes())
            .map_err(|_| FileStorageWriteError::FailedToGetFileChunk)?
        {
            return Err(FileStorageWriteError::FileChunkAlreadyExists);
        }

        // Insert the chunk into the file trie.
        trie.insert(&chunk_id.to_be_bytes(), &data)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;
        drop(trie);

        file_data.write_chunk(chunk_id, data)?;

        let metadata = self.metadata.get(file_key).expect(
            format!(
            "Invariant broken! Metadata for file key {:?} not found but associated trie is present",
            file_key
        )
            .as_str(),
        );

        // Check if we have all the chunks for the file.
        if metadata.chunk_count() != file_data.stored_chunks_count() {
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

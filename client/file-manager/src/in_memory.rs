use codec::{Decode, Encode};
use log::{debug, error, info};
use sp_trie::{recorder::Recorder, MemoryDB, Trie, TrieDBBuilder, TrieLayout, TrieMut};
use std::collections::{HashMap, HashSet};
use trie_db::TrieDBMutBuilder;

use shc_common::types::{
    Chunk, ChunkId, ChunkWithId, FileKeyProof, FileMetadata, FileProof, HashT, HasherOutT, H_LENGTH,
};

use crate::{
    traits::{
        ExcludeType, FileDataTrie, FileStorage, FileStorageError, FileStorageWriteError,
        FileStorageWriteOutcome,
    },
    LOG_TARGET,
};

pub struct InMemoryFileDataTrie<T: TrieLayout + 'static> {
    root: HasherOutT<T>,
    memdb: MemoryDB<T::Hash>,
}

impl<T: TrieLayout + 'static> InMemoryFileDataTrie<T> {
    pub fn new() -> Self {
        let (memdb, root) = MemoryDB::<HashT<T>>::default_with_root();

        Self { root, memdb }
    }
}

impl<T: TrieLayout> FileDataTrie<T> for InMemoryFileDataTrie<T> {
    fn get_root(&self) -> &HasherOutT<T> {
        &self.root
    }

    fn generate_proof(&self, chunk_ids: &HashSet<ChunkId>) -> Result<FileProof, FileStorageError> {
        let recorder: Recorder<T::Hash> = Recorder::default();

        // A `TrieRecorder` is needed to create a proof of the "visited" leafs, by the end of this process.
        let mut trie_recorder = recorder.as_trie_recorder(self.root);

        let trie = TrieDBBuilder::<T>::new(&self.memdb, &self.root)
            .with_recorder(&mut trie_recorder)
            .build();

        // Read all the chunks to prove from the trie.
        let mut chunks = Vec::new();
        for chunk_id in chunk_ids {
            // Get the encoded chunk from the trie.
            let encoded_chunk: Vec<u8> = trie
                .get(&chunk_id.as_trie_key())
                .map_err(|_| FileStorageError::FailedToGetFileChunk)?
                .ok_or(FileStorageError::FileChunkDoesNotExist)?;

            // Decode it to its chunk ID and data.
            let decoded_chunk = ChunkWithId::decode(&mut encoded_chunk.as_slice())
                .map_err(|_| FileStorageError::FailedToParseChunkWithId)?;

            chunks.push((decoded_chunk.chunk_id, decoded_chunk.data));
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

        // Get the encoded chunk from the trie.
        let encoded_chunk = trie
            .get(&chunk_id.as_trie_key())
            .map_err(|_| FileStorageError::FailedToGetFileChunk)?
            .ok_or(FileStorageError::FileChunkDoesNotExist)?;

        // Decode it to its chunk ID and data.
        let decoded_chunk = ChunkWithId::decode(&mut encoded_chunk.as_slice())
            .map_err(|_| FileStorageError::FailedToParseChunkWithId)?;

        // Return the chunk data.
        Ok(decoded_chunk.data)
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

        // Insert the encoded chunk with its ID into the file trie.
        let decoded_chunk = ChunkWithId {
            chunk_id: *chunk_id,
            data: data.clone(),
        };
        let encoded_chunk = decoded_chunk.encode();
        trie.insert(&chunk_id.as_trie_key(), &encoded_chunk)
            .map_err(|_| FileStorageWriteError::FailedToInsertFileChunk)?;

        // dropping the trie automatically commits changes to the underlying db
        drop(trie);

        Ok(())
    }

    fn delete(&mut self) -> Result<(), FileStorageWriteError> {
        let (memdb, root) = MemoryDB::<HashT<T>>::default_with_root();
        self.root = root;
        self.memdb = memdb;

        Ok(())
    }
}

pub struct InMemoryFileStorage<T: TrieLayout + 'static>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub metadata: HashMap<HasherOutT<T>, FileMetadata>,
    pub file_data: HashMap<HasherOutT<T>, InMemoryFileDataTrie<T>>,
    pub bucket_prefix_map: HashSet<[u8; 64]>,
    pub exclude_list: HashMap<ExcludeType, HashSet<HasherOutT<T>>>,
    pub chunk_bitmaps: HashMap<HasherOutT<T>, Vec<u8>>,
}

impl<T: TrieLayout> InMemoryFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    pub fn new() -> Self {
        let mut exclude_list: HashMap<ExcludeType, HashSet<HasherOutT<T>>> = HashMap::new();

        // Initialize our exclude list for each type of value we want to exclude
        exclude_list.insert(ExcludeType::File, HashSet::new());
        exclude_list.insert(ExcludeType::User, HashSet::new());
        exclude_list.insert(ExcludeType::Bucket, HashSet::new());
        exclude_list.insert(ExcludeType::Fingerprint, HashSet::new());

        Self {
            metadata: HashMap::new(),
            file_data: HashMap::new(),
            bucket_prefix_map: HashSet::new(),
            exclude_list,
            chunk_bitmaps: HashMap::new(),
        }
    }

    /// Check if a chunk is present in file storage for this file key.
    ///
    /// Returns `true` if the chunk has already been stored (byte is non-zero in the bitmap),
    /// `false` otherwise (including if the bitmap doesn't exist yet).
    fn is_chunk_present(&self, file_key: &HasherOutT<T>, chunk_id: u64) -> bool {
        if let Some(bitmap) = self.chunk_bitmaps.get(file_key) {
            let index = chunk_id as usize;
            index < bitmap.len() && bitmap[index] != 0
        } else {
            false
        }
    }

    /// Mark a chunk as present in the bitmap for this file key.
    ///
    /// Extends the bitmap if necessary to hold the chunk index.
    fn mark_chunk_present(
        &mut self,
        file_key: &HasherOutT<T>,
        chunk_id: u64,
    ) -> Result<(), FileStorageWriteError> {
        let chunk_idx = chunk_id as usize;
        let bitmap = self
            .chunk_bitmaps
            .get_mut(file_key)
            .ok_or(FileStorageWriteError::FailedToGetStoredChunksCount)?;

        // Extend it if necessary to hold this chunk_id
        if chunk_idx >= bitmap.len() {
            bitmap.resize(chunk_idx + 1, 0);
        }

        // Mark the chunk as present
        bitmap[chunk_idx] = 1;

        Ok(())
    }
}

impl<T: TrieLayout + 'static> FileStorage<T> for InMemoryFileStorage<T>
where
    HasherOutT<T>: TryFrom<[u8; H_LENGTH]>,
{
    type FileDataTrie = InMemoryFileDataTrie<T>;

    fn new_file_data_trie(&self) -> Self::FileDataTrie {
        InMemoryFileDataTrie::new()
    }

    fn generate_proof(
        &self,
        file_key: &HasherOutT<T>,
        chunk_ids: &HashSet<ChunkId>,
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

        let stored_chunks = self.stored_chunks_count(file_key)?;
        if metadata.chunks_count() != stored_chunks {
            return Err(FileStorageError::IncompleteFile);
        }

        if metadata.fingerprint() != file_data.get_root().as_ref() {
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        file_data
            .generate_proof(chunk_ids)?
            .to_file_key_proof(metadata.clone())
            .map_err(|e| {
                error!(target: LOG_TARGET, "{:?}", e);
                FileStorageError::FailedToConstructFileKeyProof
            })
    }

    fn stored_chunks_count(&self, key: &HasherOutT<T>) -> Result<u64, FileStorageError> {
        let bitmap = self
            .chunk_bitmaps
            .get(key)
            .ok_or(FileStorageError::FileDoesNotExist)?;
        Ok(bitmap.iter().filter(|&&b| b != 0).count() as u64)
    }

    fn delete_file(&mut self, key: &HasherOutT<T>) -> Result<(), FileStorageError> {
        self.metadata.remove(key);
        self.file_data.remove(key);
        self.chunk_bitmaps.remove(key);

        Ok(())
    }

    fn get_metadata(
        &self,
        file_key: &HasherOutT<T>,
    ) -> Result<Option<FileMetadata>, FileStorageError> {
        Ok(self.metadata.get(file_key).cloned())
    }

    fn is_file_complete(&self, key: &HasherOutT<T>) -> Result<bool, FileStorageError> {
        // Get the metadata of the file
        let metadata = self
            .metadata
            .get(key)
            .ok_or(FileStorageError::FileDoesNotExist)?;

        // Get the file data trie that's currently stored in memory
        let file_data = self.file_data.get(key).expect(
            format!(
                "Invariant broken! Metadata for file key {:?} found but no associated trie",
                key
            )
            .as_str(),
        );

        // Get the number of stored chunks for the file
        let stored_chunks = self.stored_chunks_count(key)?;

        // Check if the number of stored chunks is equal to the number of chunks in the metadata
        let are_chunks_complete = metadata.chunks_count() == stored_chunks;

        // Check if the fingerprint of the file is the same as the fingerprint of the file data trie
        let are_fingerprints_equal = metadata.fingerprint() == file_data.get_root().as_ref();

        // If these conditions don't match, error out since we have a critical inconsistency
        // The logic behind this is that:
        // - If the chunks are complete, then the fingerprint should be the same as the fingerprint of the file data trie
        // - If the chunks are not complete, then the fingerprint should not be the same as the fingerprint of the file data trie
        if are_chunks_complete != are_fingerprints_equal {
            error!(target: LOG_TARGET, "Inconsistency detected for file key {:?}", key);
            error!(target: LOG_TARGET, "Stored chunks: {stored_chunks}. Metadata chunks: {}", metadata.chunks_count());
            error!(target: LOG_TARGET, "Stored fingerprint: {:?}. Metadata fingerprint: {:x}", file_data.get_root(), metadata.fingerprint());
            return Err(FileStorageError::FingerprintAndStoredFileMismatch);
        }

        // We can safely return the result of the chunks count check since we have already checked the fingerprint
        Ok(are_chunks_complete)
    }

    fn insert_file(
        &mut self,
        key: HasherOutT<T>,
        metadata: FileMetadata,
    ) -> Result<(), FileStorageError> {
        if self.metadata.contains_key(&key) {
            return Err(FileStorageError::FileAlreadyExists);
        }
        self.metadata.insert(key, metadata.clone());

        let empty_file_trie = self.new_file_data_trie();
        let previous = self.file_data.insert(key, empty_file_trie);
        if previous.is_some() {
            panic!("Key already associated with File Data, but not with File Metadata. Possible inconsistency between them.");
        }

        // Initialize empty bitmap
        self.chunk_bitmaps.insert(key, Vec::new());

        let full_key = [metadata.bucket_id().as_slice(), key.as_ref()].concat();
        self.bucket_prefix_map.insert(full_key.try_into().unwrap());

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
        self.metadata.insert(key, metadata.clone());

        // To build the bitmap, we need to collect all the chunk IDs
        let trie = TrieDBBuilder::<T>::new(&file_data.memdb, &file_data.get_root()).build();
        let mut chunk_ids = Vec::new();
        for item in trie
            .iter()
            .map_err(|_| FileStorageError::FailedToConstructTrieIter)?
        {
            let (key_bytes, _value) = item.map_err(|_| FileStorageError::FailedToReadStorage)?;
            let chunk_id = ChunkId::from_trie_key(&key_bytes)
                .map_err(|_| FileStorageError::FailedToReadStorage)?;
            chunk_ids.push(chunk_id.as_u64());
        }

        // Create a bitmap with all the received chunk IDs marked as present
        let bitmap = if !chunk_ids.is_empty() {
            let max_id = *chunk_ids.iter().max().unwrap();
            let bitmap_size = (max_id + 1) as usize;
            let mut bitmap = vec![0u8; bitmap_size];

            for chunk_id in chunk_ids {
                bitmap[chunk_id as usize] = 1;
            }

            bitmap
        } else {
            Vec::new()
        };

        self.chunk_bitmaps.insert(key, bitmap);

        let previous = self.file_data.insert(key, file_data);
        if previous.is_some() {
            panic!("Key already associated with File Data, but not with File Metadata. Possible inconsistency between them.");
        }

        let full_key = [metadata.bucket_id().as_slice(), key.as_ref()].concat();
        self.bucket_prefix_map.insert(full_key.try_into().unwrap());

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
        // If the chunk is already present in storage, return early to skip all expensive trie operations
        if self.is_chunk_present(file_key, chunk_id.as_u64()) {
            debug!(
                target: LOG_TARGET,
                "Chunk {:?} already present for file key {:?}, skipping write",
                chunk_id,
                file_key
            );
            return match self.is_file_complete(file_key) {
                Ok(true) => Ok(FileStorageWriteOutcome::FileComplete),
                Ok(false) => Ok(FileStorageWriteOutcome::FileIncomplete),
                Err(e) => Err(FileStorageWriteError::FailedToCheckFileCompletion(e)),
            };
        }

        let file_data = self
            .file_data
            .get_mut(file_key)
            .ok_or(FileStorageWriteError::FileDoesNotExist)?;

        match file_data.write_chunk(chunk_id, data) {
            Ok(()) => {
                // Chunk was successfully inserted into trie
                debug!(target: LOG_TARGET, "Chunk {:?} successfully written to trie for file key {:?}", chunk_id, file_key);
            }
            Err(FileStorageWriteError::FileChunkAlreadyExists) => {
                // Chunk already exists in trie - no need to update trie
                debug!(target: LOG_TARGET, "Chunk {:?} already exists in trie for file key {:?}", chunk_id, file_key);
            }
            Err(other) => {
                error!(target: LOG_TARGET, "Error while writing chunk {:?} of file key {:?}: {:?}", chunk_id, file_key, other);
                return Err(FileStorageWriteError::FailedToInsertFileChunk);
            }
        }

        // Mark this chunk as present for this file key
        self.mark_chunk_present(file_key, chunk_id.as_u64())?;

        // Check if file is complete using the helper method (only once at the end)
        match self.is_file_complete(file_key) {
            Ok(true) => Ok(FileStorageWriteOutcome::FileComplete),
            Ok(false) => Ok(FileStorageWriteOutcome::FileIncomplete),
            Err(e) => {
                error!(target: LOG_TARGET, "Failed to check file completion status for file key {:?}: {:?}", file_key, e);
                Err(FileStorageWriteError::FailedToCheckFileCompletion(e))
            }
        }
    }

    fn delete_files_with_prefix(&mut self, prefix: &[u8; 32]) -> Result<(), FileStorageError>
    where
        HasherOutT<T>: TryFrom<[u8; 32]>,
    {
        let keys_to_delete: Vec<HasherOutT<T>> = self
            .bucket_prefix_map
            .iter()
            .filter_map(|full_key| {
                if full_key.starts_with(prefix) {
                    let key: [u8; 32] = full_key[32..].try_into().unwrap();
                    Some(
                        key.try_into()
                            .map_err(|_| FileStorageError::FailedToParseKey)
                            .unwrap(),
                    )
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_delete {
            self.metadata.remove(&key);
            self.file_data.remove(&key);
            self.chunk_bitmaps.remove(&key);
        }

        Ok(())
    }

    fn is_allowed(
        &self,
        key: &HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<bool, FileStorageError> {
        let exclude_list = match self.exclude_list.get(&exclude_type) {
            Some(list) => list,
            None => return Ok(true),
        };

        if exclude_list.contains(key) {
            return Ok(false);
        }

        return Ok(true);
    }

    fn add_to_exclude_list(
        &mut self,
        key: HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<(), FileStorageError> {
        match self.exclude_list.get_mut(&exclude_type) {
            Some(list) => list.insert(key),
            None => return Err(FileStorageError::FailedToAddEntityToExcludeList),
        };

        info!("Key added to the exclude list : {:?}", key);
        Ok(())
    }

    fn remove_from_exclude_list(
        &mut self,
        key: &HasherOutT<T>,
        exclude_type: ExcludeType,
    ) -> Result<(), FileStorageError> {
        match self.exclude_list.get_mut(&exclude_type) {
            Some(list) => list.remove(key),
            None => return Err(FileStorageError::FailedToAddEntityToExcludeList),
        };
        info!("Key removed to the exclude list : {:?}", key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shc_common::types::FILE_CHUNK_SIZE;
    use sp_core::H256;
    use sp_runtime::traits::BlakeTwo256;
    use sp_runtime::AccountId32;
    use sp_trie::LayoutV1;

    fn stored_chunks_count(
        file_trie: &InMemoryFileDataTrie<LayoutV1<BlakeTwo256>>,
    ) -> Result<u64, FileStorageError> {
        let trie =
            TrieDBBuilder::<LayoutV1<BlakeTwo256>>::new(&file_trie.memdb, &file_trie.root).build();
        let trie_iter = trie
            .iter()
            .map_err(|_| FileStorageError::FailedToConstructTrieIter)?;
        let stored_chunks = trie_iter.count() as u64;

        Ok(stored_chunks)
    }

    #[test]
    fn file_trie_create_empty_works() {
        let file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        // expected hash is the root hash of an empty tree.
        let expected_hash = HasherOutT::<LayoutV1<BlakeTwo256>>::try_from([
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
    fn file_trie_write_chunk_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
        let old_root = *file_trie.get_root();
        file_trie
            .write_chunk(&ChunkId::new(0u64), &Chunk::from([1u8; 1024]))
            .unwrap();
        let new_root = file_trie.get_root();
        assert_ne!(&old_root, new_root);

        let chunk = file_trie.get_chunk(&ChunkId::new(0u64)).unwrap();
        assert_eq!(chunk.as_slice(), [1u8; 1024]);
    }

    #[test]
    fn file_trie_get_chunk_works() {
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        let chunk = Chunk::from([3u8; 32]);
        let chunk_id = ChunkId::new(3);
        file_trie.write_chunk(&chunk_id, &chunk).unwrap();
        let chunk = file_trie.get_chunk(&chunk_id).unwrap();
        assert_eq!(chunk.as_slice(), [3u8; 32]);
    }

    #[test]
    fn file_trie_stored_chunks_count_works() {
        let chunk_ids = vec![ChunkId::new(0u64), ChunkId::new(1u64)];
        let chunks = vec![Chunk::from([0u8; 1024]), Chunk::from([1u8; 1024])];
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());
    }

    #[test]
    fn file_trie_generate_proof_works() {
        let chunk_ids = vec![ChunkId::new(0u64), ChunkId::new(1u64), ChunkId::new(2u64)];
        let chunk_ids_set: HashSet<ChunkId> = chunk_ids.iter().cloned().collect();
        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_proof = file_trie.generate_proof(&chunk_ids_set).unwrap();

        assert_eq!(
            file_proof.fingerprint.as_ref(),
            file_trie.get_root().as_ref()
        );
    }

    #[test]
    fn file_trie_delete_works() {
        let chunk_ids = vec![ChunkId::new(0u64), ChunkId::new(1u64), ChunkId::new(2u64)];

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        file_trie.delete().unwrap();
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_err());
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_err());
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_err());

        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 0);
    }

    #[test]
    fn file_storage_insert_file_works() {
        let chunks = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();

        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            32u64 * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();

        assert!(file_storage.get_metadata(&key).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[0]).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[1]).is_ok());
        assert!(file_storage.get_chunk(&key, &chunk_ids[2]).is_ok());
    }

    #[test]
    fn file_storage_delete_file_works() {
        let chunks = vec![
            Chunk::from([5u8; 32]),
            Chunk::from([6u8; 32]),
            Chunk::from([7u8; 32]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            32u64 * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();
        assert!(file_storage.get_metadata(&key).is_ok());

        assert!(file_storage.delete_file(&key).is_ok());

        // Should get a None option here when trying to get File Metadata.
        assert!(file_storage
            .get_metadata(&key)
            .is_ok_and(|metadata| metadata.is_none()));
        assert!(file_storage.get_chunk(&key, &chunk_ids[0]).is_err());
        assert!(file_storage.get_chunk(&key, &chunk_ids[1]).is_err());
        assert!(file_storage.get_chunk(&key, &chunk_ids[2]).is_err());
    }

    #[test]
    fn file_storage_generate_proof_works() {
        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();
        let chunk_ids_set: HashSet<ChunkId> = chunk_ids.iter().cloned().collect();

        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
        file_trie.write_chunk(&chunk_ids[0], &chunks[0]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 1);
        assert!(file_trie.get_chunk(&chunk_ids[0]).is_ok());

        file_trie.write_chunk(&chunk_ids[1], &chunks[1]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 2);
        assert!(file_trie.get_chunk(&chunk_ids[1]).is_ok());

        file_trie.write_chunk(&chunk_ids[2], &chunks[2]).unwrap();
        assert_eq!(stored_chunks_count(&file_trie).unwrap(), 3);
        assert!(file_trie.get_chunk(&chunk_ids[2]).is_ok());

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            1024u64 * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

        let key = file_metadata.file_key::<BlakeTwo256>();
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();

        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();

        assert!(file_storage.get_metadata(&key).is_ok());

        let file_proof = file_storage.generate_proof(&key, &chunk_ids_set).unwrap();
        let proven_leaves = file_proof.proven::<LayoutV1<BlakeTwo256>>().unwrap();
        for (id, leaf) in proven_leaves.iter().enumerate() {
            assert_eq!(chunk_ids[id], leaf.key);
            assert_eq!(chunks[id], leaf.data);
        }
    }

    #[test]
    fn delete_files_with_prefix_works() {
        fn create_file_data_trie(
            chunks: &Vec<Chunk>,
        ) -> InMemoryFileDataTrie<LayoutV1<BlakeTwo256>> {
            let chunk_ids: Vec<ChunkId> = chunks
                .iter()
                .enumerate()
                .map(|(id, _)| ChunkId::new(id as u64))
                .collect();

            let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
            for (chunk_id, chunk) in chunk_ids.iter().zip(chunks) {
                file_trie.write_chunk(chunk_id, chunk).unwrap();
            }

            file_trie
        }

        fn create_file_metadata(
            file_trie: &InMemoryFileDataTrie<LayoutV1<BlakeTwo256>>,
            location: &str,
            bucket_id: [u8; 32],
        ) -> FileMetadata {
            FileMetadata::new(
                <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
                bucket_id.to_vec(),
                location.to_string().into_bytes(),
                1024u64 * 3,
                file_trie.get_root().as_ref().into(),
            )
            .unwrap()
        }

        let chunks = vec![
            Chunk::from([0u8; 1024]),
            Chunk::from([1u8; 1024]),
            Chunk::from([2u8; 1024]),
        ];

        let file_trie_1 = create_file_data_trie(&chunks);
        let file_metadata_1 = create_file_metadata(&file_trie_1, "location_1", [1u8; 32]);
        let file_key_1 = file_metadata_1.file_key::<BlakeTwo256>();

        let file_trie_2 = create_file_data_trie(&chunks);
        let file_metadata_2 = create_file_metadata(&file_trie_2, "location_2", [2u8; 32]);
        let file_key_2 = file_metadata_2.file_key::<BlakeTwo256>();

        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();

        file_storage
            .insert_file_with_data(file_key_1, file_metadata_1, file_trie_1)
            .unwrap();
        file_storage
            .insert_file_with_data(file_key_2, file_metadata_2, file_trie_2)
            .unwrap();

        assert!(file_storage.get_metadata(&file_key_1).is_ok());
        assert!(file_storage.get_metadata(&file_key_2).is_ok());

        let prefix = [1u8; 32].to_vec();
        file_storage
            .delete_files_with_prefix(prefix.as_slice().try_into().unwrap())
            .unwrap();

        assert!(file_storage
            .get_metadata(&file_key_1)
            .is_ok_and(|metadata| metadata.is_none()));
        assert!(file_storage
            .get_chunk(&file_key_1, &ChunkId::new(0u64))
            .is_err());
        assert!(file_storage
            .get_chunk(&file_key_1, &ChunkId::new(1u64))
            .is_err());
        assert!(file_storage
            .get_chunk(&file_key_1, &ChunkId::new(2u64))
            .is_err());

        assert!(file_storage.get_metadata(&file_key_2).is_ok());
        assert!(file_storage
            .get_chunk(&file_key_2, &ChunkId::new(0u64))
            .is_ok());
        assert!(file_storage
            .get_chunk(&file_key_2, &ChunkId::new(1u64))
            .is_ok());
        assert!(file_storage
            .get_chunk(&file_key_2, &ChunkId::new(2u64))
            .is_ok());
    }

    #[test]
    fn add_file_to_exclude_list() {
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();

        let hash = HasherOutT::<LayoutV1<BlakeTwo256>>::try_from([
            3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231,
            135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20,
        ])
        .unwrap();

        file_storage
            .add_to_exclude_list(hash, ExcludeType::File)
            .unwrap();

        assert!(!file_storage.is_allowed(&hash, ExcludeType::File).unwrap());

        file_storage
            .add_to_exclude_list(hash, ExcludeType::Fingerprint)
            .unwrap();

        assert!(!file_storage
            .is_allowed(&hash, ExcludeType::Fingerprint)
            .unwrap())
    }

    #[test]
    fn chunk_bitmap_prevents_double_counting() {
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();

        // Create a file with 3 chunks
        let chunks = vec![
            Chunk::from([5u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([6u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([7u8; FILE_CHUNK_SIZE as usize]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        // Create a file trie to get the expected fingerprint
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
        for (chunk_id, chunk) in chunk_ids.iter().zip(chunks.iter()) {
            file_trie.write_chunk(chunk_id, chunk).unwrap();
        }

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            FILE_CHUNK_SIZE * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

        let key = file_metadata.file_key::<BlakeTwo256>();

        // Insert file metadata first
        file_storage.insert_file(key, file_metadata).unwrap();

        // Write chunk 0 for the first time
        file_storage
            .write_chunk(&key, &chunk_ids[0], &chunks[0])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 1);

        // Write chunk 0 again (simulating concurrent upload)
        file_storage
            .write_chunk(&key, &chunk_ids[0], &chunks[0])
            .unwrap();
        // Should still be 1, not 2
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 1);

        // Write chunk 1
        file_storage
            .write_chunk(&key, &chunk_ids[1], &chunks[1])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 2);

        // Write chunk 1 again (simulating concurrent upload)
        file_storage
            .write_chunk(&key, &chunk_ids[1], &chunks[1])
            .unwrap();
        // Should still be 2, not 3
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 2);

        // Write chunk 2
        file_storage
            .write_chunk(&key, &chunk_ids[2], &chunks[2])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 3);

        // Verify final state
        assert!(file_storage.is_file_complete(&key).unwrap());
    }

    #[test]
    fn chunk_bitmap_handles_non_sequential_writes() {
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();

        // Create a file with 5 chunks
        let chunks = vec![
            Chunk::from([0u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([1u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([2u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([3u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([4u8; FILE_CHUNK_SIZE as usize]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        // Create a file trie to get the expected fingerprint
        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
        for (chunk_id, chunk) in chunk_ids.iter().zip(chunks.iter()) {
            file_trie.write_chunk(chunk_id, chunk).unwrap();
        }

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            FILE_CHUNK_SIZE * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

        let key = file_metadata.file_key::<BlakeTwo256>();
        file_storage.insert_file(key, file_metadata).unwrap();

        // Write chunks in non-sequential order: 4, 1, 3, 0, 2
        file_storage
            .write_chunk(&key, &chunk_ids[4], &chunks[4])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 1);

        file_storage
            .write_chunk(&key, &chunk_ids[1], &chunks[1])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 2);

        file_storage
            .write_chunk(&key, &chunk_ids[3], &chunks[3])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 3);

        file_storage
            .write_chunk(&key, &chunk_ids[0], &chunks[0])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 4);

        file_storage
            .write_chunk(&key, &chunk_ids[2], &chunks[2])
            .unwrap();
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 5);

        // Verify bitmap is correctly sized and all chunks are marked
        let bitmap = file_storage.chunk_bitmaps.get(&key).unwrap();
        assert_eq!(bitmap.len(), 5);
        for i in 0..5 {
            assert_eq!(bitmap[i], 1, "Chunk {} should be marked as present", i);
        }
    }

    #[test]
    fn chunk_bitmap_initialized_correctly_in_insert_file_with_data() {
        let mut file_storage = InMemoryFileStorage::<LayoutV1<BlakeTwo256>>::new();

        let chunks = vec![
            Chunk::from([5u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([6u8; FILE_CHUNK_SIZE as usize]),
            Chunk::from([7u8; FILE_CHUNK_SIZE as usize]),
        ];

        let chunk_ids: Vec<ChunkId> = chunks
            .iter()
            .enumerate()
            .map(|(id, _)| ChunkId::new(id as u64))
            .collect();

        let mut file_trie = InMemoryFileDataTrie::<LayoutV1<BlakeTwo256>>::new();
        for (chunk_id, chunk) in chunk_ids.iter().zip(chunks.iter()) {
            file_trie.write_chunk(chunk_id, chunk).unwrap();
        }

        let file_metadata = FileMetadata::new(
            <AccountId32 as AsRef<[u8]>>::as_ref(&AccountId32::new([0u8; 32])).to_vec(),
            [1u8; 32].to_vec(),
            "location".to_string().into_bytes(),
            FILE_CHUNK_SIZE * chunks.len() as u64,
            file_trie.get_root().as_ref().into(),
        )
        .unwrap();

        let key = file_metadata.file_key::<BlakeTwo256>();

        // Insert file with data
        file_storage
            .insert_file_with_data(key, file_metadata, file_trie)
            .unwrap();

        // Verify bitmap is correctly initialized
        let bitmap = file_storage.chunk_bitmaps.get(&key).unwrap();
        assert_eq!(bitmap.len(), 3);
        assert_eq!(bitmap[0], 1);
        assert_eq!(bitmap[1], 1);
        assert_eq!(bitmap[2], 1);

        // Verify stored chunks count matches
        assert_eq!(file_storage.stored_chunks_count(&key).unwrap(), 3);
    }
}

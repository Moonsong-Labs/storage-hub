use anyhow::Result;
use log::*;
use std::{marker::PhantomData, path::PathBuf};

use shc_common::{
    traits::StorageEnableRuntime,
    typed_store::{
        BufferedWriteSupport, CFRangeMapAPI, CompositeKey, ProvidesDbContext,
        ProvidesTypedDbAccess, ScaleDbCodec, ScaleEncodedCf, TypedCf, TypedDbContext, TypedRocksDB,
    },
    types::{BucketId, FileMetadata},
};
use shp_file_metadata::ChunkId;

use sp_core::H256;

const LOG_TARGET: &str = "download_state_store";

// Column family definitions
/// Column family that tracks missing chunks for files being downloaded.
///
/// This CF implements a range map pattern where:
/// - The primary key is a file key (H256 hash)
/// - The values are ChunkIds representing chunks that still need to be downloaded
///
/// When a file download starts, all chunks are added to this CF.
/// As chunks are successfully downloaded, they are removed from the CF.
/// When all chunks for a file are downloaded, no entries with that file key remain.
pub struct MissingChunksCf;

impl Default for MissingChunksCf {
    fn default() -> Self {
        Self
    }
}

impl ScaleEncodedCf for MissingChunksCf {
    type Key = H256; // File key
    type Value = ChunkId; // Chunk ID

    const SCALE_ENCODED_NAME: &'static str = "missing_chunks";
}

/// Non-generic name holder for the `MissingChunks` column family
pub struct MissingChunksName;
impl MissingChunksName {
    pub const NAME: &'static str = "missing_chunks";
}

/// A separate column family for the composite key implementation
pub struct MissingChunksCompositeCf;

impl Default for MissingChunksCompositeCf {
    fn default() -> Self {
        Self
    }
}

impl TypedCf for MissingChunksCompositeCf {
    type Key = CompositeKey<H256, ChunkId>;
    type Value = ();
    type KeyCodec = ScaleDbCodec;
    type ValueCodec = ScaleDbCodec;

    const NAME: &'static str = "missing_chunks";
}

/// Non-generic name holder for the `MissingChunksComposite` column family
pub struct MissingChunksCompositeName;
impl MissingChunksCompositeName {
    pub const NAME: &'static str = "missing_chunks";
}

/// Column family that stores file metadata for files being downloaded.
///
/// This CF uses a simple key-value structure where:
/// - The key is a file key (H256 hash)
/// - The value is the complete FileMetadata for that file
///
/// File metadata is stored when a download begins and is used to validate
/// downloaded chunks and provide information about the file (size, owner, etc.).
/// It is removed when a download is completed or cancelled.
pub struct FileMetadataCf;

/// Column family that tracks pending bucket downloads.
///
/// This CF uses a simple key-value structure where:
/// - The key is a bucket ID (BucketId)
/// - The value is a boolean flag indicating whether the download is in progress
///
/// This CF is used to track which buckets are being downloaded so that
/// downloads can be resumed if interrupted.
pub struct PendingBucketDownloadsCf<Runtime: StorageEnableRuntime> {
    pub phantom: PhantomData<Runtime>,
}

impl<Runtime: StorageEnableRuntime> Default for PendingBucketDownloadsCf<Runtime> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<Runtime: StorageEnableRuntime> ScaleEncodedCf for PendingBucketDownloadsCf<Runtime> {
    type Key = BucketId<Runtime>; // Bucket ID
    type Value = bool; // Download in progress flag

    const SCALE_ENCODED_NAME: &'static str = "pending_bucket_downloads";
}

/// Non-generic name holder for the `PendingBucketDownloads` column family
pub struct PendingBucketDownloadsName;
impl PendingBucketDownloadsName {
    pub const NAME: &'static str = "pending_bucket_downloads";
}

impl Default for FileMetadataCf {
    fn default() -> Self {
        Self
    }
}

impl ScaleEncodedCf for FileMetadataCf {
    type Key = H256; // File key
    type Value = FileMetadata; // Original file metadata

    const SCALE_ENCODED_NAME: &'static str = "file_metadata";
}

/// Non-generic name holder for the `FileMetadata` column family
pub struct FileMetadataName;
impl FileMetadataName {
    pub const NAME: &'static str = "file_metadata";
}

/// Current column families used by the download state store.
const CURRENT_COLUMN_FAMILIES: &[&str] = &[
    MissingChunksCompositeName::NAME,
    FileMetadataName::NAME,
    PendingBucketDownloadsName::NAME,
];

/// Persistent store for file download state using RocksDB.
///
/// This store manages two main pieces of download state:
/// 1. Missing chunks for each file (which chunks still need to be downloaded)
/// 2. File metadata for each file being downloaded
///
/// The store uses separate column families to store different types of data
/// and provides a context-based API for reading and writing data.
pub struct DownloadStateStore<Runtime: StorageEnableRuntime> {
    /// The RocksDB database
    rocks: TypedRocksDB,
    phantom: PhantomData<Runtime>,
}

impl<Runtime: StorageEnableRuntime> DownloadStateStore<Runtime> {
    pub fn new(root_path: PathBuf) -> Result<Self> {
        let mut path = root_path;
        path.push("storagehub/download_state/");

        let db_path_str = path.to_str().expect("Failed to convert path to string");
        info!(target: LOG_TARGET, "Download state store path: {}", db_path_str);
        std::fs::create_dir_all(db_path_str).expect("Failed to create directory");

        let rocks = TypedRocksDB::open(db_path_str, CURRENT_COLUMN_FAMILIES)?;

        Ok(DownloadStateStore {
            rocks,
            phantom: PhantomData,
        })
    }

    /// Starts a read/write interaction with the DB
    pub fn open_rw_context(&self) -> DownloadStateStoreRwContext<'_, Runtime> {
        DownloadStateStoreRwContext::<Runtime>::new(TypedDbContext::new(
            &self.rocks,
            BufferedWriteSupport::new(&self.rocks),
        ))
    }
}

/// Read/write transaction context for interacting with the download state store.
///
/// This context manages a transaction with the underlying RocksDB database and
/// provides methods to access the different components of the download state:
/// - Missing chunks map for tracking which chunks need to be downloaded
/// - File metadata for storing and retrieving metadata about files being downloaded
///
/// Changes are not persisted until the `commit()` method is called, which flushes
/// all pending changes to the database.
pub struct DownloadStateStoreRwContext<'a, Runtime: StorageEnableRuntime> {
    /// The RocksDB database context
    db_context: TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
    phantom: PhantomData<Runtime>,
}

impl<'a, Runtime: StorageEnableRuntime> DownloadStateStoreRwContext<'a, Runtime> {
    pub fn new(
        db_context: TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
    ) -> Self {
        Self {
            db_context,
            phantom: PhantomData,
        }
    }

    pub fn missing_chunks_map(&'a self) -> MissingChunksMap<'a> {
        MissingChunksMap {
            db_context: &self.db_context,
        }
    }

    pub fn commit(self) {
        self.db_context.flush();
    }

    pub fn delete_file_metadata(&self, file_key: &H256) {
        self.db_context
            .cf(&FileMetadataCf::default())
            .delete(file_key);
        self.db_context.flush();
    }

    // Methods to store and retrieve pending bucket downloads
    pub fn mark_bucket_download_started(&self, bucket_id: &BucketId<Runtime>) {
        self.db_context
            .cf(&PendingBucketDownloadsCf::<Runtime>::default())
            .put(bucket_id, &true);
        self.db_context.flush();
    }

    pub fn mark_bucket_download_completed(&self, bucket_id: &BucketId<Runtime>) {
        self.db_context
            .cf(&PendingBucketDownloadsCf::<Runtime>::default())
            .delete(bucket_id);
        self.db_context.flush();
    }

    pub fn is_bucket_download_in_progress(&self, bucket_id: &BucketId<Runtime>) -> bool {
        self.db_context
            .cf(&PendingBucketDownloadsCf::<Runtime>::default())
            .get(bucket_id)
            .is_some()
    }

    pub fn get_all_pending_bucket_downloads(&self) -> Vec<BucketId<Runtime>> {
        self.db_context
            .cf(&PendingBucketDownloadsCf::<Runtime>::default())
            .iterate_with_range(..)
            .map(|(bucket_id, _)| bucket_id)
            .collect()
    }

    /// Get all file keys that need to be downloaded for a specific bucket
    pub fn get_missing_files_for_bucket(&self, bucket_id: &BucketId<Runtime>) -> Vec<H256> {
        // If the bucket is not in progress, return empty list
        if !self.is_bucket_download_in_progress(bucket_id) {
            return Vec::new();
        }

        // Get all files with pending downloads for this bucket
        // For now, we'll just return all files in the store since we don't track by bucket
        self.missing_chunks_map()
            .db_context()
            .cf(&MissingChunksCf::default())
            .iterate_with_range(..)
            .map(|(file_key, _)| file_key)
            .collect()
    }
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesDbContext
    for DownloadStateStoreRwContext<'a, Runtime>
{
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesTypedDbAccess
    for DownloadStateStoreRwContext<'a, Runtime>
{
}

/// Map-like interface for tracking missing chunks per file.
///
/// This structure provides methods to:
/// - Initialize a file's missing chunks
/// - Mark chunks as downloaded
/// - Check if a file download is complete
/// - Get a list of missing chunks for a file
///
/// It uses the MissingChunksCf column family as its backing storage.
pub struct MissingChunksMap<'a> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> ProvidesDbContext for MissingChunksMap<'a> {
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        self.db_context
    }
}

impl<'a> ProvidesTypedDbAccess for MissingChunksMap<'a> {}

impl<'a> CFRangeMapAPI for MissingChunksMap<'a> {
    type Key = H256; // File key
    type Value = ChunkId; // Chunk ID
    type MapCF = MissingChunksCompositeCf;
}

impl<'a> MissingChunksMap<'a> {
    // Initialize missing chunks for a file
    pub fn initialize_file(&self, metadata: &FileMetadata) {
        let file_key = metadata.file_key::<sp_core::Blake2Hasher>();
        let file_key = H256::from_slice(file_key.as_ref());
        let chunks_count = metadata.chunks_count();

        // Remove any existing chunks first (clean state)
        self.remove_key(&file_key);

        // Add all chunks as missing
        for chunk_id in 0..chunks_count {
            self.insert(&file_key, ChunkId::new(chunk_id));
        }

        // Commit changes
        self.db_context().flush();
    }

    // Mark a chunk as successfully downloaded (remove from missing)
    pub fn mark_chunk_downloaded(&self, file_key: &H256, chunk_id: ChunkId) -> bool {
        let result = self.remove(file_key, &chunk_id);
        self.db_context().flush();
        result
    }

    // Check if a file download is complete (no missing chunks)
    pub fn is_file_complete(&self, file_key: &H256) -> bool {
        !self.contains_key(file_key)
    }

    // Get all missing chunks for a file
    pub fn get_missing_chunks(&self, file_key: &H256) -> Vec<ChunkId> {
        self.values_for_key(file_key)
    }
}

// Methods to store and retrieve file metadata
impl<'a, Runtime: StorageEnableRuntime> DownloadStateStoreRwContext<'a, Runtime> {
    pub fn store_file_metadata(&self, file_key: &H256, metadata: &FileMetadata) {
        self.db_context
            .cf(&FileMetadataCf::default())
            .put(file_key, metadata);
        self.db_context.flush();
    }

    pub fn get_file_metadata(&self, file_key: &H256) -> Option<FileMetadata> {
        self.db_context.cf(&FileMetadataCf::default()).get(file_key)
    }
}

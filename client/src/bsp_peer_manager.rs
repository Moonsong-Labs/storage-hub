use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};

use anyhow::anyhow;
use codec::{Decode, Encode};
use log::*;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;
use rand::{prelude::SliceRandom, thread_rng};
use sc_network::PeerId;
use sp_core::H256;
use tokio::sync::RwLock;

use shc_common::typed_store::{
    BufferedWriteSupport, ProvidesDbContext, ProvidesTypedDbAccess, ProvidesTypedDbSingleAccess,
    ScaleEncodedCf, SingleScaleEncodedValueCf, TypedCf, TypedDbContext, TypedRocksDB,
};

// PeerId wrapper that implements Encode/Decode
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, Hash)]
pub struct EncodablePeerId(Vec<u8>);

impl From<PeerId> for EncodablePeerId {
    fn from(peer_id: PeerId) -> Self {
        Self(peer_id.to_bytes())
    }
}

impl From<EncodablePeerId> for PeerId {
    fn from(encodable_id: EncodablePeerId) -> Self {
        PeerId::from_bytes(&encodable_id.0).expect("Valid peer ID bytes")
    }
}

// Column family definitions with proper types

#[derive(Default, Clone)]
pub struct PeerIdCf;
impl ScaleEncodedCf for PeerIdCf {
    type Key = EncodablePeerId;
    type Value = PersistentBspPeerStats;

    const SCALE_ENCODED_NAME: &'static str = "bsp_peer_stats";
}

#[derive(Default, Clone)]
pub struct PeerFileKeyCf;
impl ScaleEncodedCf for PeerFileKeyCf {
    type Key = (EncodablePeerId, H256); // Composite key of PeerId and file_key
    type Value = (); // No value needed, just tracking the association

    const SCALE_ENCODED_NAME: &'static str = "bsp_peer_file_keys";
}

#[derive(Default)]
pub struct LastUpdateTimeCf;
impl SingleScaleEncodedValueCf for LastUpdateTimeCf {
    type Value = u64; // Timestamp in millis

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "bsp_peer_last_update";
}

/// Current column families used by the BSP peer manager store.
///
/// Note: Deprecated column families are NOT listed here. They are automatically
/// discovered via `DB::list_cf()` when opening the database, and then removed
/// by the migration system.
const CURRENT_COLUMN_FAMILIES: [&str; 3] =
    [PeerIdCf::NAME, PeerFileKeyCf::NAME, LastUpdateTimeCf::NAME];

/// Version of BspPeerStats that can be persisted with SCALE codec
#[derive(Debug, Clone, Encode, Decode)]
pub struct PersistentBspPeerStats {
    /// The number of successful downloads for the peer
    pub successful_downloads: u64,
    /// The number of failed downloads for the peer
    pub failed_downloads: u64,
    /// The total number of bytes downloaded for the peer
    pub total_bytes_downloaded: u64,
    /// The total download time for the peer
    pub total_download_time_ms: u64,
    /// Timestamp (in millis since UNIX epoch) of the last successful download
    pub last_success_time_millis: Option<u64>,
}

impl From<&BspPeerStats> for PersistentBspPeerStats {
    fn from(stats: &BspPeerStats) -> Self {
        Self {
            successful_downloads: stats.successful_downloads,
            failed_downloads: stats.failed_downloads,
            total_bytes_downloaded: stats.total_bytes_downloaded,
            total_download_time_ms: stats.total_download_time_ms,
            last_success_time_millis: stats
                .last_success_time
                .map(|time| time.elapsed().as_millis().try_into().unwrap_or(u64::MAX)),
        }
    }
}

/// Statistics about a BSP peer's performance
#[derive(Debug, Clone)]
pub struct BspPeerStats {
    /// The number of successful downloads for the peer
    pub successful_downloads: u64,
    /// The number of failed downloads for the peer
    pub failed_downloads: u64,
    /// The total number of bytes downloaded for the peer
    pub total_bytes_downloaded: u64,
    /// The total download time for the peer
    pub total_download_time_ms: u64,
    /// The time of the last successful download for the peer
    pub last_success_time: Option<Instant>,
    /// The set of file keys that the peer can provide
    pub file_keys: HashSet<H256>,
}

impl BspPeerStats {
    fn new() -> Self {
        Self {
            successful_downloads: 0,
            failed_downloads: 0,
            total_bytes_downloaded: 0,
            total_download_time_ms: 0,
            last_success_time: None,
            file_keys: HashSet::new(),
        }
    }

    fn from_persistent(persistent: PersistentBspPeerStats, file_keys: HashSet<H256>) -> Self {
        Self {
            successful_downloads: persistent.successful_downloads,
            failed_downloads: persistent.failed_downloads,
            total_bytes_downloaded: persistent.total_bytes_downloaded,
            total_download_time_ms: persistent.total_download_time_ms,
            last_success_time: persistent.last_success_time_millis.map(|_| Instant::now()),
            file_keys,
        }
    }

    /// Record a successful download and update the stats
    fn add_success(&mut self, bytes_downloaded: u64, download_time_ms: u64) {
        self.successful_downloads += 1;
        self.total_bytes_downloaded += bytes_downloaded;
        self.total_download_time_ms += download_time_ms;
        self.last_success_time = Some(Instant::now());
    }

    /// Record a failed download attempt
    fn add_failure(&mut self) {
        self.failed_downloads += 1;
    }

    /// Calculate the success rate (0.0 to 1.0)
    fn get_success_rate(&self) -> f64 {
        let total = self.successful_downloads + self.failed_downloads;
        if total == 0 {
            return 0.0;
        }
        self.successful_downloads as f64 / total as f64
    }

    /// Calculate average download speed in bytes/second
    fn get_average_speed_bytes_per_sec(&self) -> f64 {
        if self.total_download_time_ms == 0 {
            return 0.0;
        }
        (self.total_bytes_downloaded as f64 * 1000.0) / self.total_download_time_ms as f64
    }

    /// Calculate an overall score for this peer (0.0 to 1.0)
    /// The score is a weighted combination of success rate and speed
    fn get_score(&self) -> f64 {
        // Weight success rate (70%) and speed (30%)
        let success_weight = 0.7;
        let speed_weight = 0.3;

        let success_score = self.get_success_rate();
        let speed_score = if self.successful_downloads == 0 {
            0.0
        } else {
            // Normalize speed score (50MB/s is considered 1.0)
            let max_speed = 50.0 * 1024.0 * 1024.0;
            (self.get_average_speed_bytes_per_sec() / max_speed).min(1.0)
        };

        (success_score * success_weight) + (speed_score * speed_weight)
    }

    /// Add a file key that this peer can provide
    fn add_file_key(&mut self, file_key: H256) -> bool {
        self.file_keys.insert(file_key)
    }
}

/// Persistent storage for the BSP peer manager
pub struct BspPeerManagerStore {
    /// The RocksDB database.
    rocks: TypedRocksDB,
}

impl BspPeerManagerStore {
    pub fn new(db_path: PathBuf) -> anyhow::Result<Self> {
        // Ensure the directory exists
        std::fs::create_dir_all(&db_path)?;

        let db_path_str = db_path.to_str().expect("Failed to convert path to string");
        info!("BSP peer manager DB path: {}", db_path_str);

        let rocks = TypedRocksDB::open(db_path_str, &CURRENT_COLUMN_FAMILIES)
            .map_err(|e| anyhow!("Failed to open BSP peer manager database: {}", e))?;

        Ok(Self { rocks })
    }

    /// Starts a read/write interaction with the DB through typed APIs
    pub fn open_rw_context(&self) -> BspPeerManagerRwContext<'_> {
        BspPeerManagerRwContext::new(TypedDbContext::new(
            &self.rocks,
            BufferedWriteSupport::new(&self.rocks),
        ))
    }
}

/// Read/write context for the BSP peer manager store
pub struct BspPeerManagerRwContext<'a> {
    /// The RocksDB database context
    db_context: TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> BspPeerManagerRwContext<'a> {
    pub fn new(
        db_context: TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
    ) -> Self {
        Self { db_context }
    }

    /// Commits the changes to the database
    pub fn commit(self) {
        self.db_context.flush();
    }
}

impl<'a> ProvidesDbContext for BspPeerManagerRwContext<'a> {
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbAccess for BspPeerManagerRwContext<'a> {}
impl<'a> ProvidesTypedDbSingleAccess for BspPeerManagerRwContext<'a> {}

/// Thread-safe persistent BSP peer manager
///
/// This service tracks BSP peer performance metrics and provides methods to:
/// - Record successful and failed download attempts
/// - Select the best peers for downloading specific files
/// - Persist performance data across node restarts
///
/// All operations are thread-safe, use interior mutability, and changes are
/// immediately persisted to the database.
pub struct BspPeerManager {
    /// Inner state protected by a read-write lock
    inner: RwLock<BspPeerManagerInner>,
    /// Database state store
    store: BspPeerManagerStore,
}

/// Inner state of the BSP peer manager
struct BspPeerManagerInner {
    /// Performance stats for each peer
    peers: HashMap<PeerId, BspPeerStats>,
    /// Priority queues for each file, mapping peers to their scores
    peer_queues: HashMap<H256, PriorityQueue<PeerId, OrderedFloat<f64>>>,
}

impl BspPeerManager {
    /// Create a new BspPeerManager with persistent storage
    pub fn new(db_path: PathBuf) -> anyhow::Result<Self> {
        let store = BspPeerManagerStore::new(db_path)?;

        // Load existing data
        let (peers, peer_queues) = Self::load_from_db(&store)?;

        Ok(Self {
            inner: RwLock::new(BspPeerManagerInner { peers, peer_queues }),
            store,
        })
    }

    /// Load peer stats from the database
    fn load_from_db(
        store: &BspPeerManagerStore,
    ) -> anyhow::Result<(
        HashMap<PeerId, BspPeerStats>,
        HashMap<H256, PriorityQueue<PeerId, OrderedFloat<f64>>>,
    )> {
        let rw_context = store.open_rw_context();

        // Load all peer stats
        let mut peers = HashMap::new();
        let mut peer_file_keys: HashMap<PeerId, HashSet<H256>> = HashMap::new();

        // Load peer stats
        {
            let cf = rw_context.db_context().cf(&PeerIdCf::default());
            let mut iter = cf.iterate_with_range(..);
            while let Some((encodable_id, stats)) = iter.next() {
                let peer_id = PeerId::from(encodable_id);
                peer_file_keys.entry(peer_id).or_insert_with(HashSet::new);
                peers.insert(
                    peer_id,
                    BspPeerStats::from_persistent(stats, HashSet::new()),
                );
            }
        }

        // Load file keys
        {
            let cf = rw_context.db_context().cf(&PeerFileKeyCf::default());
            let mut iter = cf.iterate_with_range(..);
            while let Some(((encodable_id, file_key), _)) = iter.next() {
                let peer_id = PeerId::from(encodable_id);
                if let Some(file_keys) = peer_file_keys.get_mut(&peer_id) {
                    file_keys.insert(file_key);
                }
            }
        }

        // Combine stats with file keys
        for (peer_id, file_keys) in peer_file_keys {
            if let Some(stats) = peers.get_mut(&peer_id) {
                stats.file_keys = file_keys;
            }
        }

        // Build priority queues
        let mut peer_queues = HashMap::new();
        for (peer_id, stats) in &peers {
            for file_key in &stats.file_keys {
                let queue = peer_queues
                    .entry(*file_key)
                    .or_insert_with(PriorityQueue::new);
                queue.push(*peer_id, OrderedFloat::from(stats.get_score()));
            }
        }

        info!("Loaded {} BSP peers from database", peers.len());
        Ok((peers, peer_queues))
    }

    /// Register a BSP peer for a specific file
    pub async fn add_peer(&self, peer_id: PeerId, file_key: H256) {
        // Update in-memory state first
        let add_to_db = {
            let mut inner = self.inner.write().await;

            let stats = inner.peers.entry(peer_id).or_insert_with(BspPeerStats::new);

            // Check if this is a new file key for this peer
            let is_new_file_key = stats.add_file_key(file_key);

            // Store the score outside the borrow if we need to update the queue
            let score = if is_new_file_key {
                stats.get_score()
            } else {
                0.0 // Default value, won't be used
            };

            // Add to priority queue if it's a new file key
            if is_new_file_key {
                let queue = inner
                    .peer_queues
                    .entry(file_key)
                    .or_insert_with(PriorityQueue::new);
                queue.push(peer_id, OrderedFloat::from(score));
            }

            is_new_file_key
        };

        // Only update the database if we added a new file key
        if add_to_db {
            let rw_context = self.store.open_rw_context();

            // Store the file key association using the typed API
            let encodable_id = EncodablePeerId::from(peer_id);
            rw_context
                .db_context()
                .cf(&PeerFileKeyCf::default())
                .put(&(encodable_id.clone(), file_key), &());

            // Update peer stats
            {
                let inner = self.inner.read().await;
                if let Some(stats) = inner.peers.get(&peer_id) {
                    let cf = rw_context.db_context().cf(&PeerIdCf::default());
                    cf.put(&encodable_id, &PersistentBspPeerStats::from(stats));
                }
            }

            // Update timestamp
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            rw_context
                .access_value(&LastUpdateTimeCf::default())
                .write(&now);

            // Commit all changes
            rw_context.commit();
        }
    }

    /// Record a successful download
    pub async fn record_success(
        &self,
        peer_id: PeerId,
        bytes_downloaded: u64,
        download_time_ms: u64,
    ) {
        // Update in-memory state first
        let should_update_db = {
            let mut inner = self.inner.write().await;

            // First, get and update the stats
            let peer_exists = if let Some(stats) = inner.peers.get_mut(&peer_id) {
                stats.add_success(bytes_downloaded, download_time_ms);

                // Store what we need outside the borrow
                let new_score = stats.get_score();
                let file_keys: Vec<H256> = stats.file_keys.iter().cloned().collect();

                // Now update all queues (using a separate borrow)
                for file_key in file_keys {
                    if let Some(queue) = inner.peer_queues.get_mut(&file_key) {
                        queue.change_priority(&peer_id, OrderedFloat::from(new_score));
                    }
                }

                true
            } else {
                false
            };

            peer_exists
        };

        // Update the database if we updated the in-memory state
        if should_update_db {
            // Get the updated stats to save
            let stats_data = {
                let inner = self.inner.read().await;
                inner.peers.get(&peer_id).map(PersistentBspPeerStats::from)
            };

            // Save the updated stats using the typed API
            if let Some(stats_data) = stats_data {
                let rw_context = self.store.open_rw_context();
                let encodable_id = EncodablePeerId::from(peer_id);

                // Store updated stats
                rw_context
                    .db_context()
                    .cf(&PeerIdCf::default())
                    .put(&encodable_id, &stats_data);

                // Update timestamp
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                rw_context
                    .access_value(&LastUpdateTimeCf::default())
                    .write(&now);

                // Commit all changes
                rw_context.commit();
            }
        }
    }

    /// Record a failed download attempt
    pub async fn record_failure(&self, peer_id: PeerId) {
        // Update in-memory state first
        let should_update_db = {
            let mut inner = self.inner.write().await;

            // First, get and update the stats
            let peer_exists = if let Some(stats) = inner.peers.get_mut(&peer_id) {
                stats.add_failure();

                // Store what we need outside the borrow
                let new_score = stats.get_score();
                let file_keys: Vec<H256> = stats.file_keys.iter().cloned().collect();

                // Now update all queues (using a separate borrow)
                for file_key in file_keys {
                    if let Some(queue) = inner.peer_queues.get_mut(&file_key) {
                        queue.change_priority(&peer_id, OrderedFloat::from(new_score));
                    }
                }

                true
            } else {
                false
            };

            peer_exists
        };

        // Update the database if we updated the in-memory state
        if should_update_db {
            // Get the updated stats to save
            let stats_data = {
                let inner = self.inner.read().await;
                inner.peers.get(&peer_id).map(PersistentBspPeerStats::from)
            };

            // Save the updated stats using the typed API
            if let Some(stats_data) = stats_data {
                let rw_context = self.store.open_rw_context();
                let encodable_id = EncodablePeerId::from(peer_id);

                // Store updated stats
                rw_context
                    .db_context()
                    .cf(&PeerIdCf::default())
                    .put(&encodable_id, &stats_data);

                // Update timestamp
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                rw_context
                    .access_value(&LastUpdateTimeCf::default())
                    .write(&now);

                // Commit all changes
                rw_context.commit();
            }
        }
    }

    /// Select the best peers for downloading a specific file
    ///
    /// Returns a list of peers in preferred order for download attempts.
    /// The selection combines the best performing peers with some random peers
    /// to ensure both performance and network health.
    ///
    /// # Arguments
    /// * `count_best` - Number of top-performing peers to select
    /// * `count_random` - Number of additional random peers to select
    /// * `file_key` - The file to download
    pub async fn select_peers(
        &self,
        count_best: usize,
        count_random: usize,
        file_key: &H256,
    ) -> Vec<PeerId> {
        let inner = self.inner.read().await;

        let queue = match inner.peer_queues.get(file_key) {
            Some(queue) => queue,
            None => return Vec::new(),
        };

        let mut selected_peers = Vec::with_capacity(count_best + count_random);
        let mut queue_clone = queue.clone();

        // Get top performers first
        let actual_best_count = count_best.min(queue_clone.len());
        for _ in 0..actual_best_count {
            if let Some((peer_id, _)) = queue_clone.pop() {
                selected_peers.push(peer_id);
            }
        }

        // Add some random peers for diversity
        if count_random > 0 && !queue_clone.is_empty() {
            let remaining_peers: Vec<_> = queue_clone
                .into_iter()
                .map(|(peer_id, _)| peer_id)
                .collect();

            let mut remaining_peers = remaining_peers;
            remaining_peers.shuffle(&mut thread_rng());

            let actual_random_count = count_random.min(remaining_peers.len());
            selected_peers.extend_from_slice(&remaining_peers[0..actual_random_count]);
        }

        selected_peers
    }
}

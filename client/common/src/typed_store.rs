//! # Typed RocksDB Storage Framework
//!
//! This module provides a type-safe abstraction layer on top of RocksDB for persisting structured data.
//! It addresses several challenges when working with RocksDB directly:
//!
//! 1. **Type Safety**: RocksDB is a key-value store that works with raw bytes, requiring manual
//!    serialization/deserialization. This framework ensures type safety by handling this automatically.
//!
//! 2. **Column Family Organization**: While RocksDB supports column families for organizing data,
//!    using them directly requires string names and careful handling. This framework provides
//!    strongly-typed column family definitions.
//!
//! 3. **Higher-level Data Structures**: This framework implements common data structures on top of
//!    RocksDB such as:
//!    - Deques (CFDequeAPI) for queue-like operations
//!    - Hash Sets (CFHashSetAPI) for set-like operations
//!    - Range Maps (CFRangeMapAPI) for map structures with range querying capabilities
//!
//! ## Architecture
//!
//! The framework is organized into several abstraction layers:
//!
//! ### 1. Base Layer: Encoding/Decoding
//!
//! The `DbCodec<T>` trait defines how types are encoded to and decoded from bytes:
//! - `encode(value: &T) -> Vec<u8>`: Converts a value to bytes for storage
//! - `decode(bytes: &[u8]) -> T`: Converts bytes back to the original type
//!
//! The most common implementation is `ScaleDbCodec`, which uses SCALE (Simple Concatenated Aggregate Little-Endian)
//! encoding, the same serialization format used throughout the Substrate ecosystem.
//!
//! ### 2. Column Family Definitions
//!
//! Column families are defined as types implementing various traits:
//!
//! - `TypedCf`: The fundamental trait for column families, defining key and value types and their codecs
//! - `ScaleEncodedCf`: For column families using SCALE encoding for both keys and values
//! - `SingleScaleEncodedValueCf`: For column families storing a single value (essentially a global variable)
//!
//! ### 3. Database Access Layer
//!
//! - `ReadableRocks`/`WriteableRocks`: Traits that abstract RocksDB read/write operations
//! - `TypedDbContext`: Provides a context for interacting with the database, supporting transactions and overlays
//! - `BufferedWriteSupport`: Enables batched writes for performance
//!
//! ### 4. Higher-level Data Structures
//!
//! - `CFDequeAPI`: Implements a double-ended queue abstraction over RocksDB
//! - `CFHashSetAPI`: Implements a set abstraction for storing unique values
//! - `CFRangeMapAPI`: Implements a map abstraction with range query capabilities
//!
//! ## Common Usage Patterns
//!
//! ### Defining Column Families
//!
//! Column families are typically defined as empty structs implementing the appropriate traits:
//!
//! ```ignore
//! // Single value column family (global variable)
//! pub struct LastProcessedBlockNumberCf;
//! impl SingleScaleEncodedValueCf for LastProcessedBlockNumberCf {
//!     type Value = BlockNumber;
//!     const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "last_processed_block_number";
//! }
//!
//! // Key-value column family
//! pub struct PendingRequestsCf;
//! impl ScaleEncodedCf for PendingRequestsCf {
//!     type Key = u64;
//!     type Value = Request;
//!     const SCALE_ENCODED_NAME: &'static str = "pending_requests";
//! }
//! ```
//!
//! ### Reading/Writing Data
//!
//! Operations are performed through a context that provides type safety:
//!
//! ```ignore
//! // Read a single value
//! let block_number = context.access_value(&LastProcessedBlockNumberCf::default()).read();
//!
//! // Write a value
//! context.access_value(&LastProcessedBlockNumberCf::default()).write(&new_block_number);
//!
//! // Access a key-value column family
//! let entry = context.cf(&PendingRequestsCf::default()).get(&key);
//! context.cf(&PendingRequestsCf::default()).put(&key, &value);
//! ```
//!
//! ### Using Higher-level Data Structures
//!
//! The framework provides APIs for common data structures:
//!
//! ```ignore
//! // Using a deque (queue)
//! let deque = context.pending_requests_deque();
//! deque.push_back(request);  // Add to the end
//! let next_request = deque.pop_front();  // Get from the front
//!
//! // Using a set
//! let hashset = context.unique_addresses();
//! hashset.insert(&address);
//! let contains = hashset.contains(&address);
//!
//! // Using a range map
//! let range_map = context.file_chunks_map();
//! range_map.insert(&file_key, chunk_id);
//! let chunks = range_map.values_for_key(&file_key);
//! ```
//!
//! ## Transaction Management
//!
//! Operations can be batched and committed atomically:
//!
//! ```ignore
//! let context = store.open_rw_context_with_overlay();
//! // Perform multiple operations
//! context.commit();  // Flushes all changes to the database
//! ```
//!
//! ## Design Benefits
//!
//! 1. **Type Safety**: Compile-time checks prevent using wrong types with column families
//! 2. **Code Organization**: Clear separation between storage definition and usage
//! 3. **Performance**: Batched operations and efficient encodings
//! 4. **Abstraction**: Higher-level data structures hide RocksDB complexity
//! 5. **Flexibility**: Easy to add new column families or data structures

use crate::rocksdb::{
    default_db_options, open_db, open_db_with_migrations, DatabaseError, MigrationRunner,
};
use codec::{Decode, Encode};
use rocksdb::{
    AsColumnFamilyRef, ColumnFamily, DBPinnableSlice, Direction, IteratorMode, ReadOptions,
    WriteBatch, DB,
};
use std::{
    cell::{Ref, RefCell},
    collections::BTreeMap,
    marker::PhantomData,
    ops::RangeBounds,
};

/// Defines how types are encoded to and decoded from bytes for storage in RocksDB.
///
/// This trait abstracts the serialization and deserialization process, allowing
/// different encoding formats to be used with the same storage framework.
pub trait DbCodec<T> {
    /// Encode a value to bytes.
    fn encode(value: &T) -> Vec<u8>;

    /// Decode a value from bytes.
    fn decode(bytes: &[u8]) -> T;
}

/// A DbCodec for the SCALE codec.
///
/// This implementation uses the SCALE (Simple Concatenated Aggregate Little-Endian) codec,
/// which is the standard serialization format used throughout the Substrate ecosystem.
/// It works with any type that implements the `Encode` and `Decode` traits.
#[derive(Clone)]
pub struct ScaleDbCodec;

/// Implement the DbCodec trait for any type that implements Encode and Decode (SCALE codec).
impl<T> DbCodec<T> for ScaleDbCodec
where
    T: Encode + Decode,
{
    fn encode(value: &T) -> Vec<u8> {
        value.encode()
    }

    fn decode(bytes: &[u8]) -> T {
        T::decode(&mut &bytes[..]).expect("ScaleDbCodec: Failed to decode value")
    }
}

/// A typed RocksDB column family.
///
/// This trait defines the core properties of a column family in RocksDB, including
/// the types of keys and values it stores, how they are encoded/decoded, and the
/// name used to identify the column family in the database.
///
/// By implementing this trait on empty structs, you can create type-safe column
/// family definitions that ensure correct usage throughout your codebase.
pub trait TypedCf {
    /// Type of the key.
    type Key;
    /// Type of the value.
    type Value;

    /// Type of the [`DbCodec`] for the keys.
    type KeyCodec: DbCodec<Self::Key>;

    /// Type of the [`DbCodec`] for the values.
    type ValueCodec: DbCodec<Self::Value>;

    /// Column family name (as known to the DB).
    const NAME: &'static str;
}

/// A DbCodec for the unit type, used for single row column families.
///
/// This codec is used with column families that store only a single value,
/// effectively implementing a global variable. The key type is `()` (unit),
/// indicating there's only one possible key.
#[derive(Debug, Clone)]
pub struct SingleRowDbCodec;

impl DbCodec<()> for SingleRowDbCodec {
    fn encode(_value: &()) -> Vec<u8> {
        vec![]
    }

    fn decode(_bytes: &[u8]) -> () {
        ()
    }
}

/// A convenience trait implementing [`TypedCf`] for when [`Self::Key`] and [`Self::Value`] support
/// SCALE encode/decode.
///
/// This trait simplifies the definition of column families that use SCALE encoding.
/// It requires only the key and value types and a name, then automatically implements
/// `TypedCf` with the appropriate codec settings.
pub trait ScaleEncodedCf {
    type Key: Encode + Decode;
    type Value: Encode + Decode;

    const SCALE_ENCODED_NAME: &'static str;
}

impl<K: Encode + Decode, V: Encode + Decode, S: ScaleEncodedCf<Key = K, Value = V>> TypedCf for S {
    type Key = K;
    type Value = V;

    type KeyCodec = ScaleDbCodec;
    type ValueCodec = ScaleDbCodec;

    const NAME: &'static str = Self::SCALE_ENCODED_NAME;
}

/// A convenience trait implementing [`ScaleEncodedCf`] for a single SCALE-encoded value column family.
///
/// This trait is used for column families that act as global variables, storing only
/// a single value with the unit type `()` as the key. It further simplifies the definition
/// of these common storage patterns.
pub trait SingleScaleEncodedValueCf {
    type Value: Encode + Decode;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str;
}

impl<V: Encode + Decode, S: SingleScaleEncodedValueCf<Value = V>> ScaleEncodedCf for S {
    type Key = ();
    type Value = V;

    const SCALE_ENCODED_NAME: &'static str = Self::SINGLE_SCALE_ENCODED_VALUE_NAME;
}

/// A RocksDb write buffer used for batching.
///
/// This struct collects multiple write operations (puts, deletes, etc.) before
/// committing them to the database in a single atomic write. This improves performance
/// and ensures consistency by avoiding partial updates.
#[derive(Default)]
pub struct WriteBuffer {
    write_batch: RefCell<WriteBatch>,
}

impl WriteBuffer {
    /// Updates the key of the column family with a value.
    pub fn put(&self, cf: &ColumnFamily, key: Vec<u8>, value: Vec<u8>) {
        self.write_batch.borrow_mut().put_cf(cf, key, value);
    }

    /// Deletes the key of the column family.
    pub fn delete(&self, cf: &ColumnFamily, key: Vec<u8>) {
        self.write_batch.borrow_mut().delete_cf(cf, key);
    }

    /// Deletes all keys in the range of the column family.
    pub fn delete_range(&self, cf: &ColumnFamily, from: Vec<u8>, to: Vec<u8>) {
        self.write_batch.borrow_mut().delete_range_cf(cf, from, to);
    }

    /// Clears the write buffer and returns the previous contents to be flushed.
    pub fn flip(&self) -> WriteBatch {
        self.write_batch.replace(WriteBatch::default())
    }
}

/// Defines read operations for a RocksDB database.
///
/// This trait abstracts the read operations that can be performed on a RocksDB database,
/// such as getting values, iterating over column families, etc. It is implemented by
/// database connection types like `TypedRocksDB`.
pub trait ReadableRocks {
    /// Resolves the column family by name.
    fn cf_handle(&self, name: &str) -> &ColumnFamily;

    /// Gets a single value by key.
    fn get_pinned_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: impl AsRef<[u8]>,
    ) -> Option<DBPinnableSlice<'_>>;

    /// Gets an iterator over the column family.
    fn iterator_cf<'a>(
        &'a self,
        cf: &impl AsColumnFamilyRef,
        mode: IteratorMode,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a;

    /// Gets an iterator over the column family with custom read options.
    fn iterator_cf_opt<'a>(
        &'a self,
        cf: &impl AsColumnFamilyRef,
        mode: IteratorMode,
        read_opts: ReadOptions,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a;
}

/// A write-supporting interface of a RocksDB database.
///
/// This trait extends `ReadableRocks` with the ability to write data to the database.
/// It is implemented by database connection types that support writing, such as `TypedRocksDB`.
pub trait WriteableRocks: ReadableRocks {
    /// Atomically writes the given batch of updates.
    fn write(&self, batch: WriteBatch);
}

/// An internal wrapper for a [`TypedCf`] and dependencies resolved from it.
///
/// This struct connects a typed column family definition with its actual RocksDB handle,
/// ensuring operations have the correct type information.
struct CfHandle<'r, CF: TypedCf> {
    handle: &'r ColumnFamily,
    phantom: PhantomData<CF>,
}

impl<'r, CF: TypedCf> CfHandle<'r, CF> {
    /// Resolves a [`ColumnFamily`] from a [`TypedCf`].
    pub fn resolve<R: ReadableRocks>(rocks: &'r R, _cf: &CF) -> Self {
        let handle = rocks.cf_handle(CF::NAME);
        Self {
            handle,
            phantom: PhantomData,
        }
    }
}

/// A write enabling marker trait to be used with [`TypedDbContext`].
///
/// This trait is used as a type parameter to indicate whether a context
/// supports write operations. It doesn't define any methods but serves
/// as a compile-time constraint on what operations are allowed.
pub trait WriteSupport {}

/// Type that indicates no write support is available.
///
/// This is used with `TypedDbContext` to create a read-only context.
pub struct NoWriteSupport;

impl WriteSupport for NoWriteSupport {}

/// A higher-level database context.
///
/// All reads see the current DB state.
/// All (optional) write capabilities depend upon the used [`WriteSupport`].
///
/// This struct provides a context for interacting with a RocksDB database
/// in a type-safe manner. It keeps track of changes in memory and allows
/// them to be committed as a single transaction if write support is available.
pub struct TypedDbContext<'r, R: ReadableRocks, W: WriteSupport> {
    rocks: &'r R,
    overlay: DbOverlay,
    write_support: W,
}

impl<'r, R: ReadableRocks, W: WriteSupport> TypedDbContext<'r, R, W> {
    /// Creates an instance using the given RocksDB.
    /// The write capabilities depend on the given [`WriteSupport`] implementation.
    pub fn new(rocks: &'r R, write_support: W) -> Self {
        Self {
            rocks,
            overlay: DbOverlay::new(),
            write_support,
        }
    }
}

/// Buffered write support.
///
/// All writes are accumulated in the internal buffer and are not visible to any subsequent reads,
/// until [`BufferedWriteSupport::flush()`] happens (either an explicit one, likely propagated from
/// [`TypedDbContext::flush()`], or an implicit one on [`Drop`]).
///
/// This struct enables batched writes for better performance and atomicity.
pub struct BufferedWriteSupport<'r, R: WriteableRocks> {
    buffer: WriteBuffer,
    rocks: &'r R,
}

impl<'r, R: WriteableRocks> BufferedWriteSupport<'r, R> {
    /// Creates an instance that will flush to the given RocksDB.
    pub fn new(rocks: &'r R) -> Self {
        Self {
            buffer: WriteBuffer::default(),
            rocks,
        }
    }
}

impl<'r, R: WriteableRocks> WriteSupport for BufferedWriteSupport<'r, R> {}

impl<'r, R: WriteableRocks> BufferedWriteSupport<'r, R> {
    /// Writes the batch to the RocksDB and flips the internal buffer.
    fn flush(&self) {
        let write_batch = self.buffer.flip();
        if !write_batch.is_empty() {
            self.rocks.write(write_batch);
        }
    }
}

impl<'r, R: WriteableRocks> Drop for BufferedWriteSupport<'r, R> {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Implementation specific to BufferedWriteSupport
impl<'r, R: WriteableRocks> TypedDbContext<'r, R, BufferedWriteSupport<'r, R>> {
    /// Explicitly flushes the current contents of the write buffer and clears the associated
    /// overlay.
    pub fn flush(&self) {
        self.write_support.flush();
        self.overlay.cfs.borrow_mut().clear();
    }
}

/// A higher-level DB access API bound to its [`TypedDbContext`] and scoped at a specific column
/// family.
///
/// This struct is the main interface for performing type-safe operations on a column family.
/// It ensures that all operations use the correct types for keys and values.
pub struct TypedCfApi<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> {
    cf: CfHandle<'r, CF>,
    rocks: &'r R,
    cf_overlay: Ref<'o, DbCfOverlay>,
    write_support: &'w W,
}

impl<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> TypedCfApi<'r, 'o, 'w, CF, R, W> {
    /// Creates an instance for the given column family.
    fn new(
        cf: CfHandle<'r, CF>,
        rocks: &'r R,
        cf_overlay: Ref<'o, DbCfOverlay>,
        write_support: &'w W,
    ) -> Self {
        Self {
            cf,
            rocks,
            cf_overlay,
            write_support,
        }
    }
}

impl<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> TypedCfApi<'r, 'o, 'w, CF, R, W> {
    /// Gets value by key.
    pub fn get(&self, key: &CF::Key) -> Option<CF::Value> {
        match self.cf_overlay.get(CF::KeyCodec::encode(key)) {
            Some(DbCfOverlayValueOp::Put(value)) => {
                return Some(CF::ValueCodec::decode(&value));
            }
            Some(DbCfOverlayValueOp::Delete) => {
                return None;
            }
            None => {}
        }

        self.rocks
            .get_pinned_cf(self.cf.handle, CF::KeyCodec::encode(key).as_slice())
            .map(|pinnable_slice| CF::ValueCodec::decode(pinnable_slice.as_ref()))
    }
}

impl<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> TypedCfApi<'r, 'o, 'w, CF, R, W> {
    /// Iterates over a range of keys in the column family using Rust's range syntax.
    /// This provides an ergonomic way to express all possible range queries.
    ///
    /// The method supports all standard Rust range types:
    ///
    /// # Examples:
    /// - `iterate_with_range(key1..key2)` - Iterate from key1 to key2 (exclusive)
    /// - `iterate_with_range(key1..=key2)` - Iterate from key1 to key2 (inclusive)
    /// - `iterate_with_range(key1..)` - Iterate from key1 to the end
    /// - `iterate_with_range(..key2)` - Iterate from the beginning to key2 (exclusive)
    /// - `iterate_with_range(..=key2)` - Iterate from the beginning to key2 (inclusive)
    /// - `iterate_with_range(..)` - Iterate over all keys
    ///
    /// For reverse iteration, you can compare the keys manually and use the appropriate range:
    /// - To iterate from key1 to key2 in reverse (where key1 > key2): `iterate_with_range(key1..=key2)`
    /// - To iterate from a key backwards to the beginning: `iterate_with_range(..=key)`
    ///
    /// The direction is automatically determined based on the comparison of the encoded keys.
    pub fn iterate_with_range<Range>(
        &'r self,
        range: Range,
    ) -> Box<dyn Iterator<Item = (CF::Key, CF::Value)> + 'r>
    where
        Range: RangeBounds<CF::Key>,
    {
        use std::ops::Bound;

        match (range.start_bound(), range.end_bound()) {
            (Bound::Included(start), Bound::Excluded(end)) => {
                // Range: start..end
                let from_encoded = CF::KeyCodec::encode(start);
                let to_encoded = CF::KeyCodec::encode(end);

                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(from_encoded.clone());
                read_opts.set_iterate_upper_bound(to_encoded);

                // Clone the slice to extend its lifetime
                let from_slice = from_encoded.clone();
                Box::new(
                    self.rocks
                        .iterator_cf_opt(
                            self.cf.handle,
                            IteratorMode::From(&from_slice, Direction::Forward),
                            read_opts,
                        )
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Included(start), Bound::Included(end)) => {
                // Range: start..=end
                let from_encoded = CF::KeyCodec::encode(start);
                let to_encoded = CF::KeyCodec::encode(end);

                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(from_encoded.clone());

                // For inclusive end, we need to make the upper bound the next possible key
                let mut end_bytes = to_encoded.clone();
                end_bytes.push(0);
                read_opts.set_iterate_upper_bound(end_bytes);

                // Clone the slice to extend its lifetime
                let from_slice = from_encoded.clone();
                Box::new(
                    self.rocks
                        .iterator_cf_opt(
                            self.cf.handle,
                            IteratorMode::From(&from_slice, Direction::Forward),
                            read_opts,
                        )
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Included(start), Bound::Unbounded) => {
                // Range: start..
                let from_encoded = CF::KeyCodec::encode(start);

                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(from_encoded.clone());

                // Clone the slice to extend its lifetime
                let from_slice = from_encoded.clone();
                Box::new(
                    self.rocks
                        .iterator_cf_opt(
                            self.cf.handle,
                            IteratorMode::From(&from_slice, Direction::Forward),
                            read_opts,
                        )
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Excluded(start), Bound::Excluded(end)) => {
                // Range: start+1..end
                let from_encoded = CF::KeyCodec::encode(start);
                let to_encoded = CF::KeyCodec::encode(end);

                let mut read_opts = ReadOptions::default();
                // We need to find the next key after 'start' to implement 'Excluded' semantics
                // For simplicity, we'll just use the same lower bound and filter in the iterator
                read_opts.set_iterate_lower_bound(from_encoded.clone());
                read_opts.set_iterate_upper_bound(to_encoded);

                Box::new(
                    self.rocks
                        .iterator_cf_opt(self.cf.handle, IteratorMode::Start, read_opts)
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Excluded(start), Bound::Included(end)) => {
                // Range: start+1..=end
                let from_encoded = CF::KeyCodec::encode(start);
                let to_encoded = CF::KeyCodec::encode(end);

                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(from_encoded.clone());

                let mut end_bytes = to_encoded.clone();
                end_bytes.push(0);
                read_opts.set_iterate_upper_bound(end_bytes);

                Box::new(
                    self.rocks
                        .iterator_cf_opt(self.cf.handle, IteratorMode::Start, read_opts)
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Excluded(start), Bound::Unbounded) => {
                // Range: start+1..
                let start_bytes = CF::KeyCodec::encode(start);

                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(start_bytes.clone());

                Box::new(
                    self.rocks
                        .iterator_cf_opt(
                            self.cf.handle,
                            IteratorMode::From(&start_bytes, Direction::Forward),
                            read_opts,
                        )
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Unbounded, Bound::Included(end)) => {
                // Range: ..=end
                let end_bytes = CF::KeyCodec::encode(end);
                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_upper_bound(end_bytes);

                Box::new(
                    self.rocks
                        .iterator_cf_opt(self.cf.handle, IteratorMode::Start, read_opts)
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Unbounded, Bound::Excluded(end)) => {
                // Range: ..end
                let end_bytes = CF::KeyCodec::encode(end);
                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_upper_bound(end_bytes);

                Box::new(
                    self.rocks
                        .iterator_cf_opt(self.cf.handle, IteratorMode::Start, read_opts)
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Unbounded, Bound::Unbounded) => {
                // Range: ..
                Box::new(
                    self.rocks
                        .iterator_cf_opt(
                            self.cf.handle,
                            IteratorMode::Start,
                            ReadOptions::default(),
                        )
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
        }
    }
}

impl<'r, R: ReadableRocks, W: WriteSupport> TypedDbContext<'r, R, W> {
    /// Returns a typed helper scoped at the given column family.
    pub fn cf<CF: TypedCf>(&self, typed_cf: &CF) -> TypedCfApi<'r, '_, '_, CF, R, W> {
        // Capture the Ref<DbCfOverlay> in a local variable
        let overlay_cf_ref = self.overlay.cf(CF::NAME);

        TypedCfApi::new(
            CfHandle::resolve(self.rocks, typed_cf),
            self.rocks,
            overlay_cf_ref,
            &self.write_support,
        )
    }
}

/// A RocksDB wrapper which implements [`ReadableRocks`] and [`WriteableRocks`].
///
/// This struct wraps a RocksDB database and provides type-safe access through
/// the traits defined in this module.
pub struct TypedRocksDB {
    pub db: DB,
}

impl TypedRocksDB {
    /// Opens a RocksDB database without migrations.
    ///
    /// Use this for stores that don't need migration support.
    ///
    /// # Schema Version Behavior
    ///
    /// This method creates the `__schema_version__` column family and writes version 0 (no migrations applied).
    ///
    /// Since migrations must start at version 1, this ensures:
    ///
    /// - The database format is consistent with migration-enabled databases
    /// - If the store later switches to [`open_with_migrations`](Self::open_with_migrations),
    ///   all migrations will run
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the database directory
    /// * `current_column_families` - The column families defined in the current schema
    ///
    /// # Example
    ///
    /// ```ignore
    /// const CURRENT_CFS: &[&str] = &["cf1", "cf2", "cf3"];
    /// let db = TypedRocksDB::open("/path/to/db", CURRENT_CFS)?;
    /// ```
    pub fn open(path: &str, current_column_families: &[&str]) -> Result<Self, DatabaseError> {
        let opts = default_db_options();
        let db = open_db(&opts, path, current_column_families)?;
        Ok(Self { db })
    }

    /// Opens a RocksDB database with migration support.
    ///
    /// This method:
    /// 1. Discovers any existing column families in the database (including deprecated ones)
    /// 2. Opens the database with all existing + current column families
    /// 3. Runs pending migrations to drop deprecated column families
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the database directory
    /// * `current_column_families` - The column families defined in the current schema
    ///   (deprecated column families should NOT be included here)
    /// * `migrations` - The store-specific migrations to apply
    ///
    /// # Example
    ///
    /// ```ignore
    /// use shc_common::rocksdb::Migration;
    ///
    /// // 1. Define your migration struct
    /// struct MyStoreV1Migration;
    ///
    /// impl Migration for MyStoreV1Migration {
    ///     fn version(&self) -> u32 {
    ///         1  // Migrations start at version 1
    ///     }
    ///
    ///     fn deprecated_column_families(&self) -> &'static [&'static str] {
    ///         &["old_cf_to_remove", "another_old_cf"]
    ///     }
    ///
    ///     fn description(&self) -> &'static str {
    ///         "Remove deprecated column families from v0 schema"
    ///     }
    /// }
    ///
    /// // 2. Create a function returning all migrations for your store
    /// fn my_store_migrations() -> Vec<Box<dyn Migration>> {
    ///     vec![Box::new(MyStoreV1Migration)]
    /// }
    ///
    /// // 3. Open the database with migrations
    /// const CURRENT_CFS: &[&str] = &["cf1", "cf2", "cf3"];
    /// let db = TypedRocksDB::open_with_migrations("/path/to/db", CURRENT_CFS, my_store_migrations())?;
    /// ```
    pub fn open_with_migrations(
        path: &str,
        current_column_families: &[&str],
        migrations: impl Into<MigrationRunner>,
    ) -> Result<Self, DatabaseError> {
        let opts = default_db_options();
        let db = open_db_with_migrations(&opts, path, current_column_families, migrations)?;
        Ok(Self { db })
    }
}

impl ReadableRocks for TypedRocksDB {
    fn cf_handle(&self, name: &str) -> &ColumnFamily {
        self.db.cf_handle(name).expect(name)
    }

    fn get_pinned_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: impl AsRef<[u8]>,
    ) -> Option<DBPinnableSlice<'_>> {
        self.db.get_pinned_cf(cf, key).expect("DB get by key")
    }

    fn iterator_cf<'a>(
        &'a self,
        cf: &impl AsColumnFamilyRef,
        mode: IteratorMode,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a {
        self.db
            .iterator_cf(cf, mode)
            .map(|result| result.expect("DB iterator"))
    }

    fn iterator_cf_opt<'a>(
        &'a self,
        cf: &impl AsColumnFamilyRef,
        mode: IteratorMode,
        read_opts: ReadOptions,
    ) -> impl Iterator<Item = (Box<[u8]>, Box<[u8]>)> + 'a {
        self.db
            .iterator_cf_opt(cf, read_opts, mode)
            .map(|result| result.expect("DB iterator"))
    }
}

impl WriteableRocks for TypedRocksDB {
    fn write(&self, batch: WriteBatch) {
        self.db.write(batch).expect("DB write batch");
    }
}

/// A key-value operation in the overlay.
///
/// This enum represents operations that will be applied to the database:
/// - `Put`: Set a value
/// - `Delete`: Remove a value
///
/// The overlay stores these operations in memory before committing them to the database.
#[derive(Debug, Clone)]
pub enum DbCfOverlayValueOp {
    Put(Vec<u8>),
    Delete,
}

/// A key in the overlay, composed of the column_family and the key.
///
/// This struct represents a key for the in-memory overlay, wrapping the actual key bytes.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct DbCfOverlayKey {
    pub key: Vec<u8>,
}

impl DbCfOverlayKey {
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }
}

/// An in-memory overlay for all column families in a database.
///
/// This struct maintains a map of column family names to their respective overlays.
/// It accumulates changes in memory before they are committed to the database.
pub struct DbOverlay {
    pub cfs: RefCell<BTreeMap<String, DbCfOverlay>>,
}

impl DbOverlay {
    pub fn new() -> Self {
        Self {
            cfs: RefCell::new(BTreeMap::new()),
        }
    }

    // Return a Ref<DbCfOverlay> instead of &DbCfOverlay
    pub fn cf(&self, cf: &str) -> Ref<'_, DbCfOverlay> {
        if !self.cfs.borrow().contains_key(cf) {
            self.cfs
                .borrow_mut()
                .insert(cf.to_string(), DbCfOverlay::new());
        }
        Ref::map(self.cfs.borrow(), |cfs| cfs.get(cf).expect("Overlay CF"))
    }
}

/// An in memory overlay for a column family used by [`TypedCfApi`].
///
/// This struct maintains a map of keys to their pending operations (put or delete).
/// It allows changes to be accumulated in memory before being committed to the database.
pub struct DbCfOverlay {
    pub key_value: RefCell<BTreeMap<DbCfOverlayKey, DbCfOverlayValueOp>>,
}

impl DbCfOverlay {
    pub fn new() -> Self {
        Self {
            key_value: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn get(&self, key: Vec<u8>) -> Option<DbCfOverlayValueOp> {
        self.key_value
            .borrow()
            .get(&DbCfOverlayKey::new(key))
            .cloned()
    }

    pub fn put(&self, key: Vec<u8>, value: Vec<u8>) {
        self.key_value
            .borrow_mut()
            .insert(DbCfOverlayKey::new(key), DbCfOverlayValueOp::Put(value));
    }

    pub fn delete(&self, key: Vec<u8>) {
        self.key_value
            .borrow_mut()
            .insert(DbCfOverlayKey::new(key), DbCfOverlayValueOp::Delete);
    }
}

/// A scoped access to a single value column family.
///
/// This struct provides a convenient interface for accessing column families
/// that store only a single value (global variables).
pub struct SingleValueScopedAccess<'a, CF: SingleScaleEncodedValueCf> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
    cf: &'a CF,
}

impl<'a, CF: SingleScaleEncodedValueCf> SingleValueScopedAccess<'a, CF> {
    pub fn new(
        db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
        cf: &'a CF,
    ) -> Self {
        SingleValueScopedAccess { db_context, cf }
    }

    pub fn read(&self) -> Option<CF::Value> {
        self.db_context.cf(self.cf).get(&())
    }

    pub fn write(&mut self, value: &CF::Value) {
        self.db_context.cf(self.cf).put(&(), &value);
    }

    pub fn delete(&mut self) {
        self.db_context.cf(self.cf).delete(&());
    }
}

/// A trait for providing a database context.
///
/// This trait is implemented by types that can provide access to a `TypedDbContext`,
/// which is required for interacting with the database.
pub trait ProvidesDbContext {
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>>;
}

/// A trait which provides access to single value CFs.
///
/// This trait extends `ProvidesDbContext` with a convenience method for
/// accessing column families that store only a single value (global variables).
pub trait ProvidesTypedDbSingleAccess: ProvidesDbContext {
    fn access_value<'a, CF: SingleScaleEncodedValueCf>(
        &'a self,
        cf: &'a CF,
    ) -> SingleValueScopedAccess<'a, CF> {
        SingleValueScopedAccess::new(self.db_context(), cf)
    }
}

/// A trait which provides access to all column families.
///
/// This trait extends `ProvidesDbContext` with a convenience method for
/// accessing any column family in a type-safe manner.
pub trait ProvidesTypedDbAccess: ProvidesDbContext {
    fn access<'a, CF: TypedCf>(
        &'a self,
        cf: &'a CF,
    ) -> TypedCfApi<'a, 'a, 'a, CF, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>> {
        self.db_context().cf(cf)
    }
}

/// A trait for a deque-like on top of RocksDb.
///
/// This trait implements queue operations (push_back, pop_front, etc.) using
/// three column families:
/// - A left index CF that tracks the front of the queue
/// - A right index CF that tracks the back of the queue
/// - A data CF that stores the actual values
///
/// This allows efficient queue operations on persisted data.
pub trait CFDequeAPI: ProvidesTypedDbSingleAccess {
    /// The type of the value stored in the deque.
    type Value;
    /// The left index column family.
    type LeftIndexCF: Default + SingleScaleEncodedValueCf<Value = u64>;
    /// The right index column family.
    type RightIndexCF: Default + SingleScaleEncodedValueCf<Value = u64>;
    /// The actual data column family.
    type DataCF: Default + TypedCf<Key = u64, Value = Self::Value>;

    fn left_index(&self) -> u64 {
        self.access_value(&Self::LeftIndexCF::default())
            .read()
            .unwrap_or(0)
    }

    fn right_index(&self) -> u64 {
        self.access_value(&Self::RightIndexCF::default())
            .read()
            .unwrap_or(0)
    }

    fn push_back(&mut self, value: Self::Value) {
        let right_index = self.right_index();
        self.db_context()
            .cf(&Self::DataCF::default())
            .put(&right_index, &value);
        self.access_value(&Self::RightIndexCF::default())
            .write(&(right_index + 1));
    }

    fn pop_front(&mut self) -> Option<Self::Value> {
        if self.size() == 0 {
            return None;
        }
        let left_index = self.left_index();
        let value = self
            .db_context()
            .cf(&Self::DataCF::default())
            .get(&left_index);
        self.db_context()
            .cf(&Self::DataCF::default())
            .delete(&left_index);
        self.access_value(&Self::LeftIndexCF::default())
            .write(&(left_index + 1));
        value
    }

    fn size(&self) -> u64 {
        self.right_index() - self.left_index()
    }
}

// Add implementation for TypedCfApi with BufferedWriteSupport
impl<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks>
    TypedCfApi<'r, 'o, 'w, CF, R, BufferedWriteSupport<'r, R>>
where
    R: WriteableRocks,
{
    /// Updates the key with a value.
    pub fn put(&self, key: &CF::Key, value: &CF::Value) {
        let key_bytes = CF::KeyCodec::encode(key);
        let value_bytes = CF::ValueCodec::encode(value);
        self.write_support
            .buffer
            .put(self.cf.handle, key_bytes.clone(), value_bytes.clone());
        self.cf_overlay.put(key_bytes, value_bytes);
    }

    /// Deletes the key.
    pub fn delete(&self, key: &CF::Key) {
        let key_bytes = CF::KeyCodec::encode(key);
        self.write_support
            .buffer
            .delete(self.cf.handle, key_bytes.clone());
        self.cf_overlay.delete(key_bytes);
    }
}

/// A trait for a hashset-like structure on top of RocksDB.
/// This trait provides common operations for working with sets of keys.
///
/// This implements set operations (insert, remove, contains, etc.) using
/// a single column family where the keys are the set elements and the values are
/// empty (unit type). This is useful for implementing collections of unique items.
pub trait CFHashSetAPI: ProvidesTypedDbAccess {
    /// The type of the key stored in the hashset.
    type Value: Encode + Decode;

    /// The column family used to store the hashset.
    type SetCF: Default + TypedCf<Key = Self::Value, Value = ()>;

    /// Checks if the hashset contains the given key.
    fn contains(&self, key: &Self::Value) -> bool {
        self.db_context()
            .cf(&Self::SetCF::default())
            .get(key)
            .is_some()
    }

    /// Inserts a key into the hashset.
    /// Returns true if the key was not present in the set.
    fn insert(&mut self, key: &Self::Value) -> bool {
        let was_present = self.contains(key);
        if !was_present {
            self.db_context().cf(&Self::SetCF::default()).put(key, &());
        }
        !was_present
    }

    /// Removes a key from the hashset.
    /// Returns true if the key was present in the set.
    fn remove(&mut self, key: &Self::Value) -> bool {
        let was_present = self.contains(key);
        if was_present {
            self.db_context().cf(&Self::SetCF::default()).delete(key);
        }
        was_present
    }

    /// Returns all keys in the hashset as a vector, in order.
    fn keys(&self) -> Vec<Self::Value> {
        self.db_context()
            .cf(&Self::SetCF::default())
            .iterate_with_range(..)
            .map(|(key, _)| key)
            .collect()
    }

    /// Returns keys in the given range as a vector, in order.
    fn keys_in_range<R: RangeBounds<Self::Value>>(&self, range: R) -> Vec<Self::Value> {
        self.db_context()
            .cf(&Self::SetCF::default())
            .iterate_with_range(range)
            .map(|(key, _)| key)
            .collect()
    }

    /// Performs an operation on each key in the hashset.
    /// This method iterates over the keys without collecting them all into memory.
    fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Value),
    {
        for (key, _) in self
            .db_context()
            .cf(&Self::SetCF::default())
            .iterate_with_range(..)
        {
            f(&key);
        }
    }

    /// Performs an operation on each key in the given range.
    /// This method iterates over the keys without collecting them all into memory.
    fn for_each_in_range<R, F>(&self, range: R, mut f: F)
    where
        R: RangeBounds<Self::Value>,
        F: FnMut(&Self::Value),
    {
        for (key, _) in self
            .db_context()
            .cf(&Self::SetCF::default())
            .iterate_with_range(range)
        {
            f(&key);
        }
    }

    /// Clears all keys from the hashset.
    fn clear(&mut self) {
        let keys: Vec<_> = self.keys();
        for key in keys {
            self.remove(&key);
        }
    }

    /// Returns the number of keys in the hashset.
    fn len(&self) -> usize {
        self.keys().len()
    }

    /// Returns true if the hashset is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A composite key that combines a primary key and a value for efficient range queries
///
/// This struct is used with `CFRangeMapAPI` to implement a multi-value map
/// by encoding both the key and value into a single composite key. This allows
/// efficient range queries for all values associated with a specific key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompositeKey<K, V> {
    pub key: K,
    pub value: V,
}

/// Implement the Encode trait for the CompositeKey struct.
/// This allows the CompositeKey struct to be encoded using the SCALE codec.
impl<K: Encode, V: Encode> Encode for CompositeKey<K, V> {
    fn encode(&self) -> Vec<u8> {
        // Encode key and value separately, then concatenate
        let mut result = self.key.encode();
        result.extend(self.value.encode());
        result
    }
}

/// Implement the Decode trait for the CompositeKey struct.
/// This allows the CompositeKey struct to be decoded using the SCALE codec.
impl<K: Decode, V: Decode> Decode for CompositeKey<K, V> {
    fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
        // This is a simplified implementation that assumes we can decode the key and value
        // from the input stream. In a real implementation, we would need to know the boundary
        // between the key and value.
        let key = K::decode(input)?;
        let value = V::decode(input)?;
        Ok(CompositeKey { key, value })
    }
}

/// A trait for a hashmap-like structure on top of RocksDB that supports efficient range queries within keys.
/// This implementation uses a composite key approach where the key and value are combined into a single key,
/// and the actual value stored is empty (unit type).
///
/// This trait implements map operations with the ability to store multiple values per key
/// and efficiently query ranges of values for a specific key. It's particularly useful for
/// implementing relationships like tracking chunks belonging to a file.
pub trait CFRangeMapAPI: ProvidesTypedDbAccess {
    /// The type of the key stored in the hashmap.
    type Key: Encode + Decode + Clone + PartialEq + Eq + PartialOrd + Ord;

    /// The type of the value elements stored for each key.
    /// The Default trait is required for creating empty values in range queries.
    type Value: Encode + Decode + Clone + PartialEq + Eq + PartialOrd + Ord + Default;

    /// The column family used to store the hashmap.
    type MapCF: Default + TypedCf<Key = CompositeKey<Self::Key, Self::Value>, Value = ()>;

    /// Checks if the hashmap contains the given key.
    fn contains_key(&self, key: &Self::Key) -> bool {
        // Create a range that only includes the specific key
        let start = CompositeKey {
            key: key.clone(),
            value: Self::Value::default(),
        };
        let mut end_key = key.clone();
        // This assumes we can increment the last byte to get the next key
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }
        let end = CompositeKey {
            key: end_key,
            value: Self::Value::default(),
        };

        // Check if there's at least one entry for this key
        self.db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(start..end)
            .next()
            .is_some()
    }

    /// Checks if the hashmap contains the given key-value pair.
    fn contains(&self, key: &Self::Key, value: &Self::Value) -> bool {
        let composite_key = CompositeKey {
            key: key.clone(),
            value: value.clone(),
        };
        self.db_context()
            .cf(&Self::MapCF::default())
            .get(&composite_key)
            .is_some()
    }

    /// Inserts a key-value pair into the hashmap.
    /// Returns true if the pair was not present in the map.
    fn insert(&self, key: &Self::Key, value: Self::Value) -> bool {
        let composite_key = CompositeKey {
            key: key.clone(),
            value,
        };
        if self
            .db_context()
            .cf(&Self::MapCF::default())
            .get(&composite_key)
            .is_some()
        {
            return false;
        }
        self.db_context()
            .cf(&Self::MapCF::default())
            .put(&composite_key, &());
        true
    }

    /// Removes a specific key-value pair from the hashmap.
    /// Returns true if the pair was found and removed.
    fn remove(&self, key: &Self::Key, value: &Self::Value) -> bool {
        let composite_key = CompositeKey {
            key: key.clone(),
            value: value.clone(),
        };
        if self
            .db_context()
            .cf(&Self::MapCF::default())
            .get(&composite_key)
            .is_none()
        {
            return false;
        }
        self.db_context()
            .cf(&Self::MapCF::default())
            .delete(&composite_key);
        true
    }

    /// Removes all values for a specific key.
    /// Returns the number of values removed.
    fn remove_key(&self, key: &Self::Key) -> usize {
        let start = CompositeKey {
            key: key.clone(),
            value: Self::Value::default(),
        };
        let mut end_key = key.clone();
        // This assumes we can increment the last byte to get the next key
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }
        let end = CompositeKey {
            key: end_key,
            value: Self::Value::default(),
        };

        // Use a batched approach to avoid loading everything into memory at once
        const BATCH_SIZE: usize = 1000;
        let mut total_removed = 0;

        loop {
            let values: Vec<_> = self
                .db_context()
                .cf(&Self::MapCF::default())
                .iterate_with_range(start.clone()..end.clone())
                .take(BATCH_SIZE)
                .map(|(k, _)| k)
                .collect();

            if values.is_empty() {
                break;
            }

            let batch_size = values.len();
            for composite_key in values {
                self.db_context()
                    .cf(&Self::MapCF::default())
                    .delete(&composite_key);
            }

            total_removed += batch_size;

            // Flush after each batch to ensure changes are visible
            self.db_context().flush();
        }

        total_removed
    }

    /// Returns all keys in the hashmap.
    fn keys(&self) -> Vec<Self::Key> {
        let mut result = Vec::new();
        let mut current_key: Option<Self::Key> = None;

        for (composite_key, _) in self
            .db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(..)
        {
            if current_key.as_ref() != Some(&composite_key.key) {
                current_key = Some(composite_key.key.clone());
                result.push(composite_key.key);
            }
        }

        result
    }

    /// Returns all values for a specific key.
    fn values_for_key(&self, key: &Self::Key) -> Vec<Self::Value> {
        // Create a range that only includes the specific key
        let start = CompositeKey {
            key: key.clone(),
            value: Self::Value::default(),
        };
        let mut end_key = key.clone();
        // This assumes we can increment the last byte to get the next key
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }
        let end = CompositeKey {
            key: end_key,
            value: Self::Value::default(),
        };

        // Only iterate over the range for this specific key
        self.db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(start..end)
            .map(|(composite_key, _)| composite_key.value)
            .collect()
    }

    /// Returns values for a specific key within a given range.
    /// Uses the streaming iterator internally for efficiency.
    fn values_in_range<R>(&self, key: &Self::Key, range: R) -> Vec<Self::Value>
    where
        R: RangeBounds<Self::Value>,
    {
        use std::ops::Bound;

        let start_bound = match range.start_bound() {
            Bound::Included(v) => Bound::Included(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Excluded(v) => Bound::Excluded(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Unbounded => Bound::Included(CompositeKey {
                key: key.clone(),
                value: Self::Value::default(),
            }),
        };

        let mut end_key = key.clone();
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }

        let end_bound = match range.end_bound() {
            Bound::Included(v) => Bound::Included(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Excluded(v) => Bound::Excluded(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Unbounded => Bound::Excluded(CompositeKey {
                key: end_key,
                value: Self::Value::default(),
            }),
        };

        let range = (start_bound, end_bound);

        self.db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(range)
            .map(|(k, _)| k.value)
            .collect()
    }

    /// Performs an operation on each value for a specific key.
    fn for_each_value<F>(&self, key: &Self::Key, mut f: F)
    where
        F: FnMut(&Self::Value),
    {
        let start = CompositeKey {
            key: key.clone(),
            value: Self::Value::default(),
        };
        let mut end_key = key.clone();
        // This assumes we can increment the last byte to get the next key
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }
        let end = CompositeKey {
            key: end_key,
            value: Self::Value::default(),
        };

        for (composite_key, _) in self
            .db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(start..end)
        {
            f(&composite_key.value);
        }
    }

    /// Performs an operation on each value for a specific key within a given range.
    fn for_each_value_in_range<R, F>(&self, key: &Self::Key, range: R, mut f: F)
    where
        R: RangeBounds<Self::Value>,
        F: FnMut(&Self::Value),
    {
        use std::ops::Bound;

        let start_bound = match range.start_bound() {
            Bound::Included(v) => Bound::Included(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Excluded(v) => Bound::Excluded(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Unbounded => Bound::Included(CompositeKey {
                key: key.clone(),
                value: Self::Value::default(),
            }),
        };

        let mut end_key = key.clone();
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }

        let end_bound = match range.end_bound() {
            Bound::Included(v) => Bound::Included(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Excluded(v) => Bound::Excluded(CompositeKey {
                key: key.clone(),
                value: v.clone(),
            }),
            Bound::Unbounded => Bound::Excluded(CompositeKey {
                key: end_key,
                value: Self::Value::default(),
            }),
        };

        let range = (start_bound, end_bound);

        for (composite_key, _) in self
            .db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(range)
        {
            f(&composite_key.value);
        }
    }

    /// Returns the number of keys in the hashmap.
    fn len(&self) -> usize {
        let mut count = 0;
        let mut current_key: Option<Self::Key> = None;

        // Count unique keys without collecting them
        for (composite_key, _) in self
            .db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(..)
        {
            if current_key.as_ref() != Some(&composite_key.key) {
                current_key = Some(composite_key.key.clone());
                count += 1;
            }
        }

        count
    }

    /// Returns true if the hashmap is empty.
    fn is_empty(&self) -> bool {
        self.db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(..)
            .next()
            .is_none()
    }

    /// Returns the number of values for a specific key.
    fn values_len(&self, key: &Self::Key) -> usize {
        // Create a range that only includes the specific key
        let start = CompositeKey {
            key: key.clone(),
            value: Self::Value::default(),
        };
        let mut end_key = key.clone();
        // This assumes we can increment the last byte to get the next key
        let end_bytes = end_key.encode();
        if let Some(last) = end_bytes.last().cloned() {
            let mut new_end = end_bytes.clone();
            *new_end.last_mut().unwrap() = last.wrapping_add(1);
            end_key = Self::Key::decode(&mut &new_end[..]).unwrap_or(end_key);
        }
        let end = CompositeKey {
            key: end_key,
            value: Self::Value::default(),
        };

        // Count values without collecting them
        self.db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(start..end)
            .count()
    }

    /// Clears all key-value pairs from the hashmap.
    fn clear(&self) {
        // Use a batched approach to avoid loading everything into memory at once
        const BATCH_SIZE: usize = 1000;

        loop {
            let keys: Vec<_> = self
                .db_context()
                .cf(&Self::MapCF::default())
                .iterate_with_range(..)
                .take(BATCH_SIZE)
                .map(|(k, _)| k)
                .collect();

            if keys.is_empty() {
                break;
            }

            for composite_key in keys {
                self.db_context()
                    .cf(&Self::MapCF::default())
                    .delete(&composite_key);
            }

            // Flush after each batch to ensure changes are visible
            self.db_context().flush();
        }
    }

    /// Performs an operation on each unique key in the hashmap.
    /// This is more efficient than collecting all keys and then iterating.
    fn for_each_key<F>(&self, mut f: F)
    where
        F: FnMut(&Self::Key),
    {
        let mut current_key: Option<Self::Key> = None;

        for (composite_key, _) in self
            .db_context()
            .cf(&Self::MapCF::default())
            .iterate_with_range(..)
        {
            if current_key.as_ref() != Some(&composite_key.key) {
                current_key = Some(composite_key.key.clone());
                f(&composite_key.key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::{Decode, Encode};
    use rocksdb::{ColumnFamilyDescriptor, Options};
    use tempfile::tempdir;

    // Define test types for our RangeMap
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
    struct TestKey(u32);

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct TestValue(u32);

    impl Default for TestValue {
        // This implementation is required for the CFRangeMapAPI trait
        // which uses Default::default() for empty values in range queries
        fn default() -> Self {
            TestValue(0)
        }
    }

    impl Encode for TestValue {
        fn encode(&self) -> Vec<u8> {
            self.0.encode()
        }
    }

    impl Decode for TestValue {
        fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
            let value = u32::decode(input)?;
            Ok(TestValue(value))
        }
    }

    // Define column family for our test
    struct TestRangeMapCF;

    impl Default for TestRangeMapCF {
        fn default() -> Self {
            Self
        }
    }

    impl TypedCf for TestRangeMapCF {
        type Key = CompositeKey<TestKey, TestValue>;
        type Value = ();

        type KeyCodec = ScaleDbCodec;
        type ValueCodec = ScaleDbCodec;

        const NAME: &'static str = "test_range_map_cf";
    }

    // Define our test struct that implements CFRangeMapAPI
    struct TestRangeMap<'db> {
        db_context: TypedDbContext<'db, TypedRocksDB, BufferedWriteSupport<'db, TypedRocksDB>>,
    }

    impl<'db> ProvidesDbContext for TestRangeMap<'db> {
        fn db_context(
            &self,
        ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
            &self.db_context
        }
    }

    impl<'db> ProvidesTypedDbAccess for TestRangeMap<'db> {}

    impl<'db> CFRangeMapAPI for TestRangeMap<'db> {
        type Key = TestKey;
        type Value = TestValue;
        type MapCF = TestRangeMapCF;
    }

    // Helper function to create a test database and range map
    fn setup_test_range_map() -> (tempfile::TempDir, TestRangeMap<'static>) {
        // Create a temporary directory for the database
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // Open the database with the test column family
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descriptor = ColumnFamilyDescriptor::new(TestRangeMapCF::NAME, Options::default());
        let db = DB::open_cf_descriptors(&opts, path, vec![cf_descriptor]).unwrap();

        // We need to leak the DB to extend its lifetime
        let db_box = Box::new(TypedRocksDB { db });
        let typed_db = Box::leak(db_box);

        // Create a buffered write support
        let write_support = BufferedWriteSupport::new(typed_db);

        // Create a database context
        let db_context = TypedDbContext::new(typed_db, write_support);

        // Create our test range map
        let range_map = TestRangeMap { db_context };

        (temp_dir, range_map)
    }

    #[test]
    fn test_range_map_basic_operations() {
        let (_temp_dir, range_map) = setup_test_range_map();

        // Test basic operations
        let key1 = TestKey(1);
        let value1 = TestValue(10);
        let value2 = TestValue(20);
        let value3 = TestValue(30);

        // Initially the map should be empty
        assert!(range_map.is_empty());
        assert_eq!(range_map.len(), 0);

        // Insert some values
        assert!(range_map.insert(&key1, value1.clone()));
        assert!(range_map.insert(&key1, value2.clone()));
        assert!(range_map.insert(&key1, value3.clone()));

        // Flush the changes to make them visible
        range_map.db_context.flush();

        // Check contains
        assert!(range_map.contains_key(&key1));
        assert!(range_map.contains(&key1, &value1));
        assert!(range_map.contains(&key1, &value2));
        assert!(range_map.contains(&key1, &value3));

        // Check values for key
        let values = range_map.values_for_key(&key1);
        assert_eq!(values.len(), 3);
        assert!(values.contains(&value1));
        assert!(values.contains(&value2));
        assert!(values.contains(&value3));

        // Test range queries
        let values_range = range_map.values_in_range(&key1, value1.clone()..value3.clone());
        assert_eq!(values_range.len(), 2);
        assert!(values_range.contains(&value1));
        assert!(values_range.contains(&value2));

        // Test removal
        assert!(range_map.remove(&key1, &value2));
        range_map.db_context.flush();
        assert!(!range_map.contains(&key1, &value2));

        // Check values after removal
        let values = range_map.values_for_key(&key1);
        assert_eq!(values.len(), 2);
        assert!(values.contains(&value1));
        assert!(values.contains(&value3));

        // Test remove_key
        assert_eq!(range_map.remove_key(&key1), 2);
        range_map.db_context.flush();
        assert!(!range_map.contains_key(&key1));
        assert!(range_map.is_empty());
    }

    #[test]
    fn test_range_map_multiple_keys() {
        let (_temp_dir, range_map) = setup_test_range_map();

        // Test with multiple keys
        let key1 = TestKey(1);
        let key2 = TestKey(2);

        // Insert values for key1
        range_map.insert(&key1, TestValue(10));
        range_map.insert(&key1, TestValue(20));

        // Insert values for key2
        range_map.insert(&key2, TestValue(30));
        range_map.insert(&key2, TestValue(40));

        // Flush the changes to make them visible
        range_map.db_context.flush();

        // Check keys
        let keys = range_map.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&key1));
        assert!(keys.contains(&key2));

        // Check values for each key
        let values1 = range_map.values_for_key(&key1);
        assert_eq!(values1.len(), 2);
        assert!(values1.contains(&TestValue(10)));
        assert!(values1.contains(&TestValue(20)));

        let values2 = range_map.values_for_key(&key2);
        assert_eq!(values2.len(), 2);
        assert!(values2.contains(&TestValue(30)));
        assert!(values2.contains(&TestValue(40)));

        // Test for_each_value
        let mut count = 0;
        range_map.for_each_value(&key1, |_| count += 1);
        assert_eq!(count, 2);

        // Test for_each_value_in_range
        let mut values = Vec::new();
        range_map.for_each_value_in_range(&key2, TestValue(30)..=TestValue(40), |v| {
            values.push(v.clone())
        });
        assert_eq!(values.len(), 2);

        // Test clear
        range_map.clear();
        range_map.db_context.flush();
        assert!(range_map.is_empty());
        assert_eq!(range_map.len(), 0);
    }

    #[test]
    fn test_range_map_duplicate_inserts() {
        let (_temp_dir, range_map) = setup_test_range_map();

        let key = TestKey(1);
        let value = TestValue(10);

        // First insert should succeed
        assert!(range_map.insert(&key, value.clone()));
        range_map.db_context.flush();

        // Second insert of the same key-value pair should fail
        assert!(!range_map.insert(&key, value.clone()));
        range_map.db_context.flush();

        // Check that there's still only one value
        assert_eq!(range_map.values_for_key(&key).len(), 1);
    }

    #[test]
    fn test_range_map_empty_operations() {
        let (_temp_dir, range_map) = setup_test_range_map();

        let key = TestKey(1);
        let value = TestValue(10);

        // Operations on empty map
        assert!(!range_map.contains_key(&key));
        assert!(!range_map.contains(&key, &value));
        assert!(range_map.values_for_key(&key).is_empty());
        assert!(range_map
            .values_in_range(&key, TestValue(0)..TestValue(100))
            .is_empty());

        // Removing from empty map
        assert!(!range_map.remove(&key, &value));
        assert_eq!(range_map.remove_key(&key), 0);

        // Empty map properties
        assert!(range_map.is_empty());
        assert_eq!(range_map.len(), 0);
        assert_eq!(range_map.values_len(&key), 0);

        // for_each on empty map
        let mut count = 0;
        range_map.for_each_value(&key, |_| count += 1);
        assert_eq!(count, 0);

        // for_each_in_range on empty map
        let mut count = 0;
        range_map.for_each_value_in_range(&key, TestValue(0)..TestValue(100), |_| count += 1);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_range_map_range_queries() {
        let (_temp_dir, range_map) = setup_test_range_map();

        let key = TestKey(1);

        // Insert values 10 through 100 in steps of 10
        for i in 1..=10 {
            range_map.insert(&key, TestValue(i * 10));
        }
        range_map.db_context.flush();

        // Test inclusive range
        let values = range_map.values_in_range(&key, TestValue(20)..=TestValue(50));
        assert_eq!(values.len(), 4); // 20, 30, 40, 50
        assert!(values.contains(&TestValue(20)));
        assert!(values.contains(&TestValue(30)));
        assert!(values.contains(&TestValue(40)));
        assert!(values.contains(&TestValue(50)));

        // Test exclusive range
        let values = range_map.values_in_range(&key, TestValue(20)..TestValue(50));
        assert_eq!(values.len(), 3); // 20, 30, 40
        assert!(values.contains(&TestValue(20)));
        assert!(values.contains(&TestValue(30)));
        assert!(values.contains(&TestValue(40)));
        assert!(!values.contains(&TestValue(50)));

        // Test unbounded start
        let values = range_map.values_in_range(&key, ..TestValue(30));
        assert_eq!(values.len(), 2); // 10, 20
        assert!(values.contains(&TestValue(10)));
        assert!(values.contains(&TestValue(20)));
        assert!(!values.contains(&TestValue(30))); // Exclusive upper bound

        // Test inclusive unbounded start
        let values = range_map.values_in_range(&key, ..=TestValue(30));
        assert_eq!(values.len(), 3); // 10, 20, 30
        assert!(values.contains(&TestValue(10)));
        assert!(values.contains(&TestValue(20)));
        assert!(values.contains(&TestValue(30)));

        // Test unbounded end
        let values = range_map.values_in_range(&key, TestValue(80)..);
        assert_eq!(values.len(), 3); // 80, 90, 100
        assert!(values.contains(&TestValue(80)));
        assert!(values.contains(&TestValue(90)));
        assert!(values.contains(&TestValue(100)));

        // Test full range
        let values = range_map.values_in_range(&key, ..);
        assert_eq!(values.len(), 10); // All values

        // Test for_each_value_in_range with different range types
        let mut values = Vec::new();
        range_map.for_each_value_in_range(&key, TestValue(60)..=TestValue(80), |v| {
            values.push(v.clone())
        });
        assert_eq!(values.len(), 3); // 60, 70, 80
        assert!(values.contains(&TestValue(60)));
        assert!(values.contains(&TestValue(70)));
        assert!(values.contains(&TestValue(80)));
    }

    #[test]
    fn test_range_map_edge_cases() {
        let (_temp_dir, range_map) = setup_test_range_map();

        // Test with minimum and maximum values
        let key = TestKey(u32::MAX);
        let min_value = TestValue(u32::MIN);
        let max_value = TestValue(u32::MAX);

        range_map.insert(&key, min_value.clone());
        range_map.insert(&key, max_value.clone());
        range_map.db_context.flush();

        assert!(range_map.contains(&key, &min_value));
        assert!(range_map.contains(&key, &max_value));

        // Test with adjacent keys
        let key1 = TestKey(100);
        let key2 = TestKey(101);

        range_map.insert(&key1, TestValue(1));
        range_map.insert(&key2, TestValue(2));
        range_map.db_context.flush();

        assert_eq!(range_map.values_for_key(&key1).len(), 1);
        assert_eq!(range_map.values_for_key(&key2).len(), 1);

        // Test removing non-existent values
        assert!(!range_map.remove(&key1, &TestValue(999)));
        assert!(!range_map.remove(&TestKey(999), &TestValue(1)));

        // Test with empty range
        let values = range_map.values_in_range(&key1, TestValue(2)..TestValue(1));
        assert!(values.is_empty());

        // Test with point range (single value)
        let values = range_map.values_in_range(&key1, TestValue(1)..=TestValue(1));
        assert_eq!(values.len(), 1);
        assert!(values.contains(&TestValue(1)));
    }

    #[test]
    fn test_range_map_persistence() {
        // This test verifies that data persists across database connections
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().to_str().unwrap();

        // First connection
        {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            let cf_descriptor =
                ColumnFamilyDescriptor::new(TestRangeMapCF::NAME, Options::default());
            let db = DB::open_cf_descriptors(&opts, path, vec![cf_descriptor]).unwrap();
            let typed_db = TypedRocksDB { db };

            let write_support = BufferedWriteSupport::new(&typed_db);
            let db_context = TypedDbContext::new(&typed_db, write_support);
            let range_map = TestRangeMap { db_context };

            // Insert some data
            range_map.insert(&TestKey(1), TestValue(10));
            range_map.insert(&TestKey(1), TestValue(20));
            range_map.insert(&TestKey(2), TestValue(30));

            // Flush to ensure data is written to disk
            range_map.db_context.flush();

            // DB will be closed when it goes out of scope
        }

        // Second connection
        {
            let opts = Options::default();
            let cf_descriptor =
                ColumnFamilyDescriptor::new(TestRangeMapCF::NAME, Options::default());
            let db = DB::open_cf_descriptors(&opts, path, vec![cf_descriptor]).unwrap();
            let typed_db = TypedRocksDB { db };

            let write_support = BufferedWriteSupport::new(&typed_db);
            let db_context = TypedDbContext::new(&typed_db, write_support);
            let range_map = TestRangeMap { db_context };

            // Verify data persisted
            assert!(range_map.contains(&TestKey(1), &TestValue(10)));
            assert!(range_map.contains(&TestKey(1), &TestValue(20)));
            assert!(range_map.contains(&TestKey(2), &TestValue(30)));

            // Verify counts
            assert_eq!(range_map.values_for_key(&TestKey(1)).len(), 2);
            assert_eq!(range_map.values_for_key(&TestKey(2)).len(), 1);

            // Add more data
            range_map.insert(&TestKey(3), TestValue(40));
            range_map.db_context.flush();
        }

        // Third connection to verify second connection's changes
        {
            let opts = Options::default();
            let cf_descriptor =
                ColumnFamilyDescriptor::new(TestRangeMapCF::NAME, Options::default());
            let db = DB::open_cf_descriptors(&opts, path, vec![cf_descriptor]).unwrap();
            let typed_db = TypedRocksDB { db };

            let write_support = BufferedWriteSupport::new(&typed_db);
            let db_context = TypedDbContext::new(&typed_db, write_support);
            let range_map = TestRangeMap { db_context };

            // Verify all data including the new addition
            assert!(range_map.contains(&TestKey(1), &TestValue(10)));
            assert!(range_map.contains(&TestKey(1), &TestValue(20)));
            assert!(range_map.contains(&TestKey(2), &TestValue(30)));
            assert!(range_map.contains(&TestKey(3), &TestValue(40)));

            // Clean up
            range_map.clear();
            range_map.db_context.flush();
            assert!(range_map.is_empty());
        }
    }

    #[test]
    fn test_range_map_concurrent_operations() {
        let (_temp_dir, range_map) = setup_test_range_map();

        // Insert a large number of values
        for i in 0..100 {
            range_map.insert(&TestKey(i), TestValue(i));
        }
        range_map.db_context.flush();

        // Verify all values were inserted
        for i in 0..100 {
            assert!(range_map.contains(&TestKey(i), &TestValue(i)));
        }

        // Remove every other value
        for i in (0..100).step_by(2) {
            assert!(range_map.remove(&TestKey(i), &TestValue(i)));
        }
        range_map.db_context.flush();

        // Verify correct values remain
        for i in 0..100 {
            if i % 2 == 0 {
                assert!(!range_map.contains(&TestKey(i), &TestValue(i)));
            } else {
                assert!(range_map.contains(&TestKey(i), &TestValue(i)));
            }
        }

        // Count remaining keys
        let keys = range_map.keys();
        assert_eq!(keys.len(), 50); // Only odd-numbered keys remain
    }

    /// Tests for `TypedRocksDB::open()` and `TypedRocksDB::open_with_migrations()`.
    ///
    /// These tests verify the database opening methods work correctly with the typed API.
    mod typed_rocks_db_open_tests {
        use super::*;
        use crate::rocksdb::{Migration, MigrationRunner};

        // Test column family names
        const DEPRECATED_CF: &str = "deprecated_cf";

        // Test column family definition
        struct TestDataCf;
        impl ScaleEncodedCf for TestDataCf {
            type Key = u32;
            type Value = String;
            const SCALE_ENCODED_NAME: &'static str = "test_data_cf";
        }
        impl Default for TestDataCf {
            fn default() -> Self {
                Self
            }
        }

        // Test migration definition
        struct TestV1Migration;
        impl Migration for TestV1Migration {
            fn version(&self) -> u32 {
                1
            }
            fn deprecated_column_families(&self) -> &'static [&'static str] {
                &[DEPRECATED_CF]
            }
            fn description(&self) -> &'static str {
                "Remove deprecated column family"
            }
        }

        fn test_migrations() -> Vec<Box<dyn Migration>> {
            vec![Box::new(TestV1Migration)]
        }

        #[test]
        fn open_creates_fresh_db_and_works_with_typed_context() {
            let temp_dir = tempdir().unwrap();
            let path = temp_dir.path().to_str().unwrap();

            // TypedRocksDB::open() creates the database from scratch
            let typed_db = TypedRocksDB::open(path, &[TestDataCf::SCALE_ENCODED_NAME]).unwrap();

            // Use typed context API
            let write_support = BufferedWriteSupport::new(&typed_db);
            let context = TypedDbContext::new(&typed_db, write_support);

            context.cf(&TestDataCf).put(&42u32, &"hello".to_string());
            context.flush();

            assert_eq!(
                context.cf(&TestDataCf).get(&42u32),
                Some("hello".to_string())
            );
        }

        #[test]
        fn open_sets_schema_version_to_zero() {
            let temp_dir = tempdir().unwrap();
            let path = temp_dir.path().to_str().unwrap();

            let typed_db = TypedRocksDB::open(path, &[TestDataCf::SCALE_ENCODED_NAME]).unwrap();

            // Verify schema version is 0 (baseline for future migrations)
            let version = MigrationRunner::read_schema_version(&typed_db.db).unwrap();
            assert_eq!(version, Some(0));
        }

        #[test]
        fn open_with_migrations_on_fresh_db_applies_all_migrations() {
            let temp_dir = tempdir().unwrap();
            let path = temp_dir.path().to_str().unwrap();

            // Open fresh database with migrations
            let typed_db = TypedRocksDB::open_with_migrations(
                path,
                &[TestDataCf::SCALE_ENCODED_NAME],
                test_migrations(),
            )
            .unwrap();

            // Schema version should be at latest (v1)
            let version = MigrationRunner::read_schema_version(&typed_db.db).unwrap();
            assert_eq!(version, Some(1));

            // Typed context should work
            let write_support = BufferedWriteSupport::new(&typed_db);
            let context = TypedDbContext::new(&typed_db, write_support);

            context.cf(&TestDataCf).put(&1u32, &"data".to_string());
            context.flush();
            assert_eq!(context.cf(&TestDataCf).get(&1u32), Some("data".to_string()));
        }

        #[test]
        fn open_with_migrations_drops_deprecated_cfs_from_existing_db() {
            let temp_dir = tempdir().unwrap();
            let path = temp_dir.path().to_str().unwrap();

            // Simulate an existing database with deprecated column families.
            // This raw setup is necessary because we're testing the upgrade path
            // where an old database has CFs that need to be removed.
            {
                let mut opts = Options::default();
                opts.create_if_missing(true);
                opts.create_missing_column_families(true);

                let cf_descriptors = vec![
                    ColumnFamilyDescriptor::new("default", Options::default()),
                    ColumnFamilyDescriptor::new(TestDataCf::SCALE_ENCODED_NAME, Options::default()),
                    ColumnFamilyDescriptor::new(DEPRECATED_CF, Options::default()),
                ];

                let db = DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap();

                // Write data to the deprecated CF (simulating old data)
                let deprecated_cf = db.cf_handle(DEPRECATED_CF).unwrap();
                db.put_cf(&deprecated_cf, b"old_key", b"old_value").unwrap();

                // Write data to the active CF (should survive migration)
                let active_cf = db.cf_handle(TestDataCf::SCALE_ENCODED_NAME).unwrap();
                db.put_cf(
                    &active_cf,
                    &1u32.encode(),
                    &"preserved".to_string().encode(),
                )
                .unwrap();
            }

            // Open with migrations - deprecated CF should be dropped
            let typed_db = TypedRocksDB::open_with_migrations(
                path,
                &[TestDataCf::SCALE_ENCODED_NAME],
                test_migrations(),
            )
            .unwrap();

            // Verify deprecated CF was removed
            assert!(
                typed_db.db.cf_handle(DEPRECATED_CF).is_none(),
                "Migration should have dropped deprecated CF"
            );

            // Verify data in active CF was preserved
            let write_support = BufferedWriteSupport::new(&typed_db);
            let context = TypedDbContext::new(&typed_db, write_support);
            assert_eq!(
                context.cf(&TestDataCf).get(&1u32),
                Some("preserved".to_string())
            );
        }
    }
}

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

pub trait DbCodec<T> {
    /// Encode a value to bytes.
    fn encode(value: &T) -> Vec<u8>;

    /// Decode a value from bytes.
    fn decode(bytes: &[u8]) -> T;
}

/// A DbCodec for the SCALE codec.
#[derive(Clone)]
pub struct ScaleDbCodec;

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
pub trait TypedCf {
    type Key;
    type Value;

    /// Type of the [`DbCodec`] for the keys.
    type KeyCodec: DbCodec<Self::Key>;

    /// Type of the [`DbCodec`] for the values.
    type ValueCodec: DbCodec<Self::Value>;

    /// Column family name (as known to the DB).
    const NAME: &'static str;
}

/// A DbCodec for the unit type, used for single row column families.
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

pub trait ReadableRocks {
    /// Resolves the column family by name.
    fn cf_handle(&self, name: &str) -> &ColumnFamily;

    /// Gets a single value by key.
    fn get_pinned_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: impl AsRef<[u8]>,
    ) -> Option<DBPinnableSlice>;

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
pub trait WriteableRocks: ReadableRocks {
    /// Atomically writes the given batch of updates.
    fn write(&self, batch: WriteBatch);
}

/// An internal wrapper for a [`TypedCf`] and dependencies resolved from it.
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

/// An write enabling marker trait to be used with [`TypedDbContext`].
pub trait WriteSupport {}

pub struct NoWriteSupport;

impl WriteSupport for NoWriteSupport {}

/// A higher-level database context.
///
/// All reads see the current DB state.
/// All (optional) write capabilities depend upon the used [`WriteSupport`].
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

                // Compare the encoded bytes to determine direction
                let direction = if from_encoded > to_encoded {
                    Direction::Reverse
                } else {
                    Direction::Forward
                };

                let mut read_opts = ReadOptions::default();

                // Set bounds based on direction
                match direction {
                    Direction::Forward => {
                        read_opts.set_iterate_lower_bound(from_encoded.clone());
                        read_opts.set_iterate_upper_bound(to_encoded);

                        Box::new(
                            self.rocks
                                .iterator_cf_opt(
                                    self.cf.handle,
                                    IteratorMode::From(from_encoded.as_slice(), direction),
                                    read_opts,
                                )
                                .map(|(key, value)| {
                                    (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                                }),
                        )
                    }
                    Direction::Reverse => {
                        // For reverse iteration, we need to swap the bounds
                        read_opts.set_iterate_lower_bound(to_encoded);
                        read_opts.set_iterate_upper_bound(from_encoded.clone());

                        Box::new(
                            self.rocks
                                .iterator_cf_opt(
                                    self.cf.handle,
                                    IteratorMode::From(from_encoded.as_slice(), direction),
                                    read_opts,
                                )
                                .map(|(key, value)| {
                                    (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                                }),
                        )
                    }
                }
            }
            (Bound::Included(start), Bound::Included(end)) => {
                // Range: start..=end
                let from_encoded = CF::KeyCodec::encode(start);
                let to_encoded = CF::KeyCodec::encode(end);

                // Compare the encoded bytes to determine direction
                let direction = if from_encoded > to_encoded {
                    Direction::Reverse
                } else {
                    Direction::Forward
                };

                let mut read_opts = ReadOptions::default();

                match direction {
                    Direction::Forward => {
                        // We need to handle this specially since RocksDB's upper bound is exclusive
                        read_opts.set_iterate_lower_bound(from_encoded.clone());

                        // For inclusive end, we need to make the upper bound the next possible key
                        let mut end_bytes = to_encoded.clone();
                        // Add a byte to make it the next possible key (to simulate inclusive end)
                        end_bytes.push(0);
                        read_opts.set_iterate_upper_bound(end_bytes);

                        Box::new(
                            self.rocks
                                .iterator_cf_opt(
                                    self.cf.handle,
                                    IteratorMode::From(from_encoded.as_slice(), direction),
                                    read_opts,
                                )
                                .map(|(key, value)| {
                                    (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                                }),
                        )
                    }
                    Direction::Reverse => {
                        // For reverse iteration with inclusive end, we need to swap the bounds
                        read_opts.set_iterate_lower_bound(to_encoded.clone());
                        read_opts.set_iterate_upper_bound(from_encoded.clone());

                        Box::new(
                            self.rocks
                                .iterator_cf_opt(
                                    self.cf.handle,
                                    IteratorMode::From(from_encoded.as_slice(), direction),
                                    read_opts,
                                )
                                .map(|(key, value)| {
                                    (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                                }),
                        )
                    }
                }
            }
            (Bound::Included(start), Bound::Unbounded) => {
                // Range: start..
                let from_encoded = CF::KeyCodec::encode(start);
                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(from_encoded.clone());

                Box::new(
                    self.rocks
                        .iterator_cf_opt(
                            self.cf.handle,
                            IteratorMode::From(from_encoded.as_slice(), Direction::Forward),
                            read_opts,
                        )
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Unbounded, Bound::Excluded(end)) => {
                // Range: ..end
                let to_encoded = CF::KeyCodec::encode(end);
                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_upper_bound(to_encoded);

                Box::new(
                    self.rocks
                        .iterator_cf_opt(self.cf.handle, IteratorMode::Start, read_opts)
                        .map(|(key, value)| {
                            (CF::KeyCodec::decode(&key), CF::ValueCodec::decode(&value))
                        }),
                )
            }
            (Bound::Unbounded, Bound::Included(end)) => {
                // Range: ..=end
                // Similar to start..=end, but from the beginning
                let end_encoded = CF::KeyCodec::encode(end);
                let mut end_bytes = end_encoded.clone();
                // Add a byte to make it the next possible key (to simulate inclusive end)
                end_bytes.push(0);

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
            _ => {
                // This should never happen with Rust's range types
                panic!("Unsupported range bounds")
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
pub struct TypedRocksDB {
    pub db: DB,
}

impl ReadableRocks for TypedRocksDB {
    fn cf_handle(&self, name: &str) -> &ColumnFamily {
        self.db.cf_handle(name).expect(name)
    }

    fn get_pinned_cf(
        &self,
        cf: &impl AsColumnFamilyRef,
        key: impl AsRef<[u8]>,
    ) -> Option<DBPinnableSlice> {
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
#[derive(Debug, Clone)]
pub enum DbCfOverlayValueOp {
    Put(Vec<u8>),
    Delete,
}

/// A key in the overlay, composed of the column_family and the key.
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct DbCfOverlayKey {
    pub key: Vec<u8>,
}

impl DbCfOverlayKey {
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }
}

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
    pub fn cf(&self, cf: &str) -> Ref<DbCfOverlay> {
        if !self.cfs.borrow().contains_key(cf) {
            self.cfs
                .borrow_mut()
                .insert(cf.to_string(), DbCfOverlay::new());
        }
        Ref::map(self.cfs.borrow(), |cfs| cfs.get(cf).expect("Overlay CF"))
    }
}

/// An in memory overlay for a column family used by [`TypedCfApi`].
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
pub trait ProvidesDbContext {
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>>;
}

/// A trait which provides access to single value CFs.
pub trait ProvidesTypedDbSingleAccess: ProvidesDbContext {
    fn access_value<'a, CF: SingleScaleEncodedValueCf>(
        &'a self,
        cf: &'a CF,
    ) -> SingleValueScopedAccess<'a, CF> {
        SingleValueScopedAccess::new(self.db_context(), cf)
    }
}

pub trait ProvidesTypedDbAccess: ProvidesDbContext {
    fn access<'a, CF: TypedCf>(
        &'a self,
        cf: &'a CF,
    ) -> TypedCfApi<'a, 'a, 'a, CF, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>> {
        self.db_context().cf(cf)
    }
}

/// A trait for a deque-like on top of RocksDb.
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

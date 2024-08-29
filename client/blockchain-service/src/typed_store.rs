use codec::{Decode, Encode};
use rocksdb::{AsColumnFamilyRef, ColumnFamily, DBPinnableSlice, WriteBatch, DB};
use std::{cell::RefCell, collections::HashMap};

pub trait DbCodec<T> {
    /// Encode a value to bytes.
    fn encode(&self, value: &T) -> Vec<u8>;

    /// Decode a value from bytes.
    fn decode(&self, bytes: &[u8]) -> T;
}

/// A DbCodec for the SCALE codec.
#[derive(Default, Clone)]
pub struct ScaleDbCodec;

impl<T> DbCodec<T> for ScaleDbCodec
where
    T: codec::Encode + codec::Decode,
{
    fn encode(&self, value: &T) -> Vec<u8> {
        value.encode()
    }

    fn decode(&self, bytes: &[u8]) -> T {
        T::decode(&mut &bytes[..]).expect("ScaleDbCodec: Failed to decode value")
    }
}

/// A typed RocksDB column family.
pub trait TypedCf {
    type Key;
    type Value;

    /// Type of the [`DbCodec`] for the keys.
    type KeyCodec: DbCodec<Self::Key> + Clone;

    /// Type of the [`DbCodec`] for the values.
    type ValueCodec: DbCodec<Self::Value> + Clone;

    /// Column family name (as known to the DB).
    const NAME: &'static str;
    /// Creates a new [`DbCodec`] for keys within this column family.
    fn key_codec(&self) -> Self::KeyCodec;
    /// Creates a new [`DbCodec`] for values within this column family.
    fn value_codec(&self) -> Self::ValueCodec;
}

/// A DbCodec for the unit type, used for single row column families.
#[derive(Debug, Clone, Default)]
pub struct SingleRowDbCodec;

impl DbCodec<()> for SingleRowDbCodec {
    fn encode(&self, _value: &()) -> Vec<u8> {
        vec![]
    }

    fn decode(&self, _bytes: &[u8]) -> () {
        ()
    }
}

/// A convenience trait implementing [`TypedCf`] for a simple case where both [`DbCodec`]s have
/// cheap [`Default`] implementations.
pub trait DefaultCf {
    /// Type of the key.
    type Key;
    /// Type of the value.
    type Value;

    /// Column family name (as known to the DB).
    const DEFAULT_NAME: &'static str;
    /// Key codec type.
    type KeyCodec: Default;
    /// Value codec type.
    type ValueCodec: Default;
}

impl<
        K,
        V,
        KC: Default + DbCodec<K> + Clone,
        VC: Default + DbCodec<V> + Clone,
        D: DefaultCf<Key = K, Value = V, KeyCodec = KC, ValueCodec = VC>,
    > TypedCf for D
{
    type Key = K;
    type Value = V;

    type KeyCodec = KC;
    type ValueCodec = VC;

    const NAME: &'static str = Self::DEFAULT_NAME;

    fn key_codec(&self) -> KC {
        KC::default()
    }

    fn value_codec(&self) -> VC {
        VC::default()
    }
}

/// A convenience trait implementing [`DefaultCf`] for a simple case where both [`DefaultCf::Key`]
/// and [`DefaultCf::Key`] implement scale encoding and decoding.
pub trait ScaleEncodedCf {
    type Key: Encode + Decode;
    type Value: Encode + Decode;

    const SCALE_ENCODED_NAME: &'static str;
}

impl<K, V, S: ScaleEncodedCf<Key = K, Value = V>> DefaultCf for S {
    type Key = K;
    type Value = V;

    type KeyCodec = ScaleDbCodec;
    type ValueCodec = ScaleDbCodec;

    const DEFAULT_NAME: &'static str = Self::SCALE_ENCODED_NAME;
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
}

/// A write-supporting interface of a RocksDB database.
pub trait WriteableRocks: ReadableRocks {
    /// Atomically writes the given batch of updates.
    fn write(&self, batch: WriteBatch);
}

/// An internal wrapper for a [`TypedCf`] and dependencies resolved from it.
struct ResolvedCf<'r, CF: TypedCf> {
    handle: &'r ColumnFamily,
    key_codec: CF::KeyCodec,
    value_codec: CF::ValueCodec,
}

impl<'r, CF: TypedCf> ResolvedCf<'r, CF> {
    /// Resolves and caches properties of the given [`TypedCf`].
    pub fn resolve<R: ReadableRocks>(rocks: &'r R, cf: &CF) -> Self {
        let handle = rocks.cf_handle(CF::NAME);
        let key_codec = cf.key_codec();
        let value_codec = cf.value_codec();
        Self {
            handle,
            key_codec,
            value_codec,
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

impl<'r, R: WriteableRocks> TypedDbContext<'r, R, BufferedWriteSupport<'r, R>> {
    /// Explicitly flushes the current contents of the write buffer and clears the associated
    /// overlay.
    pub fn flush(&self) {
        self.write_support.flush();
        self.overlay.key_value.borrow_mut().clear();
    }
}

/// A higher-level DB access API bound to its [`TypedDbContext`] and scoped at a specific column
/// family.
pub struct TypedCfApi<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> {
    cf: ResolvedCf<'r, CF>,
    rocks: &'r R,
    overlay: &'o DbOverlay,
    write_support: &'w W,
}

impl<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> TypedCfApi<'r, 'o, 'w, CF, R, W> {
    /// Creates an instance for the given column family.
    fn new(
        cf: ResolvedCf<'r, CF>,
        rocks: &'r R,
        overlay: &'o DbOverlay,
        write_support: &'w W,
    ) -> Self {
        Self {
            cf,
            rocks,
            overlay,
            write_support,
        }
    }
}

impl<'r, 'o, 'w, CF: TypedCf, R: ReadableRocks, W: WriteSupport> TypedCfApi<'r, 'o, 'w, CF, R, W> {
    /// Gets value by key.
    pub fn get(&self, key: &CF::Key) -> Option<CF::Value> {
        match self.overlay.get::<CF>(self.cf.key_codec.encode(key)) {
            Some(DbOverlayValueOp::Put(value)) => {
                return Some(self.cf.value_codec.decode(&value));
            }
            Some(DbOverlayValueOp::Delete) => {
                return None;
            }
            None => {}
        }

        self.rocks
            .get_pinned_cf(self.cf.handle, self.cf.key_codec.encode(key).as_slice())
            .map(|pinnable_slice| self.cf.value_codec.decode(pinnable_slice.as_ref()))
    }
}

impl<'r, 'o, 'w, CF: TypedCf, R: WriteableRocks>
    TypedCfApi<'r, 'o, 'w, CF, R, BufferedWriteSupport<'r, R>>
{
    /// Upserts the new value at the given key.
    pub fn put(&self, key: &CF::Key, value: &CF::Value) {
        self.overlay.put::<CF>(
            self.cf.key_codec.encode(key),
            self.cf.value_codec.encode(value),
        );
        self.write_support.buffer.put(
            self.cf.handle,
            self.cf.key_codec.encode(key),
            self.cf.value_codec.encode(value),
        );
    }

    /// Deletes the entry of the given key.
    pub fn delete(&self, key: &CF::Key) {
        self.overlay.delete::<CF>(self.cf.key_codec.encode(key));
        self.write_support
            .buffer
            .delete(self.cf.handle, self.cf.key_codec.encode(key));
    }
}

impl<'r, R: ReadableRocks, W: WriteSupport> TypedDbContext<'r, R, W> {
    /// Returns a typed helper scoped at the given column family.
    pub fn cf<CF: TypedCf>(&self, typed_cf: &CF) -> TypedCfApi<'r, '_, '_, CF, R, W> {
        TypedCfApi::new(
            ResolvedCf::resolve(self.rocks, typed_cf),
            self.rocks,
            &self.overlay,
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
}

impl WriteableRocks for TypedRocksDB {
    fn write(&self, batch: WriteBatch) {
        self.db.write(batch).expect("DB write batch");
    }
}

/// A key-value operation in the overlay.
#[derive(Debug, Clone)]
pub enum DbOverlayValueOp {
    Put(Vec<u8>),
    Delete,
}

/// A key in the overlay, composed of the column_family and the key.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DbOverlayKey {
    pub key: Vec<u8>,
    pub column_family: &'static str,
}

impl DbOverlayKey {
    pub fn new<CF: TypedCf>(key: Vec<u8>) -> Self {
        Self {
            key,
            column_family: CF::NAME,
        }
    }
}

/// An in memory overlay used by [`TypedCfApi`].
pub struct DbOverlay {
    pub key_value: RefCell<HashMap<DbOverlayKey, DbOverlayValueOp>>,
}

impl DbOverlay {
    pub fn new() -> Self {
        Self {
            key_value: RefCell::new(HashMap::new()),
        }
    }

    pub fn get<CF: TypedCf>(&self, key: Vec<u8>) -> Option<DbOverlayValueOp> {
        self.key_value
            .borrow()
            .get(&DbOverlayKey::new::<CF>(key))
            .cloned()
    }

    pub fn put<CF: TypedCf>(&self, key: Vec<u8>, value: Vec<u8>) {
        self.key_value
            .borrow_mut()
            .insert(DbOverlayKey::new::<CF>(key), DbOverlayValueOp::Put(value));
    }

    pub fn delete<CF: TypedCf>(&self, key: Vec<u8>) {
        self.key_value
            .borrow_mut()
            .insert(DbOverlayKey::new::<CF>(key), DbOverlayValueOp::Delete);
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
    fn access<'a, CF: SingleScaleEncodedValueCf>(
        &'a self,
        cf: &'a CF,
    ) -> SingleValueScopedAccess<'a, CF> {
        SingleValueScopedAccess::new(self.db_context(), cf)
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
        self.access(&Self::LeftIndexCF::default())
            .read()
            .unwrap_or(0)
    }

    fn right_index(&self) -> u64 {
        self.access(&Self::RightIndexCF::default())
            .read()
            .unwrap_or(0)
    }

    fn push_back(&mut self, value: Self::Value) {
        let right_index = self.right_index();
        self.db_context()
            .cf(&Self::DataCF::default())
            .put(&right_index, &value);
        self.access(&Self::RightIndexCF::default())
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
        self.access(&Self::LeftIndexCF::default())
            .write(&(left_index + 1));
        value
    }

    fn size(&self) -> u64 {
        self.right_index() - self.left_index()
    }
}

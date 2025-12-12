use log::info;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::PathBuf;

use shc_common::{
    traits::StorageEnableRuntime,
    typed_store::{
        BufferedWriteSupport, CFDequeAPI, ProvidesDbContext, ProvidesTypedDbAccess,
        ProvidesTypedDbSingleAccess, ScaleEncodedCf, SingleScaleEncodedValueCf, TypedDbContext,
        TypedRocksDB,
    },
    types::BlockNumber,
};

use crate::types::{
    ConfirmStoringRequest, FileDeletionRequest, StopStoringForInsolventUserRequest,
};

/// Last processed block number.
pub struct LastProcessedBlockNumberCf<Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
}
impl<Runtime: StorageEnableRuntime> SingleScaleEncodedValueCf
    for LastProcessedBlockNumberCf<Runtime>
{
    type Value = BlockNumber<Runtime>;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = LastProcessedBlockNumberName::NAME;
}

/// Non-generic name holder for the `LastProcessedBlockNumber` column family
pub struct LastProcessedBlockNumberName;
impl LastProcessedBlockNumberName {
    pub const NAME: &'static str = "last_processed_block_number";
}

/// Pending confirm storing requests.
pub struct PendingConfirmStoringRequestCf<Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
}
impl<Runtime: StorageEnableRuntime> ScaleEncodedCf for PendingConfirmStoringRequestCf<Runtime> {
    type Key = u64;
    type Value = ConfirmStoringRequest<Runtime>;

    const SCALE_ENCODED_NAME: &'static str = PendingConfirmStoringRequestName::NAME;
}

/// Non-generic name holder for the `PendingConfirmStoringRequest` column family
pub struct PendingConfirmStoringRequestName;
impl PendingConfirmStoringRequestName {
    pub const NAME: &'static str = "pending_confirm_storing_request";
}

impl<Runtime: StorageEnableRuntime> Default for PendingConfirmStoringRequestCf<Runtime> {
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

/// Pending stop storing requests.
pub struct PendingStopStoringForInsolventUserRequestCf<Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
}
impl<Runtime: StorageEnableRuntime> ScaleEncodedCf
    for PendingStopStoringForInsolventUserRequestCf<Runtime>
{
    type Key = u64;
    type Value = StopStoringForInsolventUserRequest<Runtime>;

    const SCALE_ENCODED_NAME: &'static str = PendingStopStoringForInsolventUserRequestName::NAME;
}

impl<Runtime: StorageEnableRuntime> Default
    for PendingStopStoringForInsolventUserRequestCf<Runtime>
{
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

/// Non-generic name holder for the `PendingStopStoringForInsolventUserRequest` column family
pub struct PendingStopStoringForInsolventUserRequestName;
impl PendingStopStoringForInsolventUserRequestName {
    pub const NAME: &'static str = "pending_stop_storing_for_insolvent_user_request";
}

/// Pending submit proof requests left side (inclusive) index for the [`PendingConfirmStoringRequestCf`] CF.
#[derive(Default)]
pub struct PendingConfirmStoringRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for PendingConfirmStoringRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        PendingConfirmStoringRequestLeftIndexName::NAME;
}

/// Non-generic name holder for the `PendingConfirmStoringRequestLeftIndex` column family
pub struct PendingConfirmStoringRequestLeftIndexName;
impl PendingConfirmStoringRequestLeftIndexName {
    pub const NAME: &'static str = "pending_confirm_storing_request_left_index";
}

/// Pending submit proof requests right side (exclusive) index for the [`PendingConfirmStoringRequestCf`] CF.
#[derive(Default)]
pub struct PendingConfirmStoringRequestRightIndexCf;
impl SingleScaleEncodedValueCf for PendingConfirmStoringRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        PendingConfirmStoringRequestRightIndexName::NAME;
}

/// Non-generic name holder for the `PendingConfirmStoringRequestRightIndex` column family
pub struct PendingConfirmStoringRequestRightIndexName;
impl PendingConfirmStoringRequestRightIndexName {
    pub const NAME: &'static str = "pending_confirm_storing_request_right_index";
}

/// Pending submit proof requests left side (inclusive) index for the [`PendingStopStoringForInsolventUserRequestCf`] CF.
#[derive(Default)]
pub struct PendingStopStoringForInsolventUserRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for PendingStopStoringForInsolventUserRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        PendingStopStoringForInsolventUserRequestLeftIndexName::NAME;
}

/// Non-generic name holder for the `PendingStopStoringForInsolventUserRequestLeftIndex` column family
pub struct PendingStopStoringForInsolventUserRequestLeftIndexName;
impl PendingStopStoringForInsolventUserRequestLeftIndexName {
    pub const NAME: &'static str = "pending_stop_storing_for_insolvent_user_request_left_index";
}

/// Pending submit proof requests right side (exclusive) index for the [`PendingStopStoringForInsolventUserRequestCf`] CF.
#[derive(Default)]
pub struct PendingStopStoringForInsolventUserRequestRightIndexCf;
impl SingleScaleEncodedValueCf for PendingStopStoringForInsolventUserRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        PendingStopStoringForInsolventUserRequestRightIndexName::NAME;
}

/// Non-generic name holder for the `PendingStopStoringForInsolventUserRequestRightIndex` column family
pub struct PendingStopStoringForInsolventUserRequestRightIndexName;
impl PendingStopStoringForInsolventUserRequestRightIndexName {
    pub const NAME: &'static str = "pending_stop_storing_for_insolvent_user_request_right_index";
}

/// Pending file deletion requests.
pub struct FileDeletionRequestCf<Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
}
impl<Runtime: StorageEnableRuntime> ScaleEncodedCf for FileDeletionRequestCf<Runtime> {
    type Key = u64;
    type Value = FileDeletionRequest<Runtime>;

    const SCALE_ENCODED_NAME: &'static str = FileDeletionRequestName::NAME;
}

/// Non-generic name holder for the `FileDeletionRequest` column family
pub struct FileDeletionRequestName;
impl FileDeletionRequestName {
    pub const NAME: &'static str = "pending_file_deletion_request";
}

impl<Runtime: StorageEnableRuntime> Default for FileDeletionRequestCf<Runtime> {
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

/// Pending file deletion requests left side (inclusive) index for the [`PendingFileDeletionRequestCf`] CF.
#[derive(Default)]
pub struct FileDeletionRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for FileDeletionRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = FileDeletionRequestLeftIndexName::NAME;
}

/// Non-generic name holder for the `FileDeletionRequestLeftIndex` column family
pub struct FileDeletionRequestLeftIndexName;
impl FileDeletionRequestLeftIndexName {
    pub const NAME: &'static str = "pending_file_deletion_request_left_index";
}

/// Pending file deletion requests right side (exclusive) index for the [`PendingFileDeletionRequestCf`] CF.
#[derive(Default)]
pub struct FileDeletionRequestRightIndexCf;
impl SingleScaleEncodedValueCf for FileDeletionRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = FileDeletionRequestRightIndexName::NAME;
}

/// Non-generic name holder for the `FileDeletionRequestRightIndex` column family
pub struct FileDeletionRequestRightIndexName;
impl FileDeletionRequestRightIndexName {
    pub const NAME: &'static str = "pending_file_deletion_request_right_index";
}

// Deprecated column families - kept for backward compatibility with existing RocksDB databases.
// The functionality has been replaced with in-memory queueing in `MspHandler`.
#[allow(deprecated, dead_code)]
mod deprecated_cfs {
    use super::*;

    /// Pending respond storage requests.
    ///
    /// # Deprecated
    /// This column family is deprecated and no longer used.
    /// It is kept for backward compatibility with existing RocksDB databases.
    /// The functionality has been replaced with in-memory queueing in `MspHandler`.
    #[deprecated(note = "Replaced with in-memory queueing. Kept for backward compatibility.")]
    pub struct PendingMspRespondStorageRequestCf<Runtime: StorageEnableRuntime> {
        pub(crate) phantom: std::marker::PhantomData<Runtime>,
    }
    impl<Runtime: StorageEnableRuntime> ScaleEncodedCf for PendingMspRespondStorageRequestCf<Runtime> {
        type Key = u64;
        type Value = crate::types::RespondStorageRequest<Runtime>;

        const SCALE_ENCODED_NAME: &'static str = PendingMspRespondStorageRequestName::NAME;
    }

    impl<Runtime: StorageEnableRuntime> Default for PendingMspRespondStorageRequestCf<Runtime> {
        fn default() -> Self {
            Self {
                phantom: std::marker::PhantomData,
            }
        }
    }

    /// Non-generic name holder for the `PendingMspRespondStorageRequest` column family
    #[deprecated(note = "Replaced with in-memory queueing. Kept for backward compatibility.")]
    pub struct PendingMspRespondStorageRequestName;
    impl PendingMspRespondStorageRequestName {
        pub const NAME: &'static str = "pending_msp_respond_storage_request";
    }

    /// Pending respond storage requests left side (inclusive) index for the [`PendingMspRespondStorageRequestCf`] CF.
    ///
    /// # Deprecated
    /// This column family is deprecated and no longer used.
    /// It is kept for backward compatibility with existing RocksDB databases.
    #[derive(Default)]
    #[deprecated(note = "Replaced with in-memory queueing. Kept for backward compatibility.")]
    pub struct PendingMspRespondStorageRequestLeftIndexCf;
    impl SingleScaleEncodedValueCf for PendingMspRespondStorageRequestLeftIndexCf {
        type Value = u64;

        const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
            PendingMspRespondStorageRequestLeftIndexName::NAME;
    }

    /// Non-generic name holder for the `PendingMspRespondStorageRequestLeftIndex` column family
    #[deprecated(note = "Replaced with in-memory queueing. Kept for backward compatibility.")]
    pub struct PendingMspRespondStorageRequestLeftIndexName;
    impl PendingMspRespondStorageRequestLeftIndexName {
        pub const NAME: &'static str = "pending_msp_respond_storage_request_left_index";
    }

    /// Pending respond storage requests right side (exclusive) index for the [`PendingMspRespondStorageRequestCf`] CF.
    ///
    /// # Deprecated
    /// This column family is deprecated and no longer used.
    /// It is kept for backward compatibility with existing RocksDB databases.
    #[derive(Default)]
    #[deprecated(note = "Replaced with in-memory queueing. Kept for backward compatibility.")]
    pub struct PendingMspRespondStorageRequestRightIndexCf;
    impl SingleScaleEncodedValueCf for PendingMspRespondStorageRequestRightIndexCf {
        type Value = u64;

        const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
            PendingMspRespondStorageRequestRightIndexName::NAME;
    }

    /// Non-generic name holder for the `PendingMspRespondStorageRequestRightIndex` column family
    #[deprecated(note = "Replaced with in-memory queueing. Kept for backward compatibility.")]
    pub struct PendingMspRespondStorageRequestRightIndexName;
    impl PendingMspRespondStorageRequestRightIndexName {
        pub const NAME: &'static str = "pending_msp_respond_storage_request_right_index";
    }
}

use deprecated_cfs::*;

// Deprecated column families are included for backward compatibility with existing RocksDB databases
#[allow(deprecated)]
const ALL_COLUMN_FAMILIES: [&str; 13] = [
    LastProcessedBlockNumberName::NAME,
    PendingConfirmStoringRequestLeftIndexName::NAME,
    PendingConfirmStoringRequestRightIndexName::NAME,
    PendingConfirmStoringRequestName::NAME,
    PendingStopStoringForInsolventUserRequestLeftIndexName::NAME,
    PendingStopStoringForInsolventUserRequestRightIndexName::NAME,
    PendingStopStoringForInsolventUserRequestName::NAME,
    FileDeletionRequestLeftIndexName::NAME,
    FileDeletionRequestRightIndexName::NAME,
    FileDeletionRequestName::NAME,
    // Deprecated column families - kept for backward compatibility with existing RocksDB databases
    PendingMspRespondStorageRequestLeftIndexName::NAME,
    PendingMspRespondStorageRequestRightIndexName::NAME,
    PendingMspRespondStorageRequestName::NAME,
];

/// A persistent blockchain service state store.
pub struct BlockchainServiceStateStore {
    /// The RocksDB database.
    rocks: TypedRocksDB,
}

impl BlockchainServiceStateStore {
    pub fn new(root_path: PathBuf) -> Self {
        let mut path = root_path;
        path.push("storagehub/blockchain_service/");

        let db_path_str = path.to_str().expect("Failed to convert path to string");
        info!("Blockchain service state store path: {}", db_path_str);
        std::fs::create_dir_all(&db_path_str).expect("Failed to create directory");

        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        let column_families: Vec<ColumnFamilyDescriptor> = ALL_COLUMN_FAMILIES
            .iter()
            .map(|cf| ColumnFamilyDescriptor::new(cf.to_string(), Options::default()))
            .collect();

        let db = DB::open_cf_descriptors(&db_opts, db_path_str, column_families).unwrap();

        BlockchainServiceStateStore {
            rocks: TypedRocksDB { db },
        }
    }

    /// Starts a read/buffered-write interaction with the DB through per-CF type-safe APIs.
    pub fn open_rw_context_with_overlay(&self) -> BlockchainServiceStateStoreRwContext<'_> {
        BlockchainServiceStateStoreRwContext::new(TypedDbContext::new(
            &self.rocks,
            BufferedWriteSupport::new(&self.rocks),
        ))
    }
}

pub struct BlockchainServiceStateStoreRwContext<'a> {
    /// The RocksDB database.
    db_context: TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> BlockchainServiceStateStoreRwContext<'a> {
    pub fn new(
        db_context: TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
    ) -> Self {
        BlockchainServiceStateStoreRwContext { db_context }
    }

    pub fn pending_confirm_storing_request_deque<Runtime: StorageEnableRuntime>(
        &'a self,
    ) -> PendingConfirmStoringRequestDequeAPI<'a, Runtime> {
        PendingConfirmStoringRequestDequeAPI {
            phantom: std::marker::PhantomData,
            db_context: &self.db_context,
        }
    }

    pub fn pending_stop_storing_for_insolvent_user_request_deque<Runtime: StorageEnableRuntime>(
        &'a self,
    ) -> PendingStopStoringForInsolventUserRequestDequeAPI<'a, Runtime> {
        PendingStopStoringForInsolventUserRequestDequeAPI {
            phantom: std::marker::PhantomData,
            db_context: &self.db_context,
        }
    }

    pub fn pending_file_deletion_request_deque<Runtime: StorageEnableRuntime>(
        &'a self,
    ) -> PendingFileDeletionRequestDequeAPI<'a, Runtime> {
        PendingFileDeletionRequestDequeAPI {
            phantom: std::marker::PhantomData,
            db_context: &self.db_context,
        }
    }

    /// Flushes the buffered writes to the DB.
    pub fn commit(self) {
        self.db_context.flush();
    }
}

impl<'a> ProvidesDbContext for BlockchainServiceStateStoreRwContext<'a> {
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbSingleAccess for BlockchainServiceStateStoreRwContext<'a> {}

impl<'a> ProvidesTypedDbAccess for BlockchainServiceStateStoreRwContext<'a> {}

pub struct PendingConfirmStoringRequestDequeAPI<'a, Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
    pub(crate) db_context:
        &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesDbContext
    for PendingConfirmStoringRequestDequeAPI<'a, Runtime>
{
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesTypedDbSingleAccess
    for PendingConfirmStoringRequestDequeAPI<'a, Runtime>
{
}

impl<'a, Runtime: StorageEnableRuntime> CFDequeAPI
    for PendingConfirmStoringRequestDequeAPI<'a, Runtime>
{
    type Value = ConfirmStoringRequest<Runtime>;
    type LeftIndexCF = PendingConfirmStoringRequestLeftIndexCf;
    type RightIndexCF = PendingConfirmStoringRequestRightIndexCf;
    type DataCF = PendingConfirmStoringRequestCf<Runtime>;
}

pub struct PendingStopStoringForInsolventUserRequestDequeAPI<'a, Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesDbContext
    for PendingStopStoringForInsolventUserRequestDequeAPI<'a, Runtime>
{
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesTypedDbSingleAccess
    for PendingStopStoringForInsolventUserRequestDequeAPI<'a, Runtime>
{
}

impl<'a, Runtime: StorageEnableRuntime> CFDequeAPI
    for PendingStopStoringForInsolventUserRequestDequeAPI<'a, Runtime>
{
    type Value = StopStoringForInsolventUserRequest<Runtime>;
    type LeftIndexCF = PendingStopStoringForInsolventUserRequestLeftIndexCf;
    type RightIndexCF = PendingStopStoringForInsolventUserRequestRightIndexCf;
    type DataCF = PendingStopStoringForInsolventUserRequestCf<Runtime>;
}

pub struct PendingFileDeletionRequestDequeAPI<'a, Runtime: StorageEnableRuntime> {
    pub(crate) phantom: std::marker::PhantomData<Runtime>,
    pub(crate) db_context:
        &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesDbContext
    for PendingFileDeletionRequestDequeAPI<'a, Runtime>
{
    fn db_context(
        &self,
    ) -> &TypedDbContext<'_, TypedRocksDB, BufferedWriteSupport<'_, TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a, Runtime: StorageEnableRuntime> ProvidesTypedDbSingleAccess
    for PendingFileDeletionRequestDequeAPI<'a, Runtime>
{
}

impl<'a, Runtime: StorageEnableRuntime> CFDequeAPI
    for PendingFileDeletionRequestDequeAPI<'a, Runtime>
{
    type Value = FileDeletionRequest<Runtime>;
    type LeftIndexCF = FileDeletionRequestLeftIndexCf;
    type RightIndexCF = FileDeletionRequestRightIndexCf;
    type DataCF = FileDeletionRequestCf<Runtime>;
}

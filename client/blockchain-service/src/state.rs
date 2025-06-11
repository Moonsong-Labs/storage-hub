use log::info;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::PathBuf;

use shc_common::traits::StorageEnableRuntimeConfig;
use shc_common::{
    typed_store::{
        BufferedWriteSupport, CFDequeAPI, ProvidesDbContext, ProvidesTypedDbAccess,
        ProvidesTypedDbSingleAccess, ScaleEncodedCf, SingleScaleEncodedValueCf, TypedCf,
        TypedDbContext, TypedRocksDB,
    },
    types::BlockNumber,
};

use crate::{
    events::{
        ProcessConfirmStoringRequestData, ProcessFileDeletionRequestData,
        ProcessMspRespondStoringRequestData, ProcessStopStoringForInsolventUserRequestData,
    },
    types::{
        ConfirmStoringRequest, FileDeletionRequest, RespondStorageRequest,
        StopStoringForInsolventUserRequest,
    },
};

/// Last processed block number.
pub struct LastProcessedBlockNumberCf;
impl<Runtime: StorageEnableRuntimeConfig> SingleScaleEncodedValueCf for LastProcessedBlockNumberCf {
    type Value = BlockNumber<Runtime>;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "last_processed_block_number";
}

/// Current ongoing task which requires a forest write lock.
pub struct OngoingProcessConfirmStoringRequestCf;
impl SingleScaleEncodedValueCf for OngoingProcessConfirmStoringRequestCf {
    type Value = ProcessConfirmStoringRequestData;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "ongoing_process_confirm_storing_request";
}

/// Current ongoing task which requires a forest write lock.
pub struct OngoingProcessStopStoringForInsolventUserRequestCf;
impl SingleScaleEncodedValueCf for OngoingProcessStopStoringForInsolventUserRequestCf {
    type Value = ProcessStopStoringForInsolventUserRequestData;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "ongoing_process_stop_storing_for_insolvent_user_request";
}

/// Pending confirm storing requests.
#[derive(Default)]
pub struct PendingConfirmStoringRequestCf;
impl ScaleEncodedCf for PendingConfirmStoringRequestCf {
    type Key = u64;
    type Value = ConfirmStoringRequest;

    const SCALE_ENCODED_NAME: &'static str = "pending_confirm_storing_request";
}

/// Pending stop storing requests.
#[derive(Default)]
pub struct PendingStopStoringForInsolventUserRequestCf;
impl ScaleEncodedCf for PendingStopStoringForInsolventUserRequestCf {
    type Key = u64;
    type Value = StopStoringForInsolventUserRequest;

    const SCALE_ENCODED_NAME: &'static str = "pending_stop_storing_for_insolvent_user_request";
}

/// Pending submit proof requests left side (inclusive) index for the [`PendingConfirmStoringRequestCf`] CF.
#[derive(Default)]
pub struct PendingConfirmStoringRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for PendingConfirmStoringRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_confirm_storing_request_left_index";
}

/// Pending submit proof requests right side (exclusive) index for the [`PendingConfirmStoringRequestCf`] CF.
#[derive(Default)]
pub struct PendingConfirmStoringRequestRightIndexCf;
impl SingleScaleEncodedValueCf for PendingConfirmStoringRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_confirm_storing_request_right_index";
}

pub struct OngoingProcessMspRespondStorageRequestCf;
impl SingleScaleEncodedValueCf for OngoingProcessMspRespondStorageRequestCf {
    type Value = ProcessMspRespondStoringRequestData;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "ongoing_process_msp_respond_storage_request";
}

/// Pending respond storage requests.
#[derive(Default)]
pub struct PendingMspRespondStorageRequestCf;
impl ScaleEncodedCf for PendingMspRespondStorageRequestCf {
    type Key = u64;
    type Value = RespondStorageRequest;

    const SCALE_ENCODED_NAME: &'static str = "pending_msp_respond_storage_request";
}

/// Pending respond storage requests left side (inclusive) index for the [`PendingMspRespondStorageRequestCf`] CF.
#[derive(Default)]
pub struct PendingMspRespondStorageRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for PendingMspRespondStorageRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_msp_respond_storage_request_left_index";
}

/// Pending respond storage requests right side (exclusive) index for the [`PendingMspRespondStorageRequestCf`] CF.
#[derive(Default)]
pub struct PendingMspRespondStorageRequestRightIndexCf;
impl SingleScaleEncodedValueCf for PendingMspRespondStorageRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_msp_respond_storage_request_right_index";
}

/// Pending submit proof requests left side (inclusive) index for the [`PendingStopStoringForInsolventUserRequestCf`] CF.
#[derive(Default)]
pub struct PendingStopStoringForInsolventUserRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for PendingStopStoringForInsolventUserRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_stop_storing_for_insolvent_user_request_left_index";
}

/// Pending submit proof requests right side (exclusive) index for the [`PendingStopStoringForInsolventUserRequestCf`] CF.
#[derive(Default)]
pub struct PendingStopStoringForInsolventUserRequestRightIndexCf;
impl SingleScaleEncodedValueCf for PendingStopStoringForInsolventUserRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_stop_storing_for_insolvent_user_request_right_index";
}

/// Current ongoing task which requires a forest write lock.
pub struct OngoingProcessFileDeletionRequestCf;
impl SingleScaleEncodedValueCf for OngoingProcessFileDeletionRequestCf {
    type Value = ProcessFileDeletionRequestData;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "ongoing_process_file_deletion_request";
}

/// Pending file deletion requests.
#[derive(Default)]
pub struct FileDeletionRequestCf<Runtime> {
    _phantom: std::marker::PhantomData<Runtime>,
}
impl<Runtime: StorageEnableRuntimeConfig> ScaleEncodedCf for FileDeletionRequestCf<Runtime> {
    type Key = u64;
    type Value = FileDeletionRequest<Runtime>;

    const SCALE_ENCODED_NAME: &'static str = "pending_file_deletion_request";
}

/// Pending file deletion requests left side (inclusive) index for the [`PendingFileDeletionRequestCf`] CF.
#[derive(Default)]
pub struct FileDeletionRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for FileDeletionRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_file_deletion_request_left_index";
}

/// Pending file deletion requests right side (exclusive) index for the [`PendingFileDeletionRequestCf`] CF.
#[derive(Default)]
pub struct FileDeletionRequestRightIndexCf;
impl SingleScaleEncodedValueCf for FileDeletionRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_file_deletion_request_right_index";
}

const ALL_COLUMN_FAMILIES: [&str; 17] = [
    LastProcessedBlockNumberCf::NAME,
    OngoingProcessConfirmStoringRequestCf::NAME,
    PendingConfirmStoringRequestLeftIndexCf::NAME,
    PendingConfirmStoringRequestRightIndexCf::NAME,
    PendingConfirmStoringRequestCf::NAME,
    OngoingProcessMspRespondStorageRequestCf::NAME,
    PendingMspRespondStorageRequestLeftIndexCf::NAME,
    PendingMspRespondStorageRequestRightIndexCf::NAME,
    PendingMspRespondStorageRequestCf::NAME,
    OngoingProcessStopStoringForInsolventUserRequestCf::NAME,
    PendingStopStoringForInsolventUserRequestLeftIndexCf::NAME,
    PendingStopStoringForInsolventUserRequestRightIndexCf::NAME,
    PendingStopStoringForInsolventUserRequestCf::NAME,
    OngoingProcessFileDeletionRequestCf::NAME,
    FileDeletionRequestLeftIndexCf::NAME,
    FileDeletionRequestRightIndexCf::NAME,
    FileDeletionRequestCf::NAME,
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

    pub fn pending_confirm_storing_request_deque(
        &'a self,
    ) -> PendingConfirmStoringRequestDequeAPI<'a> {
        PendingConfirmStoringRequestDequeAPI {
            db_context: &self.db_context,
        }
    }

    pub fn pending_msp_respond_storage_request_deque(
        &'a self,
    ) -> PendingMspRespondStorageRequestDequeAPI<'a> {
        PendingMspRespondStorageRequestDequeAPI {
            db_context: &self.db_context,
        }
    }

    pub fn pending_stop_storing_for_insolvent_user_request_deque(
        &'a self,
    ) -> PendingStopStoringForInsolventUserRequestDequeAPI<'a> {
        PendingStopStoringForInsolventUserRequestDequeAPI {
            db_context: &self.db_context,
        }
    }

    pub fn pending_file_deletion_request_deque(&'a self) -> PendingFileDeletionRequestDequeAPI<'a> {
        PendingFileDeletionRequestDequeAPI {
            db_context: &self.db_context,
        }
    }

    /// Flushes the buffered writes to the DB.
    pub fn commit(self) {
        self.db_context.flush();
    }
}

impl<'a> ProvidesDbContext for BlockchainServiceStateStoreRwContext<'a> {
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbSingleAccess for BlockchainServiceStateStoreRwContext<'a> {}

impl<'a> ProvidesTypedDbAccess for BlockchainServiceStateStoreRwContext<'a> {}

pub struct PendingConfirmStoringRequestDequeAPI<'a> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> ProvidesDbContext for PendingConfirmStoringRequestDequeAPI<'a> {
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbSingleAccess for PendingConfirmStoringRequestDequeAPI<'a> {}

impl<'a> CFDequeAPI for PendingConfirmStoringRequestDequeAPI<'a> {
    type Value = ConfirmStoringRequest;
    type LeftIndexCF = PendingConfirmStoringRequestLeftIndexCf;
    type RightIndexCF = PendingConfirmStoringRequestRightIndexCf;
    type DataCF = PendingConfirmStoringRequestCf;
}

pub struct PendingMspRespondStorageRequestDequeAPI<'a> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> ProvidesDbContext for PendingMspRespondStorageRequestDequeAPI<'a> {
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbSingleAccess for PendingMspRespondStorageRequestDequeAPI<'a> {}

impl<'a> CFDequeAPI for PendingMspRespondStorageRequestDequeAPI<'a> {
    type Value = RespondStorageRequest;
    type LeftIndexCF = PendingMspRespondStorageRequestLeftIndexCf;
    type RightIndexCF = PendingMspRespondStorageRequestRightIndexCf;
    type DataCF = PendingMspRespondStorageRequestCf;
}

pub struct PendingStopStoringForInsolventUserRequestDequeAPI<'a> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> ProvidesDbContext for PendingStopStoringForInsolventUserRequestDequeAPI<'a> {
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbSingleAccess for PendingStopStoringForInsolventUserRequestDequeAPI<'a> {}

impl<'a> CFDequeAPI for PendingStopStoringForInsolventUserRequestDequeAPI<'a> {
    type Value = StopStoringForInsolventUserRequest;
    type LeftIndexCF = PendingStopStoringForInsolventUserRequestLeftIndexCf;
    type RightIndexCF = PendingStopStoringForInsolventUserRequestRightIndexCf;
    type DataCF = PendingStopStoringForInsolventUserRequestCf;
}

pub struct PendingFileDeletionRequestDequeAPI<'a, Runtime: StorageEnableRuntimeConfig> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
    _phantom: std::marker::PhantomData<Runtime>,
}

impl<'a, Runtime: StorageEnableRuntimeConfig> ProvidesDbContext
    for PendingFileDeletionRequestDequeAPI<'a, Runtime>
{
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a, Runtime: StorageEnableRuntimeConfig> ProvidesTypedDbSingleAccess
    for PendingFileDeletionRequestDequeAPI<'a, Runtime>
{
}

impl<'a, Runtime: StorageEnableRuntimeConfig> CFDequeAPI
    for PendingFileDeletionRequestDequeAPI<'a, Runtime>
{
    type Value = FileDeletionRequest<Runtime>;
    type LeftIndexCF = FileDeletionRequestLeftIndexCf;
    type RightIndexCF = FileDeletionRequestRightIndexCf;
    type DataCF = FileDeletionRequestCf<Runtime>;
}

use std::path::PathBuf;

use log::info;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use shc_common::types::BlockNumber;

use crate::events::ProcessMspRespondStoringRequestData;
use crate::{
    events::ProcessConfirmStoringRequestData,
    typed_store::{
        BufferedWriteSupport, CFDequeAPI, ProvidesDbContext, ProvidesTypedDbAccess,
        ProvidesTypedDbSingleAccess, ScaleEncodedCf, SingleScaleEncodedValueCf, TypedCf,
        TypedDbContext, TypedRocksDB,
    },
    types::{ConfirmStoringRequest, RespondStorageRequest},
};

/// Last processed block number.
pub struct LastProcessedBlockNumberCf;
impl SingleScaleEncodedValueCf for LastProcessedBlockNumberCf {
    type Value = BlockNumber;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "last_processed_block_number";
}

/// Current ongoing task which requires a forest write lock.
pub struct OngoingProcessConfirmStoringRequestCf;
impl SingleScaleEncodedValueCf for OngoingProcessConfirmStoringRequestCf {
    type Value = ProcessConfirmStoringRequestData;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "ongoing_process_confirm_storing_request";
}

/// Pending confirm storing requests.
#[derive(Default)]
pub struct PendingConfirmStoringRequestCf;
impl ScaleEncodedCf for PendingConfirmStoringRequestCf {
    type Key = u64;
    type Value = ConfirmStoringRequest;

    const SCALE_ENCODED_NAME: &'static str = "pending_confirm_storing_request";
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

const ALL_COLUMN_FAMILIES: [&str; 9] = [
    LastProcessedBlockNumberCf::NAME,
    OngoingProcessConfirmStoringRequestCf::NAME,
    PendingConfirmStoringRequestLeftIndexCf::NAME,
    PendingConfirmStoringRequestRightIndexCf::NAME,
    PendingConfirmStoringRequestCf::NAME,
    OngoingProcessMspRespondStorageRequestCf::NAME,
    PendingMspRespondStorageRequestLeftIndexCf::NAME,
    PendingMspRespondStorageRequestRightIndexCf::NAME,
    PendingMspRespondStorageRequestCf::NAME,
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

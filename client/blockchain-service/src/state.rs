use std::path::PathBuf;

use log::info;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use shc_common::types::BlockNumber;

use crate::{
    events::OngoingForestWriteLockTaskData,
    handler::{ConfirmStoringRequest, SubmitProofRequest},
    typed_store::{
        BufferedWriteSupport, CFDequeAPI, ProvidesDbContext, ProvidesTypedDbSingleAccess,
        ScaleEncodedCf, SingleScaleEncodedValueCf, TypedCf, TypedDbContext, TypedRocksDB,
    },
};

/// Last processed block number.
struct LastProcessedBlockNumberCf;
impl SingleScaleEncodedValueCf for LastProcessedBlockNumberCf {
    type Value = BlockNumber;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "last_processed_block_number";
}

/// Current ongoing task which requires a forest write lock.
struct OngoingForestWriteLockTaskDataCf;
impl SingleScaleEncodedValueCf for OngoingForestWriteLockTaskDataCf {
    type Value = OngoingForestWriteLockTaskData;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "ongoing_forest_write_lock_task_data";
}

/// Pending submit proof requests.
#[derive(Default)]
pub struct PendingSubmitProofRequestCf;
impl ScaleEncodedCf for PendingSubmitProofRequestCf {
    type Key = u64;
    type Value = SubmitProofRequest;

    const SCALE_ENCODED_NAME: &'static str = "pending_submit_proof_request";
}

/// Pending submit proof requests left side (inclusive) index for the [`PendingSubmitProofRequestLeftIndexCf`] CF.
#[derive(Default)]
pub struct PendingSubmitProofRequestLeftIndexCf;
impl SingleScaleEncodedValueCf for PendingSubmitProofRequestLeftIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str = "pending_submit_proof_request_left_index";
}

/// Pending submit proof requests right side (exclusive) index for the [`PendingSubmitProofRequestLeftIndexCf`] CF.
#[derive(Default)]
pub struct PendingSubmitProofRequestRightIndexCf;
impl SingleScaleEncodedValueCf for PendingSubmitProofRequestRightIndexCf {
    type Value = u64;

    const SINGLE_SCALE_ENCODED_VALUE_NAME: &'static str =
        "pending_submit_proof_request_right_index";
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

const ALL_COLUMN_FAMILIES: [&str; 7] = [
    OngoingForestWriteLockTaskDataCf::NAME,
    PendingSubmitProofRequestLeftIndexCf::NAME,
    PendingSubmitProofRequestRightIndexCf::NAME,
    PendingSubmitProofRequestCf::NAME,
    PendingConfirmStoringRequestLeftIndexCf::NAME,
    PendingConfirmStoringRequestRightIndexCf::NAME,
    PendingConfirmStoringRequestCf::NAME,
];

/// A persistent blockchain service state store.
pub struct BlockchainServiceStateStore {
    /// The RocksDB database.
    rocks: TypedRocksDB,
}

impl BlockchainServiceStateStore {
    pub fn new(root_path: PathBuf) -> Self {
        let mut path = root_path;
        path.push("storagehub/blockchain_service");

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

    pub fn pending_submit_proof_request_deque(&'a self) -> PendingSubmitProofRequestDequeAPI<'a> {
        PendingSubmitProofRequestDequeAPI {
            db_context: &self.db_context,
        }
    }

    pub fn pending_confirm_storing_request_deque(
        &'a self,
    ) -> PendingConfirmStoringRequestDequeAPI<'a> {
        PendingConfirmStoringRequestDequeAPI {
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

pub struct PendingSubmitProofRequestDequeAPI<'a> {
    db_context: &'a TypedDbContext<'a, TypedRocksDB, BufferedWriteSupport<'a, TypedRocksDB>>,
}

impl<'a> ProvidesDbContext for PendingSubmitProofRequestDequeAPI<'a> {
    fn db_context(&self) -> &TypedDbContext<TypedRocksDB, BufferedWriteSupport<TypedRocksDB>> {
        &self.db_context
    }
}

impl<'a> ProvidesTypedDbSingleAccess for PendingSubmitProofRequestDequeAPI<'a> {}

impl<'a> CFDequeAPI for PendingSubmitProofRequestDequeAPI<'a> {
    type Value = SubmitProofRequest;
    type LeftIndexCF = PendingSubmitProofRequestLeftIndexCf;
    type RightIndexCF = PendingSubmitProofRequestRightIndexCf;
    type DataCF = PendingSubmitProofRequestCf;
}

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
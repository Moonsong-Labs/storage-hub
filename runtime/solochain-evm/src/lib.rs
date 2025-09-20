#![cfg_attr(not(feature = "std"), no_std)]
// `frame_support::runtime` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

pub mod configs;
mod genesis_config_presets;
mod weights;

extern crate alloc;

use codec::Encode;
use cumulus_primitives_core::BlockT;
use fp_account::EthereumSignature;
use frame_support::{
    genesis_builder_helper::{build_state, get_preset},
    traits::{Hooks, KeyOwnerProofSystem},
    weights::{
        constants::WEIGHT_REF_TIME_PER_SECOND, Weight, WeightToFeeCoefficient,
        WeightToFeeCoefficients, WeightToFeePolynomial,
    },
};
use pallet_evm::{FeeCalculator, GasWeightMapping, Runner};
use pallet_file_system::types::StorageRequestMetadata;
use pallet_file_system_runtime_api::{
    GenericApplyDeltaEventInfoError, IsStorageRequestOpenToVolunteersError,
    QueryBspConfirmChunksToProveForFileError, QueryFileEarliestVolunteerTickError,
    QueryIncompleteStorageRequestMetadataError, QueryMspConfirmChunksToProveForFileError,
};
use pallet_payment_streams_runtime_api::GetUsersWithDebtOverThresholdError;
use pallet_proofs_dealer::types::{
    CustomChallenge, KeyFor, ProviderIdFor as ProofsDealerProviderIdFor, RandomnessOutputFor,
};
use pallet_proofs_dealer_runtime_api::{
    GetChallengePeriodError, GetChallengeSeedError, GetCheckpointChallengesError,
    GetNextDeadlineTickError, GetProofSubmissionRecordError,
};
use pallet_storage_providers::types::{
    BackupStorageProvider, BackupStorageProviderId, BucketId, MainStorageProviderId,
    Multiaddresses, ProviderIdFor, StorageDataUnit, StorageProviderId, ValuePropositionWithId,
};
use pallet_storage_providers_runtime_api::{
    GetBspInfoError, GetStakeError, QueryAvailableStorageCapacityError, QueryBucketsForMspError,
    QueryBucketsOfUserStoredByMspError, QueryEarliestChangeCapacityBlockError,
    QueryMspIdOfBucketIdError, QueryProviderMultiaddressesError, QueryStorageProviderCapacityError,
};
pub use parachains_common::BlockNumber;
use shp_file_metadata::ChunkId;
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, Get, OpaqueMetadata, H160, H256};
use sp_runtime::{
    generic, impl_opaque_keys,
    traits::{
        BlakeTwo256, DispatchInfoOf, Dispatchable, IdentifyAccount, PostDispatchInfoOf, Verify,
    },
    transaction_validity::{TransactionSource, TransactionValidity, TransactionValidityError},
    ApplyExtrinsicResult, ExtrinsicInclusionMode, Perbill,
};
use sp_std::{
    collections::btree_map::BTreeMap,
    prelude::{Vec, *},
};
use sp_version::RuntimeVersion;
use weights::ExtrinsicBaseWeight;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

impl_opaque_keys! {
    pub struct SessionKeys {
        pub babe: Babe,
        pub grandpa: Grandpa,
    }
}

pub mod currency {
    use super::Balance;

    // Provide a common factor between runtimes based on a supply of 10_000_000 tokens.
    pub const SUPPLY_FACTOR: Balance = 1;

    pub const WEI: Balance = 1;
    pub const KILOWEI: Balance = 1_000;
    pub const MEGAWEI: Balance = 1_000_000;
    pub const GIGAWEI: Balance = 1_000_000_000;
    pub const MICROHAVE: Balance = 1_000_000_000_000;
    pub const MILLIHAVE: Balance = 1_000_000_000_000_000;
    pub const HAVE: Balance = 1_000_000_000_000_000_000;
    pub const KILOHAVE: Balance = 1_000_000_000_000_000_000_000;

    pub const TRANSACTION_BYTE_FEE: Balance = 1 * GIGAWEI * SUPPLY_FACTOR;
    pub const STORAGE_BYTE_FEE: Balance = 100 * MICROHAVE * SUPPLY_FACTOR;
    pub const WEIGHT_FEE: Balance = 50 * KILOWEI * SUPPLY_FACTOR / 4;

    pub const fn deposit(items: u32, bytes: u32) -> Balance {
        items as Balance * 1 * HAVE * SUPPLY_FACTOR + (bytes as Balance) * STORAGE_BYTE_FEE
    }
}

pub mod gas {
    use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;

    /// Current approximation of the gas/s consumption considering
    /// EVM execution over compiled WASM (on 4.4Ghz CPU).
    /// Given the 1000ms Weight, from which 75% only are used for transactions,
    /// the total EVM execution gas limit is: GAS_PER_SECOND * 1 * 0.75 ~= 30_000_000.
    pub const GAS_PER_SECOND: u64 = 40_000_000;

    /// Approximate ratio of the amount of Weight per Gas.
    /// u64 works for approximations because Weight is a very small unit compared to gas.
    pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND / GAS_PER_SECOND;

    /// The highest amount of new storage that can be created in a block (160KB).
    pub const BLOCK_STORAGE_LIMIT: u64 = 160 * 1024;
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = EthereumSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// The address format for describing accounts.
pub type Address = AccountId;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The TransactionExtension to the basic transaction logic.
pub type TxExtension = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        // in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1 MILLIUNIT:
        // in our template, we map to 1/10 of that, or 1/10 MILLIUNIT
        let p = MILLIUNIT / 10;
        let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
        smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: alloc::borrow::Cow::Borrowed("sh-solochain-evm"),
    impl_name: alloc::borrow::Cow::Borrowed("sh-solochain-evm"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// Unit = the base number of indivisible units for balances
pub const UNIT: Balance = 1_000_000_000_000;
pub const CENTS: Balance = UNIT / 100;
pub const MILLIUNIT: Balance = 1_000_000_000;
pub const MICROUNIT: Balance = 1_000_000;
pub const NANOUNIT: Balance = 1_000;
pub const PICOUNIT: Balance = 1;

/// The existential deposit.
pub const EXISTENTIAL_DEPOSIT: Balance = 100;

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 2 seconds of compute with a 6 second average block.
pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
    WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
    cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

// Create the runtime by composing the FRAME pallets that were previously configured.
#[frame_support::runtime]
mod runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Runtime;

    // ╔══════════════════ System and Consensus Pallets ═════════════════╗
    #[runtime::pallet_index(0)]
    pub type System = frame_system;

    // Babe must be before session.
    #[runtime::pallet_index(1)]
    pub type Babe = pallet_babe;

    #[runtime::pallet_index(2)]
    pub type Timestamp = pallet_timestamp;

    #[runtime::pallet_index(3)]
    pub type Balances = pallet_balances;

    // Consensus support.
    // Authorship must be before session in order to note author in the correct session and era.
    #[runtime::pallet_index(4)]
    pub type Authorship = pallet_authorship;

    #[runtime::pallet_index(5)]
    pub type Offences = pallet_offences;

    #[runtime::pallet_index(6)]
    pub type Historical = pallet_session::historical;

    #[runtime::pallet_index(8)]
    pub type Session = pallet_session;

    #[runtime::pallet_index(10)]
    pub type Grandpa = pallet_grandpa;

    #[runtime::pallet_index(11)]
    pub type TransactionPayment = pallet_transaction_payment;
    // ╚═════════════════ System and Consensus Pallets ══════════════════╝

    // ╔═════════════════ Polkadot SDK Utility Pallets ══════════════════╗

    #[runtime::pallet_index(35)]
    pub type Parameters = pallet_parameters;

    #[runtime::pallet_index(36)]
    pub type Sudo = pallet_sudo;

    #[runtime::pallet_index(90)]
    pub type Nfts = pallet_nfts;
    // ╚═════════════════ Polkadot SDK Utility Pallets ══════════════════╝

    // ╔════════════════════ Frontier (EVM) Pallets ═════════════════════╗
    #[runtime::pallet_index(50)]
    pub type Ethereum = pallet_ethereum;

    #[runtime::pallet_index(51)]
    pub type Evm = pallet_evm;

    #[runtime::pallet_index(52)]
    pub type EvmChainId = pallet_evm_chain_id;
    // ╚════════════════════ Frontier (EVM) Pallets ═════════════════════╝

    // ╔══════════════════════ StorageHub Pallets ═══════════════════════╗
    // Start with index 80
    #[runtime::pallet_index(80)]
    pub type Providers = pallet_storage_providers;

    #[runtime::pallet_index(81)]
    pub type FileSystem = pallet_file_system;

    #[runtime::pallet_index(82)]
    pub type ProofsDealer = pallet_proofs_dealer;

    #[runtime::pallet_index(83)]
    pub type Randomness = pallet_randomness;

    #[runtime::pallet_index(84)]
    pub type PaymentStreams = pallet_payment_streams;

    #[runtime::pallet_index(85)]
    pub type BucketNfts = pallet_bucket_nfts;

    // TODO: Add `pallet_cr_randomness` to the runtime when it's ready.
    // #[runtime::pallet_index(46)]
    // pub type CrRandomness = pallet_cr_randomness;
    // ╚══════════════════════ StorageHub Pallets ═══════════════════════╝
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    frame_benchmarking::define_benchmarks!(
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
        [pallet_sudo, Sudo]
        [pallet_nfts, Nfts]
        [pallet_parameters, Parameters]
        [pallet_payment_streams, PaymentStreams]
        [pallet_proofs_dealer, ProofsDealer]
        [pallet_storage_providers, Providers]
        [pallet_randomness, Randomness]
        [pallet_file_system, FileSystem]
        [pallet_bucket_nfts, BucketNfts]
    );
}

impl fp_self_contained::SelfContainedCall for RuntimeCall {
    type SignedInfo = H160;

    fn is_self_contained(&self) -> bool {
        match self {
            RuntimeCall::Ethereum(call) => call.is_self_contained(),
            _ => false,
        }
    }

    fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => call.check_self_contained(),
            _ => None,
        }
    }

    fn validate_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<Self>,
        len: usize,
    ) -> Option<TransactionValidity> {
        match self {
            RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
            _ => None,
        }
    }

    fn pre_dispatch_self_contained(
        &self,
        info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<Self>,
        len: usize,
    ) -> Option<Result<(), TransactionValidityError>> {
        match self {
            RuntimeCall::Ethereum(call) => {
                call.pre_dispatch_self_contained(info, dispatch_info, len)
            }
            _ => None,
        }
    }

    fn apply_self_contained(
        self,
        info: Self::SignedInfo,
    ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
        match self {
            call @ RuntimeCall::Ethereum(_) => Some(call.dispatch(RuntimeOrigin::from(
                pallet_ethereum::RawOrigin::EthereumTransaction(info),
            ))),
            _ => None,
        }
    }
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeConfiguration {
            let epoch_config = Babe::epoch_config().unwrap_or(crate::configs::BABE_GENESIS_EPOCH_CONFIG);
            sp_consensus_babe::BabeConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: crate::configs::time::EpochDurationInBlocks::get().into(),
                c: epoch_config.c,
                authorities: Babe::authorities().to_vec(),
                randomness: Babe::randomness(),
                allowed_slots: epoch_config.allowed_slots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            use codec::Encode;

            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Babe::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> Vec<(pallet_grandpa::AuthorityId, u64)> {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                sp_runtime::traits::NumberFor<Block>,
            >,
            key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;

            Grandpa::submit_unsigned_equivocation_report(
                equivocation_proof,
                key_owner_proof,
            )
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            authority_id: sp_consensus_grandpa::AuthorityId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
        for Runtime
    {
        fn query_call_info(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_call_info(call, len)
        }
        fn query_call_fee_details(
            call: RuntimeCall,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_call_fee_details(call, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, configs::RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            // NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
            // have a backtrace here.
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;


            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();
            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch};
            use sp_storage::TrackedStorageKey;
            use frame_system_benchmarking::Pallet as SystemBench;


            use frame_support::traits::WhitelistedStorageKeys;
            let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmarks!(params, batches);

            Ok(batches)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, crate::genesis_config_presets::get_preset)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            crate::genesis_config_presets::preset_names()
        }
    }

    impl pallet_file_system_runtime_api::FileSystemApi<Block, AccountId, BackupStorageProviderId<Runtime>, MainStorageProviderId<Runtime>, H256, BlockNumber, ChunkId, BucketId<Runtime>, StorageRequestMetadata<Runtime>, BucketId<Runtime>, StorageDataUnit<Runtime>, H256> for Runtime {
        fn is_storage_request_open_to_volunteers(file_key: H256) -> Result<bool, IsStorageRequestOpenToVolunteersError> {
            FileSystem::is_storage_request_open_to_volunteers(file_key)
        }

        fn query_earliest_file_volunteer_tick(bsp_id: BackupStorageProviderId<Runtime>, file_key: H256) -> Result<BlockNumber, QueryFileEarliestVolunteerTickError> {
            FileSystem::query_earliest_file_volunteer_tick(bsp_id, file_key)
        }

        fn query_bsp_confirm_chunks_to_prove_for_file(bsp_id: BackupStorageProviderId<Runtime>, file_key: H256) -> Result<Vec<ChunkId>, QueryBspConfirmChunksToProveForFileError> {
            FileSystem::query_bsp_confirm_chunks_to_prove_for_file(bsp_id, file_key)
        }

        fn query_msp_confirm_chunks_to_prove_for_file(msp_id: MainStorageProviderId<Runtime>, file_key: H256) -> Result<Vec<ChunkId>, QueryMspConfirmChunksToProveForFileError> {
            FileSystem::query_msp_confirm_chunks_to_prove_for_file(msp_id, file_key)
        }

        fn decode_generic_apply_delta_event_info(encoded_event_info: Vec<u8>) -> Result<BucketId<Runtime>, GenericApplyDeltaEventInfoError> {
            FileSystem::decode_generic_apply_delta_event_info(encoded_event_info)
        }

        fn pending_storage_requests_by_msp(msp_id: MainStorageProviderId<Runtime>) -> BTreeMap<H256, StorageRequestMetadata<Runtime>> {
            FileSystem::pending_storage_requests_by_msp(msp_id)
        }
        fn query_incomplete_storage_request_metadata(file_key: H256) -> Result<pallet_file_system_runtime_api::IncompleteStorageRequestMetadataResponse<AccountId, BucketId<Runtime>, StorageDataUnit<Runtime>, H256, BackupStorageProviderId<Runtime>>, QueryIncompleteStorageRequestMetadataError> {
            FileSystem::query_incomplete_storage_request_metadata(file_key)
        }
    }

    impl pallet_payment_streams_runtime_api::PaymentStreamsApi<Block, ProviderIdFor<Runtime>, Balance, AccountId> for Runtime {
        fn get_users_with_debt_over_threshold(provider_id: &ProviderIdFor<Runtime>, threshold: Balance) -> Result<Vec<AccountId>, GetUsersWithDebtOverThresholdError> {
            PaymentStreams::get_users_with_debt_over_threshold(provider_id, threshold)
        }
        fn get_users_of_payment_streams_of_provider(provider_id: &ProviderIdFor<Runtime>) -> Vec<AccountId> {
            PaymentStreams::get_users_of_payment_streams_of_provider(provider_id)
        }
        fn get_providers_with_payment_streams_with_user(user_account: &AccountId) -> Vec<ProviderIdFor<Runtime>> {
            PaymentStreams::get_providers_with_payment_streams_with_user(user_account)
        }
    }

    impl pallet_proofs_dealer_runtime_api::ProofsDealerApi<Block, ProofsDealerProviderIdFor<Runtime>, BlockNumber, KeyFor<Runtime>, RandomnessOutputFor<Runtime>, CustomChallenge<Runtime>> for Runtime {
        fn get_last_tick_provider_submitted_proof(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetProofSubmissionRecordError> {
            ProofsDealer::get_last_tick_provider_submitted_proof(provider_id)
        }

        fn get_next_tick_to_submit_proof_for(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetProofSubmissionRecordError> {
            ProofsDealer::get_next_tick_to_submit_proof_for(provider_id)
        }

        fn get_last_checkpoint_challenge_tick() -> BlockNumber {
            ProofsDealer::get_last_checkpoint_challenge_tick()
        }

        fn get_checkpoint_challenges(
            tick: BlockNumber
        ) -> Result<Vec<CustomChallenge<Runtime>>, GetCheckpointChallengesError> {
            ProofsDealer::get_checkpoint_challenges(tick)
        }

        fn get_challenge_seed(tick: BlockNumber) -> Result<RandomnessOutputFor<Runtime>, GetChallengeSeedError> {
            ProofsDealer::get_challenge_seed(tick)
        }

        fn get_challenge_period(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetChallengePeriodError> {
            ProofsDealer::get_challenge_period(provider_id)
        }

        fn get_checkpoint_challenge_period() -> BlockNumber {
            ProofsDealer::get_checkpoint_challenge_period()
        }

        fn get_challenges_from_seed(seed: &RandomnessOutputFor<Runtime>, provider_id: &ProofsDealerProviderIdFor<Runtime>, count: u32) -> Vec<KeyFor<Runtime>> {
            ProofsDealer::get_challenges_from_seed(seed, provider_id, count)
        }

        fn get_forest_challenges_from_seed(seed: &RandomnessOutputFor<Runtime>, provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Vec<KeyFor<Runtime>> {
            ProofsDealer::get_forest_challenges_from_seed(seed, provider_id)
        }

        fn get_current_tick() -> BlockNumber {
            ProofsDealer::get_current_tick()
        }

        fn get_next_deadline_tick(provider_id: &ProofsDealerProviderIdFor<Runtime>) -> Result<BlockNumber, GetNextDeadlineTickError> {
            ProofsDealer::get_next_deadline_tick(provider_id)
        }
    }


    impl pallet_storage_providers_runtime_api::StorageProvidersApi<Block, BlockNumber, BackupStorageProviderId<Runtime>, BackupStorageProvider<Runtime>, MainStorageProviderId<Runtime>, AccountId, ProviderIdFor<Runtime>, StorageProviderId<Runtime>, StorageDataUnit<Runtime>, Balance, BucketId<Runtime>, Multiaddresses<Runtime>, ValuePropositionWithId<Runtime>> for Runtime {
        fn get_bsp_info(bsp_id: &BackupStorageProviderId<Runtime>) -> Result<BackupStorageProvider<Runtime>, GetBspInfoError> {
            Providers::get_bsp_info(bsp_id)
        }

        fn get_storage_provider_id(who: &AccountId) -> Option<StorageProviderId<Runtime>> {
            Providers::get_storage_provider_id(who)
        }

        fn query_msp_id_of_bucket_id(bucket_id: &BucketId<Runtime>) -> Result<Option<ProviderIdFor<Runtime>>, QueryMspIdOfBucketIdError> {
            Providers::query_msp_id_of_bucket_id(bucket_id)
        }

        fn query_provider_multiaddresses(provider_id: &ProviderIdFor<Runtime>) -> Result<Multiaddresses<Runtime>, QueryProviderMultiaddressesError> {
            Providers::query_provider_multiaddresses(provider_id)
        }

        fn query_storage_provider_capacity(provider_id: &ProviderIdFor<Runtime>) -> Result<StorageDataUnit<Runtime>, QueryStorageProviderCapacityError> {
            Providers::query_storage_provider_capacity(provider_id)
        }

        fn query_available_storage_capacity(provider_id: &ProviderIdFor<Runtime>) -> Result<StorageDataUnit<Runtime>, QueryAvailableStorageCapacityError> {
            Providers::query_available_storage_capacity(provider_id)
        }

        fn query_earliest_change_capacity_block(provider_id: &BackupStorageProviderId<Runtime>) -> Result<BlockNumber, QueryEarliestChangeCapacityBlockError> {
            Providers::query_earliest_change_capacity_block(provider_id)
        }

        fn get_worst_case_scenario_slashable_amount(provider_id: ProviderIdFor<Runtime>) -> Option<Balance> {
            Providers::get_worst_case_scenario_slashable_amount(&provider_id).ok()
        }

        fn get_slash_amount_per_max_file_size() -> Balance {
            Providers::get_slash_amount_per_max_file_size()
        }

        fn query_value_propositions_for_msp(msp_id: &MainStorageProviderId<Runtime>) -> Vec<ValuePropositionWithId<Runtime>> {
            Providers::query_value_propositions_for_msp(msp_id)
        }

        fn get_bsp_stake(bsp_id: &BackupStorageProviderId<Runtime>) -> Result<Balance, GetStakeError> {
            Providers::get_bsp_stake(bsp_id)
        }

        fn can_delete_provider(provider_id: &ProviderIdFor<Runtime>) -> bool {
            Providers::can_delete_provider(provider_id)
        }

        fn query_buckets_for_msp(msp_id: &MainStorageProviderId<Runtime>) -> Result<Vec<BucketId<Runtime>>, QueryBucketsForMspError> {
            Providers::query_buckets_for_msp(msp_id)
        }

        fn query_buckets_of_user_stored_by_msp(msp_id: &ProviderIdFor<Runtime>, user: &AccountId) -> Result<sp_runtime::Vec<BucketId<Runtime>>, QueryBucketsOfUserStoredByMspError> {
            Ok(sp_runtime::Vec::from_iter(Providers::query_buckets_of_user_stored_by_msp(msp_id, user)?))
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
        fn chain_id() -> u64 {
            <Runtime as pallet_evm::Config>::ChainId::get()
        }

        fn account_basic(address: H160) -> pallet_evm::Account {
            let (account, _) = pallet_evm::Pallet::<Runtime>::account_basic(&address);
            account
        }

        fn gas_price() -> sp_core::U256 {
            let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
            gas_price
        }

        fn account_code_at(address: H160) -> Vec<u8> {
            pallet_evm::AccountCodes::<Runtime>::get(address)
        }

        fn author() -> H160 {
            <pallet_evm::Pallet<Runtime>>::find_author()
        }

        fn storage_at(address: H160, index: sp_core::U256) -> H256 {
            let tmp = index.to_big_endian();
            pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
        }

        fn call(
            from: H160,
            to: H160,
            data: Vec<u8>,
            value: sp_core::U256,
            gas_limit: sp_core::U256,
            max_fee_per_gas: Option<sp_core::U256>,
            max_priority_fee_per_gas: Option<sp_core::U256>,
            nonce: Option<sp_core::U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };
            let is_transactional = false;
            let validate = true;

            // Estimated encoded transaction size must be based on the heaviest transaction
            // type (EIP1559Transaction) to be compatible with all transaction types.
            let mut estimated_transaction_len = data.len() +
                // pallet ethereum index: 1
                // transact call index: 1
                // Transaction enum variant: 1
                // chain_id 8 bytes
                // nonce: 32
                // max_priority_fee_per_gas: 32
                // max_fee_per_gas: 32
                // gas_limit: 32
                // action: 21 (enum variant + call address)
                // value: 32
                // access_list: 1 (empty vec size)
                // 65 bytes signature
                258;

            if access_list.is_some() {
                estimated_transaction_len += access_list.encoded_size();
            }

            let gas_limit = gas_limit.min(u64::MAX.into()).low_u64();
            let without_base_extrinsic_weight = true;

            let (weight_limit, proof_size_base_cost) =
                match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
                    gas_limit,
                    without_base_extrinsic_weight
                ) {
                    weight_limit if weight_limit.proof_size() > 0 => {
                        (Some(weight_limit), Some(estimated_transaction_len as u64))
                    }
                    _ => (None, None),
                };

            <Runtime as pallet_evm::Config>::Runner::call(
                from,
                to,
                data,
                value,
                gas_limit,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                validate,
                weight_limit,
                proof_size_base_cost,
                config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.error.into())
        }

        fn create(
            from: H160,
            data: Vec<u8>,
            value: sp_core::U256,
            gas_limit: sp_core::U256,
            max_fee_per_gas: Option<sp_core::U256>,
            max_priority_fee_per_gas: Option<sp_core::U256>,
            nonce: Option<sp_core::U256>,
            estimate: bool,
            access_list: Option<Vec<(H160, Vec<H256>)>>,
        ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
            let config = if estimate {
                let mut config = <Runtime as pallet_evm::Config>::config().clone();
                config.estimate = true;
                Some(config)
            } else {
                None
            };
            let is_transactional = false;
            let validate = true;

            let gas_limit = if gas_limit > sp_core::U256::from(u64::MAX) {
                u64::MAX
            } else {
                gas_limit.low_u64()
            };

            let (weight_limit, proof_size_base_cost) = (None, None);

            #[allow(clippy::or_fun_call)]
            <Runtime as pallet_evm::Config>::Runner::create(
                from,
                data,
                value,
                gas_limit,
                max_fee_per_gas,
                max_priority_fee_per_gas,
                nonce,
                access_list.unwrap_or_default(),
                is_transactional,
                validate,
                weight_limit,
                proof_size_base_cost,
                config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
            ).map_err(|err| err.error.into())
        }

        fn current_transaction_statuses() -> Option<Vec<fp_rpc::TransactionStatus>> {
            pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
        }

        fn current_block() -> Option<pallet_ethereum::Block> {
            pallet_ethereum::CurrentBlock::<Runtime>::get()
        }

        fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
            pallet_ethereum::CurrentReceipts::<Runtime>::get()
        }

        fn current_all() -> (
            Option<pallet_ethereum::Block>,
            Option<Vec<pallet_ethereum::Receipt>>,
            Option<Vec<fp_rpc::TransactionStatus>>,
        ) {
            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentReceipts::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
            )
        }

        fn extrinsic_filter(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> Vec<pallet_ethereum::Transaction> {
            xts.into_iter().filter_map(|xt| match xt.0.function {
                RuntimeCall::Ethereum(pallet_ethereum::Call::transact { transaction }) => Some(transaction),
                _ => None
            }).collect::<Vec<pallet_ethereum::Transaction>>()
        }

        fn elasticity() -> Option<sp_runtime::Permill> {
            None
        }

        fn gas_limit_multiplier_support() {}

        fn pending_block(
            xts: Vec<<Block as BlockT>::Extrinsic>,
        ) -> (Option<pallet_ethereum::Block>, Option<Vec<fp_rpc::TransactionStatus>>) {
            for ext in xts.into_iter() {
                let _ = Executive::apply_extrinsic(ext);
            }

            Ethereum::on_finalize(System::block_number() + 1);

            (
                pallet_ethereum::CurrentBlock::<Runtime>::get(),
                pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
            )
        }

        fn initialize_pending_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header);
        }
    }

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
        fn convert_transaction(transaction: pallet_ethereum::Transaction) -> <Block as BlockT>::Extrinsic {
            UncheckedExtrinsic::new_bare(
                pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
            )
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
impl frame_system_benchmarking::Config for Runtime {}

#[cfg(feature = "runtime-benchmarks")]
impl frame_benchmarking::baseline::Config for Runtime {}

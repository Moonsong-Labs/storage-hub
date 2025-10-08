mod runtime_params;
pub mod xcm_config;

// Substrate and Polkadot dependencies
use crate::mock_message_queue;
use crate::storagehub::{configs::xcm_config::XcmConfig, MessageQueue, ParachainInfo, PolkadotXcm};
use core::marker::PhantomData;
use cumulus_pallet_parachain_system::{
    DefaultCoreSelector, RelayChainStateProof, RelayNumberMonotonicallyIncreases,
};
use cumulus_primitives_core::{
    relay_chain::well_known_keys, AggregateMessageOrigin, AssetId, ParaId,
};
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    parameter_types,
    traits::{
        AsEnsureOriginWithArg, ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse,
        TransformOrigin,
    },
    weights::{ConstantMultiplier, Weight},
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    pallet_prelude::BlockNumberFor,
    EnsureRoot, EnsureSigned,
};
use num_bigint::BigUint;
use pallet_nfts::PalletFeatures;
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::{prod_or_fast, BlockHashCount, SlowAdjustingFeeUpdate};
use runtime_params::RuntimeParameters;
use shp_data_price_updater::NoUpdatePriceIndexUpdater;
use shp_file_metadata::ChunkId;
use shp_traits::{
    CommitmentVerifier, IdentityAdapter, MaybeDebug, TrieMutation, TrieProofDeltaApplier,
};
use shp_treasury_funding::{
    LinearThenPowerOfTwoTreasuryCutCalculator, LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{ConstU128, Get, Hasher, H256};
use sp_runtime::traits::Zero;
use sp_runtime::{
    traits::{BlakeTwo256, Convert, ConvertBack, Verify},
    AccountId32, DispatchError, Perbill, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::vec;
use sp_trie::{CompactProof, LayoutV1, MemoryDB, TrieConfiguration, TrieLayout};
use sp_version::RuntimeVersion;
use xcm::latest::prelude::BodyId;
use xcm_simulator::XcmExecutor;

// Local module imports
use super::{
    weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
    AccountId, Aura, Balance, Balances, Block, BlockNumber, BucketNfts, CollatorSelection, Hash,
    Hashing, Nfts, Nonce, PalletInfo, ParachainSystem, PaymentStreams, ProofsDealer, Providers,
    Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin,
    RuntimeTask, Session, SessionKeys, Signature, System, WeightToFee, XcmpQueue,
    AVERAGE_ON_INITIALIZE_RATIO, BLOCK_PROCESSING_VELOCITY, CENTS, DAYS, EXISTENTIAL_DEPOSIT,
    HOURS, MAXIMUM_BLOCK_WEIGHT, MICROUNIT, MINUTES, NORMAL_DISPATCH_RATIO,
    RELAY_CHAIN_SLOT_DURATION_MILLIS, SLOT_DURATION, UNINCLUDED_SEGMENT_CAPACITY, UNIT, VERSION,
};
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;

    // This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
    //  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
    // `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
    // the lazy contract deletion.
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 42;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`ParaChainDefaultConfig`](`struct@frame_system::config_preludes::ParaChainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The index type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The block type.
    type Block = Block;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The action to take on a Runtime Upgrade
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = (CollatorSelection,);
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type DoneSlashHandler = ();
}

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

impl mock_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type WeightInfo = ();
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type OutboundXcmpMessageSource = XcmpQueue;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
    type ConsensusHook = ConsensusHook;
    type SelectCore = DefaultCoreSelector<Runtime>;
}
pub(crate) type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
    Runtime,
    RELAY_CHAIN_SLOT_DURATION_MILLIS,
    BLOCK_PROCESSING_VELOCITY,
    UNINCLUDED_SEGMENT_CAPACITY,
>;

impl parachain_info::Config for Runtime {}

parameter_types! {
    pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    #[cfg(feature = "runtime-benchmarks")]
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
        cumulus_primitives_core::AggregateMessageOrigin,
    >;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MessageProcessor = xcm_builder::ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
        RuntimeCall,
    >;
    type Size = u32;
    // The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
    type MaxStale = sp_core::ConstU32<8>;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = ();
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
    /// The asset ID for the asset that we use to pay for message delivery fees.
    pub FeeAssetId: AssetId = AssetId(xcm_config::RelayLocation::get());
    /// The base fee for the message delivery fees.
    pub const ToSiblingBaseDeliveryFee: u128 = CENTS.saturating_mul(3);
}

pub type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
    FeeAssetId,
    ToSiblingBaseDeliveryFee,
    TransactionByteFee,
    XcmpQueue,
>;

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    // Enqueue XCMP messages from siblings for later processing.
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = ConstU32<1_000>;
    type MaxActiveOutboundChannels = ConstU32<128>;
    // Most on-chain HRMP channels are configured to use 102400 bytes of max message size, so we
    // need to set the page size larger than that until we reduce the channel size on-chain.
    type MaxPageSize = ConstU32<{ 103 * 1024 }>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = ();
    type PriceForSiblingDelivery = PriceForSiblingParachainDelivery;
}

parameter_types! {
    pub const Period: BlockNumber = 6 * HOURS;
    pub const Offset: BlockNumber = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = CollatorSelection;
    // Essentially just Aura, but let's be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100_000>;
    type AllowMultipleBlocksPerSlot = ConstBool<true>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
    pub const PotId: PalletId = PalletId(*b"PotStake");
    pub const SessionLength: BlockNumber = 6 * HOURS;
    // StakingAdmin pluralistic body.
    pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
    EnsureRoot<AccountId>,
    EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
>;

impl pallet_collator_selection::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type UpdateOrigin = CollatorSelectionUpdateOrigin;
    type PotId = PotId;
    type MaxCandidates = ConstU32<100>;
    type MinEligibleCollators = ConstU32<4>;
    type MaxInvulnerables = ConstU32<20>;
    // should be a multiple of session or things will get inconsistent
    type KickThreshold = Period;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
    type ValidatorRegistration = Session;
    type WeightInfo = ();
}

parameter_types! {
    pub Features: PalletFeatures = PalletFeatures::all_enabled();
    pub const MaxAttributesPerCall: u32 = 10;
    pub const CollectionDeposit: Balance = 100 * UNIT;
    pub const ItemDeposit: Balance = 1 * UNIT;
    pub const ApprovalsLimit: u32 = 20;
    pub const ItemAttributesApprovalsLimit: u32 = 20;
    pub const MaxTips: u32 = 10;
    pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
    pub const MetadataDepositBase: Balance = 10 * UNIT;
    pub const MetadataDepositPerByte: Balance = 1 * UNIT;
}

impl pallet_parameters::Config for Runtime {
    type AdminOrigin = EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeParameters = RuntimeParameters;
    type WeightInfo = ();
}

impl pallet_nfts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type CollectionDeposit = CollectionDeposit;
    type ItemDeposit = ItemDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type AttributeDepositBase = MetadataDepositBase;
    type DepositPerByte = MetadataDepositPerByte;
    type StringLimit = ConstU32<256>;
    type KeyLimit = ConstU32<64>;
    type ValueLimit = ConstU32<256>;
    type ApprovalsLimit = ApprovalsLimit;
    type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
    type MaxTips = MaxTips;
    type MaxDeadlineDuration = MaxDeadlineDuration;
    type MaxAttributesPerCall = MaxAttributesPerCall;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = <Signature as Verify>::Signer;
    type WeightInfo = pallet_nfts::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
    type Locker = ();
}

/// Only callable after `set_validation_data` is called which forms this proof the same way
fn relay_chain_state_proof() -> RelayChainStateProof {
    let relay_storage_root = cumulus_pallet_parachain_system::ValidationData::<Runtime>::get()
        .expect("set in `set_validation_data`")
        .relay_parent_storage_root;
    let relay_chain_state = cumulus_pallet_parachain_system::RelayStateProof::<Runtime>::get()
        .expect("set in `set_validation_data`");
    RelayChainStateProof::new(ParachainInfo::get(), relay_storage_root, relay_chain_state)
        .expect("Invalid relay chain state proof, already constructed in `set_validation_data`")
}

pub struct BabeDataGetter;
impl pallet_randomness::GetBabeData<u64, Hash> for BabeDataGetter {
    // Tolerate panic here because this is only ever called in an inherent (so can be omitted)
    fn get_epoch_index() -> u64 {
        if cfg!(feature = "runtime-benchmarks") {
            // storage reads as per actual reads
            let _relay_storage_root =
                cumulus_pallet_parachain_system::ValidationData::<Runtime>::get();
            let _relay_chain_state =
                cumulus_pallet_parachain_system::RelayStateProof::<Runtime>::get();
            const BENCHMARKING_NEW_EPOCH: u64 = 10u64;
            return BENCHMARKING_NEW_EPOCH;
        }
        relay_chain_state_proof()
            .read_optional_entry(well_known_keys::EPOCH_INDEX)
            .ok()
            .flatten()
            .expect("expected to be able to read epoch index from relay chain state proof")
    }
    fn get_epoch_randomness() -> Hash {
        if cfg!(feature = "runtime-benchmarks") {
            // storage reads as per actual reads
            let _relay_storage_root =
                cumulus_pallet_parachain_system::ValidationData::<Runtime>::get();
            let _relay_chain_state =
                cumulus_pallet_parachain_system::RelayStateProof::<Runtime>::get();
            let benchmarking_babe_output = Hash::default();
            return benchmarking_babe_output;
        }
        relay_chain_state_proof()
            .read_optional_entry(well_known_keys::ONE_EPOCH_AGO_RANDOMNESS)
            .ok()
            .flatten()
            .expect("expected to be able to read epoch randomness from relay chain state proof")
    }
    fn get_parent_randomness() -> Hash {
        if cfg!(feature = "runtime-benchmarks") {
            // storage reads as per actual reads
            let _relay_storage_root =
                cumulus_pallet_parachain_system::ValidationData::<Runtime>::get();
            let _relay_chain_state =
                cumulus_pallet_parachain_system::RelayStateProof::<Runtime>::get();
            let benchmarking_babe_output = Hash::default();
            return benchmarking_babe_output;
        }
        // Note: we use the `CURRENT_BLOCK_RANDOMNESS` key here as it also represents the parent randomness, the only difference
        // is the block since this randomness is valid, but we don't care about that because we are setting that directly in the `randomness` pallet.
        relay_chain_state_proof()
            .read_optional_entry(well_known_keys::CURRENT_BLOCK_RANDOMNESS)
            .ok()
            .flatten()
            .expect("expected to be able to read parent randomness from relay chain state proof")
    }
}

parameter_types! {
    pub const MaxBlocksForRandomness: BlockNumber = prod_or_fast!(2 * runtime_constants::time::EPOCH_DURATION_IN_SLOTS, 2 * MINUTES);
}

/// Configure the randomness pallet
impl pallet_randomness::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BabeDataGetter = BabeDataGetter;
    type BabeBlockGetter = BlockNumberGetter;
    type WeightInfo = ();
    type BabeDataGetterBlockNumber = BlockNumber;
}

pub struct BlockNumberGetter {}
impl sp_runtime::traits::BlockNumberProvider for BlockNumberGetter {
    type BlockNumber = BlockNumberFor<Runtime>;

    fn current_block_number() -> Self::BlockNumber {
        cumulus_pallet_parachain_system::RelaychainDataProvider::<Runtime>::current_block_number()
    }
}

/// Type representing the storage data units in StorageHub.
pub type StorageDataUnit = u64;

pub type StorageProofsMerkleTrieLayout = LayoutV1<BlakeTwo256>;

parameter_types! {
    pub const BucketDeposit: Balance = 20 * UNIT;
    pub const MaxMultiAddressSize: u32 = 100;
    pub const MaxMultiAddressAmount: u32 = 5;
    pub const MaxProtocols: u32 = 100;
    pub const MaxBsps: u32 = 100;
    pub const MaxMsps: u32 = 100;
    pub const BucketNameLimit: u32 = 100;
    pub const SpMinDeposit: Balance = 20 * UNIT;
    pub const SpMinCapacity: u64 = 2;
    pub const DepositPerData: Balance = 2;
    pub const MinBlocksBetweenCapacityChanges: u32 = 10;
    pub const SlashAmountPerChunkOfStorageData: Balance = 20 * UNIT;
    pub const BspSignUpLockPeriod: BlockNumber = 50;
}

pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
    }
}

pub struct StorageDataUnitAndBalanceConverter;
impl Convert<StorageDataUnit, Balance> for StorageDataUnitAndBalanceConverter {
    fn convert(data_unit: StorageDataUnit) -> Balance {
        data_unit.saturated_into()
    }
}
impl ConvertBack<StorageDataUnit, Balance> for StorageDataUnitAndBalanceConverter {
    fn convert_back(balance: Balance) -> StorageDataUnit {
        balance.saturated_into()
    }
}

// Benchmark helpers for the Providers pallet
#[cfg(feature = "runtime-benchmarks")]
pub struct ProvidersBenchmarkHelpers;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_storage_providers::benchmarking::BenchmarkHelpers<Runtime>
    for ProvidersBenchmarkHelpers
{
    type ProviderId = <Runtime as pallet_storage_providers::Config>::ProviderId;
    fn set_accrued_failed_proofs(provider_id: Self::ProviderId, value: u32) {
        pallet_proofs_dealer::SlashableProviders::<Runtime>::insert(provider_id, value);
    }

    fn get_accrued_failed_proofs(provider_id: Self::ProviderId) -> u32 {
        pallet_proofs_dealer::SlashableProviders::<Runtime>::get(provider_id).unwrap_or(0)
    }
}

impl pallet_storage_providers::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_storage_providers::weights::SubstrateWeight<Runtime>;
    type ProvidersRandomness = pallet_randomness::RandomnessFromOneEpochAgo<Runtime>;
    type PaymentStreams = PaymentStreams;
    type ProofDealer = ProofsDealer;
    type FileMetadataManager = shp_file_metadata::FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type NativeBalance = Balances;
    type CrRandomness = MockCrRandomness;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageDataUnit = StorageDataUnit;
    type SpCount = u32;
    type BucketCount = u128;
    type MerklePatriciaRoot = Hash;
    type MerkleTrieHashing = Hashing;
    type ProviderId = Hash;
    type ProviderIdHashing = Hashing;
    type ValuePropId = Hash;
    type ValuePropIdHashing = Hashing;
    type ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId;
    type ProvidersProofSubmitters = ProofsDealer;
    type ReputationWeightType = u32;
    type StorageHubTickGetter = ProofsDealer;
    type StorageDataUnitAndBalanceConvert = StorageDataUnitAndBalanceConverter;
    type Treasury = TreasuryAccount;
    type SpMinDeposit = SpMinDeposit;
    type SpMinCapacity = SpMinCapacity;
    type DepositPerData = DepositPerData;
    type MaxFileSize = ConstU64<{ u64::MAX }>;
    type MaxMultiAddressSize = MaxMultiAddressSize;
    type MaxMultiAddressAmount = MaxMultiAddressAmount;
    type MaxProtocols = MaxProtocols;
    type BucketDeposit = BucketDeposit;
    type BucketNameLimit = BucketNameLimit;
    type MaxBlocksForRandomness = MaxBlocksForRandomness;
    type MinBlocksBetweenCapacityChanges = MinBlocksBetweenCapacityChanges;
    type DefaultMerkleRoot = DefaultMerkleRoot<StorageProofsMerkleTrieLayout>;
    type SlashAmountPerMaxFileSize =
        runtime_params::dynamic_params::runtime_config::SlashAmountPerMaxFileSize;
    type StartingReputationWeight = ConstU32<1>;
    type BspSignUpLockPeriod = BspSignUpLockPeriod;
    type MaxCommitmentSize = ConstU32<1000>;
    type ZeroSizeBucketFixedRate = ConstU128<1>;
    type ProviderTopUpTtl = ConstU32<10>;
    type MaxExpiredItemsInBlock = ConstU32<100>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelpers = ProvidersBenchmarkHelpers;
}

parameter_types! {
    pub const PaymentStreamHoldReason: RuntimeHoldReason = RuntimeHoldReason::PaymentStreams(pallet_payment_streams::HoldReason::PaymentStreamDeposit);
    pub const UserWithoutFundsCooldown: BlockNumber = 100;
}

// Converter from the BlockNumber type to the Balance type for math
pub struct BlockNumberToBalance;

impl Convert<BlockNumber, Balance> for BlockNumberToBalance {
    fn convert(block_number: BlockNumber) -> Balance {
        block_number.into() // In this converter we assume that the block number type is smaller in size than the balance type
    }
}

impl LinearThenPowerOfTwoTreasuryCutCalculatorConfig<Perbill> for Runtime {
    type Balance = Balance;
    type ProvidedUnit = StorageDataUnit;
    type IdealUtilisationRate =
        runtime_params::dynamic_params::runtime_config::IdealUtilisationRate;
    type DecayRate = runtime_params::dynamic_params::runtime_config::DecayRate;
    type MinimumCut = runtime_params::dynamic_params::runtime_config::MinimumTreasuryCut;
    type MaximumCut = runtime_params::dynamic_params::runtime_config::MaximumTreasuryCut;
}

impl pallet_payment_streams::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_payment_streams::weights::SubstrateWeight<Runtime>;
    type NativeBalance = Balances;
    type ProvidersPallet = Providers;
    type RuntimeHoldReason = RuntimeHoldReason;
    type UserWithoutFundsCooldown = UserWithoutFundsCooldown; // Amount of blocks that a user will have to wait before being able to clear the out of funds flag
    type NewStreamDeposit = ConstU32<10>; // Amount of blocks that the deposit of a new stream should be able to pay for
    type Units = u64; // Storage unit
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = ProofsDealer;
    type TreasuryCutCalculator = LinearThenPowerOfTwoTreasuryCutCalculator<Runtime, Perbill>;
    type TreasuryAccount = TreasuryAccount;
    type MaxUsersToCharge = ConstU32<10>;
    type BaseDeposit = ConstU128<10>;
}

// TODO: remove this and replace with pallet treasury
pub struct TreasuryAccount;
impl Get<AccountId32> for TreasuryAccount {
    fn get() -> AccountId32 {
        AccountId32::from([0; 32])
    }
}

pub struct BlockFullnessHeadroom;
impl Get<Weight> for BlockFullnessHeadroom {
    fn get() -> Weight {
        // The block headroom is set to be the maximum benchmarked weight that a `submit_proof` extrinsic can have.
        // That is, when the proof includes two file key proofs for every single random challenge, and for the maximum
        // number of checkpoint challenges as well.
        <pallet_proofs_dealer::weights::SubstrateWeight<Runtime> as pallet_proofs_dealer::weights::WeightInfo>::submit_proof_with_checkpoint_challenges_key_proofs(TOTAL_MAX_CHALLENGES_PER_BLOCK * 2)
    }
}

pub struct MinNotFullBlocksRatio;
impl Get<Perbill> for MinNotFullBlocksRatio {
    fn get() -> Perbill {
        // This means that we tolerate at most 50% of misbehaving collators.
        Perbill::from_percent(50)
    }
}

pub struct MaxSubmittersPerTick;
impl Get<u32> for MaxSubmittersPerTick {
    fn get() -> u32 {
        let block_weights = <Runtime as frame_system::Config>::BlockWeights::get();

        // Not being able to get the `max_total` weight for the Normal dispatch class is considered
        // a critical bug. So we set it to be zero, essentially allowing zero submitters per tick.
        // This value can be read from the constants of a node, but with the current configuration, this is:
        //
        // max_total: {
        //   ref_time: 1,500,000,000,000
        //   proof_size: 3,932,160
        // }
        let max_weight_for_class = block_weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(Zero::zero());

        // Get the minimum weight a `submit_proof` extrinsic can have.
        // This would be the case where the proof is just made up of a single file key proof, that is a
        // response to all the random challenges. And there are no checkpoint challenges.
        // With the current benchmarking, this is:
        //
        // TODO: UPDATE THIS WITH THE FINAL BENCHMARKING
        // min_weight_for_submit_proof: {
        //   ref_time: 1,132,469,305
        //   proof_size: 35,487
        // }
        let min_weight_for_submit_proof =
            <pallet_proofs_dealer::weights::SubstrateWeight<Runtime> as pallet_proofs_dealer::weights::WeightInfo>::submit_proof_no_checkpoint_challenges_key_proofs(1);

        // Calculate the maximum number of submit proofs that is possible to have in a block/tick.
        // With the current values, this would be:
        //
        // TODO: UPDATE THIS WITH THE FINAL BENCHMARKING
        // 110 proof submissions per block (limited by `proof_size`)
        let max_proof_submissions_per_tick = max_weight_for_class
            .checked_div_per_component(&min_weight_for_submit_proof)
            .unwrap_or(0);

        // Saturating u64 to u32 should be enough.
        max_proof_submissions_per_tick.saturated_into()
    }
}

pub struct MaxSlashableProvidersPerTick;
impl Get<u32> for MaxSlashableProvidersPerTick {
    fn get() -> u32 {
        // With the maximum number of slashable providers per tick being `N`, the absolute maximum
        // weight that the `on_poll` hook can have, with the current benchmarking, is:
        //
        // TODO: UPDATE THIS WITH THE FINAL BENCHMARKING
        // new_challenge_round_weight: {
        //   ref_time: 577,000,000 + N * 553,024,845
        //   proof_size: 8,523 + N * 3,158
        // }
        // new_checkpoint_challenge_round_max_weight: {
        //   ref_time: 587,562,874 + ChallengesQueueLength * 255,790 = 613,141,874
        //   proof_size: 4,787
        // }
        // check_spamming_condition_weight: {
        //   ref_time: 313,000,000
        //   proof_size: 6,016
        // }
        //
        // For `N` = 1000, this would be:
        // max_on_poll_weight: {
        //   ref_time: 313,000,000 + 613,141,874 + 577,000,000 + N * 553,024,845 ≈ 554,527,987,000
        //   proof_size: 6,016 + 4,787 + 8,523 + N * 3,158 ≈ 3,177,326
        // }
        //
        // Consider that the maximum block weight is:
        // maxBlock: {
        //   ref_time: 2,000,000,000,000
        //   proof_size: 5,242,880
        // }
        //
        // This `on_poll` hook would consume roughly 1/4 of the block `ref_time` and 3/5 of the block `proof_size`.
        // This is naturally a lot. But it would be a very unlikely scenario.
        //
        // This would be the case where all `N` Providers have synchronised their challenge periods
        // and have the same deadline, plus, all of them missed their proof submissions.
        // The normal scenario would be that NONE (or just a small number) of the Providers have
        // missed their proof submissions.
        let max_slashable_providers_per_tick = 1000;
        max_slashable_providers_per_tick
    }
}

const RANDOM_CHALLENGES_PER_BLOCK: u32 = 10;
const MAX_CUSTOM_CHALLENGES_PER_BLOCK: u32 = 10;
const TOTAL_MAX_CHALLENGES_PER_BLOCK: u32 =
    RANDOM_CHALLENGES_PER_BLOCK + MAX_CUSTOM_CHALLENGES_PER_BLOCK;
parameter_types! {
    pub const RandomChallengesPerBlock: u32 = RANDOM_CHALLENGES_PER_BLOCK;
    pub const MaxCustomChallengesPerBlock: u32 = MAX_CUSTOM_CHALLENGES_PER_BLOCK;
    pub const TotalMaxChallengesPerBlock: u32 = TOTAL_MAX_CHALLENGES_PER_BLOCK;
    pub const TargetTicksStorageOfSubmitters: u32 = 3;
    pub const ChallengeHistoryLength: BlockNumber = 100;
    pub const ChallengesQueueLength: u32 = 100;
    pub const CheckpointChallengePeriod: u32 = 30;
    pub const ChallengesFee: Balance = 1 * UNIT;
    pub const PriorityChallengesFee: Balance = 0;
    pub const StakeToChallengePeriod: Balance = 200 * UNIT;
    pub const MinChallengePeriod: u32 = 30;
    pub const ChallengeTicksTolerance: u32 = 50;
}

impl pallet_proofs_dealer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_proofs_dealer::weights::SubstrateWeight<Runtime>;
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleTrieHash = Hash;
    type MerkleTrieHashing = BlakeTwo256;
    type ForestVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type KeyVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type StakeToBlockNumber = SaturatingBalanceToBlockNumber;
    type RandomChallengesPerBlock = RandomChallengesPerBlock;
    type MaxCustomChallengesPerBlock = MaxCustomChallengesPerBlock;
    type MaxSubmittersPerTick = MaxSubmittersPerTick;
    type TargetTicksStorageOfSubmitters = TargetTicksStorageOfSubmitters;
    type ChallengeHistoryLength = ChallengeHistoryLength;
    type ChallengesQueueLength = ChallengesQueueLength;
    type CheckpointChallengePeriod =
        runtime_params::dynamic_params::runtime_config::CheckpointChallengePeriod;
    type ChallengesFee = ChallengesFee;
    type PriorityChallengesFee = PriorityChallengesFee;
    type Treasury = TreasuryAccount;
    // TODO: Once the client logic to keep track of CR randomness deadlines and execute their submissions is implemented
    // AND after the chain has been live for enough time to have enough providers to avoid the commit-reveal randomness being
    // gameable, the randomness provider should be CrRandomness
    type RandomnessProvider = pallet_randomness::ParentBlockRandomness<Runtime>;
    type StakeToChallengePeriod =
        runtime_params::dynamic_params::runtime_config::StakeToChallengePeriod;
    type MinChallengePeriod = runtime_params::dynamic_params::runtime_config::MinChallengePeriod;
    type ChallengeTicksTolerance = ChallengeTicksTolerance;
    type BlockFullnessPeriod = ChallengeTicksTolerance; // We purposely set this to `ChallengeTicksTolerance` so that spamming of the chain is evaluated for the same blocks as the tolerance BSPs are given.
    type BlockFullnessHeadroom = BlockFullnessHeadroom;
    type MinNotFullBlocksRatio = MinNotFullBlocksRatio;
    type MaxSlashableProvidersPerTick = MaxSlashableProvidersPerTick;
    type ChallengeOrigin = EnsureSigned<AccountId>;
    type PriorityChallengeOrigin = EnsureRoot<AccountId>;
}

/// Structure to mock a verifier that returns `true` when `proof` is not empty
/// and `false` otherwise.
pub struct MockVerifier<C, T: TrieLayout, const H_LENGTH: usize> {
    _phantom: core::marker::PhantomData<(C, T)>,
}

/// Implement the `TrieVerifier` trait for the `MockForestManager` struct.
impl<C, T: TrieLayout, const H_LENGTH: usize> CommitmentVerifier for MockVerifier<C, T, H_LENGTH>
where
    C: MaybeDebug + Ord + Default + Copy + AsRef<[u8]> + AsMut<[u8]>,
{
    type Proof = CompactProof;
    type Commitment = H256;
    type Challenge = H256;

    fn verify_proof(
        _root: &Self::Commitment,
        _challenges: &[Self::Challenge],
        proof: &CompactProof,
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError> {
        if proof.encoded_nodes.len() > 0 {
            Ok(proof
                .encoded_nodes
                .iter()
                .map(|node| H256::from_slice(&node[..]))
                .collect())
        } else {
            Err("Proof is empty".into())
        }
    }
}

impl<C, T: TrieLayout, const H_LENGTH: usize> TrieProofDeltaApplier<T::Hash>
    for MockVerifier<C, T, H_LENGTH>
where
    <T::Hash as sp_core::Hasher>::Out: for<'a> TryFrom<&'a [u8; H_LENGTH]>,
{
    type Proof = CompactProof;
    type Key = <T::Hash as sp_core::Hasher>::Out;

    fn apply_delta(
        root: &Self::Key,
        mutations: &[(Self::Key, TrieMutation)],
        _proof: &Self::Proof,
    ) -> Result<
        (
            MemoryDB<T::Hash>,
            Self::Key,
            Vec<(Self::Key, Option<Vec<u8>>)>,
        ),
        DispatchError,
    > {
        Ok((
            MemoryDB::<T::Hash>::default(),
            match mutations.len() {
                0 => *root,
                _ => mutations.last().unwrap().0,
            },
            Vec::new(),
        ))
    }
}

type ThresholdType = u32;
pub type ReplicationTargetType = u32;

parameter_types! {
    pub const MaxBatchConfirmStorageRequests: u32 = 10;
    pub const BaseStorageRequestCreationDeposit: Balance = 1 * UNIT;
    pub const FileDeletionRequestCreationDeposit: Balance = 1 * UNIT;
    pub const FileSystemHoldReason: RuntimeHoldReason = RuntimeHoldReason::FileSystem(pallet_file_system::HoldReason::StorageRequestCreationHold);
}

/// Configure the pallet template in pallets/template.
impl pallet_file_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_file_system::weights::SubstrateWeight<Runtime>;
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type PaymentStreams = PaymentStreams;
    // TODO: Replace the mocked CR randomness with the actual one when it's ready
    // type CrRandomness = CrRandomness;
    type CrRandomness = MockCrRandomness;
    type UpdateStoragePrice = NoUpdatePriceIndexUpdater<Balance, u64>;
    type UserSolvency = PaymentStreams;
    type Fingerprint = Hash;
    type ReplicationTargetType = ReplicationTargetType;
    type ThresholdType = ThresholdType;
    type ThresholdTypeToTickNumber = ThresholdTypeToBlockNumberConverter;
    type HashToThresholdType = HashToThresholdTypeConverter;
    type MerkleHashToRandomnessOutput = MerkleHashToRandomnessOutputConverter;
    type ChunkIdToMerkleHash = ChunkIdToMerkleHashConverter;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = Nfts;
    type CollectionInspector = BucketNfts;
    type MaxBatchConfirmStorageRequests = MaxBatchConfirmStorageRequests;
    type BspStopStoringFilePenalty = ConstU128<1>;
    type TreasuryAccount = TreasuryAccount;
    type MaxFilePathSize = ConstU32<512u32>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = ConstU32<5>;
    type MaxDataServerMultiAddresses = ConstU32<10>;
    type MaxExpiredItemsInTick = ConstU32<100>;
    type StorageRequestTtl = runtime_params::dynamic_params::runtime_config::StorageRequestTtl;
    type MoveBucketRequestTtl = ConstU32<40u32>;
    type MaxUserPendingDeletionRequests = ConstU32<10u32>;
    type MaxUserPendingMoveBucketRequests = ConstU32<10u32>;
    type MinWaitForStopStoring =
        runtime_params::dynamic_params::runtime_config::MinWaitForStopStoring;
    type BaseStorageRequestCreationDeposit = BaseStorageRequestCreationDeposit;
    type UpfrontTicksToPay = runtime_params::dynamic_params::runtime_config::UpfrontTicksToPay;
    type WeightToFee = WeightToFee;
    type ReplicationTargetToBalance = ReplicationTargetToBalance;
    type TickNumberToBalance = TickNumberToBalance;
    type StorageDataUnitToBalance = StorageDataUnitToBalance;
    type FileDeletionRequestDeposit = FileDeletionRequestCreationDeposit;
    type BasicReplicationTarget =
        runtime_params::dynamic_params::runtime_config::BasicReplicationTarget;
    type StandardReplicationTarget =
        runtime_params::dynamic_params::runtime_config::StandardReplicationTarget;
    type HighSecurityReplicationTarget =
        runtime_params::dynamic_params::runtime_config::HighSecurityReplicationTarget;
    type SuperHighSecurityReplicationTarget =
        runtime_params::dynamic_params::runtime_config::SuperHighSecurityReplicationTarget;
    type UltraHighSecurityReplicationTarget =
        runtime_params::dynamic_params::runtime_config::UltraHighSecurityReplicationTarget;
    type MaxReplicationTarget =
        runtime_params::dynamic_params::runtime_config::MaxReplicationTarget;
    type TickRangeToMaximumThreshold =
        runtime_params::dynamic_params::runtime_config::TickRangeToMaximumThreshold;
    type OffchainSignature = Signature;
    type OffchainPublicKey = <Signature as Verify>::Signer;
    type IntentionMsgAdapter = IdentityAdapter;
}

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;

impl Convert<Balance, BlockNumberFor<Runtime>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Runtime> {
        block_number.saturated_into()
    }
}

// Converter from the ThresholdType to the BlockNumber type and vice versa.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct ThresholdTypeToBlockNumberConverter;

impl Convert<ThresholdType, BlockNumberFor<Runtime>> for ThresholdTypeToBlockNumberConverter {
    fn convert(threshold: ThresholdType) -> BlockNumberFor<Runtime> {
        threshold.saturated_into()
    }
}

impl ConvertBack<ThresholdType, BlockNumberFor<Runtime>> for ThresholdTypeToBlockNumberConverter {
    fn convert_back(block_number: BlockNumberFor<Runtime>) -> ThresholdType {
        block_number.into()
    }
}

/// Converter from the [`Hash`] type to the [`ThresholdType`].
pub struct HashToThresholdTypeConverter;
impl Convert<<Runtime as frame_system::Config>::Hash, ThresholdType>
    for HashToThresholdTypeConverter
{
    fn convert(hash: <Runtime as frame_system::Config>::Hash) -> ThresholdType {
        // Get the hash as bytes
        let hash_bytes = hash.as_ref();

        // Get the 4 least significant bytes of the hash and interpret them as an u32
        let truncated_hash_bytes: [u8; 4] =
            hash_bytes[28..].try_into().expect("Hash is 32 bytes; qed");

        ThresholdType::from_be_bytes(truncated_hash_bytes)
    }
}

// Converter from the MerkleHash (H256) type to the RandomnessOutput (H256) type.
pub struct MerkleHashToRandomnessOutputConverter;

impl Convert<H256, H256> for MerkleHashToRandomnessOutputConverter {
    fn convert(hash: H256) -> H256 {
        hash
    }
}

// Converter from the ChunkId type to the MerkleHash (H256) type.
pub struct ChunkIdToMerkleHashConverter;

impl Convert<ChunkId, H256> for ChunkIdToMerkleHashConverter {
    fn convert(chunk_id: ChunkId) -> H256 {
        let chunk_id_biguint = BigUint::from(chunk_id.as_u64());
        let mut bytes = chunk_id_biguint.to_bytes_be();

        // Ensure the byte slice is exactly 32 bytes long by padding with leading zeros
        if bytes.len() < 32 {
            let mut padded_bytes = vec![0u8; 32 - bytes.len()];
            padded_bytes.extend(bytes);
            bytes = padded_bytes;
        }

        H256::from_slice(&bytes)
    }
}

// Converter from the ReplicationTargetType type to the Balance type.
pub struct ReplicationTargetToBalance;
impl Convert<ReplicationTargetType, Balance> for ReplicationTargetToBalance {
    fn convert(replication_target: ReplicationTargetType) -> Balance {
        replication_target.into()
    }
}

// Converter from the TickNumber type to the Balance type.
pub type TickNumber = BlockNumber;
pub struct TickNumberToBalance;
impl Convert<TickNumber, Balance> for TickNumberToBalance {
    fn convert(tick_number: TickNumber) -> Balance {
        tick_number.into()
    }
}

// Converter from the StorageDataUnit type to the Balance type.
pub struct StorageDataUnitToBalance;
impl Convert<StorageDataUnit, Balance> for StorageDataUnitToBalance {
    fn convert(storage_data_unit: StorageDataUnit) -> Balance {
        storage_data_unit.into()
    }
}

impl pallet_bucket_nfts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_bucket_nfts::weights::SubstrateWeight<Runtime>;
    type Buckets = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

/****** Commit-Reveal Randomness pallet ******/
/* pub type Seed = Hash;
pub type SeedCommitment = Hash;

impl pallet_cr_randomness::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type SeedCommitment = SeedCommitment;
    type Seed = Seed;
    type SeedVerifier = SeedVerifier;
    type SeedGenerator = SeedGenerator;
    type RandomSeedMixer = RandomSeedMixer;
    type MaxSeedTolerance = MaxSeedTolerance;
    type StakeToSeedPeriod = runtime_params::dynamic_params::runtime_config::StakeToSeedPeriod;
    type MinSeedPeriod = runtime_params::dynamic_params::runtime_config::MinSeedPeriod;
}

parameter_types! {
    pub const MaxSeedTolerance: u32 = 10;
}

// TODO: We should implement seed generation and verification with signatures instead of hashes,
// so that we can have a more secure and decentralized randomness generation.
pub struct SeedVerifier;
impl pallet_cr_randomness::SeedVerifier for SeedVerifier {
    type Seed = Seed;
    type SeedCommitment = SeedCommitment;
    fn verify(seed: &Self::Seed, seed_commitment: &Self::SeedCommitment) -> bool {
        BlakeTwo256::hash(seed.as_bytes()) == *seed_commitment
    }
}

pub struct SeedGenerator;
impl pallet_cr_randomness::SeedGenerator for SeedGenerator {
    type Seed = Seed;
    fn generate_seed(generator: &[u8]) -> Self::Seed {
        Hashing::hash(&generator)
    }
}

pub struct RandomSeedMixer;
impl pallet_cr_randomness::RandomSeedMixer<Seed> for RandomSeedMixer {
    fn mix_randomness_seed(seed_1: &Seed, seed_2: &Seed, context: Option<impl Into<Seed>>) -> Seed {
        let mut seed = seed_1.as_fixed_bytes().to_vec();
        seed.extend_from_slice(seed_2.as_fixed_bytes());
        if let Some(context) = context {
            seed.extend_from_slice(context.into().as_fixed_bytes());
        }
        Hashing::hash(&seed)
    }
} */

// TODO: Replace this mock with the actual implementation above when it is ready
// We need this mock since `pallet-file-system` requires something that implements the CommitRevealRandomnessInterface trait
pub struct MockCrRandomness;
impl shp_traits::CommitRevealRandomnessInterface for MockCrRandomness {
    type ProviderId = Hash;

    fn initialise_randomness_cycle(
        _who: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn stop_randomness_cycle(_who: &Self::ProviderId) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }
}
/****** ****** ****** ******/

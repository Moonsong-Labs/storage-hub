mod runtime_params;
pub mod xcm_config;

// Substrate and Polkadot dependencies
use core::marker::PhantomData;
use cumulus_pallet_parachain_system::{RelayChainStateProof, RelayNumberMonotonicallyIncreases};
use cumulus_primitives_core::{relay_chain::well_known_keys, AggregateMessageOrigin, ParaId};
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
use polkadot_runtime_common::{
    prod_or_fast, xcm_sender::NoPriceForMessageDelivery, BlockHashCount, SlowAdjustingFeeUpdate,
};
use shp_data_price_updater::{MostlyStablePriceIndexUpdater, MostlyStablePriceIndexUpdaterConfig};
use shp_file_key_verifier::FileKeyVerifier;
use shp_file_metadata::{ChunkId, FileMetadata};
use shp_forest_verifier::ForestVerifier;
use shp_traits::{CommitmentVerifier, MaybeDebug};
use shp_treasury_funding::{
    LinearThenPowerOfTwoTreasuryCutCalculator, LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{ConstU128, Get, Hasher, H256};
use sp_runtime::{
    traits::{BlakeTwo256, Convert, ConvertBack, Verify},
    AccountId32, DispatchError, Perbill, Perquintill, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_std::vec;
use sp_trie::{CompactProof, LayoutV1, TrieConfiguration, TrieLayout};
use sp_version::RuntimeVersion;
use xcm::latest::prelude::BodyId;

// Local module imports
use crate::{
    weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
    AccountId, Aura, Balance, Balances, Block, BlockNumber, BucketNfts, CollatorSelection, Hash,
    MessageQueue, Nfts, Nonce, PalletInfo, ParachainInfo, ParachainSystem, PaymentStreams,
    PolkadotXcm, ProofsDealer, Providers, Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason,
    RuntimeHoldReason, RuntimeOrigin, RuntimeTask, Session, SessionKeys, Signature, System,
    WeightToFee, XcmpQueue, AVERAGE_ON_INITIALIZE_RATIO, BLOCK_PROCESSING_VELOCITY, DAYS,
    EXISTENTIAL_DEPOSIT, HOURS, MAXIMUM_BLOCK_WEIGHT, MICROUNIT, MINUTES, NORMAL_DISPATCH_RATIO,
    RELAY_CHAIN_SLOT_DURATION_MILLIS, SLOT_DURATION, UNINCLUDED_SEGMENT_CAPACITY, UNIT, VERSION,
};
use runtime_params::RuntimeParameters;
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

pub type StorageProofsMerkleTrieLayout = LayoutV1<BlakeTwo256>;

/// Type representing the storage data units in StorageHub.
pub type StorageDataUnit = u64;

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;

    // This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
    //  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
    // `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parametrise
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
    type MinimumPeriod = ConstU64<0>;
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
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
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
    // TODO: Set appropiate weight limit
    // The maximum weight to be used from remaining weight for processing enqueued messages on idle
    // pub const IdleMaxServiceWeight: Weight = Some(Weight);
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
    type IdleMaxServiceWeight = (); // TODO: Set appropriate weight limit
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = PolkadotXcm;
    // Enqueue XCMP messages from siblings for later processing.
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = ConstU32<1_000>;
    type MaxActiveOutboundChannels = ConstU32<128>;
    type MaxPageSize = ConstU32<{ 1 << 16 }>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = ();
    type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
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

impl pallet_parameters::Config for Runtime {
    type AdminOrigin = EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeParameters = RuntimeParameters;
    type WeightInfo = ();
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
    pub const MaxBlocksForRandomness: BlockNumber = prod_or_fast!(2 * HOURS, 2 * MINUTES);
}

// TODO: If the next line is uncommented (which should be eventually), compilation breaks (most likely because of mismatched dependency issues)
/* parameter_types! {
    pub const MaxBlocksForRandomness: BlockNumber = prod_or_fast!(2 * runtime_constants::time::EPOCH_DURATION_IN_SLOTS, 2 * MINUTES);
} */

/// Configure the randomness pallet
impl pallet_randomness::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BabeDataGetter = BabeDataGetter;
    type WeightInfo = ();
}

parameter_types! {
    pub const SpMinDeposit: Balance = 100 * UNIT;
    pub const BucketDeposit: Balance = 100 * UNIT;
    pub const BspSignUpLockPeriod: BlockNumber = 90 * DAYS; // ~3 months
}

pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
    }
}

impl pallet_storage_providers::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ProvidersRandomness = pallet_randomness::RandomnessFromOneEpochAgo<Runtime>;
    type PaymentStreams = PaymentStreams;
    type FileMetadataManager = FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type NativeBalance = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageDataUnit = StorageDataUnit;
    type SpCount = u32;
    type MerklePatriciaRoot = Hash;
    type ValuePropId = Hash;
    type ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId;
    type ProvidersProofSubmitters = ProofsDealer;
    type ReputationWeightType = u32;
    type Treasury = TreasuryAccount;
    type SpMinDeposit = SpMinDeposit;
    type SpMinCapacity = ConstU64<2>;
    type DepositPerData = ConstU128<2>;
    type MaxFileSize = ConstU64<{ u64::MAX }>;
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
    type MaxProtocols = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type BucketDeposit = BucketDeposit;
    type BucketNameLimit = ConstU32<100>;
    type MaxBlocksForRandomness = MaxBlocksForRandomness;
    type MinBlocksBetweenCapacityChanges = ConstU32<10>;
    type DefaultMerkleRoot = DefaultMerkleRoot<StorageProofsMerkleTrieLayout>;
    type SlashAmountPerMaxFileSize =
        runtime_params::dynamic_params::runtime_config::SlashAmountPerMaxFileSize;
    type StartingReputationWeight = ConstU32<1>;
    type BspSignUpLockPeriod = BspSignUpLockPeriod;
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

impl LinearThenPowerOfTwoTreasuryCutCalculatorConfig for Runtime {
    type PercentageType = Perquintill;
    type ProvidedUnit = StorageDataUnit;
    type IdealUtilizationRate =
        runtime_params::dynamic_params::runtime_config::IdealUtilizationRate;
    type DecayRate = runtime_params::dynamic_params::runtime_config::DecayRate;
}

impl pallet_payment_streams::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type ProvidersPallet = Providers;
    type RuntimeHoldReason = RuntimeHoldReason;
    type UserWithoutFundsCooldown = UserWithoutFundsCooldown; // Amount of blocks that a user will have to wait before being able to clear the out of funds flag
    type NewStreamDeposit = ConstU32<10>; // Amount of blocks that the deposit of a new stream should be able to pay for
    type Units = StorageDataUnit; // Storage unit
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = ProofsDealer;
    type TreasuryCutCalculator = LinearThenPowerOfTwoTreasuryCutCalculator<Runtime>;
    type TreasuryAccount = TreasuryAccount;
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
        // TODO: Change this to the benchmarked weight of a `submit_proof` extrinsic or more.
        // TODO: Right now, it is set to the weight of a `transfer_keep_alive` extrinsic.
        Weight::from_parts(297_297_000, 308)
    }
}

pub struct MinNotFullBlocksRatio;
impl Get<Perbill> for MinNotFullBlocksRatio {
    fn get() -> Perbill {
        // This means that we tolerate at most 50% of misbehaving collators.
        Perbill::from_percent(50)
    }
}

parameter_types! {
    pub const RandomChallengesPerBlock: u32 = 10;
    pub const MaxCustomChallengesPerBlock: u32 = 10;
    pub const ChallengeHistoryLength: BlockNumber = 100;
    pub const ChallengesQueueLength: u32 = 100;
    pub const ChallengesFee: Balance = 1 * UNIT;
    pub const ChallengeTicksTolerance: u32 = 50;
    pub const MaxSubmittersPerTick: u32 = 1000; // TODO: Change this value after benchmarking for it to coincide with the implicit limit given by maximum block weight
    pub const TargetTicksStorageOfSubmitters: u32 = 3;
}

impl pallet_proofs_dealer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleTrieHash = Hash;
    type MerkleTrieHashing = BlakeTwo256;
    type ForestVerifier = ForestVerifier<StorageProofsMerkleTrieLayout, { BlakeTwo256::LENGTH }>;
    type KeyVerifier = FileKeyVerifier<
        StorageProofsMerkleTrieLayout,
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
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
    type Treasury = TreasuryAccount;
    type RandomnessProvider = pallet_randomness::ParentBlockRandomness<Runtime>;
    type StakeToChallengePeriod =
        runtime_params::dynamic_params::runtime_config::StakeToChallengePeriod;
    type MinChallengePeriod = runtime_params::dynamic_params::runtime_config::MinChallengePeriod;
    type ChallengeTicksTolerance = ChallengeTicksTolerance;
    type BlockFullnessPeriod = ChallengeTicksTolerance; // We purposely set this to `ChallengeTicksTolerance` so that spamming of the chain is evaluated for the same blocks as the tolerance BSPs are given.
    type BlockFullnessHeadroom = BlockFullnessHeadroom;
    type MinNotFullBlocksRatio = MinNotFullBlocksRatio;
}

/// Structure to mock a verifier that returns `true` when `proof` is not empty
/// and `false` otherwise.
pub struct MockVerifier<C> {
    _phantom: core::marker::PhantomData<C>,
}

/// Implement the `TrieVerifier` trait for the `MockVerifier` struct.
impl<C> CommitmentVerifier for MockVerifier<C>
where
    C: MaybeDebug + Ord + Default + Copy + AsRef<[u8]> + AsMut<[u8]>,
{
    type Proof = CompactProof;
    type Commitment = H256;
    type Challenge = C;

    fn verify_proof(
        _root: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &CompactProof,
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError> {
        if proof.encoded_nodes.len() > 0 {
            Ok(challenges.iter().cloned().collect())
        } else {
            Err("Proof is empty".into())
        }
    }
}

type ThresholdType = u32;

parameter_types! {
    pub const MinWaitForStopStoring: BlockNumber = 10;
}

impl MostlyStablePriceIndexUpdaterConfig for Runtime {
    type Price = Balance;
    type StorageDataUnit = StorageDataUnit;
    type LowerThreshold =
        runtime_params::dynamic_params::runtime_config::SystemUtilisationLowerThresholdPercentage;
    type UpperThreshold =
        runtime_params::dynamic_params::runtime_config::SystemUtilisationUpperThresholdPercentage;
    type MostlyStablePrice = runtime_params::dynamic_params::runtime_config::MostlyStablePrice;
    type MaxPrice = runtime_params::dynamic_params::runtime_config::MaxPrice;
    type MinPrice = runtime_params::dynamic_params::runtime_config::MinPrice;
    type UpperExponentFactor = runtime_params::dynamic_params::runtime_config::UpperExponentFactor;
    type LowerExponentFactor = runtime_params::dynamic_params::runtime_config::LowerExponentFactor;
}

/// Configure the pallet template in pallets/template.
impl pallet_file_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type PaymentStreams = PaymentStreams;
    type UpdateStoragePrice = MostlyStablePriceIndexUpdater<Runtime>;
    type UserSolvency = PaymentStreams;
    type Fingerprint = Hash;
    type ReplicationTargetType = u32;
    type ThresholdType = ThresholdType;
    type ThresholdTypeToTickNumber = ThresholdTypeToBlockNumberConverter;
    type HashToThresholdType = HashToThresholdTypeConverter;
    type MerkleHashToRandomnessOutput = MerkleHashToRandomnessOutputConverter;
    type ChunkIdToMerkleHash = ChunkIdToMerkleHashConverter;
    type Currency = Balances;
    type Nfts = Nfts;
    type CollectionInspector = BucketNfts;
    type MaxBspsPerStorageRequest = ConstU32<5>;
    type MaxBatchConfirmStorageRequests = ConstU32<10>;
    type MaxBatchMspRespondStorageRequests = ConstU32<10>;
    type MaxFilePathSize = ConstU32<512u32>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = ConstU32<5>;
    type MaxDataServerMultiAddresses = ConstU32<10>;
    type MaxExpiredItemsInBlock = ConstU32<100>;
    type StorageRequestTtl = ConstU32<40>;
    type PendingFileDeletionRequestTtl = ConstU32<40u32>;
    type MoveBucketRequestTtl = ConstU32<40u32>;
    type MaxUserPendingDeletionRequests = ConstU32<10u32>;
    type MaxUserPendingMoveBucketRequests = ConstU32<10u32>;
    type MinWaitForStopStoring = MinWaitForStopStoring;
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

impl pallet_bucket_nfts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Buckets = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

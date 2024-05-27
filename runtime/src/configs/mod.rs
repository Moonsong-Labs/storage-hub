// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org>

mod xcm_config;

// Substrate and Polkadot dependencies
use cumulus_pallet_parachain_system::{RelayChainStateProof, RelayNumberMonotonicallyIncreases};
use cumulus_primitives_core::{relay_chain::well_known_keys, AggregateMessageOrigin, ParaId};
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    parameter_types,
    traits::{ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse, TransformOrigin},
    weights::{ConstantMultiplier, Weight},
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    pallet_prelude::BlockNumberFor,
    EnsureRoot,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::{
    prod_or_fast, xcm_sender::NoPriceForMessageDelivery, BlockHashCount, SlowAdjustingFeeUpdate,
};
use shp_file_key_verifier::FileKeyVerifier;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{ConstU128, Get, Hasher, H256};
use sp_runtime::{
    traits::{BlakeTwo256, Convert},
    AccountId32, DispatchError, FixedU128, Perbill, SaturatedConversion,
};
use sp_std::vec::Vec;
use sp_trie::CompactProof;
use sp_trie::LayoutV1;
use sp_version::RuntimeVersion;
use storage_hub_primitives::TrieVerifier;
use storage_hub_traits::{CommitmentVerifier, MaybeDebug};
use xcm::latest::prelude::BodyId;

use crate::{ParachainInfo, FILE_CHUNK_SIZE, FILE_SIZE_TO_CHALLENGES};

use self::currency::UNITS;

// Local module imports
use super::{
    weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
    AccountId, Aura, Balance, Balances, Block, BlockNumber, CollatorSelection, FileSystem, Hash,
    MessageQueue, Nonce, PalletInfo, ParachainSystem, ProofsDealer, Providers, Runtime,
    RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask,
    Session, SessionKeys, System, WeightToFee, XcmpQueue, AVERAGE_ON_INITIALIZE_RATIO,
    BLOCK_PROCESSING_VELOCITY, EXISTENTIAL_DEPOSIT, HOURS, MAXIMUM_BLOCK_WEIGHT, MICROUNIT,
    MINUTES, NORMAL_DISPATCH_RATIO, RELAY_CHAIN_SLOT_DURATION_MILLIS, SLOT_DURATION,
    UNINCLUDED_SEGMENT_CAPACITY, VERSION,
};
use xcm_config::{RelayLocation, XcmOriginToTransactDispatchOrigin};

pub mod currency {
    use crate::Balance;

    pub const UNITS: Balance = 1_000_000_000_000;
}

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
    #[cfg(feature = "experimental")]
    type MinimumPeriod = ConstU64<0>;
    #[cfg(not(feature = "experimental"))]
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
}

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
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
    type HeapSize = sp_core::ConstU32<{ 64 * 1024 }>;
    type MaxStale = sp_core::ConstU32<8>;
    type ServiceWeight = MessageQueueServiceWeight;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = ();
    // Enqueue XCMP messages from siblings for later processing.
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = sp_core::ConstU32<1_000>;
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
    #[cfg(feature = "experimental")]
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

/// Only callable after `set_validation_data` is called which forms this proof the same way
fn relay_chain_state_proof() -> RelayChainStateProof {
    let relay_storage_root = ParachainSystem::validation_data()
        .expect("set in `set_validation_data`")
        .relay_parent_storage_root;
    let relay_chain_state =
        ParachainSystem::relay_state_proof().expect("set in `set_validation_data`");
    RelayChainStateProof::new(ParachainInfo::get(), relay_storage_root, relay_chain_state)
        .expect("Invalid relay chain state proof, already constructed in `set_validation_data`")
}

pub struct BabeDataGetter;
impl pallet_randomness::GetBabeData<u64, Option<Hash>> for BabeDataGetter {
    // Tolerate panic here because this is only ever called in an inherent (so can be omitted)
    fn get_epoch_index() -> u64 {
        if cfg!(feature = "runtime-benchmarks") {
            // storage reads as per actual reads
            let _relay_storage_root = ParachainSystem::validation_data();
            let _relay_chain_state = ParachainSystem::relay_state_proof();
            const BENCHMARKING_NEW_EPOCH: u64 = 10u64;
            return BENCHMARKING_NEW_EPOCH;
        }
        relay_chain_state_proof()
            .read_optional_entry(well_known_keys::EPOCH_INDEX)
            .ok()
            .flatten()
            .expect("expected to be able to read epoch index from relay chain state proof")
    }
    fn get_epoch_randomness() -> Option<Hash> {
        if cfg!(feature = "runtime-benchmarks") {
            // storage reads as per actual reads
            let _relay_storage_root = ParachainSystem::validation_data();
            let _relay_chain_state = ParachainSystem::relay_state_proof();
            let benchmarking_babe_output = Hash::default();
            return Some(benchmarking_babe_output);
        }
        relay_chain_state_proof()
            .read_optional_entry(well_known_keys::ONE_EPOCH_AGO_RANDOMNESS)
            .ok()
            .flatten()
    }
    fn get_parent_randomness() -> Option<Hash> {
        if cfg!(feature = "runtime-benchmarks") {
            // storage reads as per actual reads
            let _relay_storage_root = ParachainSystem::validation_data();
            let _relay_chain_state = ParachainSystem::relay_state_proof();
            let benchmarking_babe_output = Hash::default();
            return Some(benchmarking_babe_output);
        }
        // Note: we use the `CURRENT_BLOCK_RANDOMNESS` key here as it also represents the parent randomness, the only difference
        // is the block since this randomness is valid, but we don't care about that because we are setting that directly in the `randomness` pallet.
        relay_chain_state_proof()
            .read_optional_entry(well_known_keys::CURRENT_BLOCK_RANDOMNESS)
            .ok()
            .flatten()
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

impl pallet_storage_providers::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type StorageData = u32;
    type SpCount = u32;
    type MerklePatriciaRoot = Hash;
    type ValuePropId = Hash;
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
    type MaxProtocols = ConstU32<100>;
    type MaxBsps = ConstU32<100>;
    type MaxMsps = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Subscribers = FileSystem;
    type ProvidersRandomness = pallet_randomness::RandomnessFromOneEpochAgo<Runtime>;
    type MaxBlocksForRandomness = MaxBlocksForRandomness;
    type MinBlocksBetweenCapacityChanges = ConstU32<10>;
}

// TODO: remove this and replace with pallet treasury
pub struct TreasuryAccount;
impl Get<AccountId32> for TreasuryAccount {
    fn get() -> AccountId32 {
        AccountId32::from([0; 32])
    }
}

parameter_types! {
    pub const RandomChallengesPerBlock: u32 = 10;
    pub const MaxCustomChallengesPerBlock: u32 = 10;
    pub const MaxProvidersChallengedPerBlock: u32 = 100;
    pub const ChallengeHistoryLength: BlockNumber = 100;
    pub const ChallengesQueueLength: u32 = 100;
    pub const CheckpointChallengePeriod: u32 = 10;
    pub const ChallengesFee: Balance = 1 * UNITS;
    pub const StakeToChallengePeriod: Balance = 10 * UNITS;
}

impl pallet_proofs_dealer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleTrieHash = Hash;
    type MerkleTrieHashing = BlakeTwo256;
    type ForestVerifier = TrieVerifier<LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type KeyVerifier = FileKeyVerifier<
        LayoutV1<BlakeTwo256>,
        { BlakeTwo256::LENGTH },
        { FILE_CHUNK_SIZE },
        { FILE_SIZE_TO_CHALLENGES },
    >;
    type StakeToBlockNumber = SaturatingBalanceToBlockNumber;
    type RandomChallengesPerBlock = RandomChallengesPerBlock;
    type MaxCustomChallengesPerBlock = MaxCustomChallengesPerBlock;
    type MaxProvidersChallengedPerBlock = MaxProvidersChallengedPerBlock;
    type ChallengeHistoryLength = ChallengeHistoryLength;
    type ChallengesQueueLength = ChallengesQueueLength;
    type CheckpointChallengePeriod = CheckpointChallengePeriod;
    type ChallengesFee = ChallengesFee;
    type Treasury = TreasuryAccount;
    type RandomnessProvider = pallet_randomness::ParentBlockRandomness<Runtime>;
    type StakeToChallengePeriod = StakeToChallengePeriod;
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
    ) -> Result<Vec<Self::Challenge>, DispatchError> {
        if proof.encoded_nodes.len() > 0 {
            Ok(challenges.to_vec())
        } else {
            Err("Proof is empty".into())
        }
    }
}

type ThresholdType = FixedU128;

parameter_types! {
    pub const ThresholdAsymptoticDecayFactor: FixedU128 = FixedU128::from_rational(1, 2); // 0.5
    pub const ThresholdAsymptote: FixedU128 = FixedU128::from_rational(100, 1); // 100
    pub const ThresholdMultiplier: FixedU128 = FixedU128::from_rational(100, 1); // 100
}

/// Configure the pallet template in pallets/template.
impl pallet_file_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type ThresholdType = ThresholdType;
    type AssignmentThresholdDecayFactor = ThresholdAsymptoticDecayFactor;
    type AssignmentThresholdAsymptote = ThresholdAsymptote;
    type AssignmentThresholdMultiplier = ThresholdMultiplier;
    type Fingerprint = Hash;
    type StorageRequestBspsRequiredType = u32;
    type TargetBspsRequired = ConstU32<1>;
    type MaxBspsPerStorageRequest = ConstU32<5>;
    type MaxFilePathSize = ConstU32<512u32>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = ConstU32<5>;
    type MaxDataServerMultiAddresses = ConstU32<10>;
    type StorageRequestTtl = ConstU32<40>;
    type MaxExpiredStorageRequests = ConstU32<100>;
}

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;

impl Convert<Balance, BlockNumberFor<Runtime>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Runtime> {
        block_number.saturated_into()
    }
}

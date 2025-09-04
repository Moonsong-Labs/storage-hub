mod runtime_params;

#[cfg(feature = "std")]
pub mod storage_hub;

// Substrate and Polkadot dependencies
use core::marker::PhantomData;
use fp_account::AccountId20;
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    parameter_types,
    traits::{
        fungible::{Balanced, Credit, Inspect},
        tokens::imbalance::ResolveTo,
        AsEnsureOriginWithArg, ConstU32, ConstU64, ConstU8, FindAuthor, KeyOwnerProofSystem,
        OnUnbalanced, Randomness, TypedGet, VariantCountOf,
    },
    weights::Weight,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    pallet_prelude::BlockNumberFor,
    EnsureRoot, EnsureSigned,
};
use num_bigint::BigUint;
use pallet_ethereum::PostLogContent;
use pallet_evm::{
    EVMFungibleAdapter, EnsureAddressNever, EnsureAddressRoot, FeeCalculator,
    FrameSystemAccountProvider, IdentityAddressMapping,
    OnChargeEVMTransaction as OnChargeEVMTransactionT,
};
use pallet_grandpa::AuthorityId as GrandpaId;
use pallet_nfts::PalletFeatures;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use polkadot_primitives::Moment;
use polkadot_runtime_common::{prod_or_fast, BlockHashCount};
use shp_data_price_updater::{MostlyStablePriceIndexUpdater, MostlyStablePriceIndexUpdaterConfig};
use shp_file_key_verifier::FileKeyVerifier;
use shp_file_metadata::{ChunkId, FileMetadata};
use shp_forest_verifier::ForestVerifier;
use shp_treasury_funding::{
    LinearThenPowerOfTwoTreasuryCutCalculator, LinearThenPowerOfTwoTreasuryCutCalculatorConfig,
};
use shp_types::{Hash, Hashing, StorageDataUnit, StorageProofsMerkleTrieLayout};
use sp_arithmetic::traits::One;
use sp_core::{ecdsa, ConstU128, Get, Hasher, H160, H256, U256};
use sp_runtime::{
    traits::{
        BlakeTwo256, Convert, ConvertBack, ConvertInto, IdentityLookup, OpaqueKeys,
        UniqueSaturatedInto, Verify, Zero,
    },
    FixedPointNumber, KeyTypeId, Perbill, SaturatedConversion,
};
use sp_staking::{EraIndex, SessionIndex};
use sp_std::vec;
use sp_std::vec::Vec;
use sp_trie::{TrieConfiguration, TrieLayout};
use sp_version::RuntimeVersion;
#[cfg(not(feature = "runtime-benchmarks"))]
use sp_weights::IdentityFee;
use sp_weights::RuntimeDbWeight;

// Local module imports
use crate::{
    currency::WEIGHT_FEE,
    gas::WEIGHT_PER_GAS,
    genesis_config_presets::get_account_id_from_seed,
    weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
    AccountId, Babe, Balance, Balances, Block, BlockNumber, BucketNfts, EvmChainId, Historical,
    Nfts, Nonce, PalletInfo, PaymentStreams, ProofsDealer, Providers, Runtime, RuntimeCall,
    RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask, Session,
    SessionKeys, Signature, System, Timestamp, TransactionPayment, WeightToFee,
    AVERAGE_ON_INITIALIZE_RATIO, DAYS, EXISTENTIAL_DEPOSIT, HOURS, MAXIMUM_BLOCK_WEIGHT, MICROUNIT,
    MINUTES, NORMAL_DISPATCH_RATIO, SLOT_DURATION, UNIT, VERSION,
};
use runtime_params::RuntimeParameters;

/// Time and blocks.
pub mod time {
    use polkadot_primitives::{BlockNumber, Moment, SessionIndex};
    use polkadot_runtime_common::prod_or_fast;

    pub const MILLISECS_PER_BLOCK: Moment = 6000;
    pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

    const ONE_HOUR: BlockNumber = HOURS;
    const ONE_MINUTE: BlockNumber = MINUTES;

    frame_support::parameter_types! {
        pub const EpochDurationInBlocks: BlockNumber = prod_or_fast!(ONE_HOUR, ONE_MINUTE);
        pub const SessionsPerEra: SessionIndex = prod_or_fast!(6, 3);
    }

    // These time units are defined in number of blocks.
    pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
    pub const HOURS: BlockNumber = MINUTES * 60;
    pub const DAYS: BlockNumber = HOURS * 24;
    pub const WEEKS: BlockNumber = DAYS * 7;
}

use crate::configs::time::{EpochDurationInBlocks, MILLISECS_PER_BLOCK};

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                             COMMON PARAMETERS                                                 ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

parameter_types! {
    pub const MaxAuthorities: u32 = 32;
    pub const BondingDuration: EraIndex = polkadot_runtime_common::prod_or_fast!(28, 3);
    pub const SessionsPerEra: SessionIndex = polkadot_runtime_common::prod_or_fast!(6, 1);
    pub const AuthorRewardPoints: u32 = 20;
}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                      SYSTEM AND CONSENSUS PALLETS                                             ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
// TODO move from here to where it is used
pub struct TreasuryAccount;
impl Get<AccountId20> for TreasuryAccount {
    fn get() -> AccountId20 {
        AccountId20::from([0; 20])
    }
}

impl TypedGet for TreasuryAccount {
    type Type = AccountId20;
    fn get() -> Self::Type {
        AccountId20::from([0; 20])
    }
}

// TODO move from here to where it is used
parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

/****** FRAME System ******/
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
/// [`SoloChainDefaultConfig`](`struct@frame_system::config_preludes::SolochainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
    /// The block type for the runtime.
    type Block = Block;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = IdentityLookup<AccountId>;
    /// The type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    type MaxConsumers = ConstU32<16>;
    type SystemWeightInfo = ();
}

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
    };

parameter_types! {
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
    pub ReportLongevity: u64 =
        BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * (EpochDurationInBlocks::get() as u64);
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDurationInBlocks;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<0>;

    type KeyOwnerProof =
        <Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

    type EquivocationReportSystem = ();
    // pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Babe;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type EventHandler = ();
}

impl pallet_offences::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = ();
}

pub struct FullIdentificationOf;
impl Convert<AccountId, Option<()>> for FullIdentificationOf {
    fn convert(_: AccountId) -> Option<()> {
        Some(())
    }
}

pub fn get_validators() -> Option<Vec<AccountId>> {
    Some(vec![
        get_account_id_from_seed::<ecdsa::Public>("Alice"),
        get_account_id_from_seed::<ecdsa::Public>("Bob"),
    ])
}

pub struct NoChangesSessionManager;
impl pallet_session::SessionManager<AccountId> for NoChangesSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<AccountId>> {
        get_validators()
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::SessionManager<AccountId, ()> for NoChangesSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<(AccountId, ())>> {
        get_validators().map(|validators| validators.iter().map(|v| (*v, ())).collect())
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = ();
    type FullIdentificationOf = FullIdentificationOf;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = NoChangesSessionManager;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = ();
}

parameter_types! {
    pub const EquivocationReportPeriodInEpochs: u64 = 168;
    pub const EquivocationReportPeriodInBlocks: u64 =
        EquivocationReportPeriodInEpochs::get() * (EpochDurationInBlocks::get() as u64);
    pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;

    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<0>;
    type MaxSetIdSessionEntries = MaxSetIdSessionEntries;

    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
    type EquivocationReportSystem = ();
    // pallet_grandpa::EquivocationReportSystem<
    //     Self,
    //     Offences,
    //     Historical,
    //     EquivocationReportPeriodInBlocks,
    // >;
}

/// Deal with substrate based fees and tip. This should be used with pallet_transaction_payment.
pub struct DealWithSubstrateFeesAndTip<R, FeesTreasuryProportion>(
    sp_std::marker::PhantomData<(R, FeesTreasuryProportion)>,
);
impl<R, FeesTreasuryProportion> DealWithSubstrateFeesAndTip<R, FeesTreasuryProportion>
where
    R: pallet_balances::Config + pallet_authorship::Config + frame_system::Config,
    R::AccountId: Default,
    FeesTreasuryProportion: Get<Perbill>,
{
    fn deal_with_fees(_amount: Credit<R::AccountId, pallet_balances::Pallet<R>>) {
        // TODO: This has compile errors, ignoring for now as not too relevant.
        // // Balances pallet automatically burns dropped Credits by decreasing
        // // total_supply accordingly
        // let treasury_proportion = FeesTreasuryProportion::get();
        // let treasury_part = treasury_proportion.deconstruct();
        // let burn_part = Perbill::one().deconstruct() - treasury_part;
        // let (_, to_treasury) = amount.ration(burn_part, treasury_part);
        // ResolveTo::<TreasuryAccount, pallet_balances::Pallet<R>>::on_unbalanced(to_treasury);
    }

    fn deal_with_tip(amount: Credit<R::AccountId, pallet_balances::Pallet<R>>) {
        ResolveTo::<BlockAuthorAccountId<R>, pallet_balances::Pallet<R>>::on_unbalanced(amount);
    }
}
impl<R, FeesTreasuryProportion> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>>
    for DealWithSubstrateFeesAndTip<R, FeesTreasuryProportion>
where
    R: pallet_balances::Config + pallet_authorship::Config + frame_system::Config,
    R::AccountId: Default,
    FeesTreasuryProportion: Get<Perbill>,
{
    fn on_unbalanceds(
        mut fees_then_tips: impl Iterator<Item = Credit<R::AccountId, pallet_balances::Pallet<R>>>,
    ) {
        if let Some(fees) = fees_then_tips.next() {
            Self::deal_with_fees(fees);
            if let Some(tip) = fees_then_tips.next() {
                Self::deal_with_tip(tip);
            }
        }
    }
}

pub struct BlockAuthorAccountId<R>(sp_std::marker::PhantomData<R>);
impl<R> TypedGet for BlockAuthorAccountId<R>
where
    R: frame_system::Config + pallet_authorship::Config,
    R::AccountId: Default,
{
    type Type = R::AccountId;
    fn get() -> Self::Type {
        <pallet_authorship::Pallet<R>>::author().unwrap_or_default()
    }
}

parameter_types! {
    pub FeeMultiplier: Multiplier = Multiplier::one();
    pub FeesTreasuryProportion: Perbill = Perbill::from_percent(20);
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction =
        FungibleAdapter<Balances, DealWithSubstrateFeesAndTip<Runtime, FeesTreasuryProportion>>;
    type OperationalFeeMultiplier = ConstU8<5>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type WeightToFee = IdentityFee<Balance>;
    #[cfg(feature = "runtime-benchmarks")]
    type WeightToFee = benchmark_helpers::BenchmarkWeightToFee;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type LengthToFee = IdentityFee<Balance>;
    #[cfg(feature = "runtime-benchmarks")]
    type LengthToFee = benchmark_helpers::BenchmarkWeightToFee;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type WeightInfo = ();
}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                    POLKADOT SDK UTILITY PALLETS                                               ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

impl pallet_parameters::Config for Runtime {
    type AdminOrigin = EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeParameters = RuntimeParameters;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

parameter_types! {
    pub const CollectionDeposit: Balance = 100 * UNIT;
    pub const ItemDeposit: Balance = 1 * UNIT;
    pub const MetadataDepositBase: Balance = 10 * UNIT;
    pub const MetadataDepositPerByte: Balance = 1 * UNIT;
    pub const ApprovalsLimit: u32 = 20;
    pub const ItemAttributesApprovalsLimit: u32 = 20;
    pub const MaxTips: u32 = 10;
    pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
    pub const MaxAttributesPerCall: u32 = 10;
    pub Features: PalletFeatures = PalletFeatures::all_enabled();
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
    type Helper = benchmark_helpers::NftHelper;
    type Locker = ();
}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                        FRONTIER (EVM) PALLETS                                                 ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

parameter_types! {
    pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type StateRoot = pallet_ethereum::IntermediateStateRoot<Self::Version>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

// Ported from Moonbeam, please check for reference: https://github.com/moonbeam-foundation/moonbeam/pull/1765
pub struct TransactionPaymentAsGasPrice;
impl FeeCalculator for TransactionPaymentAsGasPrice {
    fn min_gas_price() -> (U256, Weight) {
        // note: transaction-payment differs from EIP-1559 in that its tip and length fees are not
        //       scaled by the multiplier, which means its multiplier will be overstated when
        //       applied to an ethereum transaction
        // note: transaction-payment uses both a congestion modifier (next_fee_multiplier, which is
        //       updated once per block in on_finalize) and a 'WeightToFee' implementation. Our
        //       runtime implements this as a 'ConstantModifier', so we can get away with a simple
        //       multiplication here.
        let min_gas_price: u128 = TransactionPayment::next_fee_multiplier()
            .saturating_mul_int((WEIGHT_FEE).saturating_mul(WEIGHT_PER_GAS as u128));
        (
            min_gas_price.into(),
            <<Runtime as frame_system::Config>::DbWeight as Get<RuntimeDbWeight>>::get().reads(1),
        )
    }
}

pub struct FindAuthorAdapter<T>(core::marker::PhantomData<T>);
impl<T> FindAuthor<H160> for FindAuthorAdapter<T>
where
    T: frame_system::Config + pallet_session::Config,
    <T as pallet_session::Config>::ValidatorId: Into<H160>,
{
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (sp_runtime::ConsensusEngineId, &'a [u8])>,
    {
        pallet_session::FindAccountFromAuthorIndex::<T, Babe>::find_author(digests)
            .map(|author| author.into())
    }
}

pub struct OnChargeEVMTransaction<BaseFeesOU, PriorityFeesOU>(
    sp_std::marker::PhantomData<(BaseFeesOU, PriorityFeesOU)>,
);

impl<T, BaseFeesOU, PriorityFeesOU> OnChargeEVMTransactionT<T>
    for OnChargeEVMTransaction<BaseFeesOU, PriorityFeesOU>
where
    T: pallet_evm::Config,
    T::Currency: Balanced<pallet_evm::AccountIdOf<T>>,
    BaseFeesOU: OnUnbalanced<Credit<pallet_evm::AccountIdOf<T>, T::Currency>>,
    PriorityFeesOU: OnUnbalanced<Credit<pallet_evm::AccountIdOf<T>, T::Currency>>,
    U256: UniqueSaturatedInto<<T::Currency as Inspect<pallet_evm::AccountIdOf<T>>>::Balance>,
    T::AddressMapping: pallet_evm::AddressMapping<T::AccountId>,
{
    type LiquidityInfo = Option<Credit<pallet_evm::AccountIdOf<T>, T::Currency>>;

    fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, pallet_evm::Error<T>> {
        EVMFungibleAdapter::<<T as pallet_evm::Config>::Currency, ()>::withdraw_fee(who, fee)
    }

    fn correct_and_deposit_fee(
        who: &H160,
        corrected_fee: U256,
        base_fee: U256,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Self::LiquidityInfo {
        <EVMFungibleAdapter<<T as pallet_evm::Config>::Currency, BaseFeesOU> as OnChargeEVMTransactionT<
            T,
        >>::correct_and_deposit_fee(who, corrected_fee, base_fee, already_withdrawn)
    }

    fn pay_priority_fee(tip: Self::LiquidityInfo) {
        if let Some(tip) = tip {
            PriorityFeesOU::on_unbalanced(tip);
        }
    }
}

/// Deal with ethereum based fees. To handle tips/priority fees, use DealWithEthereumPriorityFees.
pub struct DealWithEthereumBaseFees<R, FeesTreasuryProportion>(
    sp_std::marker::PhantomData<(R, FeesTreasuryProportion)>,
);
impl<R, FeesTreasuryProportion> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>>
    for DealWithEthereumBaseFees<R, FeesTreasuryProportion>
where
    R: pallet_balances::Config,
    FeesTreasuryProportion: Get<Perbill>,
{
    fn on_nonzero_unbalanced(_amount: Credit<R::AccountId, pallet_balances::Pallet<R>>) {
        // TODO: This has compile errors, ignoring for now as not too relevant.
        // // Balances pallet automatically burns dropped Credits by decreasing
        // // total_supply accordingly
        // let treasury_proportion = FeesTreasuryProportion::get();
        // let treasury_part = treasury_proportion.deconstruct();
        // let burn_part = Perbill::one().deconstruct() - treasury_part;
        // let (_, to_treasury) = amount.ration(burn_part, treasury_part);
        // ResolveTo::<TreasuryAccount, pallet_balances::Pallet<R>>::on_unbalanced(to_treasury);
    }
}

/// Deal with ethereum based priority fees/tips. See DealWithEthereumBaseFees for base fees.
pub struct DealWithEthereumPriorityFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>>
    for DealWithEthereumPriorityFees<R>
where
    R: pallet_balances::Config + pallet_authorship::Config + frame_system::Config,
    R::AccountId: Default,
{
    fn on_nonzero_unbalanced(_amount: Credit<R::AccountId, pallet_balances::Pallet<R>>) {
        // TODO: This has compile errors, ignoring for now as not too relevant.
        // ResolveTo::<BlockAuthorAccountId<R>, pallet_balances::Pallet<R>>::on_unbalanced(amount);
    }
}

parameter_types! {
    pub BlockGasLimit: U256
        = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS);
    // pub PrecompilesValue: TemplatePrecompiles<Runtime> = TemplatePrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
    pub SuicideQuickClearLimit: u32 = 0;
    /// The amount of gas per pov. A ratio of 16 if we convert ref_time to gas and we compare
    /// it with the pov_size for a block. E.g.
    /// ceil(
    ///     (max_extrinsic.ref_time() / max_extrinsic.proof_size()) / WEIGHT_PER_GAS
    /// )
    /// We should re-check `xcm_config::Erc20XcmBridgeTransferGasLimit` when changing this value
    pub const GasLimitPovSizeRatio: u64 = 16;
    /// The amount of gas per storage (in bytes): BLOCK_GAS_LIMIT / BLOCK_STORAGE_LIMIT
    /// (60_000_000 / 160 kb)
    pub GasLimitStorageGrowthRatio: u64 = 366;
}

impl pallet_evm::Config for Runtime {
    type AccountProvider = FrameSystemAccountProvider<Runtime>;
    type FeeCalculator = TransactionPaymentAsGasPrice;
    type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
    type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
    type CallOrigin = EnsureAddressRoot<AccountId>;
    type WithdrawOrigin = EnsureAddressNever<AccountId>;
    type AddressMapping = IdentityAddressMapping;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    type PrecompilesType = ();
    type PrecompilesValue = ();
    type ChainId = EvmChainId;
    type BlockGasLimit = BlockGasLimit;
    type Runner = pallet_evm::runner::stack::Runner<Self>;
    type OnChargeTransaction = OnChargeEVMTransaction<
        DealWithEthereumBaseFees<Runtime, FeesTreasuryProportion>,
        DealWithEthereumPriorityFees<Runtime>,
    >;
    type OnCreate = ();
    type FindAuthor = FindAuthorAdapter<Self>;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type GasLimitStorageGrowthRatio = GasLimitStorageGrowthRatio;
    type Timestamp = Timestamp;
    type WeightInfo = ();
}

impl pallet_evm_chain_id::Config for Runtime {}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                        STORAGEHUB PALLETS                                                     ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
parameter_types! {
    pub const SpMinDeposit: Balance = 100 * UNIT;
    pub const BucketDeposit: Balance = 100 * UNIT;
    pub const BspSignUpLockPeriod: BlockNumber = 90 * DAYS; // ~3 months
    pub const MaxBlocksForRandomness: BlockNumber = prod_or_fast!(2 * HOURS, 2 * MINUTES);
    // TODO: If the next line is uncommented (which should be eventually, replacing the line above), compilation breaks (most likely because of mismatched dependency issues)
    // pub const MaxBlocksForRandomness: BlockNumber = prod_or_fast!(2 * runtime_constants::time::EPOCH_DURATION_IN_SLOTS, 2 * MINUTES);
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

pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
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
    type FileMetadataManager = FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type NativeBalance = Balances;
    type CrRandomness = MockCrRandomness;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageDataUnit = StorageDataUnit;
    type StorageDataUnitAndBalanceConvert = StorageDataUnitAndBalanceConverter;
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
    type Treasury = TreasuryAccount;
    type SpMinDeposit = SpMinDeposit;
    type SpMinCapacity = ConstU64<2>;
    type DepositPerData = ConstU128<2>;
    type MaxFileSize = ConstU64<{ u64::MAX }>;
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
    type MaxProtocols = ConstU32<100>;
    type BucketDeposit = BucketDeposit;
    type BucketNameLimit = ConstU32<100>;
    type MaxBlocksForRandomness = MaxBlocksForRandomness;
    type MinBlocksBetweenCapacityChanges = ConstU32<10>;
    type DefaultMerkleRoot = DefaultMerkleRoot<StorageProofsMerkleTrieLayout>;
    type SlashAmountPerMaxFileSize =
        runtime_params::dynamic_params::runtime_config::SlashAmountPerMaxFileSize;
    type StartingReputationWeight = ConstU32<1>;
    type BspSignUpLockPeriod = BspSignUpLockPeriod;
    type MaxCommitmentSize = ConstU32<1000>;
    type ZeroSizeBucketFixedRate =
        runtime_params::dynamic_params::runtime_config::ZeroSizeBucketFixedRate;
    type ProviderTopUpTtl = runtime_params::dynamic_params::runtime_config::ProviderTopUpTtl;
    type MaxExpiredItemsInBlock = ConstU32<100>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelpers = ProvidersBenchmarkHelpers;
}

type ThresholdType = u32;
pub type ReplicationTargetType = u32;

parameter_types! {
    pub const BaseStorageRequestCreationDeposit: Balance = 1 * UNIT;
    pub const FileDeletionRequestCreationDeposit: Balance = 1 * UNIT;
    pub const FileSystemStorageRequestCreationHoldReason: RuntimeHoldReason = RuntimeHoldReason::FileSystem(pallet_file_system::HoldReason::StorageRequestCreationHold);
    pub const FileSystemFileDeletionRequestHoldReason: RuntimeHoldReason = RuntimeHoldReason::FileSystem(pallet_file_system::HoldReason::FileDeletionRequestHold);
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

impl pallet_file_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_file_system::weights::SubstrateWeight<Runtime>;
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type PaymentStreams = PaymentStreams;
    // TODO: Replace the mocked CR randomness with the actual one when it's ready
    // type CrRandomness = CrRandomness;
    type CrRandomness = MockCrRandomness;
    type UpdateStoragePrice = MostlyStablePriceIndexUpdater<Runtime>;
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
    type BspStopStoringFilePenalty =
        runtime_params::dynamic_params::runtime_config::BspStopStoringFilePenalty;
    type TreasuryAccount = TreasuryAccount;
    type MaxBatchConfirmStorageRequests = ConstU32<10>;
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
    pub const ChallengesFee: Balance = 0;
    pub const PriorityChallengesFee: Balance = 0;
    pub const ChallengeTicksTolerance: u32 = 50;
}

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;
impl Convert<Balance, BlockNumberFor<Runtime>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Runtime> {
        block_number.saturated_into()
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
        //   ref_time: 2,980,252,675
        //   proof_size: 16,056
        // }
        let min_weight_for_submit_proof =
            <pallet_proofs_dealer::weights::SubstrateWeight<Runtime> as pallet_proofs_dealer::weights::WeightInfo>::submit_proof_no_checkpoint_challenges_key_proofs(1);

        // Calculate the maximum number of submit proofs that is possible to have in a block/tick.
        // With the current values, this would be:
        //
        // TODO: UPDATE THIS WITH THE FINAL BENCHMARKING
        // 244 proof submissions per block (limited by `proof_size`)
        let max_proof_submissions_per_tick = max_weight_for_class
            .checked_div_per_component(&min_weight_for_submit_proof)
            .unwrap_or(0);

        // Saturating u64 to u32 should be enough.
        max_proof_submissions_per_tick.saturated_into()
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

pub struct MaxSlashableProvidersPerTick;
impl Get<u32> for MaxSlashableProvidersPerTick {
    fn get() -> u32 {
        // With the maximum number of slashable providers per tick being `N`, the absolute maximum
        // weight that the `on_poll` hook can have, with the current benchmarking, is:
        //
        // TODO: UPDATE THIS WITH THE FINAL BENCHMARKING
        // new_challenges_round_weight: {
        //   ref_time: 576,000,000 + N * 551,601,146
        //   proof_size: 8,523 + N * 3,158
        // }
        // new_checkpoint_challenge_round_max_weight: {
        //   ref_time: 587,205,208 + ChallengesQueueLength * 225,083 = 610,554,678
        //   proof_size: 4,787
        // }
        // check_spamming_condition_weight: {
        //   ref_time: 313,000,000
        //   proof_size: 6,012
        // }
        //
        // For `N` = 1000, this would be:
        // max_on_poll_weight: {
        //   ref_time: 313,000,000 + 610,554,678 + 576,000,000 + N * 551,601,146 ≈ 553,100,700,678
        //   proof_size: 6,012 + 4,787 + 8,523 + N * 3,158 ≈ 3,177,322
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

impl pallet_proofs_dealer::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_proofs_dealer::weights::SubstrateWeight<Runtime>;
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
    #[cfg(feature = "runtime-benchmarks")]
    type RandomChallengesPerBlock = ConstU32<0>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type RandomChallengesPerBlock = RandomChallengesPerBlock;
    #[cfg(feature = "runtime-benchmarks")]
    type MaxCustomChallengesPerBlock = TotalMaxChallengesPerBlock;
    #[cfg(not(feature = "runtime-benchmarks"))]
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
    type ChallengeOrigin = EnsureRoot<AccountId>;
    type PriorityChallengeOrigin = EnsureRoot<AccountId>;
}

pub struct BlockNumberGetter {}
impl sp_runtime::traits::BlockNumberProvider for BlockNumberGetter {
    type BlockNumber = BlockNumberFor<Runtime>;

    fn current_block_number() -> Self::BlockNumber {
        frame_system::Pallet::<Runtime>::block_number()
    }
}

pub struct BabeDataGetter;
impl pallet_randomness::GetBabeData<u64, Hash> for BabeDataGetter {
    fn get_epoch_index() -> u64 {
        pallet_babe::Pallet::<Runtime>::epoch_index()
    }
    fn get_epoch_randomness() -> Hash {
        // We use `RandomnessFromOneEpochAgo` implementation of the `Randomness` trait here, which hashes the `NextRandomness`
        // stored by the BABE pallet, and is valid for commitments until the last block of the last epoch (`_n`). The hashed
        // received is the hash of `NextRandomness` concatenated with the `subject` parameter provided (in this case empty).
        let (h, _n) = pallet_babe::RandomnessFromOneEpochAgo::<Runtime>::random(b"");
        h
    }
    fn get_parent_randomness() -> Hash {
        // We use `ParentBlockRandomness` implementation of the `Randomness` trait here, which hashes the `AuthorVrfRandomness`
        // stored by the BABE pallet, and is valid for commitments until the parent block (`_n`). The hashed received is the
        // hash of `AuthorVrfRandomness` concatenated with the `subject` parameter provided (in this case empty).
        let (h_opt, _n) = pallet_babe::ParentBlockRandomness::<Runtime>::random(b"");
        h_opt.unwrap_or_default()
    }
}

impl pallet_randomness::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BabeDataGetter = BabeDataGetter;
    type BabeBlockGetter = BlockNumberGetter;
    type WeightInfo = ();
    type BabeDataGetterBlockNumber = BlockNumber;
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
    type Units = StorageDataUnit; // Storage unit
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = ProofsDealer;
    type TreasuryCutCalculator = LinearThenPowerOfTwoTreasuryCutCalculator<Runtime, Perbill>;
    type TreasuryAccount = TreasuryAccount;
    type MaxUsersToCharge = ConstU32<10>;
    type BaseDeposit = ConstU128<10>;
}

impl pallet_bucket_nfts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_bucket_nfts::weights::SubstrateWeight<Runtime>;
    type Buckets = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

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

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_helpers {
    use crate::{AccountId, Balance, Signature};
    use frame_support::weights::{Weight, WeightToFee};
    use sp_runtime::traits::{IdentifyAccount, Verify};

    /// Benchmark helper for transaction payment that provides minimal fees
    pub struct BenchmarkWeightToFee;

    impl WeightToFee for BenchmarkWeightToFee {
        type Balance = Balance;

        fn weight_to_fee(weight: &Weight) -> Self::Balance {
            // Divide weight by 10,000,000 to get minimal fees
            // This ensures fees are small enough to work with minimal funding
            weight.ref_time().saturating_div(10_000_000).max(1).into()
        }
    }

    /// Benchmark helper for NFTs pallet
    pub struct NftHelper;

    impl pallet_nfts::BenchmarkHelper<u32, u32, <Signature as Verify>::Signer, AccountId, Signature>
        for NftHelper
    {
        fn collection(i: u16) -> u32 {
            i.into()
        }

        fn item(i: u16) -> u32 {
            i.into()
        }

        fn signer() -> (<Signature as Verify>::Signer, AccountId) {
            // Use a dummy ECDSA public key for benchmarks
            use sp_core::ecdsa;
            let public_key: <Signature as Verify>::Signer =
                ecdsa::Public::from_raw([0u8; 33]).into();
            let account: AccountId = public_key.clone().into_account();
            (public_key, account)
        }

        fn sign(_public: &<Signature as Verify>::Signer, _message: &[u8]) -> Signature {
            // For benchmarks, return a dummy signature
            // Use MultiSignature::Ecdsa since EthereumSignature expects that
            use sp_core::ecdsa;
            use sp_runtime::MultiSignature;
            let dummy_signature = ecdsa::Signature::from_raw([0u8; 65]);
            Signature::from(MultiSignature::Ecdsa(dummy_signature))
        }
    }
}

use crate as pallet_file_system;
use core::marker::PhantomData;
use frame_support::{
    derive_impl,
    dispatch::DispatchClass,
    parameter_types,
    traits::{AsEnsureOriginWithArg, Everything, Hooks, Randomness},
    weights::{constants::RocksDbWeight, Weight, WeightMeter},
    BoundedBTreeSet,
};
use frame_system::{
    limits::BlockWeights, pallet_prelude::BlockNumberFor, BlockWeight, ConsumedWeight,
};
use num_bigint::BigUint;
use pallet_nfts::PalletFeatures;
use shp_data_price_updater::NoUpdatePriceIndexUpdater;
use shp_file_metadata::ChunkId;
use shp_traits::{
    CommitmentVerifier, MaybeDebug, ProofSubmittersInterface, ReadUserSolvencyInterface,
    TrieMutation, TrieProofDeltaApplier,
};
use shp_treasury_funding::NoCutTreasuryCutCalculator;
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Get, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{
        BlakeTwo256, BlockNumberProvider, Convert, ConvertBack, IdentifyAccount, IdentityLookup,
        Verify, Zero,
    },
    BuildStorage, DispatchError, MultiSignature, Perbill, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_trie::{CompactProof, LayoutV1, MemoryDB, TrieConfiguration, TrieLayout};
use sp_weights::FixedFee;
use std::{
    sync::{RwLock, RwLockReadGuard},
    thread,
    time::Duration,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;
type Balance = u128;
type Signature = MultiSignature;
type AccountPublic = <Signature as Verify>::Signer;
type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10;
const UNITS: Balance = 1_000_000_000_000;
const STAKE_TO_CHALLENGE_PERIOD: Balance = 100 * UNITS;
const STAKE_TO_SEED_PERIOD: Balance = 1000 * UNITS;

// We mock the Randomness trait to use a simple randomness function when testing the pallet
const BLOCKS_BEFORE_RANDOMNESS_VALID: BlockNumber = 3;
pub struct MockRandomness;
impl Randomness<H256, BlockNumber> for MockRandomness {
    fn random(subject: &[u8]) -> (H256, BlockNumber) {
        // Simple randomness mock that changes each block but its randomness is only valid after 3 blocks

        // Concatenate the subject with the block number to get a unique hash for each block
        let subject_concat_block = [
            subject,
            &frame_system::Pallet::<Test>::block_number().to_le_bytes(),
        ]
        .concat();

        let hashed_subject = blake2_256(&subject_concat_block);

        (
            H256::from_slice(&hashed_subject),
            frame_system::Pallet::<Test>::block_number()
                .saturating_sub(BLOCKS_BEFORE_RANDOMNESS_VALID),
        )
    }
}

/// Rolls to the desired block, with non-spammed blocks. Returns the number of blocks played.
pub(crate) fn roll_to(n: BlockNumber) -> BlockNumber {
    let mut num_blocks = 0;
    let mut block = System::block_number();
    while block < n {
        block = roll_one_block(false);
        num_blocks += 1;
    }
    num_blocks
}

/// Rolls to the desired block with spammed blocks. Returns the number of blocks played.
pub(crate) fn roll_to_spammed(n: BlockNumber) -> BlockNumber {
    let mut num_blocks = 0;
    let mut block = System::block_number();
    while block < n {
        block = roll_one_block(true);
        num_blocks += 1;
    }
    num_blocks
}

/// Rolls forward one block. Returns the new block number.
///
/// It can be configured whether the block is spammed or not.
/// A spammed block is one where there is no weight left for other transactions.
fn roll_one_block(spammed: bool) -> BlockNumber {
    System::set_block_number(System::block_number() + 1);
    ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());

    // Set block weight usage.
    let normal_weight = if spammed {
        let weights: BlockWeights = <Test as frame_system::Config>::BlockWeights::get();
        weights
            .get(DispatchClass::Normal)
            .max_total
            .unwrap_or(weights.max_block)
    } else {
        Zero::zero()
    };
    let block_weight = ConsumedWeight::new(|class: DispatchClass| match class {
        DispatchClass::Normal => normal_weight,
        DispatchClass::Operational => Zero::zero(),
        DispatchClass::Mandatory => Zero::zero(),
    });
    BlockWeight::<Test>::set(block_weight);

    FileSystem::on_idle(System::block_number(), Weight::MAX);
    ProofsDealer::on_finalize(System::block_number());
    System::block_number()
}

// Configure a mock runtime to test the pallet.
#[frame_support::runtime]
mod test_runtime {
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
    pub struct Test;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(2)]
    pub type FileSystem = crate;
    #[runtime::pallet_index(3)]
    pub type Providers = pallet_storage_providers;
    #[runtime::pallet_index(4)]
    pub type ProofsDealer = pallet_proofs_dealer;
    #[runtime::pallet_index(5)]
    pub type PaymentStreams = pallet_payment_streams;
    #[runtime::pallet_index(6)]
    pub type BucketNfts = pallet_bucket_nfts;
    #[runtime::pallet_index(7)]
    pub type Nfts = pallet_nfts;
    #[runtime::pallet_index(8)]
    pub type CrRandomness = pallet_cr_randomness;
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const ExistentialDeposit: u128 = 1;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = RocksDbWeight;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<10>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<10>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub storage Features: PalletFeatures = PalletFeatures::all_enabled();
}

impl pallet_nfts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u128;
    type ItemId = u128;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type Locker = ();
    type CollectionDeposit = ConstU128<2>;
    type ItemDeposit = ConstU128<1>;
    type MetadataDepositBase = ConstU128<1>;
    type AttributeDepositBase = ConstU128<1>;
    type DepositPerByte = ConstU128<1>;
    type StringLimit = ConstU32<50>;
    type KeyLimit = ConstU32<50>;
    type ValueLimit = ConstU32<50>;
    type ApprovalsLimit = ConstU32<10>;
    type ItemAttributesApprovalsLimit = ConstU32<2>;
    type MaxTips = ConstU32<10>;
    type MaxDeadlineDuration = ConstU64<10000>;
    type MaxAttributesPerCall = ConstU32<2>;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = AccountPublic;
    type WeightInfo = ();
    pallet_nfts::runtime_benchmarks_enabled! {
        type Helper = ();
    }
}

/****** Commit-Reveal Randomness pallet ******/
impl pallet_cr_randomness::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type SeedCommitment = H256;
    type Seed = H256;
    type SeedVerifier = MockSeedVerifier;
    type SeedGenerator = MockSeedGenerator;
    type RandomSeedMixer = MockRandomSeedMixer;
    type MaxSeedTolerance = MaxSeedTolerance;
    type StakeToSeedPeriod = StakeToSeedPeriod;
    type MinSeedPeriod = MinSeedPeriod;
}

parameter_types! {
    pub const MaxSeedTolerance: u32 = 10;
    pub const StakeToSeedPeriod: u128 = STAKE_TO_SEED_PERIOD;
    pub const MinSeedPeriod: u64 = 4;
}

pub type Seed = H256;
pub type SeedCommitment = H256;

pub struct MockSeedVerifier;
impl pallet_cr_randomness::SeedVerifier for MockSeedVerifier {
    type Seed = Seed;
    type SeedCommitment = SeedCommitment;
    fn verify(seed: &Self::Seed, seed_commitment: &Self::SeedCommitment) -> bool {
        BlakeTwo256::hash(seed.as_bytes()) == *seed_commitment
    }
}

pub struct MockRandomSeedMixer;
impl pallet_cr_randomness::RandomSeedMixer<Seed> for MockRandomSeedMixer {
    fn mix_randomness_seed(seed_1: &Seed, seed_2: &Seed, context: Option<impl Into<Seed>>) -> Seed {
        let mut seed = seed_1.as_fixed_bytes().to_vec();
        seed.extend_from_slice(seed_2.as_fixed_bytes());
        if let Some(context) = context {
            seed.extend_from_slice(context.into().as_fixed_bytes());
        }
        Seed::from_slice(&blake2_256(&seed))
    }
}

pub struct MockSeedGenerator;
impl pallet_cr_randomness::SeedGenerator for MockSeedGenerator {
    type Seed = Seed;
    fn generate_seed(generator: &[u8]) -> Self::Seed {
        Seed::from_slice(&blake2_256(generator))
    }
}
/****** ****** ****** ******/

// Payment streams pallet:
parameter_types! {
    pub const PaymentStreamHoldReason: RuntimeHoldReason = RuntimeHoldReason::PaymentStreams(pallet_payment_streams::HoldReason::PaymentStreamDeposit);
}

impl pallet_payment_streams::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type NativeBalance = Balances;
    type ProvidersPallet = Providers;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Units = u64;
    type NewStreamDeposit = ConstU64<10>;
    type UserWithoutFundsCooldown = ConstU64<100>;
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type TreasuryCutCalculator = NoCutTreasuryCutCalculator<Balance, Self::Units>;
    type TreasuryAccount = TreasuryAccount;
    type MaxUsersToCharge = ConstU32<10>;
    type BaseDeposit = ConstU128<10>;
}
// Converter from the BlockNumber type to the Balance type for math
pub struct BlockNumberToBalance;
impl Convert<BlockNumberFor<Test>, Balance> for BlockNumberToBalance {
    fn convert(block_number: BlockNumberFor<Test>) -> Balance {
        block_number.into() // In this converter we assume that the block number type is smaller in size than the balance type
    }
}

parameter_types! {
    pub const MaxNumberOfPeerIds: u32 = 100;
    pub const MaxMultiAddressSize: u32 = 100;
    pub const MaxMultiAddressAmount: u32 = 5;
}

pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
    }
}

/// Mock implementation of the relay chain data provider, which should return the relay chain block
/// that the previous parachain block was anchored to.
pub struct MockRelaychainDataProvider;
impl BlockNumberProvider for MockRelaychainDataProvider {
    type BlockNumber = u32;
    fn current_block_number() -> Self::BlockNumber {
        frame_system::Pallet::<Test>::block_number()
            .saturating_sub(1)
            .try_into()
            .unwrap()
    }
}

type StorageDataUnit = u64;
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

parameter_types! {
    pub const StorageProvidersHoldReason: RuntimeHoldReason = RuntimeHoldReason::Providers(pallet_storage_providers::HoldReason::StorageProviderDeposit);
    pub const SpMinDeposit: Balance = 10 * UNITS;
    pub const ProviderTopUpTtl: u64 = 10;
}

impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ProvidersRandomness = MockRandomness;
    type PaymentStreams = PaymentStreams;
    type ProofDealer = ProofsDealer;
    type FileMetadataManager = shp_file_metadata::FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type NativeBalance = Balances;
    type CrRandomness = CrRandomness;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageDataUnit = StorageDataUnit;
    type SpCount = u32;
    type BucketCount = u32;
    type MerklePatriciaRoot = H256;
    type MerkleTrieHashing = BlakeTwo256;
    type ProviderId = H256;
    type ProviderIdHashing = BlakeTwo256;
    type ValuePropId = H256;
    type ValuePropIdHashing = BlakeTwo256;
    type ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type ReputationWeightType = u32;
    type StorageHubTickGetter = ProofsDealer;
    type StorageDataUnitAndBalanceConvert = StorageDataUnitAndBalanceConverter;
    type Treasury = TreasuryAccount;
    type SpMinDeposit = SpMinDeposit;
    type SpMinCapacity = ConstU64<2>;
    type DepositPerData = ConstU128<2>;
    type MaxFileSize = ConstU64<{ u64::MAX }>;
    type MaxMultiAddressSize = MaxMultiAddressSize;
    type MaxMultiAddressAmount = MaxMultiAddressAmount;
    type MaxProtocols = ConstU32<100>;
    type BucketDeposit = ConstU128<10>;
    type BucketNameLimit = ConstU32<100>;
    type MaxBlocksForRandomness = ConstU64<{ EPOCH_DURATION_IN_BLOCKS * 2 }>;
    type MinBlocksBetweenCapacityChanges = ConstU64<10>;
    type DefaultMerkleRoot = DefaultMerkleRoot<LayoutV1<BlakeTwo256>>;
    type SlashAmountPerMaxFileSize = ConstU128<10>;
    type StartingReputationWeight = ConstU32<1>;
    type BspSignUpLockPeriod = ConstU64<10>;
    type MaxCommitmentSize = ConstU32<1000>;
    type ZeroSizeBucketFixedRate = ConstU128<1>;
    type ProviderTopUpTtl = ProviderTopUpTtl;
    type MaxExpiredItemsInBlock = ConstU32<100>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelpers = ();
}

// Mocked list of Providers that submitted proofs that can be used to test the pallet. It just returns the block number passed to it as the only submitter.
pub struct MockSubmittingProviders;
impl ProofSubmittersInterface for MockSubmittingProviders {
    type ProviderId = <Test as frame_system::Config>::Hash;
    type TickNumber = BlockNumberFor<Test>;
    type MaxProofSubmitters = ConstU32<1000>;
    fn get_proof_submitters_for_tick(
        _block_number: &Self::TickNumber,
    ) -> Option<BoundedBTreeSet<Self::ProviderId, Self::MaxProofSubmitters>> {
        None
    }

    fn get_current_tick() -> Self::TickNumber {
        System::block_number()
    }

    fn get_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) -> Option<u32> {
        None
    }

    fn clear_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) {}
}

pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        AccountId::new([0; 32])
    }
}

pub struct BlockFullnessHeadroom;
impl Get<Weight> for BlockFullnessHeadroom {
    fn get() -> Weight {
        Weight::from_parts(10_000, 0)
            + <Test as frame_system::Config>::DbWeight::get().reads_writes(0, 1)
    }
}

pub struct MinNotFullBlocksRatio;
impl Get<Perbill> for MinNotFullBlocksRatio {
    fn get() -> Perbill {
        Perbill::from_percent(50)
    }
}

parameter_types! {
    pub const StakeToChallengePeriod: Balance = STAKE_TO_CHALLENGE_PERIOD;
    pub const ChallengeTicksTolerance: BlockNumberFor<Test> = 10;
    pub const CheckpointChallengePeriod: u64 = {
        const STAKE_TO_CHALLENGE_PERIOD: u128 = StakeToChallengePeriod::get();
        const SP_MIN_DEPOSIT: u128 = SpMinDeposit::get();
        const CHALLENGE_TICKS_TOLERANCE: u128 = ChallengeTicksTolerance::get() as u128;
        ((STAKE_TO_CHALLENGE_PERIOD / SP_MIN_DEPOSIT)
            .saturating_add(CHALLENGE_TICKS_TOLERANCE)
            .saturating_add(1)) as u64
    };
}

impl pallet_proofs_dealer::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleTrieHash = H256;
    type MerkleTrieHashing = BlakeTwo256;
    type ForestVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type KeyVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type StakeToBlockNumber = SaturatingBalanceToBlockNumber;
    type RandomChallengesPerBlock = ConstU32<10>;
    type MaxCustomChallengesPerBlock = ConstU32<10>;
    type MaxSubmittersPerTick = ConstU32<100>;
    type TargetTicksStorageOfSubmitters = ConstU32<3>;
    type ChallengeHistoryLength = ConstU64<30>;
    type ChallengesQueueLength = ConstU32<25>;
    type CheckpointChallengePeriod = CheckpointChallengePeriod;
    type ChallengesFee = ConstU128<1_000_000>;
    type Treasury = TreasuryAccount;
    type RandomnessProvider = MockRandomness;
    type StakeToChallengePeriod = StakeToChallengePeriod;
    type MinChallengePeriod = ConstU64<4>;
    type ChallengeTicksTolerance = ChallengeTicksTolerance;
    type BlockFullnessPeriod = ConstU32<10>;
    type BlockFullnessHeadroom = BlockFullnessHeadroom;
    type MinNotFullBlocksRatio = MinNotFullBlocksRatio;
    type MaxSlashableProvidersPerTick = ConstU32<100>;
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

impl pallet_bucket_nfts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Buckets = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

pub(crate) type ThresholdType = u32;
pub(crate) type ReplicationTargetType = u32;

parameter_types! {
    pub const MinWaitForStopStoring: BlockNumber = 30;
    pub const BaseStorageRequestCreationDeposit: Balance = 10;
    pub const UpfrontTicksToPay: TickNumber = 10;
    pub const FileDeletionRequestCreationDeposit: Balance = 10;
    pub const FileSystemStorageRequestCreationHoldReason: RuntimeHoldReason = RuntimeHoldReason::FileSystem(pallet_file_system::HoldReason::StorageRequestCreationHold);
    pub const FileSystemFileDeletionRequestCreationHoldReason: RuntimeHoldReason = RuntimeHoldReason::FileSystem(pallet_file_system::HoldReason::FileDeletionRequestHold);
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type PaymentStreams = PaymentStreams;
    type CrRandomness = CrRandomness;
    type UpdateStoragePrice = NoUpdatePriceIndexUpdater<Balance, u64>;
    type UserSolvency = MockUserSolvency;
    type Fingerprint = H256;
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
    type BspStopStoringFilePenalty = ConstU128<1>;
    type TreasuryAccount = TreasuryAccount;
    type MaxBatchConfirmStorageRequests = ConstU32<10>;
    type MaxFilePathSize = ConstU32<512u32>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = MaxNumberOfPeerIds;
    type MaxDataServerMultiAddresses = ConstU32<5>;
    type MaxExpiredItemsInTick = ConstU32<100u32>;
    type StorageRequestTtl = ConstU32<40u32>;
    type MoveBucketRequestTtl = ConstU32<40u32>;
    type MaxUserPendingDeletionRequests = ConstU32<10u32>;
    type MaxUserPendingMoveBucketRequests = ConstU32<10u32>;
    type MinWaitForStopStoring = MinWaitForStopStoring;
    type BaseStorageRequestCreationDeposit = BaseStorageRequestCreationDeposit;
    type UpfrontTicksToPay = UpfrontTicksToPay;
    type WeightToFee = FixedFee<5, Balance>;
    type ReplicationTargetToBalance = ReplicationTargetToBalance;
    type TickNumberToBalance = TickNumberToBalance;
    type StorageDataUnitToBalance = StorageDataUnitToBalance;
    type FileDeletionRequestDeposit = FileDeletionRequestCreationDeposit;
    type BasicReplicationTarget = ConstU32<2>;
    type StandardReplicationTarget = ConstU32<3>;
    type HighSecurityReplicationTarget = ConstU32<4>;
    type SuperHighSecurityReplicationTarget = ConstU32<5>;
    type UltraHighSecurityReplicationTarget = ConstU32<6>;
    type MaxReplicationTarget = ConstU32<7>;
    type TickRangeToMaximumThreshold = ConstU64<30>;
}

// Use a RwLock for Eve’s insolvency status.
static IS_EVE_INSOLVENT: RwLock<bool> = RwLock::new(true);

/// Function to set or unset Eve's insolvency status. Returns a read lock to the RwLock
/// that makes it so the caller thread can disallow other threads from writing to the lock
/// until it has finished using it.
pub fn set_eve_insolvent(insolvent: bool) -> RwLockReadGuard<'static, bool> {
    // Spin until we can acquire the write lock without any active readers.
    let mut write_guard = loop {
        if let Ok(write_guard) = IS_EVE_INSOLVENT.try_write() {
            // Successfully acquired the lock => no other readers.
            break write_guard;
        }
        // There are active readers, so wait a bit before retrying.
        thread::sleep(Duration::from_millis(10));
    };

    // Update Eve's insolvent status.
    *write_guard = insolvent;

    // Drop the write guard to be able to acquire a read lock.
    drop(write_guard);

    // Acquire a read lock to the RwLock.
    IS_EVE_INSOLVENT.read().unwrap()
}

pub struct MockUserSolvency;
impl ReadUserSolvencyInterface for MockUserSolvency {
    type AccountId = AccountId;

    fn is_user_insolvent(user_account: &Self::AccountId) -> bool {
        if user_account == &Keyring::Eve.to_account_id() {
            *IS_EVE_INSOLVENT.read().unwrap()
        } else {
            false
        }
    }
}

// Converter from the ReplicationTarget type to the Balance type for math.
pub struct ReplicationTargetToBalance;
impl Convert<ReplicationTargetType, Balance> for ReplicationTargetToBalance {
    fn convert(replication_target: ReplicationTargetType) -> Balance {
        replication_target.into()
    }
}

// Converter from the TickNumber type to the Balance type for math.
pub type TickNumber = BlockNumber;
pub struct TickNumberToBalance;
impl Convert<TickNumber, Balance> for TickNumberToBalance {
    fn convert(tick_number: TickNumber) -> Balance {
        tick_number.into()
    }
}

// Converter from the StorageDataUnit type to the Balance type for math.
pub struct StorageDataUnitToBalance;
impl Convert<StorageDataUnit, Balance> for StorageDataUnitToBalance {
    fn convert(data_unit: StorageDataUnit) -> Balance {
        data_unit.into()
    }
}

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;

impl Convert<Balance, BlockNumberFor<Test>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Test> {
        block_number.saturated_into()
    }
}

// Converter from the ThresholdType to the BlockNumber type and vice versa.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct ThresholdTypeToBlockNumberConverter;

impl Convert<ThresholdType, BlockNumberFor<Test>> for ThresholdTypeToBlockNumberConverter {
    fn convert(threshold: ThresholdType) -> BlockNumberFor<Test> {
        threshold.saturated_into()
    }
}

impl ConvertBack<ThresholdType, BlockNumberFor<Test>> for ThresholdTypeToBlockNumberConverter {
    fn convert_back(block_number: BlockNumberFor<Test>) -> ThresholdType {
        block_number.saturated_into()
    }
}

/// Converter from the [`Hash`] type to the [`ThresholdType`].
pub struct HashToThresholdTypeConverter;
impl Convert<<Test as frame_system::Config>::Hash, ThresholdType> for HashToThresholdTypeConverter {
    fn convert(hash: <Test as frame_system::Config>::Hash) -> ThresholdType {
        // Get the hash as bytes
        let hash_bytes = hash.as_ref();

        // Get the 4 least significant bytes of the hash and interpret them as an u32
        let truncated_hash_bytes: [u8; 4] =
            hash_bytes[28..].try_into().expect("Hash is 32 bytes; qed");

        ThresholdType::from_be_bytes(truncated_hash_bytes)
    }
}

// Converter from the MerkleHash (H256) type to the RandomnessOutput type.
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

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (Keyring::Alice.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Bob.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Charlie.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Dave.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Eve.to_account_id(), 1_000_000_000_000_000),
            (TreasuryAccount::get(), ExistentialDeposit::get()),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| roll_one_block(false));
    ext
}

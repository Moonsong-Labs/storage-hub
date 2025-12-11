use crate as pallet_payment_streams;
use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::{
    derive_impl,
    pallet_prelude::Get,
    parameter_types,
    traits::{AsEnsureOriginWithArg, Everything, Randomness},
    weights::constants::RocksDbWeight,
};
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRoot, EnsureSigned};
use pallet_nfts::PalletFeatures;
use shp_constants::GIGAUNIT;
use shp_traits::{
    CommitRevealRandomnessInterface, CommitmentVerifier, MaybeDebug, ProofSubmittersInterface,
    ReadProvidersInterface, TrieMutation, TrieProofDeltaApplier,
};
use shp_treasury_funding::NoCutTreasuryCutCalculator;
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Hasher, H256};
use sp_runtime::{
    testing::TestSignature,
    traits::{BlakeTwo256, ConvertBack, IdentityLookup},
    BuildStorage, DispatchError, Perbill, SaturatedConversion,
};
use sp_runtime::{traits::Convert, BoundedBTreeSet};
use sp_trie::{CompactProof, LayoutV1, MemoryDB, TrieConfiguration, TrieLayout};
use sp_weights::Weight;
use std::collections::BTreeMap;

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type StorageDataUnit = u64;
type AccountId = u64;

const EPOCH_DURATION_IN_BLOCKS: BlockNumberFor<Test> = 10;
// We mock the Randomness trait to use a simple randomness function when testing the pallet
const BLOCKS_BEFORE_RANDOMNESS_VALID: BlockNumberFor<Test> = 3;

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
    pub type StorageProviders = pallet_storage_providers;
    #[runtime::pallet_index(3)]
    pub type PaymentStreams = crate;
    #[runtime::pallet_index(4)]
    pub type Nfts = pallet_nfts;
    #[runtime::pallet_index(5)]
    pub type ProofsDealer = pallet_proofs_dealer;
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
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<10>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const StorageProvidersHoldReason: RuntimeHoldReason = RuntimeHoldReason::StorageProviders(pallet_storage_providers::HoldReason::StorageProviderDeposit);
}
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
    }
}

pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        1000
    }
}

pub struct MockFileMetadataManager;
impl shp_traits::FileMetadataInterface for MockFileMetadataManager {
    type Metadata = shp_file_metadata::FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type StorageDataUnit = u64;

    fn encode(metadata: &Self::Metadata) -> Vec<u8> {
        metadata.encode()
    }

    fn decode(data: &[u8]) -> Result<Self::Metadata, codec::Error> {
        <shp_file_metadata::FileMetadata<
            { shp_constants::H_LENGTH },
            { shp_constants::FILE_CHUNK_SIZE },
            { shp_constants::FILE_SIZE_TO_CHALLENGES },
        > as Decode>::decode(&mut &data[..])
    }

    fn get_file_size(metadata: &Self::Metadata) -> Self::StorageDataUnit {
        metadata.file_size()
    }

    fn owner(metadata: &Self::Metadata) -> &Vec<u8> {
        metadata.owner()
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

pub struct MockRandomness;
impl Randomness<H256, BlockNumberFor<Test>> for MockRandomness {
    fn random(subject: &[u8]) -> (H256, BlockNumberFor<Test>) {
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

parameter_types! {
    pub const SpMinDeposit: Balance = 10 * UNITS;
    pub const ProviderTopUpTtl: u64 = 10;
}

pub struct MockCommitRevealRandomness;
impl CommitRevealRandomnessInterface for MockCommitRevealRandomness {
    type ProviderId = <Test as pallet_storage_providers::Config>::ProviderId;

    fn initialise_randomness_cycle(
        _who: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn stop_randomness_cycle(_who: &Self::ProviderId) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }
}

impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ProvidersRandomness = MockRandomness;
    type FileMetadataManager = MockFileMetadataManager;
    type NativeBalance = Balances;
    type CrRandomness = MockCommitRevealRandomness;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageDataUnit = StorageDataUnit;
    type PaymentStreams = PaymentStreams;
    type ProofDealer = ProofsDealer;
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
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
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
    type MaxExpiredItemsInBlock = ConstU32<10>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelpers = ();
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
    type OffchainSignature = TestSignature;
    type OffchainPublic = <TestSignature as sp_runtime::traits::Verify>::Signer;
    type WeightInfo = ();
    pallet_nfts::runtime_benchmarks_enabled! {
        type Helper = ();
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

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;

impl Convert<Balance, BlockNumberFor<Test>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Test> {
        block_number.saturated_into()
    }
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
    ) -> Result<BTreeMap<Self::Challenge, Vec<u8>>, DispatchError> {
        if proof.encoded_nodes.len() > 0 {
            Ok(proof
                .encoded_nodes
                .iter()
                .map(|node| H256::from_slice(&node[..]))
                .map(|key| (key, Vec::new()))
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
        _mutations: &[(Self::Key, TrieMutation)],
        _proof: &Self::Proof,
    ) -> Result<
        (
            MemoryDB<T::Hash>,
            Self::Key,
            Vec<(Self::Key, Option<Vec<u8>>)>,
        ),
        DispatchError,
    > {
        // Just return the root as is with no mutations
        Ok((MemoryDB::<T::Hash>::default(), *root, Vec::new()))
    }
}

const UNITS: Balance = 1_000_000_000_000;
const STAKE_TO_CHALLENGE_PERIOD: Balance = 100 * UNITS;

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
    type ProvidersPallet = StorageProviders;
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
    type PriorityChallengesFee = ConstU128<0>;
    type Treasury = TreasuryAccount;
    type RandomnessProvider = MockRandomness;
    type StakeToChallengePeriod = StakeToChallengePeriod;
    type MinChallengePeriod = ConstU64<4>;
    type ChallengeTicksTolerance = ChallengeTicksTolerance;
    type BlockFullnessPeriod = ConstU32<10>;
    type BlockFullnessHeadroom = BlockFullnessHeadroom;
    type MinNotFullBlocksRatio = MinNotFullBlocksRatio;
    type MaxSlashableProvidersPerTick = ConstU32<100>;
    type ChallengeOrigin = EnsureSigned<AccountId>;
    type PriorityChallengeOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
    pub const PaymentStreamHoldReason: RuntimeHoldReason = RuntimeHoldReason::PaymentStreams(pallet_payment_streams::HoldReason::PaymentStreamDeposit);
}

// Converter from the BlockNumber type to the Balance type for math
pub struct BlockNumberToBalance;

impl Convert<BlockNumberFor<Test>, Balance> for BlockNumberToBalance {
    fn convert(block_number: BlockNumberFor<Test>) -> Balance {
        block_number.into() // In this converter we assume that the block number type is smaller in size than the balance type
    }
}

// Mocked list of Providers that submitted proofs that can be used to test the pallet. It just returns the block number passed to it as the only submitter.
pub struct MockSubmittingProviders;
impl ProofSubmittersInterface for MockSubmittingProviders {
    type ProviderId = <Test as frame_system::Config>::Hash;
    type TickNumber = BlockNumberFor<Test>;
    type MaxProofSubmitters = ConstU32<1000>;
    fn get_proof_submitters_for_tick(
        block_number: &Self::TickNumber,
    ) -> Option<BoundedBTreeSet<Self::ProviderId, Self::MaxProofSubmitters>> {
        let mut set = BoundedBTreeSet::<Self::ProviderId, Self::MaxProofSubmitters>::new();
        // We convert the block number + 1 to the corresponding Provider ID, to simulate that the Provider submitted a proof
        <StorageProviders as ReadProvidersInterface>::get_provider_id(&(block_number + 1))
            .map(|id| set.try_insert(id));
        if set.len() == 0 {
            None
        } else {
            Some(set)
        }
    }

    fn get_current_tick() -> Self::TickNumber {
        System::block_number()
    }

    fn get_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) -> Option<u32> {
        None
    }

    fn clear_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) {}
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type NativeBalance = Balances;
    type ProvidersPallet = StorageProviders;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Units = StorageDataUnit;
    type NewStreamDeposit = ConstU64<10>;
    type UserWithoutFundsCooldown = ConstU64<100>;
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type TreasuryCutCalculator = NoCutTreasuryCutCalculator<Balance, Self::Units>;
    type TreasuryAccount = TreasuryAccount;
    type MaxUsersToCharge = ConstU32<10>;
    type BaseDeposit = ConstU128<10>;
}

// Build genesis storage according to the mock runtime.
pub fn _new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

// Externalities builder with predefined balances for accounts and starting at block number 1
pub struct ExtBuilder;
impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        pallet_balances::GenesisConfig::<Test> {
            balances: vec![
                (0, 5_000_000 * UNITS),   // Alice = 0
                (1, 10_000_000 * UNITS),  // Bob = 1
                (2, 20_000_000 * UNITS),  // Charlie = 2
                (3, 30_000_000 * UNITS),  // David = 3
                (4, 40_000_000 * UNITS),  // Eve = 4
                (5, 50_000_000 * UNITS),  // Ferdie = 5
                (6, 60_000_000 * UNITS),  // George = 6
                (123, 5_000_000 * UNITS), // Alice for `on_poll` testing = 123
                (TreasuryAccount::get(), ExistentialDeposit::get()),
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        crate::GenesisConfig::<Test> {
            current_price: GIGAUNIT.into(),
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
            pallet_payment_streams::OnPollTicker::<Test>::set(1);
        });
        ext
    }
}

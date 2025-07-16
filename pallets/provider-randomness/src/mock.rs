//! A minimal runtime including the pallet-cr-randomness pallet
use core::marker::PhantomData;
use std::collections::BTreeSet;

use super::*;
use crate as pallet_cr_randomness;
use codec::{Decode, Encode};
use frame_support::{
    derive_impl, parameter_types,
    traits::{Everything, Randomness},
    weights::{constants::RocksDbWeight, Weight},
};
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRoot, EnsureSigned};
use shp_file_metadata::{FileMetadata, Fingerprint};
use shp_traits::{
    CommitmentVerifier, MaybeDebug, ProofSubmittersInterface, StorageHubTickGetter, TrieMutation,
    TrieProofDeltaApplier,
};
use shp_treasury_funding::NoCutTreasuryCutCalculator;
use sp_core::{blake2_256, ConstU128, ConstU32, ConstU64, Get, Hasher, H256};
use sp_runtime::{
    traits::{BlakeTwo256, Convert, ConvertBack, IdentityLookup},
    BoundedBTreeSet, BoundedVec, BuildStorage, DispatchError, Perbill, SaturatedConversion,
};
use sp_std::convert::{TryFrom, TryInto};
use sp_trie::{CompactProof, LayoutV1, MemoryDB, TrieConfiguration, TrieLayout};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
pub type AccountId = u64;
pub type StorageDataUnit = u64;

const EPOCH_DURATION_IN_BLOCKS: BlockNumberFor<Test> = 10;
const UNITS: Balance = 1_000_000_000_000;
pub(crate) const STAKE_TO_CHALLENGE_PERIOD: Balance = 100 * UNITS;
pub(crate) const STAKE_TO_SEED_PERIOD: Balance = 1000 * UNITS;

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
    pub type Providers = pallet_storage_providers;
    #[runtime::pallet_index(3)]
    pub type ProofsDealer = pallet_proofs_dealer;
    #[runtime::pallet_index(4)]
    pub type PaymentStreams = pallet_payment_streams;
    #[runtime::pallet_index(5)]
    pub type CrRandomness = pallet_cr_randomness;
}

parameter_types! {
    pub const BlockHashCount: u32 = 250;
    pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 1);
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
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

pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        1000
    }
}

parameter_types! {
    pub const ProviderTopUpTtl: u64 = 10;
}

// Storage Providers pallet:
impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ProvidersRandomness = MockRandomness;
    type PaymentStreams = PaymentStreams;
    type ProofDealer = ProofsDealer;
    type FileMetadataManager = MockFileMetadataManager;
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
    type ReadAccessGroupId = u32;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type ReputationWeightType = u32;
    type StorageHubTickGetter = MockStorageHubTickGetter;
    type StorageDataUnitAndBalanceConvert = StorageDataUnitAndBalanceConverter;
    type Treasury = TreasuryAccount;
    type SpMinDeposit = ConstU128<{ 10 * UNITS }>;
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
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelpers = ();
    type ProviderTopUpTtl = ProviderTopUpTtl;
    type MaxExpiredItemsInBlock = ConstU32<10>;
}

// Mock the Randomness trait to use a simple randomness function when testing the pallet
const BLOCKS_BEFORE_RANDOMNESS_VALID: BlockNumberFor<Test> = 3;
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

// Mock the file metadata manager to use a simple file metadata struct when testing the pallet
pub struct MockFileMetadataManager;
impl shp_traits::FileMetadataInterface for MockFileMetadataManager {
    type Metadata = FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type StorageDataUnit = u64;

    fn encode(metadata: &Self::Metadata) -> Vec<u8> {
        metadata.encode()
    }

    fn decode(data: &[u8]) -> Result<Self::Metadata, codec::Error> {
        <FileMetadata<
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

// Converter from the BlockNumber type to the Balance type for math
pub struct BlockNumberToBalance;
impl Convert<BlockNumberFor<Test>, Balance> for BlockNumberToBalance {
    fn convert(block_number: BlockNumberFor<Test>) -> Balance {
        block_number.into() // In this converter we assume that the block number type is smaller in size than the balance type
    }
}

// Output type of the hasher
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;

// Default Merkle root for our trie layout
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
    }
}

pub struct MockStorageHubTickGetter;
impl StorageHubTickGetter for MockStorageHubTickGetter {
    type TickNumber = BlockNumberFor<Test>;
    fn get_current_tick() -> Self::TickNumber {
        System::block_number()
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

// Payment streams pallet:
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

parameter_types! {
    pub const StakeToChallengePeriod: Balance = STAKE_TO_CHALLENGE_PERIOD;
    pub const ChallengeTicksTolerance: BlockNumberFor<Test> = 10;
    pub const SpMinDeposit: Balance = 10 * UNITS;
    pub const CheckpointChallengePeriod: u64 = {
        const STAKE_TO_CHALLENGE_PERIOD: u128 = StakeToChallengePeriod::get();
        const SP_MIN_DEPOSIT: u128 = SpMinDeposit::get();
        const CHALLENGE_TICKS_TOLERANCE: u128 = ChallengeTicksTolerance::get() as u128;
        ((STAKE_TO_CHALLENGE_PERIOD / SP_MIN_DEPOSIT)
            .saturating_add(CHALLENGE_TICKS_TOLERANCE)
            .saturating_add(1)) as u64
    };
}

// Proofs dealer pallet:
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
    type Challenge = C;

    fn verify_proof(
        _root: &Self::Commitment,
        challenges: &[Self::Challenge],
        proof: &CompactProof,
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError> {
        if proof.encoded_nodes.len() > 0 {
            let challenges: BTreeSet<Self::Challenge> = challenges.iter().cloned().collect();
            Ok(challenges)
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
        _root: &Self::Key,
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
        let last_key = mutations.last().unwrap().0;

        let db = MemoryDB::<T::Hash>::default();

        let mutated_keys_and_values = mutations
            .iter()
            .map(|(key, mutation)| {
                let value = match mutation {
                    TrieMutation::Add(add_mutation) => Some(add_mutation.value.clone()),
                    TrieMutation::Remove(_) => {
                        let file_metadata: FileMetadata<
                            { shp_constants::H_LENGTH },
                            { shp_constants::FILE_CHUNK_SIZE },
                            { shp_constants::FILE_SIZE_TO_CHALLENGES },
                        > = match FileMetadata::new(
                            1_u64.encode(),
                            blake2_256(b"bucket").as_ref().to_vec(),
                            b"path/to/file".to_vec(),
                            1,
                            Fingerprint::default().into(),
                        ) {
                            Ok(file_metadata) => file_metadata,
                            Err(_) => {
                                return Err(DispatchError::Other("Failed to create file metadata"))
                            }
                        };
                        if key.as_ref() != [0; H_LENGTH] {
                            Some(file_metadata.encode())
                        } else {
                            Some(vec![1, 2, 3, 4, 5, 6]) // We make it so the metadata is invalid for the empty key
                        }
                    }
                };
                Ok((*key, value))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Return default db, the last key in mutations as the new root, and a
        // vector holding the supposedly mutated keys and values, so it is deterministic for testing.
        Ok((db, last_key, mutated_keys_and_values))
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

// Commit-reveal randomness pallet:
impl Config for Test {
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
impl SeedVerifier for MockSeedVerifier {
    type Seed = Seed;
    type SeedCommitment = SeedCommitment;
    fn verify(seed: &Self::Seed, seed_commitment: &Self::SeedCommitment) -> bool {
        BlakeTwo256::hash(seed.as_bytes()) == *seed_commitment
    }
}

pub struct MockRandomSeedMixer;
impl RandomSeedMixer<Seed> for MockRandomSeedMixer {
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
impl SeedGenerator for MockSeedGenerator {
    type Seed = Seed;
    fn generate_seed(generator: &[u8]) -> Self::Seed {
        Seed::from_slice(&blake2_256(generator))
    }
}

/// Panics if an event is not found in the system log of events
#[macro_export]
macro_rules! assert_event_emitted {
    ($event:expr) => {
        match &$event {
            e => {
                assert!(
                    $crate::mock::events().iter().find(|x| *x == e).is_some(),
                    "Event {:?} was not found in events: \n {:?}",
                    e,
                    $crate::mock::events()
                );
            }
        }
    };
}

/// Externality builder for pallet randomness mock runtime
pub struct ExtBuilder;
impl ExtBuilder {
    #[allow(dead_code)]
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
            tick_to_start_checking_for_slashable_providers: 0,
            initial_elements_for_randomness: BoundedVec::truncate_from(vec![
                Default::default();
                MaxSeedTolerance::get()
                    as usize
            ]),
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
            pallet_proofs_dealer::ChallengesTicker::<Test>::set(1);
            crate::TickToCheckForSlashableProviders::<Test>::set(2);
        });
        ext
    }
}

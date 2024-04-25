use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{Everything, Hooks, Randomness},
    weights::{constants::RocksDbWeight, Weight},
};
use frame_system as system;
use pallet_proofs_dealer::CompactProof;
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Get, H256};
use sp_runtime::{
    traits::{BlakeTwo256, Bounded, IdentityLookup},
    AccountId32, BuildStorage, DispatchResult, FixedU128,
};
use storage_hub_traits::CommitmentVerifier;

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;
type Balance = u128;
type AccountId = AccountId32;

const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10;

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

/// Rolls to the desired block. Returns the number of blocks played.
pub(crate) fn roll_to(n: BlockNumber) -> BlockNumber {
    let mut num_blocks = 0;
    let mut block = System::block_number();
    while block < n {
        block = roll_one_block();
        num_blocks += 1;
    }
    num_blocks
}

// Rolls forward one block. Returns the new block number.
fn roll_one_block() -> BlockNumber {
    System::set_block_number(System::block_number() + 1);
    FileSystem::on_idle(System::block_number(), Weight::MAX);
    System::block_number()
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        FileSystem: crate::{Pallet, Call, Storage, Event<T>},
        Providers: pallet_storage_providers::{Pallet, Call, Storage, Event<T>, HoldReason},
        ProofsDealer: pallet_proofs_dealer::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const StorageProvidersHoldReason: RuntimeHoldReason = RuntimeHoldReason::Providers(pallet_storage_providers::HoldReason::StorageProviderDeposit);
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
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
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<10>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<10>;
}

parameter_types! {
    pub const MaxNumberOfPeerIds: u32 = 100;
    pub const MaxMultiAddressSize: u32 = 100;
    pub const MaxMultiAddressAmount: u32 = 5;
}

impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageData = u32;
    type SpCount = u32;
    type MerklePatriciaRoot = H256;
    type ValuePropId = H256;
    type MaxMultiAddressSize = MaxMultiAddressSize;
    type MaxMultiAddressAmount = MaxMultiAddressAmount;
    type MaxProtocols = ConstU32<100>;
    type MaxBsps = ConstU32<100>;
    type MaxMsps = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
    type Subscribers = FileSystem;
    type MaxBlocksForRandomness = ConstU64<{ EPOCH_DURATION_IN_BLOCKS * 2 }>;
    type MinBlocksBetweenCapacityChanges = ConstU64<10>;
    type ProvidersRandomness = MockRandomness;
}

// TODO: remove this and replace with pallet treasury
pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        AccountId::new([0; 32])
    }
}

impl pallet_proofs_dealer::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleHash = H256;
    type KeyVerifier = MockVerifier;
    type MaxChallengesPerBlock = ConstU32<10>;
    type MaxProvidersChallengedPerBlock = ConstU32<10>;
    type ChallengeHistoryLength = ConstU32<10>;
    type ChallengesQueueLength = ConstU32<10>;
    type CheckpointChallengePeriod = ConstU32<10>;
    type ChallengesFee = ConstU128<1_000_000>;
    type Treasury = TreasuryAccount;
}

/// Structure to mock a verifier that returns `true` when `proof` is not empty
/// and `false` otherwise.
pub struct MockVerifier;

/// Implement the `TrieVerifier` trait for the `MockVerifier` struct.
impl CommitmentVerifier for MockVerifier {
    type Proof = CompactProof;
    type Key = H256;

    fn verify_proof(
        _root: &Self::Key,
        _challenges: &[Self::Key],
        proof: &CompactProof,
    ) -> DispatchResult {
        if proof.encoded_nodes.len() > 0 {
            Ok(())
        } else {
            Err("Proof is empty".into())
        }
    }
}

pub(crate) type ThresholdType = FixedU128;

parameter_types! {
    pub const ThresholdAsymptoticDecayFactor: FixedU128 = FixedU128::from_rational(1, 2); // 0.5
    pub const ThresholdAsymptote: FixedU128 = FixedU128::from_rational(100, 1); // 100.0
    pub const ThresholdMultiplier: FixedU128 = FixedU128::from_rational(100, 1); // 100.0
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type Fingerprint = H256;
    type StorageRequestBspsRequiredType = u32;
    type ThresholdType = ThresholdType;
    type AssignmentThresholdDecayFactor = ThresholdAsymptoticDecayFactor;
    type AssignmentThresholdAsymptote = ThresholdAsymptote;
    type AssignmentThresholdMultiplier = ThresholdMultiplier;
    type TargetBspsRequired = ConstU32<3>;
    type MaxBspsPerStorageRequest = ConstU32<5>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = MaxNumberOfPeerIds;
    type MaxDataServerMultiAddresses = ConstU32<5>; // TODO: this should probably be a multiplier of the number of maximum multiaddresses per storage provider
    type MaxFilePathSize = ConstU32<512u32>;
    type StorageRequestTtl = ConstU32<40u32>;
    type MaxExpiredStorageRequests = ConstU32<100u32>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    crate::GenesisConfig::<Test> {
        bsp_assignment_threshold: FixedU128::max_value(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (AccountId32::new([1; 32]), 1_000_000_000_000_000),
            (AccountId32::new([2; 32]), 1_000_000_000_000_000),
            (AccountId32::new([3; 32]), 1_000_000_000_000_000),
            (AccountId32::new([4; 32]), 1_000_000_000_000_000),
            (AccountId32::new([5; 32]), 1_000_000_000_000_000),
            (AccountId32::new([6; 32]), 1_000_000_000_000_000),
            (AccountId32::new([7; 32]), 1_000_000_000_000_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

pub(crate) fn compute_set_get_initial_threshold() -> ThresholdType {
    let initial_threshold = FileSystem::compute_asymptotic_threshold_point(1)
        .expect("Initial threshold should be computable");
    crate::BspsAssignmentThreshold::<Test>::put(initial_threshold);
    initial_threshold
}

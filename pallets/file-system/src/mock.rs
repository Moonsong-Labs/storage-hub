use frame_support::{
    derive_impl, parameter_types,
    traits::{Everything, Hooks},
    weights::{constants::RocksDbWeight, Weight},
};
use frame_system as system;
use pallet_proofs_dealer::{CompactProof, TrieVerifier};
use sp_core::{ConstU128, ConstU32, Get, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;
type Balance = u128;
type AccountId = AccountId32;

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
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        FileSystem: crate::{Pallet, Call, Storage, Event<T>},
        Providers: pallet_storage_providers::{Pallet, Call, Storage, Event<T>},
        ProofsDealer: pallet_proofs_dealer::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
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
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<10>;
    type MaxFreezes = ConstU32<10>;
}

impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageData = u32;
    type SpCount = u32;
    type MerklePatriciaRoot = H256;
    type ValuePropId = H256;
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
    type MaxProtocols = ConstU32<100>;
    type MaxBsps = ConstU32<100>;
    type MaxMsps = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
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
    type TrieVerifier = MockVerifier;
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
impl TrieVerifier for MockVerifier {
    fn verify_proof(_root: &[u8; 32], _challenges: &[u8; 32], proof: &CompactProof) -> bool {
        proof.encoded_nodes.len() > 0
    }
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    type ProofDealer = ProofsDealer;
    type AssignmentThreshold = u128;
    type AssignmentThresholdMultiplier = ConstU32<100>;
    type MinBspsAssignmentThreshold = ConstU128<{ u128::MAX / 2 }>;
    type Fingerprint = H256;
    type StorageRequestBspsRequiredType = u32;
    type TargetBspsRequired = ConstU32<1>;
    type MaxBspsPerStorageRequest = ConstU32<5>;
    type MaxMultiAddresses = ConstU32<5>; // TODO: this should probably be a multiplier of the number of maximum multiaddresses per storage provider
    type MaxFilePathSize = ConstU32<512u32>;
    type MaxMultiAddressSize = ConstU32<512u32>;
    type StorageRequestTtl = ConstU32<40u32>;
    type MaxExpiredStorageRequests = ConstU32<100u32>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

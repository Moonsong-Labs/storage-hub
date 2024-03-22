use crate as pallet_storage_providers;
use frame_support::{construct_runtime, derive_impl, parameter_types, traits::Everything};
use frame_system as system;
use pallet_babe::{RandomnessFromOneEpochAgo, SameAuthoritiesForever};
use sp_core::{ConstU128, ConstU32, Void, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        BabeRandomness: pallet_babe,
        Timestamp: pallet_timestamp,
        StorageProviders: pallet_storage_providers,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const StorageProvidersHoldReason: RuntimeHoldReason = RuntimeHoldReason::StorageProviders(pallet_storage_providers::HoldReason::StorageProviderDeposit);
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

impl pallet_babe::Config for Test {
    type EpochDuration = ();
    type ExpectedBlockTime = ();
    type EpochChangeTrigger = SameAuthoritiesForever;
    type KeyOwnerProof = Void;
    type WeightInfo = ();
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100>;
    type MaxNominators = ConstU32<100>;
    type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Test {
    type MinimumPeriod = ();
    type WeightInfo = ();
    type Moment = u64;
    type OnTimestampSet = ();
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageData = u32;
    type SpCount = u32;
    type HashId = H256;
    type Hashing = BlakeTwo256;
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
    type ProvidersRandomness = RandomnessFromOneEpochAgo<Test>;
}

// Build genesis storage according to the mock runtime.
pub fn _new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::<Test>::default()
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
                (0, 5_000_000),       // Alice = 0
                (1, 10_000_000),      // Bob = 1
                (2, 20_000_000),      // Charlie = 2
                (3, 30_000_000),      // David = 3
                (4, 400_000_000),     // Eve = 4
                (5, 5_000_000_000),   // Ferdie = 5
                (6, 600_000_000_000), // George = 6
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

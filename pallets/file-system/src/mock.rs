use frame_support::{
    derive_impl, parameter_types,
    traits::{Everything, Hooks},
    weights::Weight,
};
use frame_system as system;
use sp_core::{ConstU32, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;

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
        FileSystem: crate::{Pallet, Call, Storage, Event<T>},
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Fingerprint = H256;
    type StorageUnit = u128;
    type StorageRequestBspsRequiredType = u32;
    type DefaultBspsRequired = ConstU32<1>;
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

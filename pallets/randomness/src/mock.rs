//! A minimal runtime including the pallet-randomness pallet
use super::*;
use core::convert::{TryFrom, TryInto};
use frame_support::{derive_impl, parameter_types, traits::Everything, weights::Weight};
use sp_core::{blake2_256, H160, H256};
use sp_runtime::{
    traits::{BlakeTwo256, BlockNumberProvider, IdentityLookup},
    BuildStorage, Perbill,
};

pub type AccountId = H160;
pub type Balance = u128;

type Block = frame_system::mocking::MockBlock<Test>;

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
    pub type Randomness = crate;
}

parameter_types! {
    pub const BlockHashCount: u32 = 250;
    pub const MaximumBlockWeight: Weight = Weight::from_parts(1024, 1);
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const SS58Prefix: u8 = 42;
}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Block = Block;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
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

parameter_types! {
    pub const ExistentialDeposit: u128 = 0;
}
impl pallet_balances::Config for Test {
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 4];
    type MaxLocks = ();
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

pub struct BabeDataGetter;
impl crate::GetBabeData<u64, H256> for BabeDataGetter {
    fn get_epoch_index() -> u64 {
        frame_system::Pallet::<Test>::block_number()
    }
    fn get_epoch_randomness() -> H256 {
        H256::from_slice(&blake2_256(&Self::get_epoch_index().to_le_bytes()))
    }
    fn get_parent_randomness() -> H256 {
        H256::from_slice(&blake2_256(
            &Self::get_epoch_index().saturating_sub(1).to_le_bytes(),
        ))
    }
}

/// Mock implementation of the relay chain data provider, which should return the relay chain block
/// that the previous parachain block was anchored to.
pub struct BlockNumberGetter {}
impl BlockNumberProvider for BlockNumberGetter {
    type BlockNumber = u64;
    fn current_block_number() -> Self::BlockNumber {
        frame_system::Pallet::<Test>::block_number()
            .saturating_sub(1)
            .try_into()
            .unwrap()
    }
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type BabeDataGetter = BabeDataGetter;
    type BabeBlockGetter = BlockNumberGetter;
    type WeightInfo = ();
    type BabeDataGetterBlockNumber = BlockNumberFor<Test>;
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
        let t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("Frame system builds valid default genesis config");

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

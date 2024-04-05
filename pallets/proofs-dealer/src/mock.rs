#![allow(non_camel_case_types)]

use frame_support::{
    derive_impl, parameter_types, traits::Everything, weights::constants::RocksDbWeight,
};
use frame_system as system;
use sp_core::{ConstU128, ConstU32, ConstU64, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchResult,
};
use sp_trie::CompactProof;
use storage_hub_traits::SubscribeProvidersInterface;

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type AccountId = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Providers: pallet_storage_providers::{Pallet, Call, Storage, Event<T>, HoldReason},
        ProofsDealer: crate::{Pallet, Call, Storage, Event<T>},
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
    type Subscribers = MockedProvidersSubscriber;
}
impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleHash = H256;
    type TrieVerifier = MockVerifier;
    type MaxChallengesPerBlock = ConstU32<10>;
    type MaxProvidersChallengedPerBlock = ConstU32<10>;
    type ChallengeHistoryLength = ConstU32<10>;
    type ChallengesQueueLength = ConstU32<10>;
    type CheckpointChallengePeriod = ConstU32<2>;
    type ChallengesFee = ConstU128<1_000_000>;
    type Treasury = ConstU64<181222>;
}

pub struct MockedProvidersSubscriber;
impl SubscribeProvidersInterface for MockedProvidersSubscriber {
    type Provider = u64;

    fn subscribe_bsp_sign_up(_who: &Self::Provider) -> DispatchResult {
        Ok(())
    }
    fn subscribe_bsp_sign_off(_who: &Self::Provider) -> DispatchResult {
        Ok(())
    }
}

/// Structure to mock a verifier that returns `true` when `proof` is not empty
/// and `false` otherwise.
pub struct MockVerifier;

/// Implement the `TrieVerifier` trait for the `MockVerifier` struct.
impl crate::TrieVerifier for MockVerifier {
    fn verify_proof(root: &[u8; 32], challenges: &[u8; 32], proof: &CompactProof) -> bool {
        proof.encoded_nodes.len() > 0
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

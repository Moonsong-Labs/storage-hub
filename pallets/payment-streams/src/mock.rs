use crate as pallet_payment_streams;
use core::marker::PhantomData;
use frame_support::{
    construct_runtime, derive_impl,
    pallet_prelude::Get,
    parameter_types,
    traits::{AsEnsureOriginWithArg, Everything, Randomness},
    weights::constants::RocksDbWeight,
};
use frame_system as system;
use pallet_nfts::PalletFeatures;
use shp_traits::{ProofSubmittersInterface, ProvidersInterface, SubscribeProvidersInterface};
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Hasher, H256};
use sp_runtime::{
    testing::TestSignature,
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchResult,
};
use sp_runtime::{traits::Convert, BoundedBTreeSet};
use sp_trie::{LayoutV1, TrieConfiguration, TrieLayout};
use system::pallet_prelude::BlockNumberFor;

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type StorageUnit = u32;
type AccountId = u64;

const EPOCH_DURATION_IN_BLOCKS: BlockNumberFor<Test> = 10;
// We mock the Randomness trait to use a simple randomness function when testing the pallet
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

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        StorageProviders: pallet_storage_providers,
        PaymentStreams: pallet_payment_streams,
        Nfts: pallet_nfts
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
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
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<10>;
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


// TODO: remove this and replace with pallet treasury
pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        0
    }
}

impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageData = StorageUnit;
    type SpCount = u32;
    type MerklePatriciaRoot = H256;
    type DefaultMerkleRoot = DefaultMerkleRoot<LayoutV1<BlakeTwo256>>;
    type ValuePropId = H256;
    type ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type Treasury = TreasuryAccount;
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
    type MaxProtocols = ConstU32<100>;
    type MaxBlocksForRandomness = ConstU64<{ EPOCH_DURATION_IN_BLOCKS * 2 }>;
    type MinBlocksBetweenCapacityChanges = ConstU64<10>;
    type MaxBsps = ConstU32<100>;
    type MaxMsps = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type BucketDeposit = ConstU128<10>;
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
    type Subscribers = MockedProvidersSubscriber;
    type ProvidersRandomness = MockRandomness;
    type BucketNameLimit = ConstU32<100>;
    type SlashFactor = ConstU128<10>;
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
        <StorageProviders as ProvidersInterface>::get_provider_id(*block_number + 1)
            .map(|id| set.try_insert(id));
        Some(set)
    }

    fn get_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) -> Option<u32> {
        None
    }

    fn clear_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) {}
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type NativeBalance = Balances;
    type ProvidersPallet = StorageProviders;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Units = StorageUnit;
    type NewStreamDeposit = ConstU64<10>;
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = MockSubmittingProviders;
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
                (123, 5_000_000),     // Alice for `on_poll` testing = 123
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

pub struct MockedProvidersSubscriber;
impl SubscribeProvidersInterface for MockedProvidersSubscriber {
    type ProviderId = u64;

    fn subscribe_bsp_sign_up(_who: &Self::ProviderId) -> DispatchResult {
        Ok(())
    }
    fn subscribe_bsp_sign_off(_who: &Self::ProviderId) -> DispatchResult {
        Ok(())
    }
}

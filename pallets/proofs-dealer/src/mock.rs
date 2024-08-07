#![allow(non_camel_case_types)]

use core::marker::PhantomData;
use frame_support::{
    derive_impl,
    pallet_prelude::Get,
    parameter_types,
    traits::{Everything, Randomness},
    weights::constants::RocksDbWeight,
    BoundedBTreeSet,
};
use frame_system as system;
use shp_traits::{
    CommitmentVerifier, MaybeDebug, ProofSubmittersInterface, SubscribeProvidersInterface,
    TrieMutation, TrieProofDeltaApplier,
};
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Hasher, H256};
use sp_runtime::{
    traits::{BlakeTwo256, Convert, IdentityLookup},
    BuildStorage, DispatchError, DispatchResult, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_trie::{CompactProof, LayoutV1, MemoryDB, TrieConfiguration, TrieLayout};
use system::pallet_prelude::BlockNumberFor;

type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type AccountId = u64;

const EPOCH_DURATION_IN_BLOCKS: BlockNumberFor<Test> = 10;
const UNITS: Balance = 1_000_000_000_000;
pub(crate) const STAKE_TO_CHALLENGE_PERIOD: Balance = 10 * UNITS;

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
    type StorageData = u32;
    type SpCount = u32;
    type MerklePatriciaRoot = H256;
    type DefaultMerkleRoot = DefaultMerkleRoot<LayoutV1<BlakeTwo256>>;
    type ValuePropId = H256;
    type ReadAccessGroupId = u32;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type Treasury = TreasuryAccount;
    type MaxMultiAddressSize = ConstU32<100>;
    type MaxMultiAddressAmount = ConstU32<5>;
    type MaxProtocols = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type BucketDeposit = ConstU128<10>;
    type BucketNameLimit = ConstU32<100>;
    type SpMinDeposit = ConstU128<{ 10 * UNITS }>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
    type Subscribers = MockedProvidersSubscriber;
    type MaxBlocksForRandomness = ConstU64<{ EPOCH_DURATION_IN_BLOCKS * 2 }>;
    type MinBlocksBetweenCapacityChanges = ConstU64<10>;
    type ProvidersRandomness = MockRandomness;
    type SlashFactor = ConstU128<10>;
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

    fn get_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) -> Option<u32> {
        None
    }

    fn clear_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) {}
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ProvidersPallet = Providers;
    type NativeBalance = Balances;
    type MerkleTrieHash = H256;
    type MerkleTrieHashing = BlakeTwo256;
    type ForestVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type KeyVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type StakeToBlockNumber = SaturatingBalanceToBlockNumber;
    type RandomChallengesPerBlock = ConstU32<10>;
    type MaxCustomChallengesPerBlock = ConstU32<10>;
    type MaxSubmittersPerTick = ConstU32<1000>; // TODO: Change this value after benchmarking for it to coincide with the implicit limit given by maximum block weight
    type TargetTicksStorageOfSubmitters = ConstU32<3>;
    type ChallengeHistoryLength = ConstU64<30>;
    type ChallengesQueueLength = ConstU32<25>;
    type CheckpointChallengePeriod = ConstU64<10>;
    type ChallengesFee = ConstU128<1_000_000>;
    type Treasury = ConstU64<181222>;
    type RandomnessProvider = MockRandomness;
    type StakeToChallengePeriod = ConstU128<STAKE_TO_CHALLENGE_PERIOD>;
    type ChallengeTicksTolerance = ConstU64<20>;
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
    ) -> Result<(MemoryDB<T::Hash>, Self::Key), DispatchError> {
        let last_key = mutations.last().unwrap().0;

        let db = MemoryDB::<T::Hash>::default();

        // Return default db and the last key in mutations, so it is deterministic for testing.
        Ok((db, last_key))
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;

impl Convert<Balance, BlockNumberFor<Test>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Test> {
        block_number.saturated_into()
    }
}

use core::marker::PhantomData;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{AsEnsureOriginWithArg, Everything, Hooks, Randomness},
    weights::{constants::RocksDbWeight, Weight, WeightMeter},
    BoundedBTreeSet,
};
use frame_system as system;
use num_bigint::BigUint;
use pallet_nfts::PalletFeatures;
use shp_file_metadata::ChunkId;
use shp_traits::{
    CommitmentVerifier, MaybeDebug, ProofSubmittersInterface, TrieMutation, TrieProofDeltaApplier,
};
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Get, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{BlakeTwo256, Bounded, Convert, IdentifyAccount, IdentityLookup, Verify},
    BuildStorage, DispatchError, FixedPointNumber, FixedU128, MultiSignature, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_trie::{CompactProof, LayoutV1, MemoryDB, TrieConfiguration, TrieLayout};
use system::pallet_prelude::BlockNumberFor;

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;
type Balance = u128;
type Signature = MultiSignature;
type AccountPublic = <Signature as Verify>::Signer;
type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 10;
const UNITS: Balance = 1_000_000_000_000;
const STAKE_TO_CHALLENGE_PERIOD: Balance = 10 * UNITS;

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
    ProofsDealer::on_poll(System::block_number(), &mut WeightMeter::new());
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
        BucketNfts: pallet_bucket_nfts::{Pallet, Call, Storage, Event<T>},
        Nfts: pallet_nfts::{Pallet, Call, Storage, Event<T>},
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
    type OffchainSignature = Signature;
    type OffchainPublic = AccountPublic;
    type WeightInfo = ();
    pallet_nfts::runtime_benchmarks_enabled! {
        type Helper = ();
    }
}

parameter_types! {
    pub const MaxNumberOfPeerIds: u32 = 100;
    pub const MaxMultiAddressSize: u32 = 100;
    pub const MaxMultiAddressAmount: u32 = 5;
}

pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
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
    type ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type Treasury = TreasuryAccount;
    type MaxMultiAddressSize = MaxMultiAddressSize;
    type MaxMultiAddressAmount = MaxMultiAddressAmount;
    type MaxProtocols = ConstU32<100>;
    type MaxBuckets = ConstU32<10000>;
    type BucketDeposit = ConstU128<10>;
    type BucketNameLimit = ConstU32<100>;
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
    type Subscribers = FileSystem;
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
    type MerkleTrieHash = H256;
    type MerkleTrieHashing = BlakeTwo256;
    type ForestVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type KeyVerifier = MockVerifier<H256, LayoutV1<BlakeTwo256>, { BlakeTwo256::LENGTH }>;
    type StakeToBlockNumber = SaturatingBalanceToBlockNumber;
    type RandomChallengesPerBlock = ConstU32<10>;
    type MaxCustomChallengesPerBlock = ConstU32<10>;
    type MaxSubmittersPerTick = ConstU32<1000>; // TODO: Change this value after benchmarking for it to coincide with the implicit limit given by maximum block weight
    type TargetTicksStorageOfSubmitters = ConstU32<3>;
    type ChallengeHistoryLength = ConstU64<10>;
    type ChallengesQueueLength = ConstU32<10>;
    type CheckpointChallengePeriod = ConstU64<10>;
    type ChallengesFee = ConstU128<1_000_000>;
    type Treasury = TreasuryAccount;
    type RandomnessProvider = MockRandomness;
    type StakeToChallengePeriod = ConstU128<STAKE_TO_CHALLENGE_PERIOD>;
    type ChallengeTicksTolerance = ConstU64<20>;
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
    ) -> Result<BTreeSet<Self::Challenge>, DispatchError> {
        if proof.encoded_nodes.len() > 0 {
            Ok(proof
                .encoded_nodes
                .iter()
                .map(|node| H256::from_slice(&node[..]))
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
    ) -> Result<(MemoryDB<T::Hash>, Self::Key), DispatchError> {
        // Just return the root as is with no mutations
        Ok((MemoryDB::<T::Hash>::default(), *root))
    }
}

impl pallet_bucket_nfts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
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
    type ThresholdTypeToBlockNumber = SaturatingThresholdTypeToBlockNumberConverter;
    type BlockNumberToThresholdType = BlockNumberToThresholdTypeConverter;
    type HashToThresholdType = HashToThresholdTypeConverter;
    type MerkleHashToRandomnessOutput = MerkleHashToRandomnessOutputConverter;
    type ChunkIdToMerkleHash = ChunkIdToMerkleHashConverter;
    type Currency = Balances;
    type Nfts = Nfts;
    type CollectionInspector = BucketNfts;
    type AssignmentThresholdDecayFactor = ThresholdAsymptoticDecayFactor;
    type AssignmentThresholdAsymptote = ThresholdAsymptote;
    type AssignmentThresholdMultiplier = ThresholdMultiplier;
    type TargetBspsRequired = ConstU32<3>;
    type MaxBspsPerStorageRequest = ConstU32<5>;
    type MaxBatchConfirmStorageRequests = ConstU32<10>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = MaxNumberOfPeerIds;
    type MaxDataServerMultiAddresses = ConstU32<5>; // TODO: this should probably be a multiplier of the number of maximum multiaddresses per storage provider
    type MaxFilePathSize = ConstU32<512u32>;
    type StorageRequestTtl = ConstU32<40u32>;
    type PendingFileDeletionRequestTtl = ConstU32<40u32>;
    type MaxExpiredItemsInBlock = ConstU32<100u32>;
    type MaxUserPendingDeletionRequests = ConstU32<10u32>;
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
            (Keyring::Alice.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Bob.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Charlie.to_account_id(), 1_000_000_000_000_000),
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

// Converter from the Balance type to the BlockNumber type for math.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingBalanceToBlockNumber;

impl Convert<Balance, BlockNumberFor<Test>> for SaturatingBalanceToBlockNumber {
    fn convert(block_number: Balance) -> BlockNumberFor<Test> {
        block_number.saturated_into()
    }
}

// Converter from the ThresholdType type (FixedU128) to the BlockNumber type (u64).
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct SaturatingThresholdTypeToBlockNumberConverter;

impl Convert<ThresholdType, BlockNumberFor<Test>>
    for SaturatingThresholdTypeToBlockNumberConverter
{
    fn convert(threshold: ThresholdType) -> BlockNumberFor<Test> {
        (threshold.into_inner() / FixedU128::accuracy()).saturated_into()
    }
}

// Converter from the BlockNumber type (u64) to the ThresholdType type (FixedU128).
pub struct BlockNumberToThresholdTypeConverter;

impl Convert<BlockNumberFor<Test>, ThresholdType> for BlockNumberToThresholdTypeConverter {
    fn convert(block_number: BlockNumberFor<Test>) -> ThresholdType {
        FixedU128::from_inner((block_number as u128) * FixedU128::accuracy())
    }
}

// Converter from the Hash type from the runtime (BlakeTwo256) to the ThresholdType type (FixedU128).
// Since we can't convert directly a hash to a FixedU128 (since the hash type used in the runtime has
// 256 bits and FixedU128 has 128 bits), we convert the hash to a BigUint, then map it to a FixedU128
// by keeping its relative position between zero and the maximum 256-bit number.
pub struct HashToThresholdTypeConverter;
impl Convert<<Test as frame_system::Config>::Hash, ThresholdType> for HashToThresholdTypeConverter {
    fn convert(hash: <Test as frame_system::Config>::Hash) -> ThresholdType {
        // Get the hash as bytes
        let hash_bytes = hash.as_ref();

        // Get the 16 least significant bytes of the hash and interpret them as a u128
        let truncated_hash_bytes: [u8; 16] =
            hash_bytes[16..].try_into().expect("Hash is 32 bytes; qed");
        let hash_as_u128 = u128::from_be_bytes(truncated_hash_bytes);

        // Return it as a FixedU128
        FixedU128::from_inner(hash_as_u128)
    }
}

// Converter from the MerkleHash (H256) type to the RandomnessOutput type.
pub struct MerkleHashToRandomnessOutputConverter;

impl Convert<H256, H256> for MerkleHashToRandomnessOutputConverter {
    fn convert(hash: H256) -> H256 {
        hash
    }
}

// Converter from the ChunkId type to the MerkleHash (H256) type.
pub struct ChunkIdToMerkleHashConverter;

impl Convert<ChunkId, H256> for ChunkIdToMerkleHashConverter {
    fn convert(chunk_id: ChunkId) -> H256 {
        let chunk_id_biguint = BigUint::from(chunk_id.as_u64());
        let mut bytes = chunk_id_biguint.to_bytes_be();

        // Ensure the byte slice is exactly 32 bytes long by padding with leading zeros
        if bytes.len() < 32 {
            let mut padded_bytes = vec![0u8; 32 - bytes.len()];
            padded_bytes.extend(bytes);
            bytes = padded_bytes;
        }

        H256::from_slice(&bytes)
    }
}

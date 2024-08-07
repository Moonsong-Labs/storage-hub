use core::marker::PhantomData;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{AsEnsureOriginWithArg, Everything, Randomness},
    weights::constants::RocksDbWeight,
    BoundedBTreeSet,
};
use frame_system as system;
use num_bigint::BigUint;
use pallet_nfts::PalletFeatures;
use shp_file_metadata::ChunkId;
use shp_traits::{
    ProofSubmittersInterface, ProofsDealerInterface, SubscribeProvidersInterface, TrieMutation,
    TrieRemoveMutation,
};
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Get, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{BlakeTwo256, Convert, IdentifyAccount, IdentityLookup, Verify},
    BuildStorage, DispatchResult, FixedPointNumber, FixedU128, MultiSignature, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_trie::{LayoutV1, TrieConfiguration, TrieLayout};
use system::pallet_prelude::BlockNumberFor;

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u64;
type Balance = u128;
type Signature = MultiSignature;
type AccountPublic = <Signature as Verify>::Signer;
type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

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

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Providers: pallet_storage_providers::{Pallet, Call, Storage, Event<T>, HoldReason},
        BucketNfts: crate::{Pallet, Call, Storage, Event<T>},
        Nfts: pallet_nfts::{Pallet, Call, Storage, Event<T>},
        FileSystem: pallet_file_system::{Pallet, Call, Storage, Event<T>},
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

pub(crate) type ThresholdType = FixedU128;

parameter_types! {
    pub const ThresholdAsymptoticDecayFactor: FixedU128 = FixedU128::from_rational(1, 2); // 0.5
    pub const ThresholdAsymptote: FixedU128 = FixedU128::from_rational(100, 1); // 100.0
    pub const ThresholdMultiplier: FixedU128 = FixedU128::from_rational(100, 1); // 100.0
}

pub struct MockProofsDealer;
impl ProofsDealerInterface for MockProofsDealer {
    type ProviderId = H256;
    type ForestProof = u32;
    type KeyProof = u32;
    type MerkleHash = H256;
    type RandomnessOutput = H256;
    type MerkleHashing = BlakeTwo256;

    fn challenge(_key_challenged: &Self::MerkleHash) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn challenge_with_priority(
        _key_challenged: &Self::MerkleHash,
        _mutation: Option<TrieRemoveMutation>,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn verify_forest_proof(
        _who: &Self::ProviderId,
        _challenges: &[Self::MerkleHash],
        _proof: &Self::ForestProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, sp_runtime::DispatchError> {
        Ok(BTreeSet::new())
    }

    fn verify_key_proof(
        _key: &Self::MerkleHash,
        _challenges: &[Self::MerkleHash],
        _proof: &Self::KeyProof,
    ) -> Result<BTreeSet<Self::MerkleHash>, sp_runtime::DispatchError> {
        Ok(BTreeSet::new())
    }

    fn generate_challenges_from_seed(
        _seed: Self::RandomnessOutput,
        _provider_id: &Self::ProviderId,
        _count: u32,
    ) -> Vec<Self::MerkleHash> {
        Vec::new()
    }

    fn apply_delta(
        _commitment: &Self::MerkleHash,
        _mutations: &[(Self::MerkleHash, TrieMutation)],
        _proof: &Self::ForestProof,
    ) -> Result<Self::MerkleHash, sp_runtime::DispatchError> {
        Ok(H256::default())
    }

    fn initialise_challenge_cycle(
        _who: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }
}

impl pallet_file_system::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    type ProofDealer = MockProofsDealer;
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
    type MaxUserPendingDeletionRequests = ConstU32<5u32>;
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
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU32<2>;
    type DepositPerData = ConstU128<2>;
    type Subscribers = MockedProvidersSubscriber;
    type MaxBlocksForRandomness = ConstU64<{ EPOCH_DURATION_IN_BLOCKS * 2 }>;
    type BucketNameLimit = ConstU32<100>;
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

// TODO: remove this and replace with pallet treasury
pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        AccountId::new([0; 32])
    }
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Providers = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
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

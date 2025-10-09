use core::marker::PhantomData;
use frame_support::{
    derive_impl, parameter_types,
    traits::{AsEnsureOriginWithArg, Everything, Randomness},
    weights::{constants::RocksDbWeight, FixedFee},
    BoundedBTreeSet,
};
use frame_system::pallet_prelude::BlockNumberFor;
use num_bigint::BigUint;
use pallet_nfts::PalletFeatures;
use shp_data_price_updater::NoUpdatePriceIndexUpdater;
use shp_file_metadata::{ChunkId, FileMetadata};
use shp_traits::{
    CommitRevealRandomnessInterface, IdentityAdapter, ProofSubmittersInterface,
    ProofsDealerInterface, ReadUserSolvencyInterface, StorageHubTickGetter, TrieMutation,
};
use shp_treasury_funding::NoCutTreasuryCutCalculator;
use sp_core::{hashing::blake2_256, ConstU128, ConstU32, ConstU64, Get, Hasher, H256};
use sp_keyring::sr25519::Keyring;
use sp_runtime::{
    traits::{
        BlakeTwo256, BlockNumberProvider, Convert, ConvertBack, IdentifyAccount, IdentityLookup,
        Verify,
    },
    BuildStorage, MultiSignature, SaturatedConversion,
};
use sp_std::collections::btree_set::BTreeSet;
use sp_trie::{LayoutV1, TrieConfiguration, TrieLayout};

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
    pub type Providers = pallet_storage_providers;
    #[runtime::pallet_index(3)]
    pub type BucketNfts = crate;
    #[runtime::pallet_index(4)]
    pub type Nfts = pallet_nfts;
    #[runtime::pallet_index(5)]
    pub type FileSystem = pallet_file_system;
    #[runtime::pallet_index(6)]
    pub type PaymentStreams = pallet_payment_streams;
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const StorageProvidersHoldReason: RuntimeHoldReason = RuntimeHoldReason::Providers(pallet_storage_providers::HoldReason::StorageProviderDeposit);
    pub const ExistentialDeposit: u128 = 1;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
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
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<10>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<10>;
    type DoneSlashHandler = ();
}

pub(crate) type ThresholdType = u32;

parameter_types! {
    pub const MinWaitForStopStoring: BlockNumber = 30;
    pub const BaseStorageRequestCreationDeposit: Balance = 10;
    pub const UpfrontTicksToPay: TickNumber = 10;
    pub const FileDeletionRequestCreationDeposit: Balance = 10;
    pub const FileSystemHoldReason: RuntimeHoldReason = RuntimeHoldReason::FileSystem(pallet_file_system::HoldReason::StorageRequestCreationHold);
}

pub struct MockProofsDealer;
impl ProofsDealerInterface for MockProofsDealer {
    type ProviderId = H256;
    type ForestProof = u32;
    type KeyProof = u32;
    type MerkleHash = H256;
    type RandomnessOutput = H256;
    type MerkleHashing = BlakeTwo256;
    type TickNumber = BlockNumber;

    fn challenge(_key_challenged: &Self::MerkleHash) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn challenge_with_priority(
        _key_challenged: &Self::MerkleHash,
        _should_remove_key: bool,
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

    fn verify_generic_forest_proof(
        _root: &Self::MerkleHash,
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

    fn generic_apply_delta(
        _root: &Self::MerkleHash,
        _mutations: &[(Self::MerkleHash, TrieMutation)],
        _proof: &Self::ForestProof,
        _event_data: Option<Vec<u8>>,
    ) -> Result<Self::MerkleHash, sp_runtime::DispatchError> {
        Ok(H256::default())
    }

    fn stop_challenge_cycle(
        _provider_id: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn initialise_challenge_cycle(
        _who: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn get_current_tick() -> Self::TickNumber {
        System::block_number()
    }

    fn get_checkpoint_challenge_period() -> Self::TickNumber {
        5
    }
}

pub(crate) type ReplicationTargetType = u32;

impl pallet_file_system::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Providers = Providers;
    type ProofDealer = MockProofsDealer;
    type PaymentStreams = PaymentStreams;
    type CrRandomness = MockCommitRevealRandomness;
    type UpdateStoragePrice = NoUpdatePriceIndexUpdater<Balance, u64>;
    type UserSolvency = MockUserSolvency;
    type Fingerprint = H256;
    type ReplicationTargetType = ReplicationTargetType;
    type ThresholdType = ThresholdType;
    type ThresholdTypeToTickNumber = ThresholdTypeToBlockNumberConverter;
    type HashToThresholdType = HashToThresholdTypeConverter;
    type MerkleHashToRandomnessOutput = MerkleHashToRandomnessOutputConverter;
    type ChunkIdToMerkleHash = ChunkIdToMerkleHashConverter;
    type Currency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = Nfts;
    type CollectionInspector = BucketNfts;
    type BspStopStoringFilePenalty = ConstU128<1>;
    type TreasuryAccount = TreasuryAccount;
    type MaxBatchConfirmStorageRequests = ConstU32<10>;
    type MaxFileDeletionsPerExtrinsic = ConstU32<100>;
    type MaxFilePathSize = ConstU32<512u32>;
    type MaxPeerIdSize = ConstU32<100>;
    type MaxNumberOfPeerIds = MaxNumberOfPeerIds;
    type MaxDataServerMultiAddresses = ConstU32<5>;
    type MaxExpiredItemsInTick = ConstU32<100u32>;
    type StorageRequestTtl = ConstU32<40u32>;
    type MoveBucketRequestTtl = ConstU32<40u32>;
    type MaxUserPendingDeletionRequests = ConstU32<5u32>;
    type MaxUserPendingMoveBucketRequests = ConstU32<10u32>;
    type MinWaitForStopStoring = MinWaitForStopStoring;
    type BaseStorageRequestCreationDeposit = BaseStorageRequestCreationDeposit;
    type UpfrontTicksToPay = UpfrontTicksToPay;
    type WeightToFee = FixedFee<5, Balance>;
    type ReplicationTargetToBalance = ReplicationTargetToBalance;
    type TickNumberToBalance = TickNumberToBalance;
    type StorageDataUnitToBalance = StorageDataUnitToBalance;
    type FileDeletionRequestDeposit = FileDeletionRequestCreationDeposit;
    type BasicReplicationTarget = ConstU32<2>;
    type StandardReplicationTarget = ConstU32<3>;
    type HighSecurityReplicationTarget = ConstU32<4>;
    type SuperHighSecurityReplicationTarget = ConstU32<5>;
    type UltraHighSecurityReplicationTarget = ConstU32<6>;
    type MaxReplicationTarget = ConstU32<7>;
    type TickRangeToMaximumThreshold = ConstU64<30>;
    type OffchainSignature = Signature;
    type OffchainPublicKey = <Signature as Verify>::Signer;
    type IntentionMsgAdapter = IdentityAdapter;
}

pub struct MockUserSolvency;
impl ReadUserSolvencyInterface for MockUserSolvency {
    type AccountId = AccountId;

    fn is_user_insolvent(_user_account: &Self::AccountId) -> bool {
        false
    }
}

// Converter from the ReplicationTarget type to the Balance type for math.
pub struct ReplicationTargetToBalance;
impl Convert<ReplicationTargetType, Balance> for ReplicationTargetToBalance {
    fn convert(replication_target: ReplicationTargetType) -> Balance {
        replication_target.into()
    }
}

// Converter from the TickNumber type to the Balance type for math.
pub type TickNumber = BlockNumber;
pub struct TickNumberToBalance;
impl Convert<TickNumber, Balance> for TickNumberToBalance {
    fn convert(tick_number: TickNumber) -> Balance {
        tick_number.into()
    }
}

// Converter from the StorageDataUnit type to the Balance type for math.
pub struct StorageDataUnitToBalance;
impl Convert<StorageDataUnit, Balance> for StorageDataUnitToBalance {
    fn convert(data_unit: StorageDataUnit) -> Balance {
        data_unit.into()
    }
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

// Payment streams pallet:
impl pallet_payment_streams::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type NativeBalance = Balances;
    type ProvidersPallet = Providers;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Units = u64;
    type NewStreamDeposit = ConstU64<10>;
    type UserWithoutFundsCooldown = ConstU64<100>;
    type BlockNumberToBalance = BlockNumberToBalance;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type TreasuryCutCalculator = NoCutTreasuryCutCalculator<Balance, Self::Units>;
    type TreasuryAccount = TreasuryAccount;
    type MaxUsersToCharge = ConstU32<10>;
    type BaseDeposit = ConstU128<10>;
}
// Converter from the BlockNumber type to the Balance type for math
pub struct BlockNumberToBalance;
impl Convert<BlockNumberFor<Test>, Balance> for BlockNumberToBalance {
    fn convert(block_number: BlockNumberFor<Test>) -> Balance {
        block_number.into() // In this converter we assume that the block number type is smaller in size than the balance type
    }
}

parameter_types! {
    pub const MaxNumberOfPeerIds: u32 = 100;
    pub const MaxMultiAddressSize: u32 = 100;
    pub const MaxMultiAddressAmount: u32 = 5;
    pub const ProviderTopUpTtl: u64 = 10;
}
pub type HasherOutT<T> = <<T as TrieLayout>::Hash as Hasher>::Out;
pub struct DefaultMerkleRoot<T>(PhantomData<T>);
impl<T: TrieConfiguration> Get<HasherOutT<T>> for DefaultMerkleRoot<T> {
    fn get() -> HasherOutT<T> {
        sp_trie::empty_trie_root::<T>()
    }
}

/// Mock implementation of the relay chain data provider, which should return the relay chain block
/// that the previous parachain block was anchored to.
pub struct MockRelaychainDataProvider;
impl BlockNumberProvider for MockRelaychainDataProvider {
    type BlockNumber = u32;
    fn current_block_number() -> Self::BlockNumber {
        frame_system::Pallet::<Test>::block_number()
            .saturating_sub(1)
            .try_into()
            .unwrap()
    }
}

type StorageDataUnit = u64;
pub struct StorageDataUnitAndBalanceConverter;
impl Convert<StorageDataUnit, Balance> for StorageDataUnitAndBalanceConverter {
    fn convert(data_unit: StorageDataUnit) -> Balance {
        data_unit.saturated_into()
    }
}
impl ConvertBack<StorageDataUnit, Balance> for StorageDataUnitAndBalanceConverter {
    fn convert_back(balance: Balance) -> StorageDataUnit {
        balance.saturated_into()
    }
}

pub struct MockStorageHubTickGetter;
impl StorageHubTickGetter for MockStorageHubTickGetter {
    type TickNumber = BlockNumberFor<Test>;
    fn get_current_tick() -> Self::TickNumber {
        System::block_number()
    }
}

impl pallet_storage_providers::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ProvidersRandomness = MockRandomness;
    type PaymentStreams = PaymentStreams;
    type ProofDealer = MockProofsDealer;
    type FileMetadataManager = FileMetadata<
        { shp_constants::H_LENGTH },
        { shp_constants::FILE_CHUNK_SIZE },
        { shp_constants::FILE_SIZE_TO_CHALLENGES },
    >;
    type NativeBalance = Balances;
    type CrRandomness = MockCommitRevealRandomness;
    type RuntimeHoldReason = RuntimeHoldReason;
    type StorageDataUnit = StorageDataUnit;
    type SpCount = u32;
    type BucketCount = u32;
    type MerklePatriciaRoot = H256;
    type MerkleTrieHashing = BlakeTwo256;
    type ProviderId = H256;
    type ProviderIdHashing = BlakeTwo256;
    type ValuePropId = H256;
    type ValuePropIdHashing = BlakeTwo256;
    type ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId;
    type ProvidersProofSubmitters = MockSubmittingProviders;
    type ReputationWeightType = u32;
    type StorageHubTickGetter = MockStorageHubTickGetter;
    type StorageDataUnitAndBalanceConvert = StorageDataUnitAndBalanceConverter;
    type Treasury = TreasuryAccount;
    type SpMinDeposit = ConstU128<10>;
    type SpMinCapacity = ConstU64<2>;
    type DepositPerData = ConstU128<2>;
    type MaxFileSize = ConstU64<{ u64::MAX }>;
    type MaxMultiAddressSize = MaxMultiAddressSize;
    type MaxMultiAddressAmount = MaxMultiAddressAmount;
    type MaxProtocols = ConstU32<100>;
    type BucketDeposit = ConstU128<10>;
    type BucketNameLimit = ConstU32<100>;
    type MaxBlocksForRandomness = ConstU64<{ EPOCH_DURATION_IN_BLOCKS * 2 }>;
    type MinBlocksBetweenCapacityChanges = ConstU64<10>;
    type DefaultMerkleRoot = DefaultMerkleRoot<LayoutV1<BlakeTwo256>>;
    type SlashAmountPerMaxFileSize = ConstU128<10>;
    type StartingReputationWeight = ConstU32<1>;
    type BspSignUpLockPeriod = ConstU64<10>;
    type MaxCommitmentSize = ConstU32<1000>;
    type ZeroSizeBucketFixedRate = ConstU128<1>;
    type ProviderTopUpTtl = ProviderTopUpTtl;
    type MaxExpiredItemsInBlock = ConstU32<100u32>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelpers = ();
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

    fn get_current_tick() -> Self::TickNumber {
        System::block_number()
    }

    fn get_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) -> Option<u32> {
        None
    }

    fn clear_accrued_failed_proof_submissions(_provider_id: &Self::ProviderId) {}
}

pub struct TreasuryAccount;
impl Get<AccountId> for TreasuryAccount {
    fn get() -> AccountId {
        AccountId::new([0; 32])
    }
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Buckets = Providers;
    #[cfg(feature = "runtime-benchmarks")]
    type Helper = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (Keyring::Alice.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Bob.to_account_id(), 1_000_000_000_000_000),
            (Keyring::Charlie.to_account_id(), 1_000_000_000_000_000),
            (TreasuryAccount::get(), ExistentialDeposit::get()),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// Converter from the ThresholdType to the BlockNumber type and vice versa.
// It performs a saturated conversion, so that the result is always a valid BlockNumber.
pub struct ThresholdTypeToBlockNumberConverter;

impl Convert<ThresholdType, BlockNumberFor<Test>> for ThresholdTypeToBlockNumberConverter {
    fn convert(threshold: ThresholdType) -> BlockNumberFor<Test> {
        threshold.saturated_into()
    }
}

impl ConvertBack<ThresholdType, BlockNumberFor<Test>> for ThresholdTypeToBlockNumberConverter {
    fn convert_back(block_number: BlockNumberFor<Test>) -> ThresholdType {
        block_number.saturated_into()
    }
}

/// Converter from the [`Hash`] type to the [`ThresholdType`].
pub struct HashToThresholdTypeConverter;
impl Convert<<Test as frame_system::Config>::Hash, ThresholdType> for HashToThresholdTypeConverter {
    fn convert(hash: <Test as frame_system::Config>::Hash) -> ThresholdType {
        // Get the hash as bytes
        let hash_bytes = hash.as_ref();

        // Get the 4 least significant bytes of the hash and interpret them as an u32
        let truncated_hash_bytes: [u8; 4] =
            hash_bytes[28..].try_into().expect("Hash is 32 bytes; qed");

        ThresholdType::from_be_bytes(truncated_hash_bytes)
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

pub struct MockCommitRevealRandomness;
impl CommitRevealRandomnessInterface for MockCommitRevealRandomness {
    type ProviderId = <Test as pallet_storage_providers::Config>::ProviderId;

    fn initialise_randomness_cycle(
        _who: &Self::ProviderId,
    ) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }

    fn stop_randomness_cycle(_who: &Self::ProviderId) -> frame_support::dispatch::DispatchResult {
        Ok(())
    }
}

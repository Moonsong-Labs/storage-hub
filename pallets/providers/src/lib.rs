//! # Storage Providers Pallet
//!
//! This pallet provides the functionality to manage Main Storage Providers (MSPs)
//! and Backup Storage Providers (BSPs) in a decentralized storage network.
//!
//! The functionality allows users to sign up and sign off as MSPs or BSPs and change
//! their parameters. This is the way that users can offer their storage capacity to
//! the network and get rewarded for it.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod types;
mod utils;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

extern crate alloc;

pub use pallet::*;
pub use scale_info::Type;
use types::{BackupStorageProviderId, MainStorageProviderId, SignUpRequest, StorageDataUnit};

#[frame_support::pallet]
pub mod pallet {
    use super::{types::*, weights::WeightInfo};
    use codec::{FullCodec, HasCompact};
    use frame_support::{
        dispatch::DispatchResultWithPostInfo,
        pallet_prelude::*,
        sp_runtime::traits::{
            AtLeast32BitUnsigned, CheckEqual, CheckedAdd, MaybeDisplay, One, Saturating,
            SimpleBitOps, Zero,
        },
        traits::{fungible::*, Incrementable},
        Blake2_128Concat,
    };
    use frame_support::{traits::Randomness, weights::WeightMeter};
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use shp_traits::{
        FileMetadataInterface, NumericalParam, PaymentStreamsInterface, ProofSubmittersInterface,
        ReadUserSolvencyInterface, StorageHubTickGetter,
    };
    use sp_runtime::{
        traits::{Bounded, CheckedDiv, ConvertBack, Hash},
        Vec,
    };

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;

        /// Type to access randomness to salt AccountIds and get the corresponding ProviderId
        type ProvidersRandomness: Randomness<ProviderIdFor<Self>, BlockNumberFor<Self>>;

        /// Trait that allows the pallet to update payment streams of its Providers and Users
        type PaymentStreams: PaymentStreamsInterface<
                Balance = Self::NativeBalance,
                AccountId = Self::AccountId,
                ProviderId = ProviderIdFor<Self>,
                Units = Self::StorageDataUnit,
                TickNumber = BlockNumberFor<Self>,
            > + ReadUserSolvencyInterface<AccountId = Self::AccountId>;

        /// The trait for stopping challenge cycles of providers.
        type ProofDealer: shp_traits::ProofsDealerInterface<ProviderId = ProviderIdFor<Self>>;

        /// Trait that allows the pallet to manage generic file metadatas
        type FileMetadataManager: FileMetadataInterface<StorageDataUnit = Self::StorageDataUnit>;

        /// Type to access the Balances pallet (using the fungible trait from frame_support)
        type NativeBalance: Inspect<Self::AccountId>
            + Mutate<Self::AccountId>
            + hold::Inspect<Self::AccountId, Reason = Self::RuntimeHoldReason>
            // , Reason = Self::HoldReason> We will probably have to hold deposits
            + hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + hold::Balanced<Self::AccountId>
            + freeze::Inspect<Self::AccountId>
            + freeze::Mutate<Self::AccountId>;

        /// The trait to initialise a Provider's randomness commit-reveal cycle.
        type CrRandomness: shp_traits::CommitRevealRandomnessInterface<
            ProviderId = ProviderIdFor<Self>,
        >;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// Data type for the measurement of storage size
        type StorageDataUnit: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + AtLeast32BitUnsigned
            + Saturating
            + CheckedDiv
            + Zero
            + Copy
            + MaxEncodedLen
            + HasCompact;

        type StorageDataUnitAndBalanceConvert: ConvertBack<Self::StorageDataUnit, BalanceOf<Self>>;

        /// Type that represents the total number of registered Storage Providers.
        type SpCount: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Ord
            + AtLeast32BitUnsigned
            + FullCodec
            + Copy
            + Default
            + Debug
            + scale_info::TypeInfo
            + MaxEncodedLen;

        /// Type that is used to keep track of how many Buckets a Main Storage Provider is currently storing.
        type BucketCount: NumericalParam;

        /// The type of the Merkle Patricia Root of the storage trie for BSPs and MSPs' buckets (a hash).
        type MerklePatriciaRoot: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + FullCodec;
        /// The hashing system (algorithm) being used for the Merkle Patricia Roots (e.g. Blake2).
        type MerkleTrieHashing: Hash<Output = Self::MerklePatriciaRoot> + TypeInfo;

        /// The type that is used to represent a Provider ID.
        type ProviderId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + FullCodec;
        /// The hashing system (algorithm) being used for the Provider IDs (e.g. Blake2).
        type ProviderIdHashing: Hash<Output = Self::ProviderId> + TypeInfo;

        /// The type that is used to represent a Value Proposition ID.
        type ValuePropId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + FullCodec;
        /// The hashing system (algorithm) being used for the Provider IDs (e.g. Blake2).
        type ValuePropIdHashing: Hash<Output = Self::ValuePropId> + TypeInfo;

        /// The type of the Bucket NFT Collection ID.
        type ReadAccessGroupId: Member + Parameter + MaxEncodedLen + Copy + Incrementable;

        /// The trait exposing data of which providers failed to respond to challenges for proofs of storage.
        type ProvidersProofSubmitters: ProofSubmittersInterface<
            ProviderId = ProviderIdFor<Self>,
            TickNumber = BlockNumberFor<Self>,
        >;

        /// The type representing the reputation weight of a BSP.
        type ReputationWeightType: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + Saturating
            + Copy
            + MaxEncodedLen
            + HasCompact
            + Zero
            + One
            + CheckedAdd
            + Ord
            + Bounded;

        /// Interface to get the current Storage Hub tick number.
        type StorageHubTickGetter: StorageHubTickGetter<TickNumber = BlockNumberFor<Self>>;

        /// The Treasury AccountId.
        /// The account to which:
        /// - The fees for submitting a challenge are transferred.
        /// - The slashed funds are transferred.
        #[pallet::constant]
        type Treasury: Get<Self::AccountId>;

        /// The minimum amount that an account has to deposit to become a storage provider.
        #[pallet::constant]
        type SpMinDeposit: Get<BalanceOf<Self>>;

        /// The amount that a BSP receives as allocation of storage capacity when it deposits SpMinDeposit.
        #[pallet::constant]
        type SpMinCapacity: Get<StorageDataUnit<Self>>;

        /// The slope of the collateral vs storage capacity curve. In other terms, how many tokens a Storage Provider should add as collateral to increase its storage capacity in one unit of StorageDataUnit.
        #[pallet::constant]
        type DepositPerData: Get<BalanceOf<Self>>;

        /// The estimated maximum size of an unknown file.
        ///
        /// Used primarily to slash a Storage Provider when it fails to provide a chunk of data for an unknown file size.
        #[pallet::constant]
        type MaxFileSize: Get<StorageDataUnit<Self>>;

        /// The maximum size of a multiaddress.
        #[pallet::constant]
        type MaxMultiAddressSize: Get<u32>;

        /// The maximum amount of multiaddresses that a Storage Provider can have.
        #[pallet::constant]
        type MaxMultiAddressAmount: Get<u32>;

        /// The maximum number of protocols the MSP can support (at least within the runtime).
        #[pallet::constant]
        type MaxProtocols: Get<u32>;

        /// The amount that an account has to deposit to create a bucket.
        #[pallet::constant]
        type BucketDeposit: Get<BalanceOf<Self>>;

        /// Type that represents the byte limit of a bucket name.
        #[pallet::constant]
        type BucketNameLimit: Get<u32>;

        /// The maximum amount of blocks after which a sign up request expires so the randomness cannot be chosen
        #[pallet::constant]
        type MaxBlocksForRandomness: Get<BlockNumberFor<Self>>;

        /// The minimum amount of blocks between capacity changes for a SP
        #[pallet::constant]
        type MinBlocksBetweenCapacityChanges: Get<BlockNumberFor<Self>>;

        /// The default value of the root of the Merkle Patricia Trie of the runtime
        #[pallet::constant]
        type DefaultMerkleRoot: Get<Self::MerklePatriciaRoot>;

        /// The slash factor deducted from a Storage Provider's deposit for every single storage proof they fail to provide.
        #[pallet::constant]
        type SlashAmountPerMaxFileSize: Get<BalanceOf<Self>>;

        /// Starting reputation weight for a newly registered BSP.
        #[pallet::constant]
        type StartingReputationWeight: Get<Self::ReputationWeightType>;

        /// The amount of blocks that a BSP must wait before being able to sign off, after being signed up.
        ///
        /// This is to prevent BSPs from signing up and off too quickly, thus making it harder for an attacker
        /// to suddenly have a large portion of the total number of BSPs. The reason for this, is that the
        /// attacker would have to lock up a large amount of funds for this period of time.
        #[pallet::constant]
        type BspSignUpLockPeriod: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxCommitmentSize: Get<u32>;

        /// 0-size bucket fixed rate payment stream (i.e. the amount charged as a base
        /// fee for a bucket that doesn't have any files yet)
        #[pallet::constant]
        type ZeroSizeBucketFixedRate: Get<BalanceOf<Self>>;

        /// Trait that has benchmark helpers
        #[cfg(feature = "runtime-benchmarks")]
        type BenchmarkHelpers: crate::benchmarking::BenchmarkHelpers<Self>;

        /// Time-to-live for a provider to top up their deposit to cover a capacity deficit.
        ///
        /// This TTL is used to determine at what point to insert the expiration item in the
        /// [`ProviderTopUpExpirations`] storage which is processed in the `on_idle` hook at
        /// the time when the tick has been reached.
        #[pallet::constant]
        type ProviderTopUpTtl: Get<StorageHubTickNumber<Self>>;

        /// Maximum number of expired items (per type) to clean up in a single block.
        #[pallet::constant]
        type MaxExpiredItemsInBlock: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // Storage:

    /// The mapping from an AccountId that requested to sign up to a tuple of the metadata with type of the request, and the block
    /// number when the request was made.
    ///
    /// This is used for the two-step process of registering: when a user requests to register as a SP (either MSP or BSP),
    /// that request with the metadata and the deposit held is stored here. When the user confirms the sign up, the
    /// request is removed from this storage and the user is registered as a SP.
    ///
    /// This storage is updated in:
    /// - [request_msp_sign_up](crate::dispatchables::request_msp_sign_up) and [request_bsp_sign_up](crate::dispatchables::request_bsp_sign_up), which add a new entry to the map.
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up) and [cancel_sign_up](crate::dispatchables::cancel_sign_up), which remove an existing entry from the map.
    #[pallet::storage]
    pub type SignUpRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, SignUpRequest<T>>;

    /// The mapping from an AccountId to a MainStorageProviderId.
    ///
    /// This is used to get a Main Storage Provider's unique identifier needed to access its metadata.
    ///
    /// This storage is updated in:
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Main Storage Provider.
    /// - [msp_sign_off](crate::dispatchables::msp_sign_off), which removes the corresponding entry from the map.
    #[pallet::storage]
    pub type AccountIdToMainStorageProviderId<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, MainStorageProviderId<T>>;

    /// The mapping from a MainStorageProviderId to a MainStorageProvider.
    ///
    /// This is used to get a Main Storage Provider's metadata.
    /// It returns `None` if the Main Storage Provider ID does not correspond to any registered Main Storage Provider.
    ///
    /// This storage is updated in:
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Main Storage Provider.
    /// - [msp_sign_off](crate::dispatchables::msp_sign_off), which removes the corresponding entry from the map.
    /// - [change_capacity](crate::dispatchables::change_capacity), which changes the entry's `capacity`.
    #[pallet::storage]
    pub type MainStorageProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, MainStorageProviderId<T>, MainStorageProvider<T>>;

    /// The mapping from a BucketId to that bucket's metadata.
    ///
    /// This is used to get a bucket's metadata, such as root, user ID, and MSP ID.
    /// It returns `None` if the Bucket ID does not correspond to any registered bucket.
    ///
    /// This storage is updated in:
    /// - [add_bucket](shp_traits::MutateProvidersInterface::add_bucket), which adds a new entry to the map.
    /// - [change_root_bucket](shp_traits::MutateProvidersInterface::change_root_bucket), which changes the corresponding bucket's root.
    /// - [delete_bucket](shp_traits::MutateProvidersInterface::delete_bucket), which removes the entry of the corresponding bucket.
    #[pallet::storage]
    pub type Buckets<T: Config> = StorageMap<_, Blake2_128Concat, BucketId<T>, Bucket<T>>;

    /// The double mapping from a MainStorageProviderId to a BucketIds.
    ///
    /// This is used to efficiently retrieve the list of buckets that a Main Storage Provider is currently storing.
    ///
    /// This storage is updated in:
    /// - [add_bucket](shp_traits::MutateProvidersInterface::add_bucket)
    /// - [delete_bucket](shp_traits::MutateProvidersInterface::delete_bucket)
    #[pallet::storage]
    pub type MainStorageProviderIdsToBuckets<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        MainStorageProviderId<T>,
        Blake2_128Concat,
        BucketId<T>,
        (),
    >;

    /// The mapping from an AccountId to a BackupStorageProviderId.
    ///
    /// This is used to get a Backup Storage Provider's unique identifier needed to access its metadata.
    ///
    /// This storage is updated in:
    ///
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Backup Storage Provider.
    /// - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which removes the corresponding entry from the map.
    #[pallet::storage]
    pub type AccountIdToBackupStorageProviderId<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BackupStorageProviderId<T>>;

    /// The mapping from a BackupStorageProviderId to a BackupStorageProvider.
    ///
    /// This is used to get a Backup Storage Provider's metadata.
    /// It returns `None` if the Backup Storage Provider ID does not correspond to any registered Backup Storage Provider.
    ///
    /// This storage is updated in:
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds a new entry to the map if the account to confirm is a Backup Storage Provider.
    /// - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which removes the corresponding entry from the map.
    /// - [change_capacity](crate::dispatchables::change_capacity), which changes the entry's `capacity`.
    #[pallet::storage]
    pub type BackupStorageProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, BackupStorageProviderId<T>, BackupStorageProvider<T>>;

    /// The amount of Main Storage Providers that are currently registered in the runtime.
    ///
    /// This is used to keep track of the total amount of MSPs in the system.
    ///
    /// This storage is updated in:
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds one to this storage if the account to confirm is a Main Storage Provider.
    /// - [msp_sign_off](crate::dispatchables::msp_sign_off), which subtracts one from this storage.
    #[pallet::storage]
    pub type MspCount<T: Config> = StorageValue<_, T::SpCount, ValueQuery>;

    /// The amount of Backup Storage Providers that are currently registered in the runtime.
    ///
    /// This is used to keep track of the total amount of BSPs in the system.
    ///
    /// This storage is updated in:
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds one to this storage if the account to confirm is a Backup Storage Provider.
    /// - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which subtracts one from this storage.
    #[pallet::storage]
    pub type BspCount<T: Config> = StorageValue<_, T::SpCount, ValueQuery>;

    /// The total amount of storage capacity all BSPs have.
    ///
    /// This is used to keep track of the total amount of storage capacity all BSPs have in the system, which is also the
    /// total amount of storage capacity that can be used by users if we factor in the replication factor.
    ///
    /// This storage is updated in:
    /// - [confirm_sign_up](crate::dispatchables::confirm_sign_up), which adds the capacity of the registered Storage Provider to this storage if the account to confirm is a Backup Storage Provider.
    /// - [bsp_sign_off](crate::dispatchables::bsp_sign_off), which subtracts the capacity of the Backup Storage Provider to sign off from this storage.
    #[pallet::storage]
    pub type TotalBspsCapacity<T: Config> = StorageValue<_, StorageDataUnit<T>, ValueQuery>;

    /// The total amount of storage capacity of BSPs that is currently in use.
    ///
    /// This is used to keep track of the total amount of storage capacity that is currently in use by users, which is useful for
    /// system metrics and also to calculate the current price of storage.
    #[pallet::storage]
    pub type UsedBspsCapacity<T: Config> = StorageValue<_, StorageDataUnit<T>, ValueQuery>;

    /// The total global reputation weight of all BSPs.
    #[pallet::storage]
    pub type GlobalBspsReputationWeight<T> = StorageValue<_, ReputationWeightType<T>, ValueQuery>;

    /// Double mapping from a [`MainStorageProviderId`] to [`ValueProposition`]s.
    ///
    /// These are applied at the bucket level. Propositions are the price per [`Config::StorageDataUnit`] per block and the
    /// limit of data that can be stored in the bucket.
    #[pallet::storage]
    pub type MainStorageProviderIdsToValuePropositions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        MainStorageProviderId<T>,
        Blake2_128Concat,
        ValuePropIdFor<T>,
        ValueProposition<T>,
        OptionQuery,
    >;

    /// Storage providers currently awaited for to top up their deposit (providers whom have been slashed and as
    /// a result have a capacity deficit, i.e. their capacity is below their used capacity).
    ///
    /// This is primarily used to lookup providers and restrict certain operations while they are in this state.
    ///
    /// Providers can optionally call the `top_up_deposit` during the grace period to top up their held deposit to cover the capacity deficit.
    /// As a result, their provider account would be cleared from this storage.
    ///
    /// The `on_idle` hook will process every provider in this storage and mark them as insolvent.
    /// If a provider is marked as insolvent, the network (e.g users, other providers) can call `issue_storage_request`
    /// with a replication target of 1 to fill a slot with another BSP if the provider who was marked as insolvent is in fact a BSP.
    /// If it was an MSP, the user can decide to move their buckets to another MSP or delete their buckets (as they normally can).
    #[pallet::storage]
    pub type AwaitingTopUpFromProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, StorageProviderId<T>, TopUpMetadata<T>>;

    /// A map of Storage Hub tick numbers to expired provider top up expired items.
    ///
    /// Processed in the `on_idle` hook.
    ///
    /// Provider top up expiration items are ignored and cleared if the provider is not found in the [`AwaitingTopUpFromProviders`] storage.
    /// Providers are removed from [`AwaitingTopUpFromProviders`] storage when they have successfully topped up their deposit.
    /// If they are still part of the [`AwaitingTopUpFromProviders`] storage after the expiration period, they are marked as insolvent.
    #[pallet::storage]
    pub type ProviderTopUpExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        StorageHubTickNumber<T>,
        BoundedVec<StorageProviderId<T>, T::MaxExpiredItemsInBlock>,
        ValueQuery,
    >;

    /// A pointer to the earliest available Storage Hub tick to insert a new provider top up expiration item.
    ///
    /// This should always be greater or equal than `current_sh_tick` + [`Config::ProviderTopUpTtl`].
    #[pallet::storage]
    pub type NextAvailableProviderTopUpExpirationShTick<T: Config> =
        StorageValue<_, StorageHubTickNumber<T>, ValueQuery>;

    /// A pointer to the starting Storage Hub tick number to clean up expired items.
    ///
    /// If this Storage Hub tick is behind the one, the cleanup algorithm in `on_idle` will
    /// attempt to advance this tick pointer as close to or up to the current one. This
    /// will execute provided that there is enough remaining weight to do so.
    #[pallet::storage]
    pub type NextStartingShTickToCleanUp<T: Config> =
        StorageValue<_, StorageHubTickNumber<T>, ValueQuery>;

    /// A map of insolvent providers who have failed to top up their deposit before the end of the expiration.
    ///
    /// Providers are marked insolvent by the `on_idle` hook.
    #[pallet::storage]
    pub type InsolventProviders<T: Config> =
        StorageMap<_, Blake2_128Concat, StorageProviderId<T>, ()>;

    // Events & Errors:

    /// # Event Encoding/Decoding Stability
    ///
    /// All event variants use explicit `#[codec(index = N)]` to ensure stable SCALE encoding/decoding
    /// across runtime upgrades.
    ///
    /// These indices must NEVER be changed or reused. Any breaking changes to errors must be
    /// introduced as new variants (append-only) to ensure backward and forward compatibility.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a Main Storage Provider has requested to sign up successfully. Provides information about
        /// that MSP's account id, its multiaddresses, the total data it can store according to its stake, and its value proposition.
        #[codec(index = 0)]
        MspRequestSignUpSuccess {
            who: T::AccountId,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
        },

        /// Event emitted when a Main Storage Provider has confirmed its sign up successfully. Provides information about
        /// that MSP's account id, the total data it can store according to its stake, its multiaddress, and its value proposition.
        #[codec(index = 1)]
        MspSignUpSuccess {
            who: T::AccountId,
            msp_id: MainStorageProviderId<T>,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
            value_prop: ValuePropositionWithId<T>,
        },

        /// Event emitted when a Backup Storage Provider has requested to sign up successfully. Provides information about
        /// that BSP's account id, its multiaddresses, and the total data it can store according to its stake.
        #[codec(index = 2)]
        BspRequestSignUpSuccess {
            who: T::AccountId,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
        },

        /// Event emitted when a Backup Storage Provider has confirmed its sign up successfully. Provides information about
        /// that BSP's account id, the initial root of the Merkle Patricia Trie that it stores, the total data it can store
        /// according to its stake, and its multiaddress.
        #[codec(index = 3)]
        BspSignUpSuccess {
            who: T::AccountId,
            bsp_id: BackupStorageProviderId<T>,
            root: MerklePatriciaRoot<T>,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
        },

        /// Event emitted when a sign up request has been canceled successfully. Provides information about
        /// the account id of the user that canceled the request.
        #[codec(index = 4)]
        SignUpRequestCanceled { who: T::AccountId },

        /// Event emitted when a Main Storage Provider has signed off successfully. Provides information about
        /// that MSP's account id.
        #[codec(index = 5)]
        MspSignOffSuccess {
            who: T::AccountId,
            msp_id: MainStorageProviderId<T>,
        },

        /// Event emitted when a Backup Storage Provider has signed off successfully. Provides information about
        /// that BSP's account id.
        #[codec(index = 6)]
        BspSignOffSuccess {
            who: T::AccountId,
            bsp_id: BackupStorageProviderId<T>,
        },

        /// Event emitted when a SP has changed its capacity successfully. Provides information about
        /// that SP's account id, its old total data that could store, and the new total data.
        #[codec(index = 7)]
        CapacityChanged {
            who: T::AccountId,
            provider_id: StorageProviderId<T>,
            old_capacity: StorageDataUnit<T>,
            new_capacity: StorageDataUnit<T>,
            next_block_when_change_allowed: BlockNumberFor<T>,
        },

        /// Event emitted when a SP has been slashed.
        #[codec(index = 8)]
        Slashed {
            provider_id: ProviderIdFor<T>,
            amount: BalanceOf<T>,
        },

        /// Event emitted when a provider has been slashed and they have reached a capacity deficit (i.e. the provider's capacity fell below their used capacity)
        /// signalling the end of the grace period since an automatic top up could not be performed due to insufficient free balance.
        #[codec(index = 9)]
        AwaitingTopUp {
            provider_id: ProviderIdFor<T>,
            top_up_metadata: TopUpMetadata<T>,
        },

        /// Event emitted when an SP has topped up its deposit based on slash amount.
        #[codec(index = 10)]
        TopUpFulfilled {
            provider_id: ProviderIdFor<T>,
            /// Amount that the provider has added to the held `StorageProviderDeposit` to pay for the outstanding slash amount.
            amount: BalanceOf<T>,
        },

        /// Event emitted when the account ID of a provider that has just been marked as insolvent can't be found in storage.
        #[codec(index = 11)]
        FailedToGetOwnerAccountOfInsolventProvider { provider_id: ProviderIdFor<T> },

        /// Event emitted when there's an error slashing the now insolvent provider.
        #[codec(index = 12)]
        FailedToSlashInsolventProvider {
            provider_id: ProviderIdFor<T>,
            amount_to_slash: BalanceOf<T>,
            error: DispatchError,
        },

        /// Event emitted when there's an error stopping all cycles for an insolvent Backup Storage Provider.
        #[codec(index = 13)]
        FailedToStopAllCyclesForInsolventBsp {
            provider_id: ProviderIdFor<T>,
            error: DispatchError,
        },

        /// Event emitted when there was an inconsistency error and the provider was found in `ProviderTopUpExpirations`
        /// for a tick that wasn't actually when its top up expired, and when trying to insert it with the actual
        /// expiration tick in `ProviderTopUpExpirations` the append failed.
        ///
        /// The result of this is that the provider's top up expiration will be reinserted at the correct expiration tick based on the
        /// `TopUpMetadata` found in `AwaitingTopUpFromProviders` storage.
        #[codec(index = 14)]
        FailedToInsertProviderTopUpExpiration {
            provider_id: ProviderIdFor<T>,
            expiration_tick: StorageHubTickNumber<T>,
        },

        /// Event emitted when a provider has been marked as insolvent.
        ///
        /// This happens when the provider hasn't topped up their deposit within the grace period after being slashed
        /// and they have a capacity deficit (i.e. their capacity based on their stake is below their used capacity by the files it stores).
        #[codec(index = 15)]
        ProviderInsolvent { provider_id: ProviderIdFor<T> },

        /// Event emitted when the provider that has been marked as insolvent was a MSP. It notifies the users of that MSP
        /// the buckets that it was holding, so they can take appropriate measures.
        #[codec(index = 16)]
        BucketsOfInsolventMsp {
            msp_id: ProviderIdFor<T>,
            buckets: Vec<BucketId<T>>,
        },

        /// Event emitted when a bucket's root has been changed.
        #[codec(index = 17)]
        BucketRootChanged {
            bucket_id: BucketId<T>,
            old_root: MerklePatriciaRoot<T>,
            new_root: MerklePatriciaRoot<T>,
        },

        /// Event emitted when a Provider has added a new MultiAddress to its account.
        #[codec(index = 18)]
        MultiAddressAdded {
            provider_id: ProviderIdFor<T>,
            new_multiaddress: MultiAddress<T>,
        },

        /// Event emitted when a Provider has removed a MultiAddress from its account.
        #[codec(index = 19)]
        MultiAddressRemoved {
            provider_id: ProviderIdFor<T>,
            removed_multiaddress: MultiAddress<T>,
        },

        /// Event emitted when an MSP adds a new value proposition.
        #[codec(index = 20)]
        ValuePropAdded {
            msp_id: MainStorageProviderId<T>,
            value_prop_id: ValuePropIdFor<T>,
            value_prop: ValueProposition<T>,
        },

        /// Event emitted when an MSP's value proposition is made unavailable.
        #[codec(index = 21)]
        ValuePropUnavailable {
            msp_id: MainStorageProviderId<T>,
            value_prop_id: ValuePropIdFor<T>,
        },

        /// Event emitted when an MSP has been deleted.
        #[codec(index = 22)]
        MspDeleted { provider_id: ProviderIdFor<T> },

        /// Event emitted when a BSP has been deleted.
        #[codec(index = 23)]
        BspDeleted { provider_id: ProviderIdFor<T> },
    }

    /// # Error Encoding/Decoding Stability
    ///
    /// All error variants use explicit `#[codec(index = N)]` to ensure stable SCALE encoding/decoding
    /// across runtime upgrades.
    ///
    /// These indices must NEVER be changed or reused. Any breaking changes to errors must be
    /// introduced as new variants (append-only) to ensure backward and forward compatibility.
    #[pallet::error]
    pub enum Error<T> {
        // Sign up errors:
        /// Error thrown when a user tries to sign up as a SP but is already registered as a MSP or BSP.
        #[codec(index = 0)]
        AlreadyRegistered,
        /// Error thrown when a user tries to confirm a sign up that was not requested previously.
        #[codec(index = 1)]
        SignUpNotRequested,
        /// Error thrown when a user tries to request to sign up when it already has a sign up request pending.
        #[codec(index = 2)]
        SignUpRequestPending,
        /// Error thrown when a user tries to sign up without any multiaddress.
        #[codec(index = 3)]
        NoMultiAddress,
        /// Error thrown when a user tries to sign up as a SP but any of the provided multiaddresses is invalid.
        #[codec(index = 4)]
        InvalidMultiAddress,
        /// Error thrown when a user tries to sign up or change its capacity to store less storage than the minimum required by the runtime.
        #[codec(index = 5)]
        StorageTooLow,

        // Deposit errors:
        /// Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a SP or changing its capacity.
        #[codec(index = 6)]
        NotEnoughBalance,
        /// Error thrown when the runtime cannot hold the required deposit from the account to register it as a SP or change its capacity.
        #[codec(index = 7)]
        CannotHoldDeposit,

        // Sign off errors:
        /// Error thrown when a user tries to sign off as a SP but still has used storage.
        #[codec(index = 8)]
        StorageStillInUse,
        /// Error thrown when a user tries to sign off as a BSP but the sign off period has not passed yet.
        #[codec(index = 9)]
        SignOffPeriodNotPassed,

        // Randomness errors:
        /// Error thrown when a user tries to confirm a sign up but the randomness is too fresh to be used yet.
        #[codec(index = 10)]
        RandomnessNotValidYet,
        /// Error thrown when a user tries to confirm a sign up but too much time has passed since the request.
        #[codec(index = 11)]
        SignUpRequestExpired,

        // Capacity change errors:
        /// Error thrown when a user tries to change its capacity to less than its used storage.
        #[codec(index = 12)]
        NewCapacityLessThanUsedStorage,
        /// Error thrown when a user tries to change its capacity to the same value it already has.
        #[codec(index = 13)]
        NewCapacityEqualsCurrentCapacity,
        /// Error thrown when a user tries to change its capacity to zero (there are specific extrinsics to sign off as a SP).
        #[codec(index = 14)]
        NewCapacityCantBeZero,
        /// Error thrown when a SP tries to change its capacity but it has not been enough time since the last time it changed it.
        #[codec(index = 15)]
        NotEnoughTimePassed,
        /// Error thrown when a SP tries to change its capacity but the new capacity is not enough to store the used storage.
        #[codec(index = 16)]
        NewUsedCapacityExceedsStorageCapacity,
        /// Deposit too low to determine capacity.
        #[codec(index = 17)]
        DepositTooLow,

        // General errors:
        /// Error thrown when a user tries to interact as a SP but is not registered as a MSP or BSP.
        #[codec(index = 18)]
        NotRegistered,
        /// Error thrown when trying to get a root from a MSP without passing a User ID.
        #[codec(index = 19)]
        NoUserId,
        /// Error thrown when trying to get a root from a MSP without passing a Bucket ID.
        #[codec(index = 20)]
        NoBucketId,
        /// Error thrown when a user has a SP ID assigned to it but the SP data does not exist in storage (Inconsistency error).
        #[codec(index = 21)]
        SpRegisteredButDataNotFound,
        /// Error thrown when a bucket ID is not found in storage.
        #[codec(index = 22)]
        BucketNotFound,
        /// Error thrown when a bucket ID already exists in storage.
        #[codec(index = 23)]
        BucketAlreadyExists,
        /// Bucket cannot be deleted because it is not empty.
        #[codec(index = 24)]
        BucketNotEmpty,
        /// Error thrown when, after moving all buckets of a MSP when removing it from the system, the amount doesn't match the expected value.
        #[codec(index = 25)]
        BucketsMovedAmountMismatch,
        /// Error thrown when a bucket ID could not be added to the list of buckets of a MSP.
        #[codec(index = 26)]
        AppendBucketToMspFailed,
        /// Error thrown when an attempt was made to slash an unslashable Storage Provider.
        #[codec(index = 27)]
        ProviderNotSlashable,
        /// Error thrown when a provider attempts to top up their deposit when not required.
        #[codec(index = 28)]
        TopUpNotRequired,
        /// Error thrown when an operation requires an MSP to be storing the bucket.
        #[codec(index = 29)]
        BucketMustHaveMspForOperation,
        /// Error thrown when a Provider tries to add a new MultiAddress to its account but it already has the maximum amount of multiaddresses.
        #[codec(index = 30)]
        MultiAddressesMaxAmountReached,
        /// Error thrown when a Provider tries to delete a MultiAddress from its account but it does not have that MultiAddress.
        #[codec(index = 31)]
        MultiAddressNotFound,
        /// Error thrown when a Provider tries to add a new MultiAddress to its account but it already exists.
        #[codec(index = 32)]
        MultiAddressAlreadyExists,
        /// Error thrown when a Provider tries to remove the last MultiAddress from its account.
        #[codec(index = 33)]
        LastMultiAddressCantBeRemoved,
        /// Error thrown when the value proposition id is not found.
        #[codec(index = 34)]
        ValuePropositionNotFound,
        /// Error thrown when value proposition under a given id already exists.
        #[codec(index = 35)]
        ValuePropositionAlreadyExists,
        /// Error thrown when a value proposition is not available.
        #[codec(index = 36)]
        ValuePropositionNotAvailable,
        /// Error thrown when a MSP tries to deactivate its last value proposition.
        #[codec(index = 37)]
        CantDeactivateLastValueProp,
        /// Error thrown when, after deleting all value propositions of a MSP when removing it from the system, the amount doesn't match the expected value.
        #[codec(index = 38)]
        ValuePropositionsDeletedAmountMismatch,
        /// Error thrown when a fixed payment stream is not found.
        #[codec(index = 39)]
        FixedRatePaymentStreamNotFound,
        /// Error thrown when changing the MSP of a bucket to the same assigned MSP.
        #[codec(index = 40)]
        MspAlreadyAssignedToBucket,
        /// Error thrown when a user exceeded the bucket data limit based on the associated value proposition.
        #[codec(index = 41)]
        BucketSizeExceedsLimit,
        /// Error thrown when a bucket has no value proposition.
        #[codec(index = 42)]
        BucketHasNoValueProposition,
        /// Congratulations, you either lived long enough or were born late enough to see this error.
        #[codec(index = 43)]
        MaxBlockNumberReached,
        /// Operation not allowed for insolvent provider
        #[codec(index = 44)]
        OperationNotAllowedForInsolventProvider,
        /// Failed to delete a provider due to conditions not being met.
        ///
        /// Call `can_delete_provider` runtime API to check if the provider can be deleted.
        #[codec(index = 45)]
        DeleteProviderConditionsNotMet,
        /// Cannot stop BSP cycles without a default root
        #[codec(index = 46)]
        CannotStopCycleWithNonDefaultRoot,
        /// An operation dedicated to BSPs only
        #[codec(index = 47)]
        BspOnlyOperation,
        /// An operation dedicated to MSPs only
        #[codec(index = 48)]
        MspOnlyOperation,

        // `MutateChallengeableProvidersInterface` errors:
        /// Error thrown when failing to decode the metadata from a received trie value that was removed.
        #[codec(index = 49)]
        InvalidEncodedFileMetadata,
        /// Error thrown when failing to decode the owner Account ID from the received metadata.
        #[codec(index = 50)]
        InvalidEncodedAccountId,
        /// Error thrown when trying to update a payment stream that does not exist.
        #[codec(index = 51)]
        PaymentStreamNotFound,
    }

    /// This enum holds the HoldReasons for this pallet, allowing the runtime to identify each held balance with different reasons separately
    ///
    /// This allows us to hold tokens and be able to identify in the future that those held tokens were
    /// held because of this pallet
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Deposit that a Storage Provider has to pay to be registered as such
        StorageProviderDeposit,
        /// Deposit that a user has to pay to create a bucket
        BucketDeposit,
        // Only for testing, another unrelated hold reason
        #[cfg(test)]
        AnotherUnrelatedHold,
    }

    /// Dispatchables (extrinsics) exposed by this pallet
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Dispatchable extrinsic that allows users to request to sign up as a Main Storage Provider.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to sign up as a Main Storage Provider.
        ///
        /// Funds proportional to the capacity requested are reserved (held) from the account.
        ///
        /// Parameters:
        /// - `capacity`: The total amount of data that the Main Storage Provider will be able to store.
        /// - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
        /// [Multiaddr spec](https://github.com/multiformats/multiaddr))
        /// - `value_prop`: The value proposition that the signer will provide as a Main Storage Provider to
        /// users and wants to register on-chain. It could be data limits, communication protocols to access the user's
        /// data, and more.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is not already registered as either a MSP or BSP
        /// 3. Check that the multiaddress is valid
        /// 4. Check that the data to be stored is greater than the minimum required by the runtime.
        /// 5. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 6. Check that the signer has enough funds to pay the deposit
        /// 7. Hold the deposit from the signer
        /// 8. Update the Sign Up Requests storage to add the signer as requesting to sign up as a MSP
        ///
        /// Emits `MspRequestSignUpSuccess` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::request_msp_sign_up())]
        pub fn request_msp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageDataUnit<T>,
            multiaddresses: Multiaddresses<T>,
            value_prop_price_per_giga_unit_of_data_per_block: BalanceOf<T>,
            commitment: Commitment<T>,
            value_prop_max_data_limit: StorageDataUnit<T>,
            payment_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Set up a structure with the information of the new MSP
            let msp_info = MainStorageProvider {
                capacity,
                capacity_used: StorageDataUnit::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                amount_of_buckets: T::BucketCount::zero(),
                amount_of_value_props: 0u32,
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
                owner_account: who.clone(),
                payment_account,
                sign_up_block: frame_system::Pallet::<T>::block_number(),
            };

            // Sign up the new MSP (if possible), updating storage
            Self::do_request_msp_sign_up(MainStorageProviderSignUpRequest {
                msp_info,
                value_prop: ValueProposition::<T>::new(
                    value_prop_price_per_giga_unit_of_data_per_block,
                    commitment,
                    value_prop_max_data_limit,
                ),
            })?;

            // Emit the corresponding event
            Self::deposit_event(Event::MspRequestSignUpSuccess {
                who,
                multiaddresses,
                capacity,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to sign up as a Backup Storage Provider.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to sign up as a Backup Storage Provider.
        ///
        /// Funds proportional to the capacity requested are reserved (held) from the account.
        ///
        /// Parameters:
        /// - `capacity`: The total amount of data that the Backup Storage Provider will be able to store.
        /// - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
        /// [Multiaddr spec](https://github.com/multiformats/multiaddr))
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is not already registered as either a MSP or BSP
        /// 3. Check that the multiaddress is valid
        /// 4. Check that the data to be stored is greater than the minimum required by the runtime
        /// 5. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
        /// 6. Check that the signer has enough funds to pay the deposit
        /// 7. Hold the deposit from the signer
        /// 8. Update the Sign Up Requests storage to add the signer as requesting to sign up as a BSP
        ///
        /// Emits `BspRequestSignUpSuccess` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::request_bsp_sign_up())]
        pub fn request_bsp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageDataUnit<T>,
            multiaddresses: Multiaddresses<T>,
            payment_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Set up a structure with the information of the new BSP
            let bsp_info = BackupStorageProvider {
                capacity,
                capacity_used: StorageDataUnit::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                root: T::DefaultMerkleRoot::get(),
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
                owner_account: who.clone(),
                payment_account,
                reputation_weight: T::StartingReputationWeight::get(),
                sign_up_block: frame_system::Pallet::<T>::block_number(),
            };

            // Sign up the new BSP (if possible), updating storage
            Self::do_request_bsp_sign_up(&bsp_info)?;

            // Emit the corresponding event
            Self::deposit_event(Event::BspRequestSignUpSuccess {
                who,
                multiaddresses,
                capacity,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to confirm their sign up as a Storage Provider, either MSP or BSP.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that requested to sign up as a Storage Provider, except when providing a
        /// `provider_account` parameter, in which case the origin can be any account.
        ///
        /// Parameters:
        /// - `provider_account`: The account that requested to sign up as a Storage Provider. If not provided, the signer
        /// will be considered the account that requested to sign up.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed
        /// 2. Check that the account received has requested to register as a SP
        /// 3. Check that the current randomness is sufficiently fresh to be used as a salt for that request
        /// 4. Check that the request has not expired
        /// 5. Register the signer as a MSP or BSP with the data provided in the request
        ///
        /// Emits `MspSignUpSuccess` or `BspSignUpSuccess` event when successful, depending on the type of sign up.
        ///
        /// Notes:
        /// - This extrinsic could be called by the user itself or by a third party
        /// - The deposit that the user has to pay to register as a SP is held when the user requests to register as a SP
        /// - If this extrinsic is successful, it will be free for the caller, to incentive state de-bloating
        #[pallet::call_index(2)]
        #[pallet::weight({
			T::WeightInfo::confirm_sign_up_bsp()
				.max(T::WeightInfo::confirm_sign_up_msp())
		})]
        pub fn confirm_sign_up(
            origin: OriginFor<T>,
            provider_account: Option<T::AccountId>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage and emit event
            // We emit the event in the interior logic to not have to check again which type of sign up it is outside of it
            match provider_account {
                Some(provider_account) => Self::do_confirm_sign_up(&provider_account)?,
                None => Self::do_confirm_sign_up(&who)?,
            }

            // Return a successful DispatchResult.
            Ok(())
        }

        /// Dispatchable extrinsic that allows a user with a pending Sign Up Request to cancel it, getting the deposit back.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that requested to sign up as a Storage Provider.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer has requested to sign up as a SP
        /// 3. Delete the request from the Sign Up Requests storage
        /// 4. Return the deposit to the signer
        ///
        /// Emits `SignUpRequestCanceled` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::cancel_sign_up())]
        pub fn cancel_sign_up(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_cancel_sign_up(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::SignUpRequestCanceled { who });

            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to sign off as a Main Storage Provider.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to sign off as a Main Storage Provider.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a MSP
        /// 3. Check that the MSP has no storage assigned to it (no buckets or data used by it)
        /// 4. Update the MSPs storage, removing the signer as an MSP
        /// 5. Return the deposit to the signer
        /// 6. Decrement the storage that holds total amount of MSPs currently in the system
        ///
        /// Emits `MspSignOffSuccess` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight({
			match MainStorageProviders::<T>::get(&msp_id) {
				Some(msp) => T::WeightInfo::msp_sign_off(msp.amount_of_value_props)
								.saturating_add(T::DbWeight::get().reads(1)),
				None => T::WeightInfo::msp_sign_off(0)
							.saturating_add(T::DbWeight::get().reads(1)),
			}
		})]
        pub fn msp_sign_off(
            origin: OriginFor<T>,
            msp_id: ProviderIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_msp_sign_off(&who, msp_id)?;

            // Emit the corresponding event
            Self::deposit_event(Event::MspSignOffSuccess { who, msp_id });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to sign off as a Backup Storage Provider.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to sign off as a Backup Storage Provider.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a BSP
        /// 3. Check that the BSP has no storage assigned to it
        /// 4. Update the BSPs storage, removing the signer as an BSP
        /// 5. Update the total capacity of all BSPs, removing the capacity of the signer
        /// 6. Return the deposit to the signer
        /// 7. Decrement the storage that holds total amount of BSPs currently in the system
        ///
        /// Emits `BspSignOffSuccess` event when successful.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::bsp_sign_off())]
        pub fn bsp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let bsp_id = Self::do_bsp_sign_off(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::BspSignOffSuccess { who, bsp_id });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows users to change their amount of stored data
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to change its capacity.
        ///
        /// Parameters:
        /// - `new_capacity`: The new total amount of data that the Storage Provider wants to be able to store.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a SP
        /// 3. Check that enough time has passed since the last time the SP changed its capacity
        /// 4. Check that the new capacity is greater than the minimum required by the runtime
        /// 5. Check that the new capacity is greater than the data used by this SP
        /// 6. Calculate the new deposit needed for this new capacity
        /// 7. Check to see if the new deposit needed is greater or less than the current deposit
        /// 	a. If the new deposit is greater than the current deposit:
        /// 		i. Check that the signer has enough funds to pay this extra deposit
        /// 		ii. Hold the extra deposit from the signer
        /// 	b. If the new deposit is less than the current deposit, return the held difference to the signer
        /// 7. Update the SPs storage to change the total data
        /// 8. If user is a BSP, update the total capacity of the network (sum of all capacities of BSPs)
        ///
        /// Emits `CapacityChanged` event when successful.
        #[pallet::call_index(6)]
        #[pallet::weight({
			let weight_msp_less_deposit = T::WeightInfo::change_capacity_msp_less_deposit();
			let weight_msp_more_deposit = T::WeightInfo::change_capacity_msp_more_deposit();
			let weight_bsp_less_deposit = T::WeightInfo::change_capacity_bsp_less_deposit();
			let weight_bsp_more_deposit = T::WeightInfo::change_capacity_bsp_more_deposit();
			weight_msp_less_deposit
				.max(weight_msp_more_deposit)
				.max(weight_bsp_less_deposit)
				.max(weight_bsp_more_deposit)
		})]
        pub fn change_capacity(
            origin: OriginFor<T>,
            new_capacity: StorageDataUnit<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let (provider_id, old_capacity) = Self::do_change_capacity(&who, new_capacity)?;

            // Emit the corresponding event
            Self::deposit_event(Event::CapacityChanged {
                who,
                provider_id,
                old_capacity,
                new_capacity,
                next_block_when_change_allowed: frame_system::Pallet::<T>::block_number()
                    + T::MinBlocksBetweenCapacityChanges::get(),
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic only callable by an MSP that allows it to add a value proposition to its service
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to add a value proposition.
        ///
        /// Emits `ValuePropAdded` event when successful.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::add_value_prop())]
        pub fn add_value_prop(
            origin: OriginFor<T>,
            price_per_giga_unit_of_data_per_block: BalanceOf<T>,
            commitment: Commitment<T>,
            bucket_data_limit: StorageDataUnit<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let (msp_id, value_prop) = Self::do_add_value_prop(
                &who,
                price_per_giga_unit_of_data_per_block,
                commitment,
                bucket_data_limit,
            )?;

            // Emit event
            Self::deposit_event(Event::ValuePropAdded {
                msp_id,
                value_prop_id: value_prop.derive_id(),
                value_prop,
            });

            Ok(().into())
        }

        /// Dispatchable extrinsic only callable by an MSP that allows it to make a value proposition unavailable.
        ///
        /// This operation cannot be reversed. You can only add new value propositions.
        /// This will not affect existing buckets which are using this value proposition.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::make_value_prop_unavailable())]
        pub fn make_value_prop_unavailable(
            origin: OriginFor<T>,
            value_prop_id: ValuePropIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let msp_id = Self::do_make_value_prop_unavailable(&who, value_prop_id)?;

            // Emit event
            Self::deposit_event(Event::ValuePropUnavailable {
                msp_id,
                value_prop_id,
            });

            Ok(().into())
        }

        /// Dispatchable extrinsic that allows BSPs and MSPs to add a new multiaddress to their account.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to add a new multiaddress.
        ///
        /// Parameters:
        /// - `new_multiaddress`: The new multiaddress that the signer wants to add to its account.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a MSP or BSP.
        /// 3. Check that the Provider has not reached the maximum amount of multiaddresses.
        /// 4. Check that the multiaddress is valid (size and any other relevant checks). TODO: Implement this.
        /// 5. Update the Provider's storage to add the multiaddress.
        ///
        /// Emits `MultiAddressAdded` event when successful.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::add_multiaddress())]
        pub fn add_multiaddress(
            origin: OriginFor<T>,
            new_multiaddress: MultiAddress<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let provider_id = Self::do_add_multiaddress(&who, &new_multiaddress)?;

            // Emit the corresponding event
            Self::deposit_event(Event::MultiAddressAdded {
                provider_id,
                new_multiaddress,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows BSPs and MSPs to remove an existing multiaddress from their account.
        ///
        /// The dispatch origin for this call must be Signed.
        /// The origin must be the account that wants to remove a multiaddress.
        ///
        /// Parameters:
        /// - `multiaddress`: The multiaddress that the signer wants to remove from its account.
        ///
        /// This extrinsic will perform the following checks and logic:
        /// 1. Check that the extrinsic was signed and get the signer.
        /// 2. Check that the signer is registered as a MSP or BSP.
        /// 3. Check that the multiaddress exists in the Provider's account.
        /// 4. Update the Provider's storage to remove the multiaddress.
        ///
        /// Emits `MultiAddressRemoved` event when successful.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::remove_multiaddress())]
        pub fn remove_multiaddress(
            origin: OriginFor<T>,
            multiaddress: MultiAddress<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let provider_id = Self::do_remove_multiaddress(&who, &multiaddress)?;

            // Emit the corresponding event
            Self::deposit_event(Event::MultiAddressRemoved {
                provider_id,
                removed_multiaddress: multiaddress,
            });

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows to forcefully and automatically sign up a Main Storage Provider.
        ///
        /// The dispatch origin for this call must be Root.
        /// The `who` parameter is the account that wants to sign up as a Main Storage Provider.
        ///
        /// Funds proportional to the capacity requested are reserved (held) from the account passed as the `who` parameter.
        ///
        /// Parameters:
        /// - `who`: The account that wants to sign up as a Main Storage Provider.
        /// - `msp_id`: The Main Storage Provider ID that the account passed as the `who` parameter is requesting to sign up as.
        /// - `capacity`: The total amount of data that the Main Storage Provider will be able to store.
        /// - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
        /// [Multiaddr spec](https://github.com/multiformats/multiaddr))
        /// - `value_prop`: The value proposition that the signer will provide as a Main Storage Provider to
        /// users and wants to register on-chain. It could be data limits, communication protocols to access the user's
        /// data, and more.
        ///
        /// This extrinsic will perform the steps of:
        /// 1. [request_msp_sign_up](crate::dispatchables::request_msp_sign_up)
        /// 2. [confirm_sign_up](crate::dispatchables::confirm_sign_up)
        ///
        /// Emits `MspRequestSignUpSuccess` and `MspSignUpSuccess` events when successful.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::force_msp_sign_up())]
        pub fn force_msp_sign_up(
            origin: OriginFor<T>,
            who: T::AccountId,
            msp_id: MainStorageProviderId<T>,
            capacity: StorageDataUnit<T>,
            multiaddresses: Multiaddresses<T>,
            value_prop_price_per_giga_unit_of_data_per_block: BalanceOf<T>,
            commitment: Commitment<T>,
            value_prop_max_data_limit: StorageDataUnit<T>,
            payment_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was sent with root origin.
            ensure_root(origin)?;

            // Set up a structure with the information of the new MSP
            let msp_info = MainStorageProvider {
                capacity,
                capacity_used: StorageDataUnit::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                amount_of_buckets: T::BucketCount::zero(),
                amount_of_value_props: 0u32,
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
                owner_account: who.clone(),
                payment_account,
                sign_up_block: frame_system::Pallet::<T>::block_number(),
            };

            let sign_up_request = MainStorageProviderSignUpRequest {
                msp_info,
                value_prop: ValueProposition::<T>::new(
                    value_prop_price_per_giga_unit_of_data_per_block,
                    commitment,
                    value_prop_max_data_limit,
                ),
            };

            // Sign up the new MSP (if possible), updating storage
            Self::do_request_msp_sign_up(sign_up_request.clone())?;

            // Emit the corresponding event
            Self::deposit_event(Event::MspRequestSignUpSuccess {
                who: who.clone(),
                multiaddresses,
                capacity,
            });

            // Confirm the sign up of the account as a Main Storage Provider with the given ID
            Self::do_msp_sign_up(
                &who,
                msp_id,
                sign_up_request,
                frame_system::Pallet::<T>::block_number(),
            )?;

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic that allows to forcefully and automatically sing up a Backup Storage Provider.
        ///
        /// The dispatch origin for this call must be Root.
        /// The `who` parameter is the account that wants to sign up as a Backup Storage Provider.
        ///
        /// Funds proportional to the capacity requested are reserved (held) from the account passed as the `who` parameter.
        ///
        /// Parameters:
        /// - `who`: The account that wants to sign up as a Backup Storage Provider.
        /// - `bsp_id`: The Backup Storage Provider ID that the account passed as the `who` parameter is requesting to sign up as.
        /// - `capacity`: The total amount of data that the Backup Storage Provider will be able to store.
        /// - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
        /// [Multiaddr spec](https://github.com/multiformats/multiaddr))
        ///
        /// This extrinsic will perform the steps of:
        /// 1. [request_bsp_sign_up](crate::dispatchables::request_bsp_sign_up)
        /// 2. [confirm_sign_up](crate::dispatchables::confirm_sign_up)
        ///
        /// Emits `BspRequestSignUpSuccess` and `BspSignUpSuccess` events when successful.
        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::force_bsp_sign_up())]
        pub fn force_bsp_sign_up(
            origin: OriginFor<T>,
            who: T::AccountId,
            bsp_id: BackupStorageProviderId<T>,
            capacity: StorageDataUnit<T>,
            multiaddresses: Multiaddresses<T>,
            payment_account: T::AccountId,
            weight: Option<ReputationWeightType<T>>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was sent with root origin.
            ensure_root(origin)?;

            // Set up a structure with the information of the new BSP
            let bsp_info = BackupStorageProvider {
                capacity,
                capacity_used: StorageDataUnit::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                root: T::DefaultMerkleRoot::get(),
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
                owner_account: who.clone(),
                payment_account,
                reputation_weight: weight.unwrap_or(T::StartingReputationWeight::get()),
                sign_up_block: frame_system::Pallet::<T>::block_number(),
            };

            // Sign up the new BSP (if possible), updating storage
            Self::do_request_bsp_sign_up(&bsp_info)?;

            // Emit the corresponding event
            Self::deposit_event(Event::BspRequestSignUpSuccess {
                who: who.clone(),
                multiaddresses,
                capacity,
            });

            // Confirm the sign up of the account as a Backup Storage Provider with the given ID
            Self::do_bsp_sign_up(
                &who,
                bsp_id,
                &bsp_info,
                frame_system::Pallet::<T>::block_number(),
            )?;

            // Return a successful DispatchResultWithPostInfo
            Ok(().into())
        }

        /// Dispatchable extrinsic to slash a _slashable_ Storage Provider.
        ///
        /// A Storage Provider is _slashable_ iff it has failed to respond to challenges for providing proofs of storage.
        /// In the context of the StorageHub protocol, the proofs-dealer pallet marks a Storage Provider as _slashable_ when it fails to respond to challenges.
        ///
        /// This is a free operation to incentivise the community to slash misbehaving providers.
        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::slash_without_awaiting_top_up().max(T::WeightInfo::slash_with_awaiting_top_up()))]
        pub fn slash(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was sent with root origin.
            ensure_signed(origin)?;

            Self::do_slash(&provider_id)?;

            // Return a successful DispatchResultWithPostInfo.
            // If the extrinsic executed correctly and the Provider was slashed, the execution fee is refunded.
            // This is to incentivise the community to slash misbehaving providers.
            Ok(Pays::No.into())
        }

        /// Dispatchable extrinsic to top-up the deposit of a Storage Provider.
        ///
        /// The dispatch origin for this call must be signed.
        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::top_up_deposit())]
        pub fn top_up_deposit(origin: OriginFor<T>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            Self::do_top_up_deposit(&who)?;

            Ok(())
        }

        /// Delete a provider from the system.
        ///
        /// This can only be done if the following conditions are met:
        /// - The provider is insolvent.
        /// - The provider has no active payment streams.
        ///
        /// This is a free operation and can be called by anyone with a signed transaction.
        ///
        /// You can utilize the runtime API `can_delete_provider` to check if a provider can be deleted
        /// to automate the process.
        ///
        /// Emits `MspDeleted` or `BspDeleted` event when successful.
        ///
        /// This operation is free if successful to encourage the community to delete insolvent providers,
        /// debloating the state.
        #[pallet::call_index(15)]
        #[pallet::weight({
			let weight_required = if let Some(msp) = MainStorageProviders::<T>::get(provider_id) {
				T::WeightInfo::delete_provider_msp(msp.amount_of_value_props, msp.amount_of_buckets.try_into().unwrap_or(u32::MAX))
			} else {
				T::WeightInfo::delete_provider_bsp()
			};

			weight_required.saturating_add(T::DbWeight::get().reads(1))
		})]
        pub fn delete_provider(
            origin: OriginFor<T>,
            provider_id: ProviderIdFor<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed.
            ensure_signed(origin)?;

            Self::do_delete_provider(&provider_id)?;

            // Return a successful DispatchResultWithPostInfo.
            // If the extrinsic executed correctly and the Provider was deleted, the execution fee is refunded.
            // This is to incentivise the community to delete insolvent providers, debloating state.
            Ok(Pays::No.into())
        }

        /// BSP operation to stop all of your automatic cycles.
        ///
        /// This includes:
        ///
        /// - Commit reveal randomness cycle
        /// - Proof challenge cycle
        ///
        /// If you are an BSP, the only requirement that must be met is that your root is the default one (an empty root).
        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::stop_all_cycles())]
        pub fn stop_all_cycles(origin: OriginFor<T>) -> DispatchResult {
            // Check that the extrinsic was signed.
            let who = ensure_signed(origin)?;

            Self::do_stop_all_cycles(&who)?;

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        u32: TryFrom<BlockNumberFor<T>>,
    {
        fn on_idle(_: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            let mut meter = WeightMeter::with_limit(remaining_weight);

            // If there's enough weight to at least read the current tick number, do it and proceed.
            if meter.can_consume(T::DbWeight::get().reads(1)) {
                let current_tick = ShTickGetter::<T>::get_current_tick();
                meter.consume(T::DbWeight::get().reads(1));
                Self::do_on_idle(current_tick, &mut meter);
            }

            meter.consumed()
        }
    }
}

/// Helper functions (getters, setters, etc.) for this pallet
impl<T: Config> Pallet<T> {
    /// A helper function to get the information of a sign up request of a user.
    pub fn get_sign_up_request(who: &T::AccountId) -> Result<SignUpRequest<T>, Error<T>> {
        SignUpRequests::<T>::get(who).ok_or(Error::<T>::SignUpNotRequested)
    }

    /// A helper function to get the total capacity of a storage provider.
    pub fn get_total_capacity_of_sp(who: &T::AccountId) -> Result<StorageDataUnit<T>, Error<T>> {
        if let Some(m_id) = AccountIdToMainStorageProviderId::<T>::get(who) {
            let msp = MainStorageProviders::<T>::get(m_id).ok_or(Error::<T>::NotRegistered)?;
            Ok(msp.capacity)
        } else if let Some(b_id) = AccountIdToBackupStorageProviderId::<T>::get(who) {
            let bsp = BackupStorageProviders::<T>::get(b_id).ok_or(Error::<T>::NotRegistered)?;
            Ok(bsp.capacity)
        } else {
            Err(Error::<T>::NotRegistered)
        }
    }

    /// A helper function to get the total capacity of all BSPs which is the total capacity of the network.
    pub fn get_total_bsp_capacity() -> StorageDataUnit<T> {
        TotalBspsCapacity::<T>::get()
    }

    /// A helper function to get the total used capacity of all BSPs.
    pub fn get_used_bsp_capacity() -> StorageDataUnit<T> {
        UsedBspsCapacity::<T>::get()
    }

    /// A helper function to get the total data used by a Main Storage Provider.
    pub fn get_used_storage_of_msp(
        who: &MainStorageProviderId<T>,
    ) -> Result<StorageDataUnit<T>, Error<T>> {
        let msp = MainStorageProviders::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        Ok(msp.capacity_used)
    }

    /// A helper function to get the total data used by a Backup Storage Provider.
    pub fn get_used_storage_of_bsp(
        who: &BackupStorageProviderId<T>,
    ) -> Result<StorageDataUnit<T>, Error<T>> {
        let bsp = BackupStorageProviders::<T>::get(who).ok_or(Error::<T>::NotRegistered)?;
        Ok(bsp.capacity_used)
    }

    /// A helper function to get the total amount of Backup Storage Providers that have registered.
    pub fn get_bsp_count() -> T::SpCount {
        BspCount::<T>::get()
    }

    /// A helper function to get the total amount of Main Storage Providers that have registered.
    pub fn get_msp_count() -> T::SpCount {
        MspCount::<T>::get()
    }
}

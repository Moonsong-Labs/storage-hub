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

// TODO #[cfg(feature = "runtime-benchmarks")]
// TODO mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;
pub use scale_info::Type;
use types::{
    BackupStorageProvider, BackupStorageProviderId, BalanceOf, BucketId, HashId,
    MainStorageProviderId, MerklePatriciaRoot, SignUpRequest, StorageDataUnit,
};

#[frame_support::pallet]
pub mod pallet {
    use super::types::*;
    use codec::{FullCodec, HasCompact};
    use frame_support::traits::Randomness;
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
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use shp_traits::{FileMetadataInterface, PaymentStreamsInterface, ProofSubmittersInterface};
    use sp_runtime::traits::{Bounded, CheckedDiv};

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type to access randomness to salt AccountIds and get the corresponding HashId
        type ProvidersRandomness: Randomness<HashId<Self>, BlockNumberFor<Self>>;

        /// Trait that allows the pallet to update payment streams of its Providers and Users
        type PaymentStreams: PaymentStreamsInterface<
            Balance = Self::NativeBalance,
            AccountId = Self::AccountId,
            ProviderId = HashId<Self>,
            Units = Self::StorageDataUnit,
        >;

        /// Trait that allows the pallet to manage generic file metadatas
        type FileMetadataManager: FileMetadataInterface<
            AccountId = Self::AccountId,
            StorageDataUnit = Self::StorageDataUnit,
        >;

        /// Type to access the Balances pallet (using the fungible trait from frame_support)
        type NativeBalance: Inspect<Self::AccountId>
            + Mutate<Self::AccountId>
            + hold::Inspect<Self::AccountId, Reason = Self::RuntimeHoldReason>
            // , Reason = Self::HoldReason> We will probably have to hold deposits
            + hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + hold::Balanced<Self::AccountId>
            + freeze::Inspect<Self::AccountId>
            + freeze::Mutate<Self::AccountId>;

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
            + HasCompact
            + Into<BalanceOf<Self>>
            + Into<u64>;

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

        /// The type of the Bucket NFT Collection ID.
        type ReadAccessGroupId: Member + Parameter + MaxEncodedLen + Copy + Incrementable;

        /// The trait exposing data of which providers failed to respond to challenges for proofs of storage.
        type ProvidersProofSubmitters: ProofSubmittersInterface<
            ProviderId = HashId<Self>,
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

        /// The maximum amount of Buckets that a MSP can have.
        #[pallet::constant]
        type MaxBuckets: Get<u32>;

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
    /// - [remove_root_bucket](shp_traits::MutateProvidersInterface::remove_root_bucket), which removes the entry of the corresponding bucket.
    #[pallet::storage]
    pub type Buckets<T: Config> = StorageMap<_, Blake2_128Concat, BucketId<T>, Bucket<T>>;

    /// The mapping from a MainStorageProviderId to a vector of BucketIds.
    ///
    /// This is used to efficiently retrieve the list of buckets that a Main Storage Provider is currently storing.
    ///
    /// This storage is updated in:
    /// - [add_bucket](shp_traits::MutateProvidersInterface::add_bucket)
    /// - [remove_root_bucket](shp_traits::MutateProvidersInterface::remove_root_bucket)
    #[pallet::storage]
    pub type MainStorageProviderIdsToBuckets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        MainStorageProviderId<T>,
        BoundedVec<BucketId<T>, T::MaxBuckets>,
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
        HashId<T>,
        ValueProposition<T>,
        OptionQuery,
    >;

    // Events & Errors:

    /// The events that can be emitted by this pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a Main Storage Provider has requested to sign up successfully. Provides information about
        /// that MSP's account id, its multiaddresses, the total data it can store according to its stake, and its value proposition.
        MspRequestSignUpSuccess {
            who: T::AccountId,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
        },

        /// Event emitted when a Main Storage Provider has confirmed its sign up successfully. Provides information about
        /// that MSP's account id, the total data it can store according to its stake, its multiaddress, and its value proposition.
        MspSignUpSuccess {
            who: T::AccountId,
            msp_id: MainStorageProviderId<T>,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
            value_prop: ValuePropositionWithId<T>,
        },

        /// Event emitted when a Backup Storage Provider has requested to sign up successfully. Provides information about
        /// that BSP's account id, its multiaddresses, and the total data it can store according to its stake.
        BspRequestSignUpSuccess {
            who: T::AccountId,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
        },

        /// Event emitted when a Backup Storage Provider has confirmed its sign up successfully. Provides information about
        /// that BSP's account id, the total data it can store according to its stake, and its multiaddress.
        BspSignUpSuccess {
            who: T::AccountId,
            bsp_id: BackupStorageProviderId<T>,
            multiaddresses: Multiaddresses<T>,
            capacity: StorageDataUnit<T>,
        },

        /// Event emitted when a sign up request has been canceled successfully. Provides information about
        /// the account id of the user that canceled the request.
        SignUpRequestCanceled { who: T::AccountId },

        /// Event emitted when a Main Storage Provider has signed off successfully. Provides information about
        /// that MSP's account id.
        MspSignOffSuccess {
            who: T::AccountId,
            msp_id: MainStorageProviderId<T>,
        },

        /// Event emitted when a Backup Storage Provider has signed off successfully. Provides information about
        /// that BSP's account id.
        BspSignOffSuccess {
            who: T::AccountId,
            bsp_id: BackupStorageProviderId<T>,
        },

        /// Event emitted when a SP has changed its capacity successfully. Provides information about
        /// that SP's account id, its old total data that could store, and the new total data.
        CapacityChanged {
            who: T::AccountId,
            provider_id: StorageProviderId<T>,
            old_capacity: StorageDataUnit<T>,
            new_capacity: StorageDataUnit<T>,
            next_block_when_change_allowed: BlockNumberFor<T>,
        },

        /// Event emitted when an SP has been slashed.
        Slashed {
            provider_id: HashId<T>,
            amount_slashed: BalanceOf<T>,
        },

        /// Event emitted when an MSP adds a new value proposition.
        ValuePropAdded {
            msp_id: MainStorageProviderId<T>,
            value_prop_id: ValuePropId<T>,
            value_prop: ValueProposition<T>,
        },

        /// Event emitted when an MSP's value proposition is made unavailable.
        ValuePropUnavailable {
            msp_id: MainStorageProviderId<T>,
            value_prop_id: ValuePropId<T>,
        },
    }

    /// The errors that can be thrown by this pallet to inform users about what went wrong
    #[pallet::error]
    pub enum Error<T> {
        // Sign up errors:
        /// Error thrown when a user tries to sign up as a SP but is already registered as a MSP or BSP.
        AlreadyRegistered,
        /// Error thrown when a user tries to confirm a sign up that was not requested previously.
        SignUpNotRequested,
        /// Error thrown when a user tries to request to sign up when it already has a sign up request pending.
        SignUpRequestPending,
        /// Error thrown when a user tries to sign up without any multiaddress.
        NoMultiAddress,
        /// Error thrown when a user tries to sign up as a SP but any of the provided multiaddresses is invalid.
        InvalidMultiAddress,
        /// Error thrown when a user tries to sign up or change its capacity to store less storage than the minimum required by the runtime.
        StorageTooLow,

        // Deposit errors:
        /// Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a SP or changing its capacity.
        NotEnoughBalance,
        /// Error thrown when the runtime cannot hold the required deposit from the account to register it as a SP or change its capacity.
        CannotHoldDeposit,

        // Sign off errors:
        /// Error thrown when a user tries to sign off as a SP but still has used storage.
        StorageStillInUse,
        /// Error thrown when a user tries to sign off as a BSP but the sign off period has not passed yet.
        SignOffPeriodNotPassed,

        // Randomness errors:
        /// Error thrown when a user tries to confirm a sign up but the randomness is too fresh to be used yet.
        RandomnessNotValidYet,
        /// Error thrown when a user tries to confirm a sign up but too much time has passed since the request.
        SignUpRequestExpired,

        // Capacity change errors:
        /// Error thrown when a user tries to change its capacity to less than its used storage.
        NewCapacityLessThanUsedStorage,
        /// Error thrown when a user tries to change its capacity to the same value it already has.
        NewCapacityEqualsCurrentCapacity,
        /// Error thrown when a user tries to change its capacity to zero (there are specific extrinsics to sign off as a SP).
        NewCapacityCantBeZero,
        /// Error thrown when a SP tries to change its capacity but it has not been enough time since the last time it changed it.
        NotEnoughTimePassed,
        /// Error thrown when a SP tries to change its capacity but the new capacity is not enough to store the used storage.
        NewUsedCapacityExceedsStorageCapacity,

        // General errors:
        /// Error thrown when a user tries to interact as a SP but is not registered as a MSP or BSP.
        NotRegistered,
        /// Error thrown when trying to get a root from a MSP without passing a User ID.
        NoUserId,
        /// Error thrown when trying to get a root from a MSP without passing a Bucket ID.
        NoBucketId,
        /// Error thrown when a user has a SP ID assigned to it but the SP data does not exist in storage (Inconsistency error).
        SpRegisteredButDataNotFound,
        /// Error thrown when a bucket ID is not found in storage.
        BucketNotFound,
        /// Error thrown when a bucket ID already exists in storage.
        BucketAlreadyExists,
        /// Error thrown when a bucket ID could not be added to the list of buckets of a MSP.
        AppendBucketToMspFailed,
        /// Error thrown when an attempt was made to slash an unslashable Storage Provider.
        ProviderNotSlashable,
        /// Error thrown when the value proposition id is not found.
        ValuePropositionNotFound,
        /// Error thrown when value proposition under a given id already exists.
        ValuePropositionAlreadyExists,
        /// Error thrown when a value proposition is not available.
        ValuePropositionNotAvailable,

        // Payment streams interface errors:
        /// Error thrown when failing to decode the metadata from a received trie value that was removed.
        InvalidEncodedFileMetadata,
        /// Error thrown when failing to decode the owner Account ID from the received metadata.
        InvalidEncodedAccountId,
        /// Error thrown when trying to update a payment stream that does not exist.
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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn request_msp_sign_up(
            origin: OriginFor<T>,
            capacity: StorageDataUnit<T>,
            multiaddresses: Multiaddresses<T>,
            value_prop_price_per_unit_of_data_per_block: BalanceOf<T>,
            commitment: Commitment<T>,
            value_prop_max_data_limit: StorageDataUnit<T>,
            payment_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Set up a structure with the information of the new MSP
            let msp_info = MainStorageProvider {
                buckets: BoundedVec::default(),
                capacity,
                capacity_used: StorageDataUnit::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
                owner_account: who.clone(),
                payment_account,
                sign_up_block: frame_system::Pallet::<T>::block_number(),
            };

            // Sign up the new MSP (if possible), updating storage
            Self::do_request_msp_sign_up(MainStorageProviderSignUpRequest {
                msp_info,
                value_prop: ValueProposition::<T>::new(
                    value_prop_price_per_unit_of_data_per_block,
                    commitment,
                    value_prop_max_data_limit,
                ),
            })?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::MspRequestSignUpSuccess {
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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
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
            Self::deposit_event(Event::<T>::BspRequestSignUpSuccess {
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
        /// - If this extrinsic is successful, it will be free for the caller, to incentive state debloating
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn confirm_sign_up(
            origin: OriginFor<T>,
            provider_account: Option<T::AccountId>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage and emit event
            // We emit the event in the interior logic to not have to check again which type of sign up it is outside of it
            match provider_account {
                Some(provider_account) => Self::do_confirm_sign_up(&provider_account)?,
                None => Self::do_confirm_sign_up(&who)?,
            }

            // Return a successful DispatchResultWithPostInfo. If the extrinsic executed correctly, it will be free for the caller
            Ok(Pays::No.into())
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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn cancel_sign_up(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            Self::do_cancel_sign_up(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::SignUpRequestCanceled { who });

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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn msp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let msp_id = Self::do_msp_sign_off(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::MspSignOffSuccess { who, msp_id });

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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn bsp_sign_off(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let bsp_id = Self::do_bsp_sign_off(&who)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::BspSignOffSuccess { who, bsp_id });

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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn change_capacity(
            origin: OriginFor<T>,
            new_capacity: StorageDataUnit<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let (provider_id, old_capacity) = Self::do_change_capacity(&who, new_capacity)?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::CapacityChanged {
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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn add_value_prop(
            origin: OriginFor<T>,
            price_per_unit_of_data_per_block: BalanceOf<T>,
            commitment: Commitment<T>,
            bucket_data_limit: StorageDataUnit<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let (msp_id, value_prop) = Self::do_add_value_prop(
                &who,
                price_per_unit_of_data_per_block,
                commitment,
                bucket_data_limit,
            )?;

            // Emit event
            Self::deposit_event(Event::<T>::ValuePropAdded {
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
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn make_value_prop_unavailable(
            origin: OriginFor<T>,
            value_prop_id: ValuePropId<T>,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Execute checks and logic, update storage
            let msp_id = Self::do_make_value_prop_unavailable(&who, value_prop_id)?;

            // Emit event
            Self::deposit_event(Event::<T>::ValuePropUnavailable {
                msp_id,
                value_prop_id,
            });

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
        #[pallet::call_index(9)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn force_msp_sign_up(
            origin: OriginFor<T>,
            who: T::AccountId,
            msp_id: MainStorageProviderId<T>,
            capacity: StorageDataUnit<T>,
            multiaddresses: Multiaddresses<T>,
            value_prop_price_per_unit_of_data_per_block: BalanceOf<T>,
            commitment: Commitment<T>,
            value_prop_max_data_limit: StorageDataUnit<T>,
            payment_account: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was sent with root origin.
            ensure_root(origin)?;

            // Set up a structure with the information of the new MSP
            let msp_info = MainStorageProvider {
                buckets: BoundedVec::default(),
                capacity,
                capacity_used: StorageDataUnit::<T>::default(),
                multiaddresses: multiaddresses.clone(),
                last_capacity_change: frame_system::Pallet::<T>::block_number(),
                owner_account: who.clone(),
                payment_account,
                sign_up_block: frame_system::Pallet::<T>::block_number(),
            };

            let sign_up_request = MainStorageProviderSignUpRequest {
                msp_info,
                value_prop: ValueProposition::<T>::new(
                    value_prop_price_per_unit_of_data_per_block,
                    commitment,
                    value_prop_max_data_limit,
                ),
            };

            // Sign up the new MSP (if possible), updating storage
            Self::do_request_msp_sign_up(sign_up_request.clone())?;

            // Emit the corresponding event
            Self::deposit_event(Event::<T>::MspRequestSignUpSuccess {
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
        #[pallet::call_index(10)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
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
            Self::deposit_event(Event::<T>::BspRequestSignUpSuccess {
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
        #[pallet::call_index(11)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn slash(origin: OriginFor<T>, provider_id: HashId<T>) -> DispatchResultWithPostInfo {
            // Check that the extrinsic was sent with root origin.
            ensure_signed(origin)?;

            Self::do_slash(&provider_id)
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

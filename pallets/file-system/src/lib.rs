//! # File System Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! The file system pallet provides the following functionality:
//!
//! - Tracks Merkle Forest roots for every MSP and BSP
//! - Manages storage buckets
//! - Exposes all file related actions a user or storage provider can execute
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `issue_storage_request`: Issue a new storage request to store a file.
//! - `volunteer_bsp`: BSP volunteers to store a file for a given storage request.
//!
//! ## Hooks
//!
//! - `on_idle`: Cleanup all expired storage requests.
//!
//! ## Dependencies
//!
//! TODO
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod types;
mod utils;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// TODO #[cfg(feature = "runtime-benchmarks")]
// TODO mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use super::{types::*, weights::WeightInfo};
    use codec::HasCompact;
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::{ValueQuery, *},
        sp_runtime::traits::{CheckEqual, Convert, MaybeDisplay, SimpleBitOps},
        traits::{
            fungible::*,
            nonfungibles_v2::{Create, Inspect as NonFungiblesInspect},
        },
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use shp_file_metadata::ChunkId;
    use sp_runtime::{
        traits::{
            Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, ConvertBack, One, Saturating,
            Zero,
        },
        BoundedVec,
    };
    use sp_weights::WeightMeter;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;

        /// The trait for reading and mutating Storage Provider and Bucket data.
        type Providers: shp_traits::ReadProvidersInterface<AccountId = Self::AccountId>
            + shp_traits::MutateProvidersInterface<
                MerkleHash = <Self::Providers as shp_traits::ReadProvidersInterface>::MerkleHash,
                ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
            > + shp_traits::ReadStorageProvidersInterface<
                ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
            > + shp_traits::MutateStorageProvidersInterface<
                ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
                StorageDataUnit = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
            > + shp_traits::ReadBucketsInterface<
                AccountId = Self::AccountId,
                BucketId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
                MerkleHash = <Self::Providers as shp_traits::ReadProvidersInterface>::MerkleHash,
                ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
                ReadAccessGroupId = CollectionIdFor<Self>,
                StorageDataUnit = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
            > + shp_traits::MutateBucketsInterface<
                AccountId = Self::AccountId,
                BucketId = <Self::Providers as shp_traits::ReadBucketsInterface>::BucketId,
                MerkleHash = <Self::Providers as shp_traits::ReadProvidersInterface>::MerkleHash,
                ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
                ReadAccessGroupId = CollectionIdFor<Self>,
                StorageDataUnit = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
            > + shp_traits::SystemMetricsInterface<
                ProvidedUnit = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
            >;

        /// The trait for issuing challenges and verifying proofs.
        type ProofDealer: shp_traits::ProofsDealerInterface<
            ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
            MerkleHash = <Self::Providers as shp_traits::ReadProvidersInterface>::MerkleHash,
        >;

        /// The trait to create, update, delete and inspect payment streams.
        type PaymentStreams: shp_traits::PaymentStreamsInterface<
            AccountId = Self::AccountId,
            ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
            Units = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
        >
        + shp_traits::MutatePricePerGigaUnitPerTickInterface<PricePerGigaUnitPerTick = BalanceOf<Self>>;

        /// The trait to initialise a Provider's randomness commit-reveal cycle.
        type CrRandomness: shp_traits::CommitRevealRandomnessInterface<
            ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
        >;

        type UpdateStoragePrice: shp_traits::UpdateStoragePrice<
            Price = BalanceOf<Self>,
            StorageDataUnit = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
            >;

        /// The trait for checking user solvency in the system
        type UserSolvency: shp_traits::ReadUserSolvencyInterface<AccountId = Self::AccountId>;

        /// Type for identifying a file, generally a hash.
        type Fingerprint: Parameter
            + Member
            + MaybeSerializeDeserialize
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            + CheckEqual
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen;

        /// Type representing the storage request bsps size type.
        type ReplicationTargetType: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + From<u32>
            + Into<u64>
            + Into<Self::ThresholdType>
            + Copy
            + MaxEncodedLen
            + HasCompact
            + Default
            + scale_info::TypeInfo
            + MaybeSerializeDeserialize
            + CheckedAdd
            + One
            + Saturating
            + PartialOrd
            + Zero;

        /// Type representing the threshold a BSP must meet to be eligible to volunteer to store a file.
        type ThresholdType: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + Default
            + MaybeDisplay
            + From<u32>
            + From<<Self::Providers as shp_traits::ReadStorageProvidersInterface>::ReputationWeight>
            + From<Self::ReplicationTargetType>
            + Copy
            + MaxEncodedLen
            + Decode
            + Saturating
            + CheckedMul
            + CheckedDiv
            + CheckedAdd
            + CheckedSub
            + PartialOrd
            + Bounded
            + One
            + Zero;

        /// The type to convert a threshold to a tick number.
        ///
        /// For more information on what "ticks" are, see the [Proofs Dealer pallet](https://github.com/Moonsong-Labs/storage-hub/blob/main/pallets/proofs-dealer/README.md).
        type ThresholdTypeToTickNumber: ConvertBack<
            Self::ThresholdType,
            <Self::ProofDealer as shp_traits::ProofsDealerInterface>::TickNumber,
        >;

        /// The type to convert a hash to a threshold.
        type HashToThresholdType: Convert<Self::Hash, Self::ThresholdType>;

        /// The type to convert a MerkleHash to a RandomnessOutput.
        type MerkleHashToRandomnessOutput: Convert<
            <Self::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash,
            <Self::ProofDealer as shp_traits::ProofsDealerInterface>::RandomnessOutput,
        >;

        /// The type to convert a ChunkId to a MerkleHash
        type ChunkIdToMerkleHash: Convert<
            ChunkId,
            <Self::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash,
        >;

        /// The currency mechanism, used for paying for reserves.
        type Currency: Inspect<Self::AccountId>
            + Mutate<Self::AccountId>
            + hold::Inspect<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + hold::Mutate<Self::AccountId, Reason = Self::RuntimeHoldReason>
            + hold::Balanced<Self::AccountId>
            + freeze::Inspect<Self::AccountId>
            + freeze::Mutate<Self::AccountId>;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// Registry for minted NFTs.
        type Nfts: NonFungiblesInspect<Self::AccountId>
            + Create<Self::AccountId, CollectionConfigFor<Self>>;

        /// Collection inspector
        type CollectionInspector: shp_traits::InspectCollections<
            CollectionId = CollectionIdFor<Self>,
        >;

        /// The treasury account of the runtime, where a fraction of each payment goes.
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;

        /// Penalty payed by a BSP when they forcefully stop storing a file.
        #[pallet::constant]
        type BspStopStoringFilePenalty: Get<BalanceOf<Self>>;

        /// Maximum batch of storage requests that can be confirmed at once when calling `bsp_confirm_storing`.
        #[pallet::constant]
        type MaxBatchConfirmStorageRequests: Get<u32>;

        /// Maximum batch of storage requests that can be responded to at once when calling `msp_respond_storage_requests_multiple_buckets`.
        #[pallet::constant]
        type MaxBatchMspRespondStorageRequests: Get<u32>;

        /// Maximum byte size of a file path.
        #[pallet::constant]
        type MaxFilePathSize: Get<u32>;

        /// Maximum byte size of a peer id.
        #[pallet::constant]
        type MaxPeerIdSize: Get<u32>;

        /// Maximum number of peer ids for a storage request.
        #[pallet::constant]
        type MaxNumberOfPeerIds: Get<u32>;

        /// Maximum number of multiaddresses for a storage request.
        #[pallet::constant]
        type MaxDataServerMultiAddresses: Get<u32>;

        /// Maximum number of expired items (per type) to clean up in a single block.
        #[pallet::constant]
        type MaxExpiredItemsInBlock: Get<u32>;

        /// Time-to-live for a storage request.
        #[pallet::constant]
        type StorageRequestTtl: Get<u32>;

        /// Time-to-live for a pending file deletion request, after which a priority challenge is sent out to enforce the deletion.
        #[pallet::constant]
        type PendingFileDeletionRequestTtl: Get<u32>;

        /// Time-to-live for a move bucket request, after which the request is considered expired.
        #[pallet::constant]
        type MoveBucketRequestTtl: Get<u32>;

        /// Maximum number of file deletion requests a user can have pending.
        #[pallet::constant]
        type MaxUserPendingDeletionRequests: Get<u32>;

        /// Maximum number of move bucket requests a user can have pending.
        #[pallet::constant]
        type MaxUserPendingMoveBucketRequests: Get<u32>;

        /// Number of blocks required to pass between a BSP requesting to stop storing a file and it being able to confirm to stop storing it.
        #[pallet::constant]
        type MinWaitForStopStoring: Get<BlockNumberFor<Self>>;

        /// Deposit held from the User when creating a new storage request
        #[pallet::constant]
        type StorageRequestCreationDeposit: Get<BalanceOf<Self>>;

        /// Default replication target
        #[pallet::constant]
        type DefaultReplicationTarget: Get<ReplicationTargetType<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type StorageRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, MerkleHash<T>, StorageRequestMetadata<T>>;

    /// A double map from storage request to BSP `AccountId`s that volunteered to store the file.
    ///
    /// Any BSP under a storage request prefix is considered to be a volunteer and can be removed at any time.
    /// Once a BSP submits a valid proof to the via the `bsp_confirm_storing` extrinsic, the `confirmed` field in [`StorageRequestBspsMetadata`] will be set to `true`.
    ///
    /// When a storage request is expired or removed, the corresponding storage request prefix in this map is removed.
    #[pallet::storage]
    pub type StorageRequestBsps<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        MerkleHash<T>,
        Blake2_128Concat,
        ProviderIdFor<T>,
        StorageRequestBspsMetadata<T>,
        OptionQuery,
    >;

    /// Bookkeeping of the buckets containing open storage requests.
    #[pallet::storage]
    pub type BucketsWithStorageRequests<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BucketIdFor<T>,
        Blake2_128Concat,
        MerkleHash<T>,
        (),
        OptionQuery,
    >;

    /// A map of blocks to expired storage requests.
    #[pallet::storage]
    pub type StorageRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<StorageRequestExpirationItem<T>, T::MaxExpiredItemsInBlock>,
        ValueQuery,
    >;

    /// A map of blocks to expired file deletion requests.
    #[pallet::storage]
    pub type FileDeletionRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<FileDeletionRequestExpirationItem<T>, T::MaxExpiredItemsInBlock>,
        ValueQuery,
    >;

    /// A map of blocks to expired move bucket requests.
    #[pallet::storage]
    pub type MoveBucketRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<(ProviderIdFor<T>, BucketIdFor<T>), T::MaxExpiredItemsInBlock>,
        ValueQuery,
    >;

    /// A pointer to the earliest available block to insert a new storage request expiration.
    ///
    /// This should always be greater or equal than current block + [`Config::StorageRequestTtl`].
    #[pallet::storage]
    pub type NextAvailableStorageRequestExpirationBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A pointer to the earliest available block to insert a new file deletion request expiration.
    ///
    /// This should always be greater or equal than current block + [`Config::PendingFileDeletionRequestTtl`].
    #[pallet::storage]
    pub type NextAvailableFileDeletionRequestExpirationBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A pointer to the earliest available block to insert a new move bucket request expiration.
    ///
    /// This should always be greater or equal than current block + [`Config::MoveBucketRequestTtl`].
    #[pallet::storage]
    pub type NextAvailableMoveBucketRequestExpirationBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A pointer to the starting block to clean up expired items.
    ///
    /// If this block is behind the current block number, the cleanup algorithm in `on_idle` will
    /// attempt to advance this block pointer as close to or up to the current block number. This
    /// will execute provided that there is enough remaining weight to do so.
    #[pallet::storage]
    pub type NextStartingBlockToCleanUp<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Pending file deletion requests.
    ///
    /// A mapping from a user account id to a list of pending file deletion requests, holding a tuple of the file key and bucket id.
    #[pallet::storage]
    pub type PendingFileDeletionRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<(MerkleHash<T>, BucketIdFor<T>), T::MaxUserPendingDeletionRequests>,
        ValueQuery,
    >;

    /// Pending file stop storing requests.
    ///
    /// A double mapping from BSP IDs to a list of file keys pending stop storing requests to the block in which those requests were opened
    /// and the proven size of the file.
    /// The block number is used to avoid BSPs being able to stop storing files immediately which would allow them to avoid challenges
    /// of missing files. The size is to be able to decrease their used capacity when they confirm to stop storing the file.
    #[pallet::storage]
    pub type PendingStopStoringRequests<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Blake2_128Concat,
        MerkleHash<T>,
        (BlockNumberFor<T>, StorageData<T>),
    >;

    /// Pending move bucket requests.
    ///
    /// A double mapping from MSP IDs to a list of bucket IDs which they can accept or decline to take over.
    /// The value is the user who requested the move.
    #[pallet::storage]
    pub type PendingMoveBucketRequests<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Blake2_128Concat,
        BucketIdFor<T>,
        MoveBucketRequestMetadata<T>,
    >;

    /// Bookkeeping of buckets that are pending to be moved to a new MSP.
    #[pallet::storage]
    pub type PendingBucketsToMove<T: Config> =
        StorageMap<_, Blake2_128Concat, BucketIdFor<T>, (), ValueQuery>;

    // TODO: add this to pallet params instead of a storage element
    /// Maximum number replication target allowed to be set for a storage request to be fulfilled.
    #[pallet::storage]
    pub type MaxReplicationTarget<T: Config> =
        StorageValue<_, ReplicationTargetType<T>, ValueQuery>;

    /// Number of ticks until all BSPs would reach the [`Config::MaximumThreshold`] to ensure that all BSPs are able to volunteer.
    #[pallet::storage]
    pub type TickRangeToMaximumThreshold<T: Config> = StorageValue<_, TickNumber<T>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub max_replication_target: ReplicationTargetType<T>,
        pub tick_range_to_maximum_threshold: TickNumber<T>,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            // TODO: Find a better default value for this.
            let max_replication_target = 10u32.into();
            let tick_range_to_maximum_threshold = 10u32.into();

            MaxReplicationTarget::<T>::put(max_replication_target);
            TickRangeToMaximumThreshold::<T>::put(tick_range_to_maximum_threshold);

            Self {
                max_replication_target,
                tick_range_to_maximum_threshold,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            MaxReplicationTarget::<T>::put(self.max_replication_target);
            TickRangeToMaximumThreshold::<T>::put(self.tick_range_to_maximum_threshold);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that a new bucket has been created.
        NewBucket {
            who: T::AccountId,
            msp_id: Option<ProviderIdFor<T>>,
            bucket_id: BucketIdFor<T>,
            name: BucketNameFor<T>,
            root: MerkleHash<T>,
            collection_id: Option<CollectionIdFor<T>>,
            private: bool,
            value_prop_id: Option<ValuePropId<T>>,
        },
        /// Notifies that an empty bucket has been deleted.
        BucketDeleted {
            who: T::AccountId,
            bucket_id: BucketIdFor<T>,
            maybe_collection_id: Option<CollectionIdFor<T>>,
        },
        /// Notifies that a bucket is being moved to a new MSP.
        MoveBucketRequested {
            who: T::AccountId,
            bucket_id: BucketIdFor<T>,
            new_msp_id: ProviderIdFor<T>,
        },
        /// Notifies that a bucket's privacy has been updated.
        BucketPrivacyUpdated {
            who: T::AccountId,
            bucket_id: BucketIdFor<T>,
            collection_id: Option<CollectionIdFor<T>>,
            private: bool,
        },
        /// Notifies that a new collection has been created and associated with a bucket.
        NewCollectionAndAssociation {
            who: T::AccountId,
            bucket_id: BucketIdFor<T>,
            collection_id: CollectionIdFor<T>,
        },
        /// Notifies that a new file has been requested to be stored.
        NewStorageRequest {
            who: T::AccountId,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
            peer_ids: PeerIds<T>,
        },
        /// Notifies that a Main Storage Provider (MSP) has accepted a storage request for a specific file key.
        ///
        /// This event is emitted when an MSP agrees to store a file, but the storage request
        /// is not yet fully fulfilled (i.e., the required number of Backup Storage Providers
        /// have not yet confirmed storage).
        ///
        /// # Note
        /// This event is not emitted when the storage request is immediately fulfilled upon
        /// MSP acceptance. In such cases, a [`StorageRequestFulfilled`] event is emitted instead.
        MspAcceptedStorageRequest { file_key: MerkleHash<T> },
        /// Notifies that a BSP has been accepted to store a given file.
        AcceptedBspVolunteer {
            bsp_id: ProviderIdFor<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            multiaddresses: MultiAddresses<T>,
            owner: T::AccountId,
            size: StorageData<T>,
        },
        /// Notifies that a BSP confirmed storing a file(s).
        BspConfirmedStoring {
            who: T::AccountId,
            bsp_id: ProviderIdFor<T>,
            confirmed_file_keys: BoundedVec<MerkleHash<T>, T::MaxBatchConfirmStorageRequests>,
            skipped_file_keys: BoundedVec<MerkleHash<T>, T::MaxBatchConfirmStorageRequests>,
            new_root: MerkleHash<T>,
        },
        /// Notifies that a storage request for a file key has been fulfilled.
        /// This means that the storage request has been accepted by the MSP and the BSP target
        /// has been reached.
        StorageRequestFulfilled { file_key: MerkleHash<T> },
        /// Notifies the expiration of a storage request. This means that the storage request has
        /// been accepted by the MSP but the BSP target has not been reached (possibly 0 BSPs).
        /// Note: This is a valid storage outcome, the user being responsible to track the number
        /// of BSPs and choose to either delete the file and re-issue a storage request or continue.
        StorageRequestExpired { file_key: MerkleHash<T> },
        /// Notifies that a storage request has been revoked by the user who initiated it.
        /// Note: the BSPs who confirmed the file are also issued a priority challenge to delete the
        /// file.
        StorageRequestRevoked { file_key: MerkleHash<T> },
        /// Notifies that a storage request has either been directly rejected by the MSP or
        /// the MSP did not respond to the storage request in time.
        /// Note: There might be BSPs that have volunteered and confirmed the file already, for
        /// which a priority challenge to delete the file will be issued.
        StorageRequestRejected {
            file_key: MerkleHash<T>,
            reason: RejectedStorageRequestReason,
        },
        BspRequestedToStopStoring {
            bsp_id: ProviderIdFor<T>,
            file_key: MerkleHash<T>,
            owner: T::AccountId,
            location: FileLocation<T>,
        },
        /// Notifies that a BSP has stopped storing a file.
        BspConfirmStoppedStoring {
            bsp_id: ProviderIdFor<T>,
            file_key: MerkleHash<T>,
            new_root: MerkleHash<T>,
        },
        /// Notifies that a file key has been queued for a priority challenge for file deletion.
        PriorityChallengeForFileDeletionQueued {
            issuer: EitherAccountIdOrMspId<T>,
            file_key: MerkleHash<T>,
        },
        /// Notifies that a SP has stopped storing a file because its owner has become insolvent.
        SpStopStoringInsolventUser {
            sp_id: ProviderIdFor<T>,
            file_key: MerkleHash<T>,
            owner: T::AccountId,
            location: FileLocation<T>,
            new_root: MerkleHash<T>,
        },
        /// Notifies that a priority challenge failed to be queued for pending file deletion.
        FailedToQueuePriorityChallenge {
            user: T::AccountId,
            file_key: MerkleHash<T>,
        },
        /// Notifies that a file will be deleted.
        FileDeletionRequest {
            user: T::AccountId,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            msp_id: Option<ProviderIdFor<T>>,
            proof_of_inclusion: bool,
        },
        /// Notifies that a proof has been submitted for a pending file deletion request.
        ProofSubmittedForPendingFileDeletionRequest {
            msp_id: ProviderIdFor<T>,
            user: T::AccountId,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            proof_of_inclusion: bool,
        },
        /// Notifies that a BSP's challenge cycle has been initialised, adding the first file
        /// key(s) to the BSP's Merkle Patricia Forest.
        BspChallengeCycleInitialised {
            who: T::AccountId,
            bsp_id: ProviderIdFor<T>,
        },
        /// Notifies that a move bucket request has expired.
        MoveBucketRequestExpired {
            msp_id: ProviderIdFor<T>,
            bucket_id: BucketIdFor<T>,
        },
        /// Notifies that a bucket has been moved to a new MSP.
        MoveBucketAccepted {
            bucket_id: BucketIdFor<T>,
            msp_id: ProviderIdFor<T>,
        },
        /// Notifies that a bucket move request has been rejected by the MSP.
        MoveBucketRejected {
            bucket_id: BucketIdFor<T>,
            msp_id: ProviderIdFor<T>,
        },
        /// Notifies that a MSP has stopped storing a bucket.
        MspStoppedStoringBucket {
            msp_id: ProviderIdFor<T>,
            owner: T::AccountId,
            bucket_id: BucketIdFor<T>,
        },
        /// Failed to decrease bucket size for expired file deletion request
        FailedToDecreaseBucketSize {
            user: T::AccountId,
            bucket_id: BucketIdFor<T>,
            file_key: MerkleHash<T>,
            file_size: StorageData<T>,
            error: DispatchError,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Storage request already registered for the given file.
        StorageRequestAlreadyRegistered,
        /// Storage request not registered for the given file.
        StorageRequestNotFound,
        /// Operation not allowed while the storage request is not being revoked.
        StorageRequestNotRevoked,
        /// Operation not allowed while the storage request exists.
        StorageRequestExists,
        /// Replication target cannot be zero.
        ReplicationTargetCannotBeZero,
        /// BSPs required for storage request cannot exceed the maximum allowed.
        ReplicationTargetExceedsMaximum,
        /// Max replication target cannot be smaller than default replication target.
        MaxReplicationTargetSmallerThanDefault,
        /// Account is not a BSP.
        NotABsp,
        /// Account is not a MSP.
        NotAMsp,
        /// Account is not a SP.
        NotASp,
        /// BSP has not volunteered to store the given file.
        BspNotVolunteered,
        /// BSP has not confirmed storing the given file.
        BspNotConfirmed,
        /// BSP has already confirmed storing the given file.
        BspAlreadyConfirmed,
        /// Number of BSPs required for storage request has been reached.
        StorageRequestBspsRequiredFulfilled,
        /// BSP already volunteered to store the given file.
        BspAlreadyVolunteered,
        /// SP does not have enough storage capacity to store the file.
        InsufficientAvailableCapacity,
        /// Number of removed BSPs volunteered from storage request prefix did not match the expected number.
        UnexpectedNumberOfRemovedVolunteeredBsps,
        /// BSP cannot volunteer at this current tick.
        BspNotEligibleToVolunteer,
        /// No slot available found in blocks to insert storage request expiration time.
        StorageRequestExpiredNoSlotAvailable,
        /// Not authorized to delete the storage request.
        StorageRequestNotAuthorized,
        /// Error created in 2024. If you see this, you are well beyond the singularity and should
        /// probably stop using this pallet.
        MaxBlockNumberReached,
        /// Failed to encode BSP id as slice.
        FailedToEncodeBsp,
        /// Failed to encode fingerprint as slice.
        FailedToEncodeFingerprint,
        /// Failed to decode threshold.
        FailedToDecodeThreshold,
        /// BSP did not succeed threshold check.
        AboveThreshold,
        /// Arithmetic error in threshold calculation.
        ThresholdArithmeticError,
        /// Failed to convert to primitive type.
        FailedTypeConversion,
        /// Divided by 0
        DividedByZero,
        /// Failed to get value when just checked it existed.
        ImpossibleFailedToGetValue,
        /// Bucket is not private. Call `update_bucket_privacy` to make it private.
        BucketIsNotPrivate,
        /// Bucket does not exist
        BucketNotFound,
        /// Bucket is not empty.
        BucketNotEmpty,
        /// Operation failed because the account is not the owner of the bucket.
        NotBucketOwner,
        /// Root of the provider not found.
        ProviderRootNotFound,
        /// Failed to verify proof: required to provide a proof of non-inclusion.
        ExpectedNonInclusionProof,
        /// Failed to verify proof: required to provide a proof of inclusion.
        ExpectedInclusionProof,
        /// Metadata does not correspond to expected file key.
        InvalidFileKeyMetadata,
        /// BSPs assignment threshold cannot be below asymptote.
        ThresholdBelowAsymptote,
        /// Unauthorized operation, signer does not own the file.
        NotFileOwner,
        /// File key already pending deletion.
        FileKeyAlreadyPendingDeletion,
        /// Max number of user pending deletion requests reached.
        MaxUserPendingDeletionRequestsReached,
        /// Unauthorized operation, signer is not an MSP of the bucket id.
        MspNotStoringBucket,
        /// File key not found in pending deletion requests.
        FileKeyNotPendingDeletion,
        /// File size cannot be zero.
        FileSizeCannotBeZero,
        /// No global reputation weight set.
        NoGlobalReputationWeightSet,
        /// Maximum threshold cannot be zero.
        MaximumThresholdCannotBeZero,
        /// Tick range to maximum threshold cannot be zero.
        TickRangeToMaximumThresholdCannotBeZero,
        /// Pending stop storing request not found.
        PendingStopStoringRequestNotFound,
        /// Minimum amount of blocks between the request opening and being able to confirm it not reached.
        MinWaitForStopStoringNotReached,
        /// Pending stop storing request already exists.
        PendingStopStoringRequestAlreadyExists,
        /// A SP tried to stop storing files from a user that was supposedly insolvent, but the user is not insolvent.
        UserNotInsolvent,
        /// The MSP is trying to confirm to store a file from a storage request is not the one selected to store it.
        NotSelectedMsp,
        /// The MSP is trying to confirm to store a file from a storage request that it has already confirmed to store.
        MspAlreadyConfirmed,
        /// The MSP is trying to confirm to store a file from a storage request that does not have a MSP assigned.
        RequestWithoutMsp,
        /// The MSP is already storing the bucket.
        MspAlreadyStoringBucket,
        /// Move bucket request not found in storage.
        MoveBucketRequestNotFound,
        /// Action not allowed while the bucket is being moved.
        BucketIsBeingMoved,
        /// BSP is already a data server for the move bucket request.
        BspAlreadyDataServer,
        /// Too many registered data servers for the move bucket request.
        BspDataServersExceeded,
        /// The bounded vector that holds file metadata to process it is full but there's still more to process.
        FileMetadataProcessingQueueFull,
        /// Too many batch responses to process.
        TooManyBatchResponses,
        /// Too many storage request responses.
        TooManyStorageRequestResponses,
        /// Bucket id and file key pair is invalid.
        InvalidBucketIdFileKeyPair,
        /// Key already exists in mapping when it should not.
        InconsistentStateKeyAlreadyExists,
        /// Failed to fetch the rate for the payment stream.
        FixedRatePaymentStreamNotFound,
        /// Cannot hold the required deposit from the user
        CannotHoldDeposit,
        /// Failed to query earliest volunteer tick
        FailedToQueryEarliestFileVolunteerTick,
        /// Failed to get owner account of ID of provider
        FailedToGetOwnerAccount,
        /// No file keys to confirm storing
        NoFileKeysToConfirm,
        /// Root was not updated after applying delta
        RootNotUpdated,
        /// Privacy update results in no change
        NoPrivacyChange,
        /// Operations not allowed for insolvent provider
        OperationNotAllowedForInsolventProvider,
    }

    /// This enum holds the HoldReasons for this pallet, allowing the runtime to identify each held balance with different reasons separately
    ///
    /// This allows us to hold tokens and be able to identify in the future that those held tokens were
    /// held because of this pallet
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Deposit that a user has to pay to create a new storage request
        StorageRequestCreationHold,
        // Only for testing, another unrelated hold reason
        #[cfg(test)]
        AnotherUnrelatedHold,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_bucket())]
        pub fn create_bucket(
            origin: OriginFor<T>,
            msp_id: Option<ProviderIdFor<T>>,
            name: BucketNameFor<T>,
            private: bool,
            value_prop_id: Option<ValuePropId<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (bucket_id, maybe_collection_id) =
                Self::do_create_bucket(who.clone(), msp_id, name.clone(), private, value_prop_id)?;

            Self::deposit_event(Event::NewBucket {
                who,
                msp_id,
                bucket_id,
                name,
                root: <T::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash::default(),
                collection_id: maybe_collection_id,
                private,
                value_prop_id,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn request_move_bucket(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            new_msp_id: ProviderIdFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_request_move_bucket(who.clone(), bucket_id, new_msp_id)?;

            Self::deposit_event(Event::MoveBucketRequested {
                who,
                bucket_id,
                new_msp_id,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn msp_respond_move_bucket_request(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            response: BucketMoveRequestResponse,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let msp_id =
                Self::do_msp_respond_move_bucket_request(who.clone(), bucket_id, response.clone())?;

            match response {
                BucketMoveRequestResponse::Accepted => {
                    Self::deposit_event(Event::MoveBucketAccepted { bucket_id, msp_id });
                }
                BucketMoveRequestResponse::Rejected => {
                    Self::deposit_event(Event::MoveBucketRejected { bucket_id, msp_id });
                }
            }

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn update_bucket_privacy(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            private: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let maybe_collection_id =
                Self::do_update_bucket_privacy(who.clone(), bucket_id, private)?;

            Self::deposit_event(Event::BucketPrivacyUpdated {
                who,
                bucket_id,
                private,
                collection_id: maybe_collection_id,
            });

            Ok(())
        }

        /// Create and associate a collection with a bucket.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn create_and_associate_collection_with_bucket(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let collection_id =
                Self::do_create_and_associate_collection_with_bucket(who.clone(), bucket_id)?;

            Self::deposit_event(Event::NewCollectionAndAssociation {
                who,
                bucket_id,
                collection_id,
            });

            Ok(())
        }

        /// Dispatchable extrinsic that allows a User to delete any of their buckets if it is currently empty.
        /// This way, the User is allowed to remove now unused buckets to recover their deposit for them.
        ///
        /// The User must provide the BucketId of the bucket they want to delete, which should correspond to a
        /// bucket that is both theirs and currently empty.
        ///
        /// To check if a bucket is empty, we compare its current root with the one of an empty trie.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn delete_bucket(origin: OriginFor<T>, bucket_id: BucketIdFor<T>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and delete the bucket
            let maybe_collection_id = Self::do_delete_bucket(who.clone(), bucket_id)?;

            // Emit event.
            Self::deposit_event(Event::BucketDeleted {
                who,
                bucket_id,
                maybe_collection_id,
            });

            Ok(())
        }

        /// Issue a new storage request for a file
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::issue_storage_request())]
        pub fn issue_storage_request(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
            msp_id: Option<ProviderIdFor<T>>,
            peer_ids: PeerIds<T>,
            replication_target: Option<ReplicationTargetType<T>>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and register storage request
            Self::do_request_storage(
                who.clone(),
                bucket_id,
                location.clone(),
                fingerprint,
                size,
                msp_id,
                replication_target,
                Some(peer_ids.clone()),
            )?;

            Ok(())
        }

        /// Revoke storage request
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn revoke_storage_request(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and revoke storage request
            Self::do_revoke_storage_request(who, file_key)?;

            // Emit event.
            Self::deposit_event(Event::StorageRequestRevoked { file_key });

            Ok(())
        }

        /// Used by a MSP to accept or decline storage requests in batches, grouped by bucket.
        ///
        /// This follows a best-effort strategy, meaning that all file keys will be processed and declared to have successfully be
        /// accepted, rejected or have failed to be processed in the results of the event emitted.
        ///
        /// The MSP has to provide a file proof for all the file keys that are being accepted and a non-inclusion proof for the file keys
        /// in the bucket's Merkle Patricia Forest. The file proofs for the file keys is necessary to verify that
        /// the MSP actually has the files, while the non-inclusion proof is necessary to verify that the MSP
        /// wasn't storing it before.
        #[pallet::call_index(8)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn msp_respond_storage_requests_multiple_buckets(
            origin: OriginFor<T>,
            storage_request_msp_response: StorageRequestMspResponse<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            Self::do_msp_respond_storage_request(who.clone(), storage_request_msp_response)?;

            Ok(())
        }

        #[pallet::call_index(9)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn msp_stop_storing_bucket(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (msp_id, owner) = Self::do_msp_stop_storing_bucket(who.clone(), bucket_id)?;

            Self::deposit_event(Event::MspStoppedStoringBucket {
                msp_id,
                owner,
                bucket_id,
            });

            Ok(())
        }

        /// Used by a BSP to volunteer for storing a file.
        ///
        /// The transaction will fail if the XOR between the file ID and the BSP ID is not below the threshold,
        /// so a BSP is strongly advised to check beforehand. Another reason for failure is
        /// if the maximum number of BSPs has been reached. A successful assignment as BSP means
        /// that some of the collateral tokens of that MSP are frozen.
        #[pallet::call_index(10)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_volunteer(origin: OriginFor<T>, file_key: MerkleHash<T>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Perform validations and register Storage Provider as BSP for file.
            let (bsp_id, multiaddresses, storage_request_metadata) =
                Self::do_bsp_volunteer(who.clone(), file_key)?;

            // Emit new BSP volunteer event.
            Self::deposit_event(Event::AcceptedBspVolunteer {
                bsp_id,
                multiaddresses,
                bucket_id: storage_request_metadata.bucket_id,
                location: storage_request_metadata.location,
                fingerprint: storage_request_metadata.fingerprint,
                owner: storage_request_metadata.owner,
                size: storage_request_metadata.size,
            });

            Ok(())
        }

        /// Used by a BSP to confirm they are storing data of a storage request.
        #[pallet::call_index(11)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_confirm_storing(
            origin: OriginFor<T>,
            non_inclusion_forest_proof: ForestProof<T>,
            file_keys_and_proofs: BoundedVec<
                (MerkleHash<T>, KeyProof<T>),
                T::MaxBatchConfirmStorageRequests,
            >,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Perform validations and confirm storage.
            Self::do_bsp_confirm_storing(
                who.clone(),
                non_inclusion_forest_proof.clone(),
                file_keys_and_proofs,
            )
        }

        /// Executed by a BSP to request to stop storing a file.
        ///
        /// In the event when a storage request no longer exists for the data the BSP no longer stores,
        /// it is required that the BSP still has access to the metadata of the initial storage request.
        /// If they do not, they will at least need that metadata to reconstruct the File ID and from wherever
        /// the BSP gets that data is up to it. One example could be from the assigned MSP.
        /// This metadata is necessary since it is needed to reconstruct the leaf node key in the storage
        /// provider's Merkle Forest.
        #[pallet::call_index(12)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_request_stop_storing(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            owner: T::AccountId,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
            can_serve: bool,
            inclusion_forest_proof: ForestProof<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Perform validations and open the request to stop storing the file.
            let bsp_id = Self::do_bsp_request_stop_storing(
                who.clone(),
                file_key,
                bucket_id,
                location.clone(),
                owner.clone(),
                fingerprint,
                size,
                can_serve,
                inclusion_forest_proof,
            )?;

            // Emit event.
            Self::deposit_event(Event::BspRequestedToStopStoring {
                bsp_id,
                file_key,
                owner,
                location,
            });

            Ok(())
        }

        /// Executed by a BSP to confirm to stop storing a file.
        ///
        /// It has to have previously opened a pending stop storing request using the `bsp_request_stop_storing` extrinsic.
        /// The minimum amount of blocks between the request and the confirmation is defined by the runtime, such that the
        /// BSP can't immediately stop storing a file it has previously lost when receiving a challenge for it.
        #[pallet::call_index(13)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_confirm_stop_storing(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
            inclusion_forest_proof: ForestProof<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Perform validations and stop storing the file.
            let (bsp_id, new_root) =
                Self::do_bsp_confirm_stop_storing(who.clone(), file_key, inclusion_forest_proof)?;

            // Emit event.
            Self::deposit_event(Event::BspConfirmStoppedStoring {
                bsp_id,
                file_key,
                new_root,
            });

            Ok(())
        }

        /// Executed by a SP to stop storing a file from an insolvent user.
        ///
        /// This is used when a user has become insolvent and the SP needs to stop storing the files of that user, since
        /// it won't be getting paid for it anymore.
        /// The validations are similar to the ones in the `bsp_request_stop_storing` and `bsp_confirm_stop_storing` extrinsics, but the SP doesn't need to
        /// wait for a minimum amount of blocks to confirm to stop storing the file nor it has to be a BSP.
        #[pallet::call_index(14)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn stop_storing_for_insolvent_user(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            owner: T::AccountId,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
            inclusion_forest_proof: ForestProof<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Perform validations and stop storing the file.
            let (sp_id, new_root) = Self::do_sp_stop_storing_for_insolvent_user(
                who.clone(),
                file_key,
                bucket_id,
                location.clone(),
                owner.clone(),
                fingerprint,
                size,
                inclusion_forest_proof,
            )?;

            // Emit event.
            Self::deposit_event(Event::SpStopStoringInsolventUser {
                sp_id,
                file_key,
                owner,
                location,
                new_root,
            });

            Ok(())
        }

        #[pallet::call_index(15)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn delete_file(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            file_key: MerkleHash<T>,
            location: FileLocation<T>,
            size: StorageData<T>,
            fingerprint: Fingerprint<T>,
            maybe_inclusion_forest_proof: Option<ForestProof<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (proof_of_inclusion, msp_id) = Self::do_delete_file(
                who.clone(),
                bucket_id,
                file_key,
                location,
                fingerprint,
                size,
                maybe_inclusion_forest_proof,
            )?;

            Self::deposit_event(Event::FileDeletionRequest {
                user: who,
                file_key,
                bucket_id,
                msp_id,
                proof_of_inclusion,
            });

            Ok(())
        }

        #[pallet::call_index(16)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn pending_file_deletion_request_submit_proof(
            origin: OriginFor<T>,
            user: T::AccountId,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            forest_proof: ForestProof<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (proof_of_inclusion, msp_id) = Self::do_pending_file_deletion_request_submit_proof(
                who.clone(),
                user.clone(),
                file_key,
                bucket_id,
                forest_proof,
            )?;

            Self::deposit_event(Event::ProofSubmittedForPendingFileDeletionRequest {
                msp_id,
                user,
                file_key,
                bucket_id,
                proof_of_inclusion,
            });

            Ok(())
        }

        #[pallet::call_index(17)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn set_global_parameters(
            origin: OriginFor<T>,
            new_max_replication_target: Option<T::ReplicationTargetType>,
            tick_range_to_maximum_threshold: Option<TickNumber<T>>,
        ) -> DispatchResult {
            // Check that the extrinsic was sent with root origin.
            ensure_root(origin)?;

            if let Some(new_max_replication_target) = new_max_replication_target {
                ensure!(
                    new_max_replication_target > T::ReplicationTargetType::zero(),
                    Error::<T>::ReplicationTargetCannotBeZero
                );

                ensure!(
                    new_max_replication_target >= T::DefaultReplicationTarget::get(),
                    Error::<T>::MaxReplicationTargetSmallerThanDefault
                );

                MaxReplicationTarget::<T>::put(new_max_replication_target);
            }

            if let Some(tick_range_to_maximum_threshold) = tick_range_to_maximum_threshold {
                ensure!(
                    tick_range_to_maximum_threshold > TickNumber::<T>::zero(),
                    Error::<T>::TickRangeToMaximumThresholdCannotBeZero
                );

                TickRangeToMaximumThreshold::<T>::put(tick_range_to_maximum_threshold);
            }

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_poll(_n: BlockNumberFor<T>, weight: &mut frame_support::weights::WeightMeter) {
            // TODO: Benchmark computational weight cost of this hook.

            Self::do_on_poll(weight);
        }

        fn on_idle(current_block: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            let mut meter = WeightMeter::with_limit(remaining_weight);
            Self::do_on_idle(current_block, &mut meter);

            meter.consumed()
        }

        /// Any code located in this hook is placed in an auto-generated test, and generated as a part
        /// of crate::construct_runtime's expansion.
        /// Look for a test case with a name along the lines of: __construct_runtime_integrity_test.
        fn integrity_test() {
            let default_replication_target = T::DefaultReplicationTarget::get();

            assert!(
                default_replication_target > T::ReplicationTargetType::zero(),
                "Default replication target cannot be zero."
            );
        }
    }
}

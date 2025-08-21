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
//!
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmark_proofs;
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
            nonfungibles_v2::{Create, Destroy, Inspect as NonFungiblesInspect},
        },
        Blake2_128Concat,
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use shp_file_metadata::ChunkId;
    use shp_traits::ProofsDealerInterface;
    use sp_runtime::{
        traits::{
            Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, ConvertBack, IdentifyAccount,
            One, Saturating, Verify, Zero,
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
				SpCount = ReplicationTargetType<Self>
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
				ValuePropId = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::ValuePropId,
            > + shp_traits::MutateBucketsInterface<
                AccountId = Self::AccountId,
                BucketId = <Self::Providers as shp_traits::ReadBucketsInterface>::BucketId,
                MerkleHash = <Self::Providers as shp_traits::ReadProvidersInterface>::MerkleHash,
                ProviderId = <Self::Providers as shp_traits::ReadProvidersInterface>::ProviderId,
                ReadAccessGroupId = CollectionIdFor<Self>,
                StorageDataUnit = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::StorageDataUnit,
				ValuePropId = <Self::Providers as shp_traits::ReadStorageProvidersInterface>::ValuePropId,
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
        + shp_traits::PricePerGigaUnitPerTickInterface<PricePerGigaUnitPerTick = BalanceOf<Self>>;

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

        /// Type representing the storage request's BSP amount type.
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
            + Ord
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
            + Create<Self::AccountId, CollectionConfigFor<Self>>
            + Destroy<Self::AccountId>;

        /// Collection inspector
        type CollectionInspector: shp_traits::InspectCollections<
            CollectionId = CollectionIdFor<Self>,
        >;

        /// Converter from the Weight type to the corresponding fee.
        type WeightToFee: sp_weights::WeightToFee<Balance = BalanceOf<Self>>;

        /// Converter from the ReplicationTarget type to the Balance type.
        type ReplicationTargetToBalance: Convert<ReplicationTargetType<Self>, BalanceOf<Self>>;

        /// Converter from the TickNumber type to the Balance type.
        type TickNumberToBalance: Convert<TickNumber<Self>, BalanceOf<Self>>;

        /// Converter from the StorageDataUnit type to the Balance type.
        type StorageDataUnitToBalance: Convert<StorageDataUnit<Self>, BalanceOf<Self>>;

        /// Off-Chain signature type.
        ///
        /// Can verify whether an `Self::OffchainPublicKey` created a signature.
        type OffchainSignature: Verify<Signer = Self::OffchainPublicKey> + Parameter;

        /// Off-Chain public key.
        ///
        /// Must identify as an on-chain `Self::AccountId`.
        type OffchainPublicKey: IdentifyAccount<AccountId = Self::AccountId>;

        /// The treasury account of the runtime, where a fraction of each payment goes.
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;

        /// Penalty payed by a BSP when they forcefully stop storing a file.
        #[pallet::constant]
        type BspStopStoringFilePenalty: Get<BalanceOf<Self>>;

        /// The deposit paid by a user to create a new file deletion request.
        ///
        /// This deposit gets returned to the user when the MSP submits an inclusion proof of the file to
        /// confirm its deletion, but gets sent to the MSP if the MSP did not actually had the file and
        /// sends a non-inclusion proof instead. This is done to prevent users being able to spam MSPs
        /// with malicious file deletion requests.
        #[pallet::constant]
        type FileDeletionRequestDeposit: Get<BalanceOf<Self>>;

        /// Maximum batch of storage requests that can be confirmed at once when calling `bsp_confirm_storing`.
        #[pallet::constant]
        type MaxBatchConfirmStorageRequests: Get<u32>;

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

        /// Maximum number of expired items (per type) to clean up in a single tick.
        #[pallet::constant]
        type MaxExpiredItemsInTick: Get<u32>;

        /// Time-to-live for a storage request.
        #[pallet::constant]
        type StorageRequestTtl: Get<u32>;

        /// Time-to-live for a move bucket request, after which the request is considered expired.
        #[pallet::constant]
        type MoveBucketRequestTtl: Get<u32>;

        /// Maximum number of file deletion requests a user can have pending.
        #[pallet::constant]
        type MaxUserPendingDeletionRequests: Get<u32>;

        /// Maximum number of move bucket requests a user can have pending.
        #[pallet::constant]
        type MaxUserPendingMoveBucketRequests: Get<u32>;

        /// Number of ticks required to pass between a BSP requesting to stop storing a file and it being able to confirm to stop storing it.
        #[pallet::constant]
        type MinWaitForStopStoring: Get<TickNumber<Self>>;

        /// Base deposit held from the User when creating a new storage request. The actual deposit held is this amount
        /// plus the amount required to pay for all BSP's `bsp_volunteer` extrinsic.
        #[pallet::constant]
        type BaseStorageRequestCreationDeposit: Get<BalanceOf<Self>>;

        /// Basic security replication target for a new storage request.
        ///
        /// This should be high enough so that it gives users a ~1% chance of their file
        /// being controlled by a single malicious entity under certain network conditions.
        ///
        /// For more details, see [crate::types::ReplicationTarget].
        #[pallet::constant]
        type BasicReplicationTarget: Get<ReplicationTargetType<Self>>;

        /// Standard security replication target for a new storage request.
        ///
        /// This should be high enough so that it gives users a ~0.1% chance of their file
        /// being controlled by a single malicious entity under certain network conditions.
        ///
        /// For more details, see [crate::types::ReplicationTarget].
        #[pallet::constant]
        type StandardReplicationTarget: Get<ReplicationTargetType<Self>>;

        /// High security replication target for a new storage request.
        ///
        /// This should be high enough so that it gives users a ~0.01% chance of their file
        /// being controlled by a single malicious entity under certain network conditions.
        ///
        /// For more details, see [crate::types::ReplicationTarget].
        #[pallet::constant]
        type HighSecurityReplicationTarget: Get<ReplicationTargetType<Self>>;

        /// Super high security replication target for a new storage request.
        ///
        /// This should be high enough so that it gives users a ~0.001% chance of their file
        /// being controlled by a single malicious entity under certain network conditions.
        ///
        /// For more details, see [crate::types::ReplicationTarget].
        #[pallet::constant]
        type SuperHighSecurityReplicationTarget: Get<ReplicationTargetType<Self>>;

        /// Ultra high security replication target for a new storage request.
        ///
        /// This should be high enough so that it gives users a ~0.0001% chance of their file
        /// being controlled by a single malicious entity under certain network conditions.
        ///
        /// For more details, see [crate::types::ReplicationTarget].
        #[pallet::constant]
        type UltraHighSecurityReplicationTarget: Get<ReplicationTargetType<Self>>;

        /// Maximum replication target that a user can select for a new storage request.
        #[pallet::constant]
        type MaxReplicationTarget: Get<u32>;

        /// The amount of ticks that the user has to pay upfront when issuing a storage request.
        ///
        /// This is to compensate the system load that the process of file retrieval will have on the network.
        /// If this did not exist, a malicious user could spam the network with huge files, making BSPs change
        /// their capacity and download a lot of data while the user might not even have the balance to
        /// store and pay those BSPs in the long term.
        ///
        /// It initially exists as a deterrent, since these funds will be transferred to the treasury and not to the BSPs
        /// of the network. Governance can then decide what to do with these funds.
        ///
        /// The amount that the user is going to have to pay is calculated as follows:
        /// `Replication Target Chosen * PricePerGigaUnitPerTick * File Size in Gigabytes * UpfrontTicksToPay`
        #[pallet::constant]
        type UpfrontTicksToPay: Get<TickNumber<Self>>;

        /// The amount of ticks that have to pass for the threshold to volunteer for a specific storage request
        /// to arrive at its maximum value.
        #[pallet::constant]
        type TickRangeToMaximumThreshold: Get<TickNumber<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type StorageRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, MerkleHash<T>, StorageRequestMetadata<T>>;

    /// A double map from file key to the BSP IDs of the BSPs that volunteered to store the file to whether that BSP has confirmed storing it.
    ///
    /// Any BSP under a file key prefix is considered to be a volunteer and can be removed at any time.
    /// Once a BSP submits a valid proof via the `bsp_confirm_storing` extrinsic, the `confirmed` field in [`StorageRequestBspsMetadata`] will be set to `true`.
    ///
    /// When a storage request is expired or removed, the corresponding file key prefix in this map is removed.
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

    /// A map of ticks to expired storage requests.
    #[pallet::storage]
    pub type StorageRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TickNumber<T>,
        BoundedVec<StorageRequestExpirationItem<T>, T::MaxExpiredItemsInTick>,
        ValueQuery,
    >;

    /// A map of ticks to expired move bucket requests.
    #[pallet::storage]
    pub type MoveBucketRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        TickNumber<T>,
        BoundedVec<BucketIdFor<T>, T::MaxExpiredItemsInTick>,
        ValueQuery,
    >;

    /// A pointer to the earliest available tick to insert a new storage request expiration.
    ///
    /// This should always be greater or equal than current tick + [`Config::StorageRequestTtl`].
    #[pallet::storage]
    pub type NextAvailableStorageRequestExpirationTick<T: Config> =
        StorageValue<_, TickNumber<T>, ValueQuery>;

    /// A pointer to the earliest available tick to insert a new move bucket request expiration.
    ///
    /// This should always be greater or equal than current tick + [`Config::MoveBucketRequestTtl`].
    #[pallet::storage]
    pub type NextAvailableMoveBucketRequestExpirationTick<T: Config> =
        StorageValue<_, TickNumber<T>, ValueQuery>;

    /// A pointer to the starting tick to clean up expired items.
    ///
    /// If this tick is behind the current tick number, the cleanup algorithm in `on_idle` will
    /// attempt to advance this tick pointer as close to or up to the current tick number. This
    /// will execute provided that there is enough remaining weight to do so.
    #[pallet::storage]
    pub type NextStartingTickToCleanUp<T: Config> = StorageValue<_, TickNumber<T>, ValueQuery>;

    /// Pending file deletion requests.
    ///
    /// A mapping from a user Account ID to a list of pending file deletion requests (which have the file information).
    #[pallet::storage]
    pub type PendingFileDeletionRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<PendingFileDeletionRequest<T>, T::MaxUserPendingDeletionRequests>,
        ValueQuery,
    >;

    /// Mapping from MSPs to the amount of pending file deletion requests they have.
    ///
    /// This is used to keep track of the amount of pending file deletion requests each MSP has, so that MSPs are removed
    /// from the privileged providers list if they have at least one, and are added back if they have none.
    /// This is to ensure that MSPs are correctly incentivised to submit the required proofs for file deletions.
    #[pallet::storage]
    pub type MspsAmountOfPendingFileDeletionRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, ProviderIdFor<T>, u32, ValueQuery>;

    /// Pending file stop storing requests.
    ///
    /// A double mapping from BSP IDs to a list of file keys pending stop storing requests to the block in which those requests were opened,
    /// the proven size of the file and the owner of the file.
    /// The block number is used to avoid BSPs being able to stop storing files immediately which would allow them to avoid challenges
    /// of missing files. The size is to be able to decrease their used capacity when they confirm to stop storing the file.
    /// The owner is to be able to update the payment stream between the user and the BSP.
    #[pallet::storage]
    pub type PendingStopStoringRequests<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ProviderIdFor<T>,
        Blake2_128Concat,
        MerkleHash<T>,
        PendingStopStoringRequest<T>,
    >;

    /// Pending move bucket requests.
    ///
    /// A mapping from Bucket ID to their move bucket request metadata, which includes the new MSP
    /// and value propositions that this bucket would take if accepted.
    #[pallet::storage]
    pub type PendingMoveBucketRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, BucketIdFor<T>, MoveBucketRequestMetadata<T>>;

    /// Incomplete storage requests that need provider-by-provider file removal.
    ///
    /// This mapping tracks storage requests that have been expired or rejected but still have
    /// confirmed providers storing files. Each entry tracks which providers still need to remove
    /// their files. Once all providers have removed their files, the entry is automatically cleaned up.
    #[pallet::storage]
    pub type IncompleteStorageRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, MerkleHash<T>, IncompleteStorageRequestMetadata<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that a new bucket has been created.
        NewBucket {
            who: T::AccountId,
            msp_id: ProviderIdFor<T>,
            bucket_id: BucketIdFor<T>,
            name: BucketNameFor<T>,
            root: MerkleHash<T>,
            collection_id: Option<CollectionIdFor<T>>,
            private: bool,
            value_prop_id: ValuePropId<T>,
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
            new_value_prop_id: ValuePropId<T>,
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
            size: StorageDataUnit<T>,
            peer_ids: PeerIds<T>,
            expires_at: TickNumber<T>,
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
            size: StorageDataUnit<T>,
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
        /// Notifies that a MSP has stopped storing a bucket because its owner has become insolvent.
        MspStopStoringBucketInsolventUser {
            msp_id: ProviderIdFor<T>,
            owner: T::AccountId,
            bucket_id: BucketIdFor<T>,
        },
        /// Notifies that a priority challenge with a trie remove mutation failed to be queued in the `on_idle` hook.
        /// This can happen if the priority challenge queue is full, and the failed challenge should be manually
        /// queued at a later time.
        FailedToQueuePriorityChallenge {
            file_key: MerkleHash<T>,
            error: DispatchError,
        },
        /// Notifies that a file will be deleted.
        FileDeletionRequest {
            user: T::AccountId,
            file_key: MerkleHash<T>,
            file_size: StorageDataUnit<T>,
            bucket_id: BucketIdFor<T>,
            msp_id: ProviderIdFor<T>,
            proof_of_inclusion: bool,
        },
        /// Notifies that a proof has been submitted for a pending file deletion request.
        ProofSubmittedForPendingFileDeletionRequest {
            user: T::AccountId,
            file_key: MerkleHash<T>,
            file_size: StorageDataUnit<T>,
            bucket_id: BucketIdFor<T>,
            msp_id: ProviderIdFor<T>,
            proof_of_inclusion: bool,
        },
        /// Notifies that a BSP's challenge cycle has been initialised, adding the first file
        /// key(s) to the BSP's Merkle Patricia Forest.
        BspChallengeCycleInitialised {
            who: T::AccountId,
            bsp_id: ProviderIdFor<T>,
        },
        /// Notifies that a move bucket request has expired.
        MoveBucketRequestExpired { bucket_id: BucketIdFor<T> },
        /// Notifies that a bucket has been moved to a new MSP under a new value proposition.
        MoveBucketAccepted {
            bucket_id: BucketIdFor<T>,
            old_msp_id: Option<ProviderIdFor<T>>,
            new_msp_id: ProviderIdFor<T>,
            value_prop_id: ValuePropId<T>,
        },
        /// Notifies that a bucket move request has been rejected by the MSP.
        MoveBucketRejected {
            bucket_id: BucketIdFor<T>,
            old_msp_id: Option<ProviderIdFor<T>>,
            new_msp_id: ProviderIdFor<T>,
        },
        /// Notifies that a MSP has stopped storing a bucket.
        MspStoppedStoringBucket {
            msp_id: ProviderIdFor<T>,
            owner: T::AccountId,
            bucket_id: BucketIdFor<T>,
        },
        /// Failed to get the MSP owner of the bucket for an expired file deletion request
        /// This is different from the bucket not having a MSP, which is allowed and won't error
        FailedToGetMspOfBucket {
            bucket_id: BucketIdFor<T>,
            error: DispatchError,
        },
        /// Failed to decrease MSP's used capacity for expired file deletion request
        FailedToDecreaseMspUsedCapacity {
            user: T::AccountId,
            msp_id: ProviderIdFor<T>,
            file_key: MerkleHash<T>,
            file_size: StorageDataUnit<T>,
            error: DispatchError,
        },
        /// Event to notify of incoherencies in used capacity.
        UsedCapacityShouldBeZero {
            actual_used_capacity: StorageDataUnit<T>,
        },
        /// Event to notify if, in the `on_idle` hook when cleaning up an expired storage request,
        /// the return of that storage request's deposit to the user failed.
        FailedToReleaseStorageRequestCreationDeposit {
            file_key: MerkleHash<T>,
            owner: T::AccountId,
            amount_to_return: BalanceOf<T>,
            error: DispatchError,
        },
        /// Event to notify if, in the `on_idle` hook when cleaning up an expired storage request,
        /// the transfer of a part of that storage request's deposit to one of the volunteered BSPs failed.
        FailedToTransferDepositFundsToBsp {
            file_key: MerkleHash<T>,
            owner: T::AccountId,
            bsp_id: ProviderIdFor<T>,
            amount_to_transfer: BalanceOf<T>,
            error: DispatchError,
        },
        /// Notifies that a file deletion has been requested.
        /// Contains a signed intention that allows any actor to execute the actual deletion.
        FileDeletionRequested {
            signed_delete_intention: FileOperationIntention<T>,
            signature: T::OffchainSignature,
        },
        /// Notifies that a file deletion has been completed successfully for an MSP.
        MspFileDeletionCompleted {
            user: T::AccountId,
            file_key: MerkleHash<T>,
            file_size: StorageDataUnit<T>,
            bucket_id: BucketIdFor<T>,
            msp_id: ProviderIdFor<T>,
            old_root: MerkleHash<T>,
            new_root: MerkleHash<T>,
        },
        /// Notifies that a file deletion has been completed successfully for a BSP.
        BspFileDeletionCompleted {
            user: T::AccountId,
            file_key: MerkleHash<T>,
            file_size: StorageDataUnit<T>,
            bsp_id: ProviderIdFor<T>,
            old_root: MerkleHash<T>,
            new_root: MerkleHash<T>,
        },
        /// Notifies that a file has been deleted from a rejected storage request.
        FileDeletedFromIncompleteStorageRequest {
            file_key: MerkleHash<T>,
            provider_id: ProviderIdFor<T>,
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
        /// No slot available found in ticks to insert storage request expiration time.
        StorageRequestExpiredNoSlotAvailable,
        /// Not authorized to delete the storage request.
        StorageRequestNotAuthorized,
        /// Error created in 2024. If you see this, you are well beyond the singularity and should
        /// probably stop using this pallet.
        MaxTickNumberReached,
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
        /// The selected value proposition is not available in the MSP.
        ValuePropositionNotAvailable,
        /// Collection ID was not found.
        CollectionNotFound,
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
        /// No BSP reputation weight set.
        NoBspReputationWeightSet,
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
        /// Certain operations (such as issuing new storage requests) are not allowed when interacting with insolvent users.
        OperationNotAllowedWithInsolventUser,
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
        /// Failed to fetch the dynamic-rate payment stream.
        DynamicRatePaymentStreamNotFound,
        /// Cannot hold the required deposit from the user
        CannotHoldDeposit,
        /// Failed to query earliest volunteer tick
        FailedToQueryEarliestFileVolunteerTick,
        /// Failed to get owner account of ID of provider
        FailedToGetOwnerAccount,
        /// Failed to get the payment account of the provider.
        FailedToGetPaymentAccount,
        /// No file keys to confirm storing
        NoFileKeysToConfirm,
        /// Root was not updated after applying delta
        RootNotUpdated,
        /// Privacy update results in no change
        NoPrivacyChange,
        /// Operations not allowed for insolvent provider
        OperationNotAllowedForInsolventProvider,
        /// Operations not allowed while bucket is not being stored by an MSP
        OperationNotAllowedWhileBucketIsNotStoredByMsp,
        /// Failed to compute file key
        FailedToComputeFileKey,
        /// Failed to create file metadata
        FailedToCreateFileMetadata,
        /// Invalid signature provided for file operation
        InvalidSignature,
        /// Forest proof verification failed.
        ForestProofVerificationFailed,
        /// Provider is not storing the file.
        ProviderNotStoringFile,
        /// Invalid provider ID provided.
        InvalidProviderID,
        /// Invalid signed operation provided.
        InvalidSignedOperation,
        /// Storage request is not in rejected state.
        StorageRequestNotRejected,
        /// File key computed from metadata doesn't match the provided file key.
        FileKeyMismatch,
        /// Storage request metadata is corrupted or inconsistent.
        CorruptedStorageRequest,
    }

    /// This enum holds the HoldReasons for this pallet, allowing the runtime to identify each held balance with different reasons separately
    ///
    /// This allows us to hold tokens and be able to identify in the future that those held tokens were
    /// held because of this pallet
    #[pallet::composite_enum]
    pub enum HoldReason {
        /// Deposit that a user has to pay to create a new storage request
        StorageRequestCreationHold,
        /// Deposit that a user has to pay to create a new file deletion request
        FileDeletionRequestHold,
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
            msp_id: ProviderIdFor<T>,
            name: BucketNameFor<T>,
            private: bool,
            value_prop_id: ValuePropId<T>,
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
        #[pallet::weight(T::WeightInfo::request_move_bucket())]
        pub fn request_move_bucket(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            new_msp_id: ProviderIdFor<T>,
            new_value_prop_id: ValuePropId<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_request_move_bucket(who.clone(), bucket_id, new_msp_id, new_value_prop_id)?;

            Self::deposit_event(Event::MoveBucketRequested {
                who,
                bucket_id,
                new_msp_id,
                new_value_prop_id,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::msp_respond_move_bucket_request())]
        pub fn msp_respond_move_bucket_request(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            response: BucketMoveRequestResponse,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (old_msp_id, new_msp_id, value_prop_id) =
                Self::do_msp_respond_move_bucket_request(who.clone(), bucket_id, response.clone())?;

            match response {
                BucketMoveRequestResponse::Accepted => {
                    Self::deposit_event(Event::MoveBucketAccepted {
                        bucket_id,
                        old_msp_id,
                        new_msp_id,
                        value_prop_id,
                    });
                }
                BucketMoveRequestResponse::Rejected => {
                    Self::deposit_event(Event::MoveBucketRejected {
                        bucket_id,
                        old_msp_id,
                        new_msp_id,
                    });
                }
            }

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_bucket_privacy())]
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
        #[pallet::weight(T::WeightInfo::create_and_associate_collection_with_bucket())]
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
        #[pallet::weight(T::WeightInfo::delete_bucket())]
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
            size: StorageDataUnit<T>,
            msp_id: ProviderIdFor<T>,
            peer_ids: PeerIds<T>,
            replication_target: ReplicationTarget<T>,
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
                Some(msp_id),
                replication_target,
                Some(peer_ids.clone()),
            )?;

            Ok(())
        }

        /// Revoke storage request
        #[pallet::call_index(7)]
        #[pallet::weight({
          let confirmed = StorageRequests::<T>::get(file_key).map_or(0, |metadata| metadata.bsps_confirmed.into());
          let weight = T::WeightInfo::revoke_storage_request(confirmed as u32);

          weight.saturating_add(T::DbWeight::get().reads_writes(1, 0))
        })]
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
        #[pallet::weight({
			let total_weight: Weight = Weight::zero();
			for bucket_response in storage_request_msp_response.iter() {
				let amount_of_files_to_accept = bucket_response.accept.as_ref().map_or(0, |accept_response| accept_response.file_keys_and_proofs.len());
				let amount_of_files_to_reject = bucket_response.reject.len();

				total_weight.saturating_add(T::WeightInfo::msp_respond_storage_requests_multiple_buckets(1, amount_of_files_to_accept as u32, amount_of_files_to_reject as u32));
			}
			total_weight
		})]
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
        #[pallet::weight(T::WeightInfo::msp_stop_storing_bucket())]
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
        #[pallet::weight(T::WeightInfo::bsp_volunteer())]
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
        #[pallet::weight(T::WeightInfo::bsp_confirm_storing(file_keys_and_proofs.len() as u32))]
        pub fn bsp_confirm_storing(
            origin: OriginFor<T>,
            non_inclusion_forest_proof: ForestProof<T>,
            file_keys_and_proofs: BoundedVec<
                FileKeyWithProof<T>,
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
        #[pallet::weight(T::WeightInfo::bsp_request_stop_storing())]
        pub fn bsp_request_stop_storing(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            owner: T::AccountId,
            fingerprint: Fingerprint<T>,
            size: StorageDataUnit<T>,
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
        #[pallet::weight(T::WeightInfo::bsp_confirm_stop_storing())]
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
        #[pallet::weight(T::WeightInfo::stop_storing_for_insolvent_user_bsp().max(T::WeightInfo::stop_storing_for_insolvent_user_msp()))]
        pub fn stop_storing_for_insolvent_user(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            owner: T::AccountId,
            fingerprint: Fingerprint<T>,
            size: StorageDataUnit<T>,
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

        /// Executed by a MSP to stop storing a bucket from an insolvent user.
        ///
        /// This is used when a user has become insolvent and the MSP needs to stop storing the buckets of that user, since
        /// it won't be getting paid for them anymore.
        /// It validates that:
        /// - The sender is the MSP that's currently storing the bucket, and the bucket exists.
        /// - That the user is currently insolvent OR
        /// - That the payment stream between the MSP and user doesn't exist (which would occur as a consequence of the MSP previously
        /// having deleted another bucket it was storing for this user through this extrinsic).
        /// And then completely removes the bucket from the system.
        ///
        /// If there was a storage request pending for the bucket, it will eventually expire without being fulfilled (because the MSP can't
        /// accept storage requests for insolvent users and BSPs can't volunteer nor confirm them either) and afterwards any BSPs that
        /// had confirmed the file can just call `sp_stop_storing_for_insolvent_user` to get rid of it.
        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::msp_stop_storing_bucket_for_insolvent_user())]
        pub fn msp_stop_storing_bucket_for_insolvent_user(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Perform validations and stop storing the file.
            let (msp_id, owner) =
                Self::do_msp_stop_storing_bucket_for_insolvent_user(who.clone(), bucket_id)?;

            // Emit event.
            Self::deposit_event(Event::MspStopStoringBucketInsolventUser {
                msp_id,
                owner,
                bucket_id,
            });

            Ok(())
        }

        /// Request deletion of a file using a signed delete intention.
        ///
        /// The origin must be signed and the signature must be valid for the given delete intention.
        /// The delete intention must contain the file key and the delete operation.
        /// File metadata is provided separately for ownership verification.
        #[pallet::call_index(16)]
        #[pallet::weight(Weight::zero())]
        pub fn request_delete_file(
            origin: OriginFor<T>,
            signed_intention: FileOperationIntention<T>,
            signature: T::OffchainSignature,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            size: StorageDataUnit<T>,
            fingerprint: Fingerprint<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_request_delete_file(
                who.clone(),
                signed_intention.clone(),
                signature.clone(),
                bucket_id,
                location,
                size,
                fingerprint,
            )?;

            // Emit the event
            Self::deposit_event(Event::FileDeletionRequested {
                signed_delete_intention: signed_intention,
                signature,
            });

            Ok(())
        }

        /// Deletes a file from a provider's forest, changing its root
        ///
        /// This extrinsic allows any actor to execute file deletion based on signed intentions
        /// from the `FileDeletionRequested` event. It requires a valid forest proof showing that the
        /// file exists in the specified provider's forest before allowing deletion.
        #[pallet::call_index(17)]
        #[pallet::weight(Weight::zero())]
        pub fn delete_file(
            origin: OriginFor<T>,
            file_owner: T::AccountId,
            signed_intention: FileOperationIntention<T>,
            signature: T::OffchainSignature,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            size: StorageDataUnit<T>,
            fingerprint: Fingerprint<T>,
            provider_id: ProviderIdFor<T>,
            forest_proof: ForestProof<T>,
        ) -> DispatchResult {
            // TODO: We need to reward the caller of delete_file
            let _caller = ensure_signed(origin)?;

            Self::do_delete_file(
                file_owner,
                signed_intention,
                signature,
                bucket_id,
                location,
                size,
                fingerprint,
                provider_id,
                forest_proof,
            )?;

            Ok(())
        }

        /// Delete a file from an incomplete (rejected, expired or revoked) storage request.
        ///
        /// This extrinsic allows fisherman nodes to delete files from providers when the storage request
        /// has been marked as rejected or revoked. It validates that the storage request exists, is rejected or revoked,
        /// the provider actually stores the file, and verifies the file key matches the metadata.
        #[pallet::call_index(18)]
        #[pallet::weight(Weight::zero())]
        pub fn delete_file_for_incomplete_storage_request(
            origin: OriginFor<T>,
            file_key: MerkleHash<T>,
            provider_id: ProviderIdFor<T>,
            forest_proof: ForestProof<T>,
        ) -> DispatchResult {
            // TODO: We need to reward the caller of delete_file_for_incomplete_storage_request
            let _caller = ensure_signed(origin)?;

            Self::do_delete_file_for_incomplete_storage_request(
                file_key,
                provider_id,
                forest_proof,
            )?;

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_poll(_n: BlockNumberFor<T>, weight: &mut frame_support::weights::WeightMeter) {
            Self::do_on_poll(weight);
        }

        fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            let mut meter = WeightMeter::with_limit(remaining_weight);
            // If there's enough weight to at least read the current tick number, do it and proceed.
            if meter.can_consume(T::DbWeight::get().reads(1)) {
                let current_tick = <T::ProofDealer as ProofsDealerInterface>::get_current_tick();
                meter.consume(T::DbWeight::get().reads(1));
                Self::do_on_idle(current_tick, &mut meter);
            }

            meter.consumed()
        }

        /// Any code located in this hook is placed in an auto-generated test, and generated as a part
        /// of crate::construct_runtime's expansion.
        /// Look for a test case with a name along the lines of: __construct_runtime_integrity_test.
        fn integrity_test() {
            let basic_replication_target = T::BasicReplicationTarget::get();
            let standard_replication_target = T::StandardReplicationTarget::get();
            let high_security_replication_target = T::HighSecurityReplicationTarget::get();
            let super_high_security_replication_target =
                T::SuperHighSecurityReplicationTarget::get();
            let ultra_high_security_replication_target =
                T::UltraHighSecurityReplicationTarget::get();
            let max_replication_target: ReplicationTargetType<T> =
                T::MaxReplicationTarget::get().into();
            let storage_request_ttl = T::StorageRequestTtl::get();
            let tick_range_to_max_threshold = T::TickRangeToMaximumThreshold::get();
            let min_wait_for_stop_storing = T::MinWaitForStopStoring::get();
            let checkpoint_challenge_period =
                <<T as crate::Config>::ProofDealer as ProofsDealerInterface>::get_checkpoint_challenge_period();
            let base_storage_request_creation_deposit = T::BaseStorageRequestCreationDeposit::get();
            let bsp_volunteer_fee = <T::WeightToFee as sp_weights::WeightToFee>::weight_to_fee(
                &T::WeightInfo::bsp_volunteer(),
            );

            assert!(
                basic_replication_target > T::ReplicationTargetType::zero(),
                "Basic security replication target cannot be zero."
            );
            assert!(
				standard_replication_target >= basic_replication_target,
				"Standard security replication target cannot be smaller than basic security replication target."
			);
            assert!(
				high_security_replication_target >= standard_replication_target,
				"High security replication target cannot be smaller than standard security replication target."
			);
            assert!(
				super_high_security_replication_target >= high_security_replication_target,
				"Super high security replication target cannot be smaller than high security replication target."
			);
            assert!(
				ultra_high_security_replication_target >= super_high_security_replication_target,
				"Ultra high security replication target cannot be smaller than super high security replication target."
			);
            assert!(
                max_replication_target >= ultra_high_security_replication_target,
                "Max replication target cannot be smaller than the most secure replication target."
            );

            assert!(tick_range_to_max_threshold < storage_request_ttl.into(), "Storage request TTL must be greater than the tick range to maximum threshold so storage requests get to their maximum threshold before expiring.");

            // The checkpoint challenge period already greater than the longest challenge period a BSP can have + the tolerance,
            // so by ensuring the minimum wait for stop storing is greater than the checkpoint challenge period, we ensure that
            // the BSP cannot immediately stop storing a file it has lost when receiving a challenge for it.
            assert!(min_wait_for_stop_storing > checkpoint_challenge_period, "Minimum amount of blocks between the stop storing request opening and being able to confirm it cannot be smaller than the checkpoint challenge period.");

            // The base deposit for a storage request creation should be enough to cover the fees to volunteer for at least `basic_replication_target` BSPs.
            assert!(base_storage_request_creation_deposit >= bsp_volunteer_fee.saturating_mul(T::ReplicationTargetToBalance::convert(basic_replication_target)), "Base storage request creation deposit should be enough to cover the fees to volunteer for at least `basic_replication_target` BSPs.");
        }
    }
}

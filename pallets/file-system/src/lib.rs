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

pub mod types;
mod utils;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use super::types::*;
    use codec::HasCompact;
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::{ValueQuery, *},
        sp_runtime::traits::{CheckEqual, Convert, MaybeDisplay, SimpleBitOps},
        traits::{
            nonfungibles_v2::{Create, Inspect as NonFungiblesInspect},
            Currency,
        },
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use sp_runtime::BoundedVec;
    use sp_runtime::{
        traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Saturating, Zero},
        FixedPointNumber,
    };

    use shp_file_metadata::ChunkId;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The trait for reading and mutating storage provider data.
        type Providers: shp_traits::ReadProvidersInterface<AccountId = Self::AccountId, BucketId = <Self::Providers as shp_traits::ProvidersInterface>::ProviderId>
            + shp_traits::MutateProvidersInterface<AccountId = Self::AccountId, ReadAccessGroupId = CollectionIdFor<Self>,
            MerklePatriciaRoot = <Self::ProofDealer as shp_traits::ProofsDealerInterface>::MerkleHash>;

        /// The trait for issuing challenges and verifying proofs.
        type ProofDealer: shp_traits::ProofsDealerInterface<
            ProviderId = <Self::Providers as shp_traits::ProvidersInterface>::ProviderId,
            MerkleHash = <Self::Providers as shp_traits::ProvidersInterface>::MerkleHash,
        >;

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
        type StorageRequestBspsRequiredType: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + Into<u32>
            + Copy
            + MaxEncodedLen
            + HasCompact
            + Copy
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
            + Copy
            + MaxEncodedLen
            + Decode
            + Saturating
            + CheckedMul
            + CheckedDiv
            + CheckedAdd
            + CheckedSub
            + PartialOrd
            + FixedPointNumber;

        /// The type to convert a threshold to a block number.
        type ThresholdTypeToBlockNumber: Convert<Self::ThresholdType, BlockNumberFor<Self>>;

        /// The type to convert a block number to a threshold.
        type BlockNumberToThresholdType: Convert<BlockNumberFor<Self>, Self::ThresholdType>;

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
        type Currency: Currency<Self::AccountId>;

        /// Registry for minted NFTs.
        type Nfts: NonFungiblesInspect<Self::AccountId>
            + Create<Self::AccountId, CollectionConfigFor<Self>>;

        /// Collection inspector
        type CollectionInspector: shp_traits::InspectCollections<
            CollectionId = CollectionIdFor<Self>,
        >;

        /// The multiplier increases the threshold over time (blocks) which increases the
        /// likelihood of a BSP successfully volunteering to store a file.
        #[pallet::constant]
        type AssignmentThresholdMultiplier: Get<Self::ThresholdType>;

        /// Horizontal asymptote which the volunteering threshold approaches as more BSPs are registered in the system.
        #[pallet::constant]
        type AssignmentThresholdAsymptote: Get<Self::ThresholdType>;

        /// Asymptotic decay function for the assignment threshold.
        #[pallet::constant]
        type AssignmentThresholdDecayFactor: Get<Self::ThresholdType>;

        /// Minimum number of BSPs required to store a file.
        ///
        /// This is also used as a default value if the BSPs required are not specified when creating a storage request.
        #[pallet::constant]
        type TargetBspsRequired: Get<Self::StorageRequestBspsRequiredType>;

        /// Maximum number of BSPs that can store a file.
        ///
        /// This is used to limit the number of BSPs storing a file and claiming rewards for it.
        /// If this number is too high, then the reward for storing a file might be to diluted and pointless to store.
        #[pallet::constant]
        type MaxBspsPerStorageRequest: Get<u32>;

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

        /// Maximum number of expired storage requests to clean up in a single block.
        #[pallet::constant]
        type MaxExpiredItemsInBlock: Get<u32>;

        /// Time-to-live for a storage request.
        #[pallet::constant]
        type StorageRequestTtl: Get<u32>;

        /// Time-to-live for a pending file deletion request, after which a priority challenge is sent out to enforce the deletion.        #[pallet::constant]
        type PendingFileDeletionRequestTtl: Get<u32>;

        /// Maximum number of file deletion requests a user can have pending.
        #[pallet::constant]
        type MaxUserPendingDeletionRequests: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn storage_requests)]
    pub type StorageRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, MerkleHash<T>, StorageRequestMetadata<T>>;

    /// A double map from storage request to BSP `AccountId`s that volunteered to store the file.
    ///
    /// Any BSP under a storage request prefix is considered to be a volunteer and can be removed at any time.
    /// Once a BSP submits a valid proof to the via the `bsp_confirm_storing` extrinsic, the `confirmed` field in [`StorageRequestBspsMetadata`] will be set to `true`.
    ///
    /// When a storage request is expired or removed, the corresponding storage request prefix in this map is removed.
    #[pallet::storage]
    #[pallet::getter(fn storage_request_bsps)]
    pub type StorageRequestBsps<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        MerkleHash<T>,
        Blake2_128Concat,
        ProviderIdFor<T>,
        StorageRequestBspsMetadata<T>,
        OptionQuery,
    >;

    /// A map of blocks to expired storage requests.
    #[pallet::storage]
    #[pallet::getter(fn item_expirations)]
    pub type ItemExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<ExpiredItems<T>, T::MaxExpiredItemsInBlock>,
        ValueQuery,
    >;

    /// A pointer to the earliest available block to insert a new storage request expiration.
    ///
    /// This should always be greater or equal than current block + [`Config::StorageRequestTtl`].
    #[pallet::storage]
    #[pallet::getter(fn next_available_expiration_insertion_block)]
    pub type NextAvailableExpirationInsertionBlock<T: Config> =
        StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// A pointer to the starting block to clean up expired storage requests.
    ///
    /// If this block is behind the current block number, the cleanup algorithm in `on_idle` will
    /// attempt to accelerate this block pointer as close to or up to the current block number. This
    /// will execute provided that there is enough remaining weight to do so.
    #[pallet::storage]
    #[pallet::getter(fn next_starting_block_to_clean_up)]
    pub type NextStartingBlockToCleanUp<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Pending file deletion requests.
    ///
    /// A mapping from a user account id to a list of pending file deletion requests, holding a tuple of the file key and bucket id.
    #[pallet::storage]
    #[pallet::getter(fn pending_file_deletion_requests)]
    pub type PendingFileDeletionRequests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<(MerkleHash<T>, BucketIdFor<T>), T::MaxUserPendingDeletionRequests>,
        ValueQuery,
    >;

    /// Minimum BSP assignment threshold.
    ///
    /// This is the minimum threshold that a BSP must have to be assigned to store a file.
    /// It is reduced or increased when BSPs sign off or sign up respectively.
    #[pallet::storage]
    #[pallet::getter(fn bsps_assignment_threshold)]
    pub type BspsAssignmentThreshold<T: Config> = StorageValue<_, T::ThresholdType, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub bsp_assignment_threshold: T::ThresholdType,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            let total_bsps =
                <T::Providers as shp_traits::ReadProvidersInterface>::get_number_of_bsps()
                    .try_into()
                    .map_err(|_| Error::<T>::FailedTypeConversion)
                    .unwrap();

            let bsp_assignment_threshold =
                Pallet::<T>::compute_asymptotic_threshold_point(total_bsps).unwrap();

            BspsAssignmentThreshold::<T>::put(bsp_assignment_threshold);

            Self {
                bsp_assignment_threshold: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            BspsAssignmentThreshold::<T>::put(self.bsp_assignment_threshold);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that a new bucket has been created.
        NewBucket {
            who: T::AccountId,
            msp_id: ProviderIdFor<T>,
            bucket_id: BucketIdFor<T>,
            name: BucketNameFor<T>,
            collection_id: Option<CollectionIdFor<T>>,
            private: bool,
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
            file_keys: BoundedVec<MerkleHash<T>, T::MaxBatchConfirmStorageRequests>,
            new_root: MerkleHash<T>,
        },
        /// Notifies the expiration of a storage request.
        StorageRequestExpired { file_key: MerkleHash<T> },
        /// Notifies that a storage request has been revoked by the user who initiated it.
        StorageRequestRevoked { file_key: MerkleHash<T> },
        /// Notifies that a BSP has stopped storing a file.
        BspStoppedStoring {
            bsp_id: ProviderIdFor<T>,
            file_key: MerkleHash<T>,
            new_root: MerkleHash<T>,
            owner: T::AccountId,
            location: FileLocation<T>,
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
            bucket_id: ProviderIdFor<T>,
            msp_id: ProviderIdFor<T>,
            proof_of_inclusion: bool,
        },
        /// Notifies that a proof has been submitted for a pending file deletion request.
        ProofSubmittedForPendingFileDeletionRequest {
            msp_id: ProviderIdFor<T>,
            user: T::AccountId,
            file_key: MerkleHash<T>,
            bucket_id: ProviderIdFor<T>,
            proof_of_inclusion: bool,
        },
        /// Notifies that a BSP's challenge cycle has been initialised, adding the first file
        /// key(s) to the BSP's Merkle Patricia Forest.
        BspChallengeCycleInitialised {
            who: T::AccountId,
            bsp_id: ProviderIdFor<T>,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Storage request already registered for the given file.
        StorageRequestAlreadyRegistered,
        /// Storage request not registered for the given file.
        StorageRequestNotFound,
        /// BSPs required for storage request cannot be 0.
        BspsRequiredCannotBeZero,
        /// BSPs required for storage request cannot exceed the maximum allowed.
        BspsRequiredExceedsMax,
        /// Account is not a BSP.
        NotABsp,
        /// Account is not a MSP.
        NotAMsp,
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
        /// Number of removed BSPs volunteered from storage request prefix did not match the expected number.
        UnexpectedNumberOfRemovedVolunteeredBsps,
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
        /// Failed to convert block number to threshold.
        FailedToConvertBlockNumber,
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
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn create_bucket(
            origin: OriginFor<T>,
            msp_id: ProviderIdFor<T>,
            name: BucketNameFor<T>,
            private: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (bucket_id, maybe_collection_id) =
                Self::do_create_bucket(who.clone(), msp_id, name.clone(), private)?;

            Self::deposit_event(Event::NewBucket {
                who,
                msp_id,
                bucket_id,
                name,
                collection_id: maybe_collection_id,
                private,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
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
        #[pallet::call_index(2)]
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

        /// Issue a new storage request for a file
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn issue_storage_request(
            origin: OriginFor<T>,
            bucket_id: BucketIdFor<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
            msp_id: ProviderIdFor<T>,
            peer_ids: PeerIds<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and register storage request
            let file_key = Self::do_request_storage(
                who.clone(),
                bucket_id,
                location.clone(),
                fingerprint,
                size,
                Some(msp_id),
                None,
                Some(peer_ids.clone()),
                Default::default(),
            )?;

            // BSPs listen to this event and volunteer to store the file
            Self::deposit_event(Event::NewStorageRequest {
                who,
                file_key,
                bucket_id,
                location,
                fingerprint,
                size,
                peer_ids,
            });

            Ok(())
        }

        /// Revoke storage request
        #[pallet::call_index(4)]
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

        /// Used by a BSP to volunteer for storing a file.
        ///
        /// The transaction will fail if the XOR between the file ID and the BSP ID is not below the threshold,
        /// so a BSP is strongly advised to check beforehand. Another reason for failure is
        /// if the maximum number of BSPs has been reached. A successful assignment as BSP means
        /// that some of the collateral tokens of that MSP are frozen.
        #[pallet::call_index(5)]
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
        #[pallet::call_index(6)]
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

        /// Executed by a BSP to stop storing a file.
        ///
        /// In the event when a storage request no longer exists for the data the BSP no longer stores,
        /// it is required that the BSP still has access to the metadata of the initial storage request.
        /// If they do not, they will at least need that metadata to reconstruct the File ID and. Wherever
        /// the BSP gets the data it needs is up to it, but one example could be the assigned MSP.
        /// This metadata is necessary since it is needed to reconstruct the leaf node key in the storage
        /// provider's Merkle Forest.
        #[pallet::call_index(7)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_stop_storing(
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

            // Perform validations and stop storing the file.
            let (bsp_id, new_root) = Self::do_bsp_stop_storing(
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
            Self::deposit_event(Event::BspStoppedStoring {
                bsp_id,
                file_key,
                new_root,
                owner,
                location,
            });

            Ok(())
        }

        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn delete_file(
            origin: OriginFor<T>,
            bucket_id: ProviderIdFor<T>,
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

        #[pallet::call_index(9)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn pending_file_deletion_request_submit_proof(
            origin: OriginFor<T>,
            user: T::AccountId,
            file_key: MerkleHash<T>,
            bucket_id: ProviderIdFor<T>,
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

        #[pallet::call_index(10)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn force_update_bsps_assignment_threshold(
            origin: OriginFor<T>,
            bsp_assignment_threshold: T::ThresholdType,
        ) -> DispatchResult {
            // Check that the extrinsic was sent with root origin.
            ensure_root(origin)?;

            ensure!(
                bsp_assignment_threshold >= T::AssignmentThresholdAsymptote::get(),
                Error::<T>::ThresholdBelowAsymptote
            );

            BspsAssignmentThreshold::<T>::put(bsp_assignment_threshold);

            Ok(().into())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        u32: TryFrom<BlockNumberFor<T>>,
    {
        fn on_idle(current_block: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            let mut remaining_weight = remaining_weight;

            Self::do_on_idle(current_block, &mut remaining_weight);

            remaining_weight
        }
    }
}

//! # Voting Pallet
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

mod types;
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
        sp_runtime::traits::{AtLeast32Bit, CheckEqual, MaybeDisplay, SimpleBitOps},
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use sp_runtime::traits::{CheckedAdd, One, Zero};
    use sp_runtime::BoundedVec;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The trait for issuing challenges and verifying proofs.
        type Providers: storage_hub_traits::ProvidersInterface<AccountId = Self::AccountId>;

        /// The trait for issuing challenges and verifying proofs.
        type ProofDealer: storage_hub_traits::ProofsDealerInterface<
            Provider = <Self::Providers as storage_hub_traits::ProvidersInterface>::Provider,
        >;

        /// Type representing the threshold a BSP must meet to be eligible to volunteer to store a file.
        type AssignmentThreshold: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Debug
            + Default
            + MaybeDisplay
            + AtLeast32Bit
            + Copy
            + MaxEncodedLen
            + Decode
            + HasCompact;

        /// The multiplier increases the threshold over time (blocks) which increases the
        /// likelihood of a BSP successfully vulenteering to store a file.
        type AssignmentThresholdMultiplier: Get<u32>;

        /// Minimum BSP assignment threshold.
        #[pallet::constant]
        type MinBspsAssignmentThreshold: Get<Self::AssignmentThreshold>;

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

        /// Unit representing the size of a file.
        type StorageUnit: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + AtLeast32Bit
            + Copy
            + MaxEncodedLen
            + HasCompact;

        /// Type representing the storage request bsps size type.
        type StorageRequestBspsRequiredType: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Default
            + MaybeDisplay
            + AtLeast32Bit
            + Copy
            + MaxEncodedLen
            + HasCompact
            + Copy
            + Default
            + scale_info::TypeInfo
            + MaybeSerializeDeserialize
            + One
            + Zero;

        /// Minimum number of BSPs required to store a file.
        ///
        /// This is also used as a default value if the BSPs required are not specified when creating a storage request.
        #[pallet::constant]
        type TargetBspsRequired: Get<Self::StorageRequestBspsRequiredType>;

        /// Maximum number of BSPs that can store a file.
        ///
        /// This is used to limit the number of BSPs storing a file and claiming rewards for it.
        /// If this number is to high, then the reward for storing a file might be to diluted and pointless to store.
        #[pallet::constant]
        type MaxBspsPerStorageRequest: Get<u32>;

        /// Maximum byte size of a file path.
        #[pallet::constant]
        type MaxFilePathSize: Get<u32>;

        /// Maximum byte size of a libp2p multiaddress.
        #[pallet::constant]
        type MaxMultiAddressSize: Get<u32>;

        /// Maximum number of multiaddresses for a storage request.
        #[pallet::constant]
        type MaxMultiAddresses: Get<u32>;

        /// Time-to-live for a storage request.
        #[pallet::constant]
        type StorageRequestTtl: Get<u32>;

        /// Maximum number of expired storage requests to clean up in a single block.
        #[pallet::constant]
        type MaxExpiredStorageRequests: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn storage_requests)]
    pub type StorageRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, FileLocation<T>, StorageRequestMetadata<T>>;

    /// A double map of [`storage request`](FileLocation) to [`BSPs`](StorageProviderId) that volunteered to store data.
    ///
    /// Any BSP under a storage request is considered to be a volunteer and can be removed at any time.
    /// Once a BSP submits a valid proof to the `pallet-proofs-dealer`, the `confirmed` field in [`StorageRequestBsps`] should be set to `true`.
    ///
    /// When a storage request is expired or removed, the corresponding storage request key in this map should be removed.
    #[pallet::storage]
    #[pallet::getter(fn storage_request_bsps)]
    pub type StorageRequestBsps<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        FileLocation<T>,
        Blake2_128Concat,
        StorageProviderId<T>,
        StorageRequestBspsMetadata<T>,
        OptionQuery,
    >;

    /// A map of blocks to expired storage requests.
    #[pallet::storage]
    #[pallet::getter(fn storage_request_expirations)]
    pub type StorageRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<FileLocation<T>, T::MaxExpiredStorageRequests>,
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that a new file has been requested to be stored.
        NewStorageRequest {
            who: StorageProviderId<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageUnit<T>,
            multiaddresses: BoundedVec<MultiAddress<T>, T::MaxMultiAddresses>,
        },
        /// Notifies that a BSP has been accepted to store a given file.
        AcceptedBspVolunteer {
            who: StorageProviderId<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            multiaddresses: MultiAddresses<T>,
        },
        /// Notifies that a BSP confirmed storing a file.
        BspConfirmedStoring {
            who: StorageProviderId<T>,
            location: FileLocation<T>,
        },
        /// Notifies the expiration of a storage request.
        StorageRequestExpired { location: FileLocation<T> },
        /// Notifies that a storage request has been revoked by the user who initiated it.
        StorageRequestRevoked { location: FileLocation<T> },
        /// Notifies that a BSP has stopped storing a file.
        BspStoppedStoring {
            bsp: StorageProviderId<T>,
            file_key: FileKey<T>,
            owner: StorageProviderId<T>,
            location: FileLocation<T>,
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
        /// BSP already volunteered to store the given file.
        BspVolunteerFailed,
        /// Account is not a BSP.
        NotABsp,
        /// BSP has not volunteered to store the given file.
        BspNotVolunteered,
        /// BSP has already confirmed storing the given file.
        BspAlreadyConfirmed,
        /// Number of BSPs required for storage request has been reached.
        StorageRequestBspsRequiredFulfilled,
        /// BSP already volunteered to store the given file.
        BspAlreadyVolunteered,
        /// No slot available found in blocks to insert storage request expiration time.
        StorageRequestExpiredNoSlotAvailable,
        /// Not authorized to delete the storage request.
        StorageRequestNotAuthorized,
        /// Error created in 2024. If you see this, you are well beyond the singularity and should
        /// probably stop using this pallet.
        MaxBlockNumberReached,
        /// Invalid proof.
        InvalidProof,
        /// Failed to encode BSP id as slice.
        FailedToEncodeBsp,
        /// Failed to encode fingerprint as slice.
        FailedToEncodeFingerprint,
        /// Failed to decode threshold.
        FailedToDecodeThreshold,
        /// BSP did not succeed threshold check.
        ThresholdTooHigh,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn create_bucket(_origin: OriginFor<T>) -> DispatchResult {
            todo!()
        }

        /// Issue a new storage request for a file
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn issue_storage_request(
            origin: OriginFor<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageUnit<T>,
            multiaddresses: MultiAddresses<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and register storage request
            Self::do_request_storage(
                who.clone(),
                location.clone(),
                fingerprint,
                size,
                None,
                Some(multiaddresses.clone()),
                Default::default(),
            )?;

            // BSPs listen to this event and volunteer to store the file
            Self::deposit_event(Event::NewStorageRequest {
                who,
                location,
                fingerprint,
                size,
                multiaddresses,
            });

            Ok(())
        }

        /// Revoke storage request
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn revoke_storage_request(
            origin: OriginFor<T>,
            location: FileLocation<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and revoke storage request
            Self::do_revoke_storage_request(who, location.clone())?;

            // Emit event.
            Self::deposit_event(Event::StorageRequestRevoked { location });

            Ok(())
        }

        /// Used by a BSP to volunteer for storing a file.
        ///
        /// The transaction will fail if the XOR between the file ID and the BSP ID is not below the threshold,
        /// so a BSP is strongly advised to check beforehand. Another reason for failure is
        /// if the maximum number of BSPs has been reached. A successful assignment as BSP means
        /// that some of the collateral tokens of that MSP are frozen.
        #[pallet::call_index(4)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_volunteer(
            origin: OriginFor<T>,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            multiaddresses: MultiAddresses<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Perform validations and register Storage Provider as BSP for file.
            Self::do_bsp_volunteer(who.clone(), location.clone(), fingerprint)?;

            // Emit new BSP volunteer event.
            Self::deposit_event(Event::AcceptedBspVolunteer {
                who,
                location,
                fingerprint,
                multiaddresses,
            });

            Ok(())
        }

        /// Used by a BSP to confirm they are storing data of a storage request.
        #[pallet::call_index(5)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_confirm_storing(
            origin: OriginFor<T>,
            location: FileLocation<T>,
            root: FileKey<T>,
            proof: Proof<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            let who = ensure_signed(origin)?;

            // Perform validations and confirm storage.
            Self::do_bsp_confirm_storing(who.clone(), location.clone(), root, proof.clone())?;

            // Emit event.
            Self::deposit_event(Event::BspConfirmedStoring { who, location });

            Ok(())
        }

        /// Executed by a BSP to stop storing a file.
        ///
        /// In the event when a storage request no longer exists for the data the BSP no longer stores,
        /// it is required that the BSP still has access to the metadata of the initial storage request.
        /// If they do not, they will at least need that metadata to reconstruct the File ID and. Wherever
        /// the BSP gets the data it needs is up to it, but one example could be the assigned MSP.
        /// This metadata is necessary since it is needed to reconstruct the leaf node key in the storage
        /// provider's Merkle Forest.
        #[pallet::call_index(6)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_stop_storing(
            origin: OriginFor<T>,
            file_key: FileKey<T>,
            location: FileLocation<T>,
            owner: StorageProviderId<T>,
            fingerprint: Fingerprint<T>,
            size: StorageUnit<T>,
            can_serve: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Perform validations and stop storing the file.
            Self::do_bsp_stop_storing(
                who.clone(),
                file_key,
                location.clone(),
                owner.clone(),
                fingerprint,
                size,
                can_serve,
            )?;

            // Emit event.
            Self::deposit_event(Event::BspStoppedStoring {
                bsp: who,
                file_key,
                owner,
                location,
            });

            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        u32: TryFrom<BlockNumberFor<T>>,
    {
        fn on_idle(current_block: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            let db_weight = T::DbWeight::get();

            // Determine the starting block for cleanup, using `NextBlockToCleanup` if available,
            // or defaulting to the current block
            let start_block = NextStartingBlockToCleanUp::<T>::get();
            let mut block_to_clean = start_block;

            let writes = match T::MaxExpiredStorageRequests::get().checked_add(1) {
                Some(writes) => writes,
                // This should never happen. It would mean that MaxExpiredStorageRequests is u32::MAX,
                // which is an irrational number to set as a limit.
                None => return Weight::zero(),
            };

            // Total weight used to avoid exceeding the remaining weight.
            // We count one write for the `NextBlockToCleanup` storage item updated at the end.
            let mut total_used_weight = Weight::zero();

            let required_weight_for_iteration = db_weight.reads_writes(1, writes.into());

            // Iterate over blocks from the start block to the current block,
            // cleaning up storage requests until the remaining weight is insufficient
            while block_to_clean <= current_block
                && remaining_weight
                    .all_gte(total_used_weight.saturating_add(required_weight_for_iteration))
            {
                let mut used_weight = db_weight.reads(1);
                let expired_requests = StorageRequestExpirations::<T>::take(&block_to_clean);

                // Remove expired storage requests for the block
                for location in expired_requests {
                    StorageRequests::<T>::remove(&location);
                    used_weight += db_weight.writes(1);
                    Self::deposit_event(Event::StorageRequestExpired { location });
                }

                // Accumulate the weight used for cleanup operations
                total_used_weight += used_weight;
                // Increment the block to clean up for the next iteration
                block_to_clean = match block_to_clean.checked_add(&1u8.into()) {
                    Some(block) => block,
                    None => {
                        return total_used_weight;
                    }
                };
            }

            // `NextStartingBlockToCleanUp` is always updated to start from the block we reached in the current `on_idle` call.
            if block_to_clean > start_block {
                NextStartingBlockToCleanUp::<T>::put(block_to_clean);
                total_used_weight += db_weight.writes(1);
            }

            total_used_weight
        }
    }
}

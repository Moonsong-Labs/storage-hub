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
        sp_runtime::traits::{AtLeast32Bit, CheckEqual, MaybeDisplay, SimpleBitOps},
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use scale_info::prelude::fmt::Debug;
    use sp_runtime::{traits::EnsureFrom, BoundedVec};
    use sp_runtime::{
        traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Saturating, Zero},
        FixedPointNumber,
    };

    // TODO: add conditional to check that block number does not exceed u64 type. It it does, the fixed point number that we convert to from a block
    // number might be too loarge to fit into the threshold type.

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The trait for reading and mutating storage provider data.
        type Providers: storage_hub_traits::ReadProvidersInterface<AccountId = Self::AccountId, Provider = <Self::Providers as storage_hub_traits::MutateProvidersInterface>::Provider>
            + storage_hub_traits::MutateProvidersInterface<AccountId = Self::AccountId, MerklePatriciaRoot = <Self::ProofDealer as storage_hub_traits::ProofsDealerInterface>::MerkleHash>;

        /// The trait for issuing challenges and verifying proofs.
        type ProofDealer: storage_hub_traits::ProofsDealerInterface<
            Provider = <Self::Providers as storage_hub_traits::ProvidersInterface>::Provider,
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
            + AtLeast32Bit
            + Copy
            + MaxEncodedLen
            + HasCompact
            + Copy
            + Default
            + scale_info::TypeInfo
            + MaybeSerializeDeserialize
            + CheckedAdd
            + One
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
            + FixedPointNumber
            + EnsureFrom<u128>;

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
        type MaxDataServerMultiAddresses: Get<u32>;

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

    /// A double map of [`storage request`](FileLocation) to BSP `AccountId`s that volunteered to store data.
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
        FileLocation<T>,
        Blake2_128Concat,
        T::AccountId,
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
                <T::Providers as storage_hub_traits::ReadProvidersInterface>::get_number_of_bsps()
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
        /// Notifies that a new file has been requested to be stored.
        NewStorageRequest {
            who: T::AccountId,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
            multiaddresses: BoundedVec<MultiAddress<T>, T::MaxDataServerMultiAddresses>,
        },
        /// Notifies that a BSP has been accepted to store a given file.
        AcceptedBspVolunteer {
            who: T::AccountId,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            multiaddresses: MultiAddresses<T>,
        },
        /// Notifies that a BSP confirmed storing a file.
        BspConfirmedStoring {
            who: T::AccountId,
            location: FileLocation<T>,
        },
        /// Notifies the expiration of a storage request.
        StorageRequestExpired { location: FileLocation<T> },
        /// Notifies that a storage request has been revoked by the user who initiated it.
        StorageRequestRevoked { location: FileLocation<T> },
        /// Notifies that a BSP has stopped storing a file.
        BspStoppedStoring {
            bsp: T::AccountId,
            file_key: FileKey<T>,
            owner: T::AccountId,
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
            size: StorageData<T>,
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
            file_key: FileKey<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and revoke storage request
            Self::do_revoke_storage_request(who, location.clone(), file_key)?;

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
            owner: T::AccountId,
            fingerprint: Fingerprint<T>,
            size: StorageData<T>,
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

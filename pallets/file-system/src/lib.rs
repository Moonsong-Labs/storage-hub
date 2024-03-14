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
        pallet_prelude::*,
        sp_runtime::traits::{AtLeast32Bit, CheckEqual, MaybeDisplay, SimpleBitOps},
    };
    use frame_system::pallet_prelude::{BlockNumberFor, *};
    use sp_runtime::BoundedVec;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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

        /// Maximum number of BSPs that can store a file.
        #[pallet::constant]
        type MaxBspsPerStorageRequest: Get<u32>;

        /// Maximum byte size of a file path.
        #[pallet::constant]
        type MaxFilePathSize: Get<u32>;

        /// Maximum byte size of a libp2p multiaddress.
        #[pallet::constant]
        type MaxMultiAddressSize: Get<u32>;

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

    #[pallet::storage]
    #[pallet::getter(fn storage_requests_expirations)]
    pub type StorageRequestExpirations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<FileLocation<T>, T::MaxExpiredStorageRequests>,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that a new file has been requested to be stored.
        NewStorageRequest {
            who: T::AccountId,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            size: StorageUnit<T>,
            user_multiaddr: MultiAddress<T>,
        },

        /// Notifies that a BSP has been accepted to store a given file.
        AcceptedBspVolunteer {
            who: T::AccountId,
            location: FileLocation<T>,
            fingerprint: Fingerprint<T>,
            bsp_multiaddress: MultiAddress<T>,
        },

        /// Notifies the expiration of a storage request.
        StorageRequestExpired { location: FileLocation<T> },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Storage request already registered for the given file.
        StorageRequestAlreadyRegistered,
        /// Storage request not registered for the given file.
        StorageRequestNotRegistered,
        /// BSP already volunteered to store the given file.
        BspVolunteerFailed,
        /// BSP already confirmed to store the given file.
        BspAlreadyConfirmed,
        /// No slot available found in blocks to insert storage request expiration time.
        StorageRequestExpiredNoSlotAvailable,
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
            user_multiaddr: MultiAddress<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer
            let who = ensure_signed(origin)?;

            // Perform validations and register storage request
            Self::do_request_storage(location.clone(), fingerprint, size, user_multiaddr.clone())?;

            // BSPs listen to this event and volunteer to store the file
            Self::deposit_event(Event::NewStorageRequest {
                who,
                location,
                fingerprint,
                size,
                user_multiaddr,
            });

            Ok(())
        }

        /// Revoke storage request
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn revoke_storage_request(_origin: OriginFor<T>) -> DispatchResult {
            todo!()
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
            bsp_multiaddress: MultiAddress<T>,
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
                bsp_multiaddress,
            });

            Ok(())
        }

        /// Executed by a BSP to stop storing a file.
        ///
        /// A compensation should be provided for the user, to deter this behaviour. A successful execution of this extrinsic automatically generates a storage request for that file with one remaining_bsps_slot left, and if a storage request for that file already exists, the slots left are incremented in one. It also automatically registers a challenge for this file, for the next round of storage proofs, so that the other BSPs and MSP who are storing it would be forced to disclose themselves then.
        #[pallet::call_index(5)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn bsp_stop_storing(_origin: OriginFor<T>) -> DispatchResult {
            todo!()
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        u32: TryFrom<BlockNumberFor<T>>,
    {
        fn on_idle(block: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
            let db_weight = T::DbWeight::get();

            // Return early if the remaining weight is not enough to perform the operation
            if !remaining_weight.all_gte(
                db_weight.reads_writes(
                    1,
                    T::MaxExpiredStorageRequests::get()
                        .try_into()
                        .expect("u32 would fit into u64"),
                ),
            ) {
                return Weight::zero();
            }

            let mut used_weight = db_weight.reads(1);

            let mut expired_requests = match StorageRequestExpirations::<T>::take(&block) {
                Some(requests) => requests,
                None => return used_weight,
            };

            // remove expired storage requests requests
            for location in expired_requests.drain(..) {
                // TODO: should probably add some fields to the `StorageRequestExpired` to facilitate SPs filtering the events
                // that are relevant to them (e.g. include the SPs that have volunteered to store the file)
                let _request = StorageRequests::<T>::take(&location);

                StorageRequests::<T>::remove(&location);
                used_weight += db_weight.writes(1);

                Self::deposit_event(Event::StorageRequestExpired { location });
            }

            // we already checked we have enough `remaining_weight` to cover this `used_weight`
            used_weight
        }
    }
}

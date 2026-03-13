#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod types;
mod utils;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// TODO #[cfg(feature = "runtime-benchmarks")]
// TODO mod benchmarking;

extern crate alloc;

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    #[cfg(feature = "runtime-benchmarks")]
    use sp_core::H256;

    use crate::types::{
        AccountIdLookupSourceOf, AccountIdLookupTargetOf, BucketIdFor, ReadAccessRegex,
    };

    #[cfg(feature = "runtime-benchmarks")]
    pub trait BenchmarkHelper<BucketId> {
        fn bucket(i: H256) -> BucketId;
    }
    #[cfg(feature = "runtime-benchmarks")]
    impl<BucketId: From<H256>> BenchmarkHelper<BucketId> for () {
        fn bucket(i: H256) -> BucketId {
            i.into()
        }
    }

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_nfts::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: crate::weights::WeightInfo;

        /// The trait for reading storage bucket data.
        type Buckets: shp_traits::ReadBucketsInterface<
            AccountId = Self::AccountId,
            ReadAccessGroupId = <Self as pallet_nfts::Config>::CollectionId,
        >;

        /// Helper for benchmarking.
        #[cfg(feature = "runtime-benchmarks")]
        type Helper: BenchmarkHelper<BucketIdFor<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

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
        /// Notifies that access to a bucket has been shared with another account.
        #[codec(index = 0)]
        AccessShared {
            issuer: T::AccountId,
            recipient: AccountIdLookupTargetOf<T>,
        },
        /// Notifies that the read access for an item has been updated.
        #[codec(index = 1)]
        ItemReadAccessUpdated {
            admin: T::AccountId,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
        },
        /// Notifies that an item has been burned.
        #[codec(index = 2)]
        ItemBurned {
            account: T::AccountId,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
        },
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
        /// Bucket is not private. Call `update_bucket_privacy` from the file system pallet to make it private.
        #[codec(index = 0)]
        BucketIsNotPrivate,
        /// Account is not the owner of the bucket.
        #[codec(index = 1)]
        NotBucketOwner,
        /// No collection corresponding to the bucket. Call `update_bucket_privacy` from the file system pallet to make it private.
        #[codec(index = 2)]
        NoCorrespondingCollection,
        /// Failed to convert bytes to `BoundedVec`
        #[codec(index = 3)]
        ConvertBytesToBoundedVec,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Share access to files within a bucket with another account.
        ///
        /// The `read_access_regex` parameter is optional and when set to `None` it means that the recipient will be denied access for any read request within the bucket.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::share_access())]
        pub fn share_access(
            origin: OriginFor<T>,
            recipient: AccountIdLookupSourceOf<T>,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
            read_access_regex: Option<ReadAccessRegex<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let recipient_account =
                Self::do_share_access(&who, recipient, bucket, item_id, read_access_regex)?;

            Self::deposit_event(Event::AccessShared {
                issuer: who,
                recipient: recipient_account,
            });

            Ok(())
        }

        /// Update read access for an item.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::update_read_access())]
        pub fn update_read_access(
            origin: OriginFor<T>,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
            read_access_regex: Option<ReadAccessRegex<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_update_read_access(&who, bucket, item_id, read_access_regex)?;

            // `who` is implicitly known as the admin of the collection otherwise the execution would have failed with a lack of permissions.
            Self::deposit_event(Event::ItemReadAccessUpdated {
                admin: who,
                bucket,
                item_id,
            });

            Ok(())
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod types;
mod utils;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// TODO #[cfg(feature = "runtime-benchmarks")]
// TODO mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use pallet_nfts::WeightInfo;
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that access to a bucket has been shared with another account.
        AccessShared {
            issuer: T::AccountId,
            recipient: AccountIdLookupTargetOf<T>,
        },
        /// Notifies that the read access for an item has been updated.
        ItemReadAccessUpdated {
            admin: T::AccountId,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
        },
        /// Notifies that an item has been burned.
        ItemBurned {
            account: T::AccountId,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Bucket is not private. Call `update_bucket_privacy` from the file system pallet to make it private.
        BucketIsNotPrivate,
        /// Account is not the owner of the bucket.
        NotBucketOwner,
        /// No collection corresponding to the bucket. Call `update_bucket_privacy` from the file system pallet to make it private.
        NoCorrespondingCollection,
        /// Failed to convert bytes to `BoundedVec`
        ConvertBytesToBoundedVec,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Share access to files within a bucket with another account.
        ///
        /// The `read_access_regex` parameter is optional and when set to `None` it means that the recipient will be denied access for any read request within the bucket.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::mint() + T::WeightInfo::set_metadata())]
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
        #[pallet::weight(T::WeightInfo::set_metadata())]
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

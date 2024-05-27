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
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use pallet_nfts::WeightInfo;

    use crate::types::{AccountIdLookupSourceOf, AccountIdLookupTargetOf, BucketIdFor};

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_nfts::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The trait for reading storage provider data.
        type Providers: storage_hub_traits::ReadProvidersInterface<
            AccountId = Self::AccountId,
            BucketNftCollectionId = <Self as pallet_nfts::Config>::CollectionId,
        >;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Notifies that a new file has been requested to be stored.
        AccessShared {
            issuer: T::AccountId,
            recipient: AccountIdLookupTargetOf<T>,
        },
        /// Notifies that the read access for an item has been updated.
        ReadAccessUpdated {
            admin: T::AccountId,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
        },
        /// Notifies that an NFT has been burned.
        NftBurned {
            account: T::AccountId,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Bucket is not private. Call `make_bucket_public` from the providers pallet to make it private.
        BucketIsNotPrivate,
        /// Account is not the owner of the bucket.
        NotBucketOwner,
        /// Item not found in the collection.
        ItemNotFound,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Share access to files under a bucket with another account.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn share_access(
            origin: OriginFor<T>,
            recipient: AccountIdLookupSourceOf<T>,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
            read_access_regex: BoundedVec<u8, T::StringLimit>,
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

        /// Update the read access for an item.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn update_read_access(
            origin: OriginFor<T>,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
            read_access_regex: BoundedVec<u8, T::StringLimit>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_update_read_access(&who, bucket, item_id, read_access_regex)?;

            // `who` is implicitly known as the admin of the collection otherwise the execution would have failed with a lack of permissions.
            Self::deposit_event(Event::ReadAccessUpdated {
                admin: who,
                bucket,
                item_id,
            });

            Ok(())
        }

        /// Burn an NFT from a collection.
        ///
        /// This function is a wrapper around the `burn` function from the `pallet-nfts` pallet.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            bucket: BucketIdFor<T>,
            item: T::ItemId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Self::do_burn(&who, bucket, item)?;

            Self::deposit_event(Event::NftBurned {
                account: who,
                bucket,
                item_id: item,
            });

            Ok(())
        }
    }
}

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod types;
mod utils;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;

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
            account: AccountIdLookupTargetOf<T>,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Bucket is not private. Call `make_bucket_public` from the providers pallet to make it private.
        BucketIsNotPrivate,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn share_access(
            origin: OriginFor<T>,
            account: AccountIdLookupSourceOf<T>,
            bucket: BucketIdFor<T>,
            item_id: T::ItemId,
            read_access_regex: BoundedVec<u8, T::StringLimit>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let target_account =
                Self::do_share_access(&who, account, bucket, item_id, read_access_regex)?;

            // Emit an event.
            Self::deposit_event(Event::AccessShared {
                issuer: who,
                account: target_account,
            });

            Ok(())
        }
    }
}

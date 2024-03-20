#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    ensure, pallet_prelude::DispatchError, sp_runtime::ArithmeticError, traits::Get,
};
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::FullCodec;
use core::{cmp::Ord, fmt::Debug};
use frame_support::{
    pallet_prelude::{DispatchResult, MaxEncodedLen, MaybeSerializeDeserialize, Member, Parameter},
    sp_runtime::traits::AtLeast32BitUnsigned,
};

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, sp_runtime::ArithmeticError};
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The maximum number of registered users.
        #[pallet::constant]
        type MaxUsers: Get<u128>;
    }

    /// Mapping of registered users. Being included in the map is equivalent to being registered.
    #[pallet::storage]
    #[pallet::getter(fn users)]
    pub type Users<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, ()>;

    /// The number of registered users.
    #[pallet::storage]
    #[pallet::getter(fn count)]
    pub type Count<T: Config> = StorageValue<_, u128, ValueQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/main-docs/build/events-errors/
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// An event that is emitted when a new user is registered.
        ///
        /// It includes the account id of the user.
        ///
        /// # Arguments
        /// 	- `user`: The account id of the user.
        NewUser { user: T::AccountId },

        /// An event that is emitted when a user is removed.
        ///
        /// It includes the account id of the user.
        ///
        /// # Arguments
        /// 	- `user`: The account id of the user.
        RemovedUser { user: T::AccountId },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// User is not registered.
        NotRegistered,

        /// User is already registered.
        AlreadyRegistered,

        /// The maximum number of users has been reached.
        MaximumOfUsersReached,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new user.
        #[pallet::call_index(0)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn register_user(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            // Check origin.
            ensure_root(origin)?;

            // Check that user is not already registered.
            ensure!(
                !Users::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );

            // Increment user count and return error if maximum is reached.
            let mut count = Count::<T>::get();
            ensure!(
                count < T::MaxUsers::get(),
                Error::<T>::MaximumOfUsersReached
            );
            count = match count.checked_add(1) {
                Some(count) => count,
                None => {
                    #[cfg(test)]
                    unreachable!("Overflow cannot happen after just checking that count < T::MaxUsers::get()");

                    #[allow(unreachable_code)]
                    {
                        Err(DispatchError::Arithmetic(ArithmeticError::Overflow))?
                    }
                }
            };
            Count::<T>::put(count);

            // Register user.
            Users::<T>::insert(&who, ());

            // Emit event.
            Self::deposit_event(Event::NewUser { user: who });

            Ok(())
        }

        /// Remove a user.
        #[pallet::call_index(1)]
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
        pub fn remove_user(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            // Check origin.
            ensure_root(origin)?;

            // Check that user is registered.
            ensure!(Users::<T>::contains_key(&who), Error::<T>::NotRegistered);

            // Decrement user count.
            let mut count = Count::<T>::get();
            count = match count.checked_sub(1) {
                Some(count) => count,
                None => {
                    #[cfg(test)]
                    unreachable!("Underflow cannot happen as it would mean that the user was not accounted for.");

                    #[allow(unreachable_code)]
                    {
                        Err(DispatchError::Arithmetic(ArithmeticError::Underflow))?
                    }
                }
            };
            Count::<T>::put(count);

            // Remove user.
            Users::<T>::remove(&who);

            // Emit event.
            Self::deposit_event(Event::RemovedUser { user: who });

            Ok(())
        }
    }
}

/// An identity trait that provides a way to lookup known registered users.
///
/// It is abstracted over the AccountId type, User type and total number of users.
pub trait IdentityInterface {
    /// The type which can be used to identify accounts.
    /// ? Are these trait bounds correct?
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents a registered user.
    type User: Parameter + Member + MaybeSerializeDeserialize + Debug + Ord + MaxEncodedLen;
    /// The type which represents the total number of registered users.
    type UserCount: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Ord
        + AtLeast32BitUnsigned
        + FullCodec
        + Copy
        + Default
        + Debug
        + scale_info::TypeInfo
        + MaxEncodedLen;

    /// Lookup a registered user by their AccountId.
    fn get_user(who: Self::AccountId) -> Option<Self::User>;

    /// Lookup the total number of registered users.
    fn total_users() -> Self::UserCount;

    /// Register a new user.
    fn register_user(who: Self::AccountId) -> DispatchResult;
}

// Look at `../interface/` to better understand this API.
impl<T: Config> IdentityInterface for Pallet<T> {
    type AccountId = T::AccountId;

    type User = ();

    type UserCount = u128;

    fn get_user(who: Self::AccountId) -> Option<Self::User> {
        if Users::<T>::contains_key(&who) {
            Some(())
        } else {
            None
        }
    }

    fn total_users() -> Self::UserCount {
        Count::<T>::get()
    }

    fn register_user(who: Self::AccountId) -> frame_support::pallet_prelude::DispatchResult {
        // Check that user is not already registered.
        ensure!(
            !Users::<T>::contains_key(&who),
            Error::<T>::AlreadyRegistered
        );

        // Increment user count and return error if maximum is reached.
        let mut count = Count::<T>::get();
        ensure!(
            count < T::MaxUsers::get(),
            Error::<T>::MaximumOfUsersReached
        );
        count = match count.checked_add(1) {
            Some(count) => count,
            None => {
                #[cfg(test)]
                unreachable!(
                    "Overflow cannot happen after just checking that count < T::MaxUsers::get()"
                );

                #[allow(unreachable_code)]
                {
                    Err(DispatchError::Arithmetic(ArithmeticError::Overflow))?
                }
            }
        };
        Count::<T>::put(count);

        // Register user.
        Users::<T>::insert(&who, ());

        Ok(())
    }
}

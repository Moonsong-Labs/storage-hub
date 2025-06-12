//! Primitives for session keys
#![cfg_attr(not(feature = "std"), no_std)]

pub mod inherent;
pub use inherent::*;

extern crate alloc;

/// A Trait to lookup keys from AuthorIds
pub trait KeysLookup<AuthorId, Keys> {
    #[cfg(feature = "runtime-benchmarks")]
    type Account;
    fn lookup_keys(author: &AuthorId) -> Option<Keys>;
    #[cfg(feature = "runtime-benchmarks")]
    fn set_keys(id: AuthorId, account: Self::Account, keys: Keys);
}

// A dummy impl used in simple tests
impl<AuthorId, Keys> KeysLookup<AuthorId, Keys> for () {
    #[cfg(feature = "runtime-benchmarks")]
    type Account = ();
    fn lookup_keys(_: &AuthorId) -> Option<Keys> {
        None
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn set_keys(_id: AuthorId, _account: Self::Account, _keys: Keys) {}
}

//! Contains the various runtime APIs that the backend accesses
//!
//! TODO: consider refactoring this to use subxt or similar
//! and auto-import/generate the bindings from the runtime

pub const CURRENT_PRICE: &str = "PaymentStreamsApi_get_current_price_per_giga_unit_per_tick";

// TODO: get type from runtime
pub type CurrentPrice = u128;

pub const NUM_OF_USERS: &str = "PaymentStreamsApi_get_users_of_payment_streams_of_provider";

// TODO: get type from runtime
pub type NumOfUsers = Vec<crate::runtime::AccountId>;

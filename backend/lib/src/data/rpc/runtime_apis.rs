//! Contains the various runtime APIs that the backend accesses
//!
//! TODO: consider refactoring this to use subxt or similar
//! and auto-import/generate the bindings from the runtime

pub const CURRENT_PRICE: &str = "PaymentStreamsApi_get_current_price_per_giga_unit_per_tick";

// TODO: get type from runtime
pub type CurrentPrice = u128;

pub const AVAILABLE_CAPACITY: &str = "StorageProvidersApi_query_available_storage_capacity";

// TODO: get type from runtime
pub type AvailableCapacity = u64;

/// Contains the various runtime APIs that the backend accesses
///
/// TODO: consider refactoring this to use subxt or similar
/// and auto-import/generate the bindings from the runtime

pub const CURRENT_PRICE: &str = "PaymentStreamsApi_get_current_price_per_giga_unit_per_tick";

// TODO: use the right type for the runtime
pub type CurrentPrice = u128;

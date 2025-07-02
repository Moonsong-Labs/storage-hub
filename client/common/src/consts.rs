/// The key used to store the current Forest root in the Forest Storage.
///
/// For BSPs, this is the actual key used to identify the current best Forest root.
/// For MSPs, who store Buckets, this key is concatenated with the Bucket ID to identify the current best Forest root
/// for that Bucket.
pub const CURRENT_FOREST_KEY: &[u8] = b":current_forest_key";

// Those version must match the one in the runtime version
pub const VERSION_SPEC_VERSION: u32 = 1;
pub const VERSION_TRANSACTION_VERSION: u32 = 1;

//! Contains the various state queries that the backend accesses
//!
//! TODO: consider refactoring this to use subxt or similar
//! and auto-import/generate the bindings from the runtime

use std::sync::LazyLock;

use bigdecimal::BigDecimal;

pub const MSP_INFO_MODULE: &str = "Providers";
pub const MSP_INFO_METHOD: &str = "MainStorageProviders";

/// This is the storage key prefix for the Providers.MainStorageProviders map
///
/// The map is indexed by the provider id
pub const MSP_INFO_KEY_PREFIX: LazyLock<[u8; 32]> = LazyLock::new(|| {
    let mut module = [0; 16];
    let mut method = [0; 16];

    sp_core::twox_128_into(MSP_INFO_MODULE.as_bytes(), &mut module);
    sp_core::twox_128_into(MSP_INFO_METHOD.as_bytes(), &mut method);

    let mut result = [0; 32];
    result[..16].copy_from_slice(&module);
    result[16..].copy_from_slice(&method);

    result
});

//TODO: import from runtime
/// This is the equivalent of `MainStorageProvider` from the storage
#[derive(Debug)]
pub struct MspInfo {
    pub capacity: BigDecimal,
    pub capacity_user: BigDecimal,
    pub multiaddresses: Vec<String>,
    pub amount_of_buckets: BigDecimal,
    pub amount_of_value_props: BigDecimal,
    pub last_capacity_change: BigDecimal,
    pub owner_account: String,
    pub payment_account: String,
    pub sign_up_block: BigDecimal,
}

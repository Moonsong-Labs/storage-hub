//! This module wraps over the runtime selected during compilation
//!
//! All runtime-dependant types used in the library should be defined here, along with appropriate wrappers if necessary

cfg_if::cfg_if! {
    // if #[cfg(all(feature = "solochain", ...))] {
    //     compile_error!("Please select only 1 runtime");
    // } else
    if #[cfg(feature = "solochain")] {
        use sh_solochain_evm_runtime::Runtime;
    } else {
        compile_error!("No runtime selected");
    }
}

pub type AccountId = shc_common::types::AccountId<Runtime>;
pub type ProviderId = pallet_storage_providers::types::ProviderIdFor<Runtime>;
pub type Balance = pallet_storage_providers::types::BalanceOf<Runtime>;

pub type MainStorageProvidersStorageMap = pallet_storage_providers::MainStorageProviders<Runtime>;
pub type MainStorageProvider = pallet_storage_providers::types::MainStorageProvider<Runtime>;

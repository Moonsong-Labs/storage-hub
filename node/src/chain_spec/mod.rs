pub mod parachain;
pub mod solochain_evm;

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Can be called for a chain spec `Configuration` to determine the network type.
pub trait NetworkType {
    /// Returns `true` if this is a configuration for the `Parachain` network.
    fn is_parachain(&self) -> bool;

    /// Returns `true` if this is a configuration for the `Solochain EVM` network.
    fn is_solochain_evm(&self) -> bool;

    /// Returns `true` if this is a configuration for a dev network.
    fn is_dev(&self) -> bool;
}

impl NetworkType for Box<dyn sc_service::ChainSpec> {
    fn is_dev(&self) -> bool {
        self.chain_type() == sc_service::ChainType::Development
    }

    fn is_parachain(&self) -> bool {
        self.id().starts_with("storage_hub_parachain")
    }

    fn is_solochain_evm(&self) -> bool {
        self.id().starts_with("storage_hub_solochain_evm")
    }
}

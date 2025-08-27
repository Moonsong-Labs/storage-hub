use super::{ChainSpec, Extensions};
use sc_service::ChainType;
use storage_hub_runtime::WASM_BINARY;

const CHAIN_ID: u64 = 1000; // Parachain ID
const SS58_FORMAT: u16 = 42;
const TOKEN_DECIMALS: u8 = 12;
const TOKEN_SYMBOL: &str = "UNIT";

pub fn development_config() -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), SS58_FORMAT.into());

    ChainSpec::builder(
        WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: "rococo-local".into(),
            // You MUST set this to the correct network!
            para_id: CHAIN_ID as u32,
        },
    )
    .with_name("Storage Hub Parachain Dev")
    .with_id("storage_hub_parachain_dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_preset_name("development")
    .with_properties(properties)
    .build()
}

pub fn local_testnet_config() -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), SS58_FORMAT.into());

    ChainSpec::builder(
        WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: "rococo-local".into(),
            // You MUST set this to the correct network!
            para_id: CHAIN_ID as u32,
        },
    )
    .with_name("Storage Hub Parachain Local Testnet")
    .with_id("storage_hub_parachain_local")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_preset_name("local_testnet")
    .with_protocol_id("storage-hub-parachain-local")
    .with_properties(properties)
    .build()
}

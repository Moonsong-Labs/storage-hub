use sc_service::ChainType;
use sh_solochain_evm_runtime::WASM_BINARY;
use sp_genesis_builder;

// For solochain, we use a simpler ChainSpec without parachain extensions
pub type SolochainChainSpec = sc_service::GenericChainSpec;

const SS58_FORMAT: u16 = 42;
const TOKEN_DECIMALS: u8 = 18; // Different decimals for EVM compatibility
const TOKEN_SYMBOL: &str = "SHUB";

pub fn development_config() -> Result<SolochainChainSpec, String> {
    // In the properties attribute, the following settings are used by various
    // front-end libraries, including the Polkadot.js API:
    // - tokenSymbol: a name for your EVM network's own token symbol.
    // - tokenDecimals: the number of decimals for your EVM network's own token.
    // - ss58Format: an integer that uniquely identifies the accounts in your network.
    //               SS58 encoding transforms the underlying 32-byte account to a
    //               network-specific representation. This attribute doesn't apply nor
    //               interfere with the ECDSA Ethereum accounts on EVM-compatible networks.
    // - isEthereum: a boolean identifying the network as EVM compatible or not.
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), SS58_FORMAT.into());
    properties.insert("isEthereum".into(), true.into());

    Ok(SolochainChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
        None,
    )
    .with_name("Storage Hub Solochain EVM Dev")
    .with_id("storage_hub_solochain_evm_dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
    .with_properties(properties)
    .build())
}

pub fn local_testnet_config() -> Result<SolochainChainSpec, String> {
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), SS58_FORMAT.into());
    properties.insert("isEthereum".into(), true.into());

    Ok(SolochainChainSpec::builder(
        WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
        None,
    )
    .with_name("Storage Hub Solochain EVM Local")
    .with_id("storage_hub_solochain_evm_local")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_preset_name(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
    .with_protocol_id("storage-hub-solochain-evm-local")
    .with_properties(properties)
    .build())
}

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Deserializer};
use std::{path::PathBuf, str::FromStr};
use storage_hub_runtime::StorageDataUnit;

use crate::command::ProviderOptions;

/// Sub-commands supported by the collator.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// Remove the whole chain.
    PurgeChain(cumulus_client_cli::PurgeChainCmd),

    /// Export the genesis head data of the parachain.
    ///
    /// Head data is the encoded block header.
    #[command(alias = "export-genesis-state")]
    ExportGenesisHead(cumulus_client_cli::ExportGenesisHeadCommand),

    /// Export the genesis wasm of the parachain.
    ExportGenesisWasm(cumulus_client_cli::ExportGenesisWasmCommand),

    /// Sub-commands concerned with benchmarking.
    /// The pallet benchmarking moved to the `pallet` sub-command.
    #[command(subcommand)]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}

#[derive(ValueEnum, Clone, Debug, Eq, PartialEq)]
pub enum ProviderType {
    /// Main Storage Provider
    Msp,
    /// Backup Storage Provider
    Bsp,
    /// User role
    User,
}

impl<'de> serde::Deserialize<'de> for ProviderType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;

        let provider_type = match s.as_str() {
            "bsp" => ProviderType::Bsp,
            "msp" => ProviderType::Msp,
            "user" => ProviderType::User,
            _ => {
                return Err(serde::de::Error::custom(
                    "Cannot parse `provider_type`. Invalid value.",
                ))
            }
        };

        Ok(provider_type)
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum StorageLayer {
    /// RocksDB with path.
    RocksDB,
    /// In Memory
    Memory,
}

impl<'de> serde::Deserialize<'de> for StorageLayer {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;

        let storage_layer = match s.as_str() {
            "rocksdb" => StorageLayer::RocksDB,
            "memory" => StorageLayer::Memory,
            _ => {
                return Err(serde::de::Error::custom(
                    "Cannot parse `storage_type`. Invalid value.",
                ))
            }
        };

        Ok(storage_layer)
    }
}

#[derive(Debug, Parser)]
#[group(skip)]
pub struct ProviderConfigurations {
    /// Run node as a StorageHub provider.
    #[arg(long)]
    pub provider: bool,

    /// Run the node in maintenance mode.
    /// In this mode, the node will not import blocks or participate in consensus,
    /// but will allow specific RPC calls for file and storage management.
    #[arg(long, default_value = "false")]
    pub maintenance_mode: bool,

    /// Type of StorageHub provider.
    #[clap(
        long,
        value_enum,
        value_name = "PROVIDER_TYPE",
        required_if_eq("provider", "true")
    )]
    pub provider_type: Option<ProviderType>,

    /// Maximum storage capacity of the provider (bytes).
    #[clap(long, required_if_eq_any([
        ("provider_type", "msp"),
        ("provider_type", "bsp")
    ]))]
    pub max_storage_capacity: Option<StorageDataUnit>,

    /// Jump capacity (bytes).
    #[clap(long, required_if_eq_any([
        ("provider_type", "msp"),
        ("provider_type", "bsp")
    ]))]
    pub jump_capacity: Option<StorageDataUnit>,

    /// Type of StorageHub provider.
    /// Currently: `memory` and `rocks-db`.
    #[clap(
        long,
        value_enum,
        value_name = "STORAGE_LAYER",
        default_value = "memory"
    )]
    pub storage_layer: Option<StorageLayer>,

    /// Storage location in the file system
    #[clap(long, required_if_eq("storage_layer", "rocks-db"))]
    pub storage_path: Option<String>,

    /// Extrinsic retry timeout in seconds.
    #[clap(long, default_value = "60")]
    pub extrinsic_retry_timeout: u64,

    /// MSP charging fees period (in blocks).
    /// Setting it to 600 with a block every 6 seconds will charge user every hour.
    #[clap(long, required_if_eq_any([
        ("provider_type", "msp"),
    ]))]
    pub msp_charging_period: Option<u32>,
}

impl ProviderConfigurations {
    pub fn provider_options(&self) -> ProviderOptions {
        ProviderOptions {
            provider_type: self
                .provider_type
                .clone()
                .expect("Provider type is required"),
            storage_layer: self
                .storage_layer
                .clone()
                .expect("Storage layer is required"),
            storage_path: self.storage_path.clone(),
            // We can default since the clap would have errored out if it was not provided when required.
            // In any other case, max_storage_capacity is not required and can be set to default.
            max_storage_capacity: self.max_storage_capacity,
            jump_capacity: self.jump_capacity,
            extrinsic_retry_timeout: self.extrinsic_retry_timeout,
            msp_charging_period: self.msp_charging_period,
            maintenance_mode: self.maintenance_mode,
        }
    }
}

#[derive(Debug, Parser, Clone)]
pub struct IndexerConfigurations {
    /// Whether to enable the indexer.
    ///
    /// By default, the indexer is disabled.
    /// If enabled, a Postgres database must be set up (see the indexer README for details).
    #[arg(long, default_value = "false")]
    pub indexer: bool,

    /// Postgres database URL.
    ///
    /// If not provided, the indexer will use the `DATABASE_URL` environment variable. If the
    /// environment variable is not set, the node will abort.
    #[arg(long)]
    pub database_url: Option<String>,
}

/// Block authoring scheme to be used by the dev service.
#[derive(Debug, Copy, Clone, Deserialize)]
pub enum Sealing {
    /// Author a block immediately upon receiving a transaction into the transaction pool
    Instant,
    /// Author a block upon receiving an RPC command
    Manual,
    /// Author blocks at a regular interval specified in milliseconds
    Interval(u64),
}

impl FromStr for Sealing {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "instant" => Self::Instant,
            "manual" => Self::Manual,
            s => {
                let millis =
                    u64::from_str_radix(s, 10).map_err(|_| "couldn't decode sealing param")?;
                Self::Interval(millis)
            }
        })
    }
}

const AFTER_HELP_EXAMPLE: &str = color_print::cstr!(
    r#"<bold><underline>Examples:</></>
   <bold>parachain-template-node build-spec --disable-default-bootnode > plain-parachain-chainspec.json</>
           Export a chainspec for a local testnet in json format.
   <bold>parachain-template-node --chain plain-parachain-chainspec.json --tmp -- --chain rococo-local</>
           Launch a full node with chain specification loaded from plain-parachain-chainspec.json.
   <bold>parachain-template-node</>
           Launch a full node with default parachain <italic>local-testnet</> and relay chain <italic>rococo-local</>.
   <bold>parachain-template-node --collator</>
           Launch a collator with default parachain <italic>local-testnet</> and relay chain <italic>rococo-local</>.
 "#
);
#[derive(Debug, clap::Parser)]
#[command(
    propagate_version = true,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
#[clap(after_help = AFTER_HELP_EXAMPLE)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[command(flatten)]
    pub run: RunCmd,

    /// Disable automatic hardware benchmarks.
    ///
    /// By default these benchmarks are automatically ran at startup and measure
    /// the CPU speed, the memory bandwidth and the disk speed.
    ///
    /// The results are then printed out in the logs, and also sent as part of
    /// telemetry, if telemetry is enabled.
    #[arg(long)]
    pub no_hardware_benchmarks: bool,

    /// Relay chain arguments
    #[arg(raw = true)]
    pub relay_chain_args: Vec<String>,

    /// Provider configurations
    #[command(flatten)]
    pub provider_config: ProviderConfigurations,

    /// Provider configurations file path (allow to specify the provider configuration in a file instead of the cli)
    #[clap(long, conflicts_with_all = ["provider", "provider_type", "max_storage_capacity", "jump_capacity", "storage_layer", "storage_path", "extrinsic_retry_timeout", "msp_charging_period"])]
    pub provider_config_file: Option<String>,

    /// Indexer configurations
    #[command(flatten)]
    pub indexer_config: IndexerConfigurations,
}

#[derive(Debug, Parser)]
#[group(skip)]
pub struct RunCmd {
    #[clap(flatten)]
    pub base: cumulus_client_cli::RunCmd,

    /// When blocks should be sealed in the dev service.
    ///
    /// Options are "instant", "manual", or timer interval in milliseconds
    #[clap(long, default_value = "instant")]
    pub sealing: Sealing,
}

impl std::ops::Deref for RunCmd {
    type Target = cumulus_client_cli::RunCmd;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug)]
pub struct RelayChainCli {
    /// The actual relay chain cli object.
    pub base: polkadot_cli::RunCmd,

    /// Optional chain id that should be passed to the relay chain.
    pub chain_id: Option<String>,

    /// The base path that should be used by the relay chain.
    pub base_path: Option<PathBuf>,
}

impl RelayChainCli {
    /// Parse the relay chain CLI parameters using the para chain `Configuration`.
    pub fn new<'a>(
        para_config: &sc_service::Configuration,
        relay_chain_args: impl Iterator<Item = &'a String>,
    ) -> Self {
        let extension = crate::chain_spec::Extensions::try_get(&*para_config.chain_spec);
        let chain_id = extension.map(|e| e.relay_chain.clone());
        let base_path = para_config.base_path.path().join("polkadot");
        Self {
            base_path: Some(base_path),
            chain_id,
            base: clap::Parser::parse_from(relay_chain_args),
        }
    }
}

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Deserializer};
use shp_types::StorageDataUnit;
use std::{path::PathBuf, str::FromStr};

use crate::command::ProviderOptions;

use shc_client::builder::{
    BlockchainServiceOptions, BspChargeFeesOptions, BspMoveBucketOptions, BspSubmitProofOptions,
    BspUploadFileOptions, FishermanOptions, IndexerOptions, MspChargeFeesOptions,
    MspMoveBucketOptions,
};
use shc_indexer_service::IndexerMode;
use shc_rpc::RpcConfig;

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

    /// Type of StorageHub provider.
    #[arg(
        long,
        value_enum,
        value_name = "PROVIDER_TYPE",
        required_if_eq("provider", "true")
    )]
    pub provider_type: Option<ProviderType>,

    /// Maximum storage capacity of the provider (bytes).
    #[arg(long, required_if_eq_all([
        ("provider", "true"),
        ("provider_type", "msp"),
    ]), required_if_eq_all([
        ("provider", "true"),
        ("provider_type", "bsp"),
    ]))]
    pub max_storage_capacity: Option<StorageDataUnit>,

    /// Jump capacity (bytes).
    #[arg(long, required_if_eq_all([
        ("provider", "true"),
        ("provider_type", "msp"),
    ]), required_if_eq_all([
        ("provider", "true"),
        ("provider_type", "bsp"),
    ]))]
    pub jump_capacity: Option<StorageDataUnit>,

    /// Type of StorageHub provider.
    /// Currently: `memory` and `rocks-db`.
    #[arg(
        long,
        value_enum,
        value_name = "STORAGE_LAYER",
        default_value = "memory"
    )]
    pub storage_layer: Option<StorageLayer>,

    /// Storage location in the file system
    #[arg(long, required_if_eq("storage_layer", "rocks-db"))]
    pub storage_path: Option<String>,

    /// Extrinsic retry timeout in seconds.
    #[arg(long, default_value = "60")]
    pub extrinsic_retry_timeout: Option<u64>,

    /// The minimum number of blocks behind the current best block to consider the node out of sync.
    #[arg(long, default_value = "5")]
    pub sync_mode_min_blocks_behind: Option<u32>,

    /// On blocks that are multiples of this number, the blockchain service will trigger the catch of proofs.
    #[arg(long, default_value = "4")]
    pub check_for_pending_proofs_period: Option<u32>,

    /// The maximum number of blocks from the past that will be processed for catching up the root changes.
    #[arg(long, default_value = "10")]
    pub max_blocks_behind_to_catch_up_root_changes: Option<u32>,

    /// Enable MSP file distribution to BSPs (disabled by default unless set via config/CLI).
    /// Only applicable when running as an MSP provider.
    #[arg(long, value_name = "BOOLEAN")]
    pub msp_distribute_files: bool,

    // ============== Provider RPC options ==============
    // ============== Remote file upload/download options ==============
    /// Maximum file size in bytes (default: 10GB)
    #[arg(
        long,
        value_name = "BYTES",
        help_heading = "RPC - Remote File Options",
        default_value = "10737418240"
    )]
    pub max_file_size: Option<u64>,

    /// Connection timeout in seconds (default: 30)
    #[arg(long, value_name = "SECONDS", default_value = "30")]
    pub connection_timeout: Option<u64>,

    /// Read timeout in seconds (default: 300)
    #[arg(long, value_name = "SECONDS", default_value = "300")]
    pub read_timeout: Option<u64>,

    /// Whether to follow redirects (default: true)
    #[arg(long, value_name = "BOOLEAN", default_value = "true")]
    pub follow_redirects: Option<bool>,

    /// Maximum number of redirects (default: 10)
    #[arg(long, value_name = "COUNT", default_value = "10")]
    pub max_redirects: Option<u64>,

    /// User agent string (default: "StorageHub-Client/1.0")
    #[arg(long, value_name = "STRING", default_value = "StorageHub-Client/1.0")]
    pub user_agent: Option<String>,

    /// Chunk size in bytes. This is different from the FILE_CHUNK_SIZE constant in the runtime, as it only affects file upload/download. (default: 8KB)
    #[arg(long, value_name = "BYTES", default_value = "8192")]
    pub chunk_size: Option<u64>,

    /// Number of `chunk_size` chunks to buffer during upload/download. (default: 512)
    #[arg(long, value_name = "COUNT", default_value = "512")]
    pub chunks_buffer: Option<u64>,

    // ============== MSP Charge Fees task options ==============
    /// Enable and configure MSP Charge Fees task.
    #[arg(long)]
    pub msp_charge_fees_task: bool,

    /// Minimum debt threshold for charging users.
    #[arg(
        long,
        value_name = "AMOUNT",
        help_heading = "MSP Charge Fees Options",
        required_if_eq_all([
            ("msp_charge_fees_task", "true"),
            ("provider_type", "msp"),
        ])
    )]
    pub msp_charge_fees_min_debt: Option<u64>,

    /// MSP charging fees period (in blocks).
    /// Setting it to 600 with a block every 6 seconds will charge user every hour.
    #[arg(long, required_if_eq_all([
        ("provider", "true"),
        ("provider_type", "msp"),
    ]))]
    pub msp_charging_period: Option<u32>,

    // ============== MSP Move Bucket task options ==============
    /// Enable and configure MSP Move Bucket task.
    #[arg(long)]
    pub msp_move_bucket_task: bool,

    /// Maximum number of times to retry a move bucket request.
    #[arg(
        long,
        value_name = "COUNT",
        help_heading = "MSP Move Bucket Options",
        required_if_eq_all([
            ("msp_move_bucket_task", "true"),
            ("provider_type", "msp"),
        ])
    )]
    pub msp_move_bucket_max_try_count: Option<u32>,

    /// Maximum tip amount to use when submitting a move bucket request extrinsic.
    #[arg(
        long,
        value_name = "AMOUNT",
        help_heading = "MSP Move Bucket Options",
        required_if_eq_all([
            ("msp_move_bucket_task", "true"),
            ("provider_type", "msp"),
        ])
    )]
    pub msp_move_bucket_max_tip: Option<u128>,

    // ============== BSP Upload File task options ==============
    /// Enable and configure BSP Upload File task.
    #[arg(long)]
    pub bsp_upload_file_task: bool,

    /// Maximum number of times to retry an upload file request.
    #[arg(
        long,
        value_name = "COUNT",
        help_heading = "BSP Upload File Options",
        required_if_eq_all([
            ("bsp_upload_file_task", "true"),
            ("provider_type", "bsp"),
        ])
    )]
    pub bsp_upload_file_max_try_count: Option<u32>,

    /// Maximum tip amount to use when submitting an upload file request extrinsic.
    #[arg(
        long,
        value_name = "AMOUNT",
        help_heading = "BSP Upload File Options",
        required_if_eq_all([
            ("bsp_upload_file_task", "true"),
            ("provider_type", "bsp"),
        ])
    )]
    pub bsp_upload_file_max_tip: Option<u128>,

    // ============== BSP Move Bucket task options ==============
    /// Enable and configure BSP Move Bucket task.
    #[arg(long)]
    pub bsp_move_bucket_task: bool,

    /// Grace period in seconds to accept download requests after a bucket move is accepted.
    #[arg(
        long,
        value_name = "SECONDS",
        help_heading = "BSP Move Bucket Options",
        required_if_eq_all([
            ("bsp_move_bucket_task", "true"),
            ("provider_type", "bsp"),
        ])
    )]
    pub bsp_move_bucket_grace_period: Option<u64>,

    // ============== BSP Charge Fees task options ==============
    /// Enable and configure BSP Charge Fees task.
    #[arg(long)]
    pub bsp_charge_fees_task: bool,

    /// Minimum debt threshold for charging users.
    #[arg(
        long,
        value_name = "AMOUNT",
        help_heading = "BSP Charge Fees Options",
        required_if_eq_all([
            ("bsp_charge_fees_task", "true"),
            ("provider_type", "bsp"),
        ])
    )]
    pub bsp_charge_fees_min_debt: Option<u64>,

    // ============== BSP Submit Proof task options ==============
    /// Enable and configure BSP Submit Proof task.
    #[arg(long)]
    pub bsp_submit_proof_task: bool,

    /// Maximum number of attempts to submit a proof.
    #[arg(
        long,
        value_name = "COUNT",
        help_heading = "BSP Submit Proof Options",
        required_if_eq_all([
            ("bsp_submit_proof_task", "true"),
            ("provider_type", "bsp"),
        ])
    )]
    pub bsp_submit_proof_max_attempts: Option<u32>,
}

impl ProviderConfigurations {
    pub fn provider_options(&self, maintenance_mode: bool) -> ProviderOptions {
        // Configure RPC options for Provider
        let mut rpc_config = RpcConfig::default();
        if let Some(max_file_size) = self.max_file_size {
            rpc_config.remote_file.max_file_size = max_file_size;
        }
        if let Some(connection_timeout) = self.connection_timeout {
            rpc_config.remote_file.connection_timeout = connection_timeout;
        }
        if let Some(read_timeout) = self.read_timeout {
            rpc_config.remote_file.read_timeout = read_timeout;
        }
        if let Some(follow_redirects) = self.follow_redirects {
            rpc_config.remote_file.follow_redirects = follow_redirects;
        }
        if let Some(max_redirects) = self.max_redirects {
            rpc_config.remote_file.max_redirects = max_redirects;
        }
        if let Some(user_agent) = self.user_agent.clone() {
            rpc_config.remote_file.user_agent = user_agent;
        }
        if let Some(chunk_size) = self.chunk_size {
            if chunk_size > 0 {
                rpc_config.remote_file.chunk_size = chunk_size as usize;
            }
        }
        if let Some(chunks_buffer) = self.chunks_buffer {
            if chunks_buffer > 0 {
                rpc_config.remote_file.chunks_buffer = chunks_buffer as usize;
            }
        }

        // Get provider type to conditionally apply options
        let provider_type = self
            .provider_type
            .clone()
            .expect("Provider type is required");

        let mut msp_charge_fees = None;
        let mut msp_move_bucket = None;
        let mut bsp_upload_file = None;
        let mut bsp_move_bucket = None;
        let mut bsp_charge_fees = None;
        let mut bsp_submit_proof = None;

        // Only set MSP options if provider_type is MSP
        if provider_type == ProviderType::Msp {
            // If specific task flags are enabled, use the provided options
            if self.msp_charge_fees_task {
                let mut options = MspChargeFeesOptions::default();
                options.min_debt = self.msp_charge_fees_min_debt;
                msp_charge_fees = Some(options);
            }

            if self.msp_move_bucket_task {
                let mut options = MspMoveBucketOptions::default();
                options.max_try_count = self.msp_move_bucket_max_try_count;
                options.max_tip = self.msp_move_bucket_max_tip;
                msp_move_bucket = Some(options);
            }
        }

        // Only set BSP options if provider_type is BSP
        if provider_type == ProviderType::Bsp {
            if self.bsp_upload_file_task {
                let mut options = BspUploadFileOptions::default();
                options.max_try_count = self.bsp_upload_file_max_try_count;
                options.max_tip = self.bsp_upload_file_max_tip;
                bsp_upload_file = Some(options);
            }

            if self.bsp_move_bucket_task {
                let mut options = BspMoveBucketOptions::default();
                options.move_bucket_accepted_grace_period = self.bsp_move_bucket_grace_period;
                bsp_move_bucket = Some(options);
            }

            if self.bsp_charge_fees_task {
                let mut options = BspChargeFeesOptions::default();
                options.min_debt = self.bsp_charge_fees_min_debt;
                bsp_charge_fees = Some(options);
            }

            if self.bsp_submit_proof_task {
                let mut options = BspSubmitProofOptions::default();
                options.max_submission_attempts = self.bsp_submit_proof_max_attempts;
                bsp_submit_proof = Some(options);
            }
        }

        let mut blockchain_service = None;

        // Accumulate blockchain service options so multiple flags combine instead of overwriting.
        let mut bs_options = BlockchainServiceOptions::default();
        let mut bs_changed = false;
        if let Some(extrinsic_retry_timeout) = self.extrinsic_retry_timeout {
            bs_options.extrinsic_retry_timeout = Some(extrinsic_retry_timeout);
            bs_changed = true;
        }

        if let Some(sync_mode_min_blocks_behind) = self.sync_mode_min_blocks_behind {
            bs_options.sync_mode_min_blocks_behind = Some(sync_mode_min_blocks_behind);
            bs_changed = true;
        }

        if let Some(check_for_pending_proofs_period) = self.check_for_pending_proofs_period {
            bs_options.check_for_pending_proofs_period = Some(check_for_pending_proofs_period);
            bs_changed = true;
        }

        if let Some(max_blocks_behind_to_catch_up_root_changes) =
            self.max_blocks_behind_to_catch_up_root_changes
        {
            bs_options.max_blocks_behind_to_catch_up_root_changes =
                Some(max_blocks_behind_to_catch_up_root_changes);
            bs_changed = true;
        }

        // Set MSP distribution flag if provided on CLI and role is MSP
        if self.msp_distribute_files && provider_type == ProviderType::Msp {
            bs_options.enable_msp_distribute_files = Some(true);
            bs_changed = true;
        }

        if bs_changed {
            blockchain_service = Some(bs_options);
        }

        ProviderOptions {
            provider_type,
            storage_layer: self
                .storage_layer
                .clone()
                .expect("Storage layer is required"),
            storage_path: self.storage_path.clone(),
            max_storage_capacity: self.max_storage_capacity,
            jump_capacity: self.jump_capacity,
            rpc_config: rpc_config,
            msp_charging_period: self.msp_charging_period,
            msp_charge_fees,
            msp_move_bucket,
            bsp_upload_file,
            bsp_move_bucket,
            bsp_charge_fees,
            bsp_submit_proof,
            blockchain_service,
            maintenance_mode,
        }
    }
}

#[derive(Debug, Parser, Clone)]
pub struct IndexerConfigurations {
    /// Enable the indexer service.
    #[arg(long)]
    pub indexer: bool,

    /// The mode in which the indexer runs.
    ///
    /// - `full`: Indexes all blockchain data
    /// - `lite`: Indexes only essential data for storage operations
    /// - `fishing`: Indexes only essential data for operating as a fisherman
    #[arg(long, value_parser = clap::value_parser!(IndexerMode), default_value = "full")]
    pub indexer_mode: IndexerMode,

    /// Postgres database URL.
    ///
    /// If not provided, the indexer will use the `INDEXER_DATABASE_URL` environment variable. If the
    /// environment variable is not set, the node will abort.
    #[arg(
        long("indexer-database-url"),
        env = "INDEXER_DATABASE_URL",
        required_if_eq("indexer", "true")
    )]
    pub indexer_database_url: Option<String>,
}

impl IndexerConfigurations {
    pub fn indexer_options(&self) -> Option<IndexerOptions> {
        if self.indexer {
            Some(IndexerOptions {
                indexer_mode: self.indexer_mode,
                database_url: self
                    .indexer_database_url
                    .clone()
                    .expect("Indexer database URL is required"),
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Parser, Clone)]
pub struct FishermanConfigurations {
    /// Enable the fisherman service.
    #[arg(long, conflicts_with = "provider")]
    pub fisherman: bool,

    /// Postgres database URL for the fisherman service.
    ///
    /// If not provided, the fisherman will use the `FISHERMAN_DATABASE_URL` environment variable.
    /// If the environment variable is not set, the node will abort.
    #[arg(
        long("fisherman-database-url"),
        env = "FISHERMAN_DATABASE_URL",
        required_if_eq("fisherman", "true")
    )]
    pub fisherman_database_url: Option<String>,

    /// Maximum number of incomplete storage requests to process after the first block processed coming out of syncing mode.
    #[arg(long, value_name = "COUNT", default_value = "10000", value_parser = clap::value_parser!(u32).range(1..))]
    pub fisherman_incomplete_sync_max: u32,

    /// Page size for incomplete storage request pagination.
    /// Must be at least 1.
    #[arg(long, value_name = "SIZE", default_value = "256", value_parser = clap::value_parser!(u32).range(1..))]
    pub fisherman_incomplete_sync_page_size: u32,

    /// The minimum number of blocks between the last processed block and the current best block to consider the fisherman out of sync.
    #[arg(long, default_value = "5")]
    pub fisherman_sync_mode_min_blocks_behind: u32,
}

impl FishermanConfigurations {
    pub fn fisherman_options(&self, maintenance_mode: bool) -> Option<FishermanOptions> {
        if self.fisherman {
            Some(FishermanOptions {
                database_url: self
                    .fisherman_database_url
                    .clone()
                    .expect("Fisherman database URL is required"),
                incomplete_sync_max: self.fisherman_incomplete_sync_max,
                incomplete_sync_page_size: self.fisherman_incomplete_sync_page_size,
                sync_mode_min_blocks_behind: self.fisherman_sync_mode_min_blocks_behind,
                maintenance_mode,
            })
        } else {
            None
        }
    }
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
#[command(after_help = AFTER_HELP_EXAMPLE)]
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

    /// Run the node in maintenance mode.
    /// In this mode, the node will not import blocks or participate in consensus,
    /// but will allow specific RPC calls for file and storage management.
    #[arg(long, default_value = "false")]
    pub maintenance_mode: bool,

    /// Provider configurations
    #[command(flatten)]
    pub provider_config: ProviderConfigurations,

    /// Provider configurations file path (allow to specify the provider configuration in a file instead of the cli)
    #[arg(long, conflicts_with_all = [
        "provider", "provider_type", "max_storage_capacity", "jump_capacity",
        "storage_layer", "storage_path", "extrinsic_retry_timeout", "sync_mode_min_blocks_behind",
        "check_for_pending_proofs_period", "max_blocks_behind_to_catch_up_root_changes",
        "msp_charging_period", "msp_charge_fees_task", "msp_charge_fees_min_debt",
        "msp_move_bucket_task", "msp_move_bucket_max_try_count", "msp_move_bucket_max_tip",
        "bsp_upload_file_task", "bsp_upload_file_max_try_count", "bsp_upload_file_max_tip",
        "bsp_move_bucket_task", "bsp_move_bucket_grace_period",
        "bsp_charge_fees_task", "bsp_charge_fees_min_debt",
        "bsp_submit_proof_task", "bsp_submit_proof_max_attempts",
    ])]
    pub provider_config_file: Option<String>,

    /// Indexer configurations
    #[command(flatten)]
    pub indexer_config: IndexerConfigurations,

    /// Fisherman configurations
    #[command(flatten)]
    pub fisherman_config: FishermanConfigurations,

    /// Relay chain arguments
    #[arg(raw = true)]
    pub relay_chain_args: Vec<String>,
}

#[derive(Debug, Parser)]
#[group(skip)]
pub struct RunCmd {
    #[command(flatten)]
    pub base: cumulus_client_cli::RunCmd,

    /// When blocks should be sealed in the dev service.
    ///
    /// Options are "instant", "manual", or timer interval in milliseconds
    #[arg(long, default_value = "instant")]
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

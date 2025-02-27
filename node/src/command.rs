use cumulus_client_service::storage_proof_size::HostFunctions as ReclaimHostFunctions;
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::info;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, RpcEndpoint, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use serde::Deserialize;
use storage_hub_runtime::{Block, StorageDataUnit};

use crate::{
    chain_spec,
    cli::{Cli, ProviderType, RelayChainCli, StorageLayer, Subcommand},
    config,
    service::new_partial,
};

// TODO: Have specific StorageHub role options (i.e. ProviderOptions, UserOptions).
/// Configuration for the provider.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderOptions {
    /// Provider type.
    pub provider_type: ProviderType,
    /// Storage layer.
    pub storage_layer: StorageLayer,
    /// RocksDB Path.
    pub storage_path: Option<String>,
    /// Maximum storage capacity of the Storage Provider (bytes).
    pub max_storage_capacity: Option<StorageDataUnit>,
    /// Jump capacity (bytes).
    pub jump_capacity: Option<StorageDataUnit>,
    /// Extrinsic retry timeout in seconds.
    pub extrinsic_retry_timeout: u64,
    /// MSP charging fees frequency.
    pub msp_charging_period: Option<u32>,

    // Task-specific configuration options
    /// Configuration options for MSP delete file task.
    #[serde(default)]
    pub msp_delete_file: MspDeleteFileOptions,
    /// Configuration options for MSP charge fees task.
    #[serde(default)]
    pub msp_charge_fees: MspChargeFeesOptions,
    /// Configuration options for MSP move bucket task.
    #[serde(default)]
    pub msp_move_bucket: MspMoveBucketOptions,
    /// Configuration options for BSP upload file task.
    #[serde(default)]
    pub bsp_upload_file: BspUploadFileOptions,
    /// Configuration options for BSP move bucket task.
    #[serde(default)]
    pub bsp_move_bucket: BspMoveBucketOptions,
    /// Configuration options for BSP charge fees task.
    #[serde(default)]
    pub bsp_charge_fees: BspChargeFeesOptions,
    /// Configuration options for BSP submit proof task.
    #[serde(default)]
    pub bsp_submit_proof: BspSubmitProofOptions,

    // Service-specific configuration options
    /// Configuration options for blockchain service.
    #[serde(default)]
    pub blockchain_service: BlockchainServiceOptions,
    /// Configuration options for file transfer service.
    #[serde(default)]
    pub file_transfer_service: FileTransferServiceOptions,
    // Add more grouped configuration options here as needed
}

/// Configuration options for the MSP Delete File task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspDeleteFileOptions {
    /// Maximum number of times to retry a file deletion request.
    #[serde(default)]
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting a file deletion request extrinsic.
    #[serde(default)]
    pub max_tip: Option<u128>,
}

/// Configuration options for the MSP Charge Fees task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspChargeFeesOptions {
    /// Minimum debt threshold for charging users.
    #[serde(default)]
    pub min_debt: Option<u128>,
}

/// Configuration options for the MSP Move Bucket task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct MspMoveBucketOptions {
    /// Maximum number of times to retry a move bucket request.
    #[serde(default)]
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting a move bucket request extrinsic.
    #[serde(default)]
    pub max_tip: Option<u128>,
    /// Processing interval between batches of move bucket requests.
    #[serde(default)]
    pub processing_interval: Option<u64>,
    /// Maximum batch size of move bucket requests to process at once.
    #[serde(default)]
    pub max_batch_size: Option<u32>,
    /// Maximum number of parallel move bucket tasks.
    #[serde(default)]
    pub max_parallel_tasks: Option<u32>,
    /// Maximum number of files to download in parallel.
    #[serde(default)]
    pub max_concurrent_file_downloads: Option<usize>,
    /// Maximum number of chunks requests to do in parallel per file.
    #[serde(default)]
    pub max_concurrent_chunks_per_file: Option<usize>,
    /// Maximum number of chunks to request in a single network request.
    #[serde(default)]
    pub max_chunks_per_request: Option<usize>,
    /// Number of peers to select for each chunk download attempt (2 best + x random).
    #[serde(default)]
    pub chunk_request_peer_retry_attempts: Option<usize>,
    /// Number of retries per peer for a single chunk request.
    #[serde(default)]
    pub download_retry_attempts: Option<usize>,
}

/// Configuration options for the BSP Upload File task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspUploadFileOptions {
    /// Maximum number of times to retry an upload file request.
    #[serde(default)]
    pub max_try_count: Option<u32>,
    /// Maximum tip amount to use when submitting an upload file request extrinsic.
    #[serde(default)]
    pub max_tip: Option<u128>,
}

/// Configuration options for the BSP Move Bucket task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspMoveBucketOptions {
    /// Grace period in seconds to accept download requests after a bucket move is accepted.
    #[serde(default)]
    pub move_bucket_accepted_grace_period: Option<u64>,
}

/// Configuration options for the BSP Charge Fees task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspChargeFeesOptions {
    /// Minimum debt threshold for charging users.
    #[serde(default)]
    pub min_debt: Option<u128>,
}

/// Configuration options for the BSP Submit Proof task.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BspSubmitProofOptions {
    /// Maximum number of attempts to submit a proof.
    #[serde(default)]
    pub max_submission_attempts: Option<u32>,
}

/// Configuration options for the Blockchain Service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BlockchainServiceOptions {
    // Reserved for future blockchain service configuration options
}

/// Configuration options for the File Transfer Service.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileTransferServiceOptions {
    // Reserved for future file transfer service configuration options
}

/// Configuration for the indexer.
#[derive(Debug, Clone, Deserialize)]
pub struct IndexerOptions {
    /// Whether to enable the indexer.
    pub indexer: bool,
    /// Postgres database URL.
    pub database_url: Option<String>,
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
    Ok(match id {
        "dev" => Box::new(chain_spec::development_config()),
        "template-rococo" => Box::new(chain_spec::local_testnet_config()),
        "" | "local" => Box::new(chain_spec::local_testnet_config()),
        path => Box::new(chain_spec::ChainSpec::from_json_file(
            std::path::PathBuf::from(path),
        )?),
    })
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Parachain Collator Template".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Parachain Collator Template\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/paritytech/polkadot-sdk/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Parachain Collator Template".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Parachain Collator Template\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relay chain node.\n\n\
		{} <parachain-args> -- <relay-chain-args>",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/paritytech/polkadot-sdk/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2020
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
    }
}

macro_rules! construct_async_run {
	(|$components:ident, $cli:ident, $cmd:ident, $config:ident, $dev_service:ident| $( $code:tt )* ) => {{
		let runner = $cli.create_runner($cmd)?;
		runner.async_run(|$config| {
			let $components = new_partial(&$config, $dev_service)?;
			let task_manager = $components.task_manager;
			{ $( $code )* }.map(|v| (v, task_manager))
		})
	}}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
    let cli = Cli::from_args();

    let dev_service = cli.run.base.base.shared_params.is_dev();

    match &cli.subcommand {
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, config, dev_service| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config, dev_service| {
                Ok(cmd.run(components.client, config.database))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config, dev_service| {
                Ok(cmd.run(components.client, config.chain_spec))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config, dev_service| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::Revert(cmd)) => {
            construct_async_run!(|components, cli, cmd, config, dev_service| {
                Ok(cmd.run(components.client, components.backend, None))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name()]
                        .iter()
                        .chain(cli.relay_chain_args.iter()),
                );

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.tokio_handle.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                cmd.run(config, polkadot_config)
            })
        }
        Some(Subcommand::ExportGenesisHead(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| {
                let partials = new_partial(&config, dev_service)?;

                cmd.run(partials.client)
            })
        }
        Some(Subcommand::ExportGenesisWasm(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                cmd.run(&*spec)
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            // Switch on the concrete benchmark sub-command-
            match cmd {
                BenchmarkCmd::Pallet(cmd) => {
                    if cfg!(feature = "runtime-benchmarks") {
                        runner.sync_run(|config| cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, ReclaimHostFunctions>(Some(config.chain_spec)))
                    } else {
                        Err("Benchmarking wasn't enabled when building the node. \
					You can enable it with `--features runtime-benchmarks`."
                            .into())
                    }
                }
                BenchmarkCmd::Block(cmd) => runner.sync_run(|config| {
                    let partials = new_partial(&config, dev_service)?;
                    cmd.run(partials.client)
                }),
                #[cfg(not(feature = "runtime-benchmarks"))]
                BenchmarkCmd::Storage(_) => {
                    return Err(sc_cli::Error::Input(
                        "Compile with --features=runtime-benchmarks \
						to enable storage benchmarks."
                            .into(),
                    )
                    .into())
                }
                #[cfg(feature = "runtime-benchmarks")]
                BenchmarkCmd::Storage(cmd) => runner.sync_run(|config| {
                    let partials = new_partial(&config, dev_service)?;
                    let db = partials.backend.expose_db();
                    let storage = partials.backend.expose_storage();
                    cmd.run(config, partials.client.clone(), db, storage)
                }),
                BenchmarkCmd::Machine(cmd) => {
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
                }
                // NOTE: this allows the Client to leniently implement
                // new benchmark commands without requiring a companion MR.
                #[allow(unreachable_patterns)]
                _ => Err("Benchmarking sub-command unsupported".into()),
            }
        }
        None => {
            let mut provider_options = None;
            let mut indexer_options = None;
            let runner = cli.create_runner(&cli.run.normalize())?;

            // If we have a provider config file
            if let Some(provider_config_file) = cli.provider_config_file {
                let config = config::read_config(&provider_config_file);
                if let Some(c) = config {
                    provider_options = Some(c.provider);
                    indexer_options = c.indexer;
                };
            };

            // We then check cli (the cli doesn't allow to have both cli parameters and a config file so we should not have overlap here)
            if cli.provider_config.provider {
                provider_options = Some(cli.provider_config.provider_options());
            };

            // Convert IndexerOptions to IndexerConfigurations if available
            let indexer_config = if let Some(opts) = indexer_options {
                crate::cli::IndexerConfigurations {
                    indexer: opts.indexer,
                    database_url: opts.database_url,
                }
            } else {
                cli.indexer_config
            };

            runner.run_node_until_exit(|config| async move {
				let hwbench = (!cli.no_hardware_benchmarks)
					.then_some(config.database.path().map(|database_path| {
						let _ = std::fs::create_dir_all(database_path);
						sc_sysinfo::gather_hwbench(Some(database_path), &SUBSTRATE_REFERENCE_HARDWARE)
					}))
					.flatten();


                let para_id = chain_spec::Extensions::try_get(&*config.chain_spec)
                    .map(|e| e.para_id)
                    .ok_or("Could not find parachain ID in chain-spec.")?;

                let id = ParaId::from(para_id);

                info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				match config.network.network_backend {
					sc_network::config::NetworkBackendType::Libp2p => {
						if dev_service {
							crate::service::start_dev_node::<sc_network::NetworkWorker<_, _>>(
								config,
								provider_options,
								indexer_config,
								hwbench,
								id,
								cli.run.sealing,
							)
							.await
							.map_err(Into::into)
						} else {
							let collator_options = cli.run.collator_options();
							let polkadot_cli = RelayChainCli::new(
								&config,
								[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
							);
							let tokio_handle = config.tokio_handle.clone();
							let polkadot_config =
								SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
									.map_err(|err| format!("Relay chain argument error: {}", err))?;
							crate::service::start_parachain_node::<sc_network::NetworkWorker<_, _>>(
								config,
								polkadot_config,
								collator_options,
								provider_options,
								indexer_config,
								id,
								hwbench,
							)
							.await
							.map(|r| r.0)
							.map_err(Into::into)
						}
					},
					sc_network::config::NetworkBackendType::Litep2p => {
						if dev_service {
							crate::service::start_dev_node::<sc_network::Litep2pNetworkBackend>(
								config,
								provider_options,
								indexer_config,
								hwbench,
								id,
								cli.run.sealing,
							)
							.await
							.map_err(Into::into)
						} else {
							let collator_options = cli.run.collator_options();
							let polkadot_cli = RelayChainCli::new(
								&config,
								[RelayChainCli::executable_name()].iter().chain(cli.relay_chain_args.iter()),
							);
							let tokio_handle = config.tokio_handle.clone();
							let polkadot_config =
								SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
									.map_err(|err| format!("Relay chain argument error: {}", err))?;
							crate::service::start_parachain_node::<sc_network::Litep2pNetworkBackend>(
								config,
								polkadot_config,
								collator_options,
								provider_options,
								indexer_config,
								id,
								hwbench,
							)
							.await
							.map(|r| r.0)
							.map_err(Into::into)
						}
					},
				}
			})
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_listen_port() -> u16 {
        9945
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()?
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_addr(&self, default_listen_port: u16) -> Result<Option<Vec<RpcEndpoint>>> {
        self.base.base.rpc_addr(default_listen_port)
    }

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<PrometheusConfig>> {
        self.base
            .base
            .prometheus_config(default_listen_port, chain_spec)
    }

    fn init<F>(&self, _support_url: &String, _impl_version: &String, _logger_hook: F) -> Result<()>
    where
        F: FnOnce(&mut sc_cli::LoggerBuilder),
    {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() {
            self.chain_id.clone().unwrap_or_default()
        } else {
            chain_id
        })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool(is_dev)
    }

    fn trie_cache_maximum_size(&self) -> Result<Option<usize>> {
        self.base.base.trie_cache_maximum_size()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_max_connections(&self) -> Result<u32> {
        self.base.base.rpc_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
        self.base.base.telemetry_endpoints(chain_spec)
    }

    fn node_name(&self) -> Result<String> {
        self.base.base.node_name()
    }
}

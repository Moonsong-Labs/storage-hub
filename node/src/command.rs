use cumulus_client_service::storage_proof_size::HostFunctions as ReclaimHostFunctions;
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::info;
use polkadot_cli::service::IdentifyNetworkBackend;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, RpcEndpoint, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use serde::Deserialize;
use shp_types::StorageDataUnit;
use storage_hub_runtime::Block;

use crate::{
    chain_spec::{self, NetworkType},
    cli::{Cli, ProviderType, RelayChainCli, StorageLayer, Subcommand},
    config,
    service::{new_partial_parachain, new_partial_solochain_evm},
};

use shc_client::builder::{
    BlockchainServiceOptions, BspChargeFeesOptions, BspMoveBucketOptions, BspSubmitProofOptions,
    BspUploadFileOptions, MspChargeFeesOptions, MspMoveBucketOptions,
};
use shc_rpc::RpcConfig;

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
    /// RPC configuration options.
    #[serde(default)]
    pub rpc_config: RpcConfig,
    /// MSP charging fees frequency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msp_charging_period: Option<u32>,
    /// Configuration options for MSP charge fees task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msp_charge_fees: Option<MspChargeFeesOptions>,
    /// Configuration options for MSP move bucket task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msp_move_bucket: Option<MspMoveBucketOptions>,
    /// Configuration options for BSP upload file task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bsp_upload_file: Option<BspUploadFileOptions>,
    /// Configuration options for BSP move bucket task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bsp_move_bucket: Option<BspMoveBucketOptions>,
    /// Configuration options for BSP charge fees task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bsp_charge_fees: Option<BspChargeFeesOptions>,
    /// Configuration options for BSP submit proof task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bsp_submit_proof: Option<BspSubmitProofOptions>,
    /// Configuration options for blockchain service.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blockchain_service: Option<BlockchainServiceOptions>,
    /// Whether the node is running in maintenance mode.
    pub maintenance_mode: bool,
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn ChainSpec>, String> {
    Ok(match id {
        // Parachain variants (default fallback for compatibility)
        "dev" | "parachain-dev" => Box::new(chain_spec::parachain::development_config()),
        "" | "local" | "parachain-local" => Box::new(chain_spec::parachain::local_testnet_config()),
        "template-rococo" => Box::new(chain_spec::parachain::local_testnet_config()),

        // Solochain EVM variants
        "solochain-evm-dev" => Box::new(chain_spec::solochain_evm::development_config()?),
        "solochain-evm-local" => Box::new(chain_spec::solochain_evm::local_testnet_config()?),

        // Custom chain spec from file
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
            let runner = cli.create_runner(cmd)?;
            let chain = cli
                .run
                .base
                .base
                .shared_params
                .chain
                .clone()
                .unwrap_or_default();
            if load_spec(&chain)?.is_parachain() {
                runner.async_run(|config| {
                    let components = new_partial_parachain(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((
                        cmd.run(components.client, components.import_queue),
                        task_manager,
                    ))
                })
            } else if load_spec(&chain)?.is_solochain_evm() {
                runner.async_run(|config| {
                    let components = new_partial_solochain_evm(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((
                        cmd.run(components.client, components.import_queue),
                        task_manager,
                    ))
                })
            } else {
                unreachable!("Invalid chain spec")
            }
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain = cli
                .run
                .base
                .base
                .shared_params
                .chain
                .clone()
                .unwrap_or_default();
            if load_spec(&chain)?.is_parachain() {
                runner.async_run(|config| {
                    let components = new_partial_parachain(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((cmd.run(components.client, config.database), task_manager))
                })
            } else if load_spec(&chain)?.is_solochain_evm() {
                runner.async_run(|config| {
                    let components = new_partial_solochain_evm(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((cmd.run(components.client, config.database), task_manager))
                })
            } else {
                unreachable!("Invalid chain spec")
            }
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain = cli
                .run
                .base
                .base
                .shared_params
                .chain
                .clone()
                .unwrap_or_default();
            if load_spec(&chain)?.is_parachain() {
                runner.async_run(|config| {
                    let components = new_partial_parachain(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((cmd.run(components.client, config.chain_spec), task_manager))
                })
            } else if load_spec(&chain)?.is_solochain_evm() {
                runner.async_run(|config| {
                    let components = new_partial_solochain_evm(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((cmd.run(components.client, config.chain_spec), task_manager))
                })
            } else {
                unreachable!("Invalid chain spec")
            }
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain = cli
                .run
                .base
                .base
                .shared_params
                .chain
                .clone()
                .unwrap_or_default();
            if load_spec(&chain)?.is_parachain() {
                runner.async_run(|config| {
                    let components = new_partial_parachain(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((
                        cmd.run(components.client, components.import_queue),
                        task_manager,
                    ))
                })
            } else if load_spec(&chain)?.is_solochain_evm() {
                runner.async_run(|config| {
                    let components = new_partial_solochain_evm(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((
                        cmd.run(components.client, components.import_queue),
                        task_manager,
                    ))
                })
            } else {
                unreachable!("Invalid chain spec")
            }
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain = cli
                .run
                .base
                .base
                .shared_params
                .chain
                .clone()
                .unwrap_or_default();
            if load_spec(&chain)?.is_parachain() {
                runner.async_run(|config| {
                    let components = new_partial_parachain(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((
                        cmd.run(components.client, components.backend, None),
                        task_manager,
                    ))
                })
            } else if load_spec(&chain)?.is_solochain_evm() {
                runner.async_run(|config| {
                    let components = new_partial_solochain_evm(&config, dev_service)?;
                    let task_manager = components.task_manager;
                    Ok((
                        cmd.run(components.client, components.backend, None),
                        task_manager,
                    ))
                })
            } else {
                unreachable!("Invalid chain spec")
            }
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
                if config.chain_spec.is_parachain() {
                    let partials = new_partial_parachain(&config, dev_service)?;
                    cmd.run(partials.client)
                } else if config.chain_spec.is_solochain_evm() {
                    let partials = new_partial_solochain_evm(&config, dev_service)?;
                    cmd.run(partials.client)
                } else {
                    unreachable!("Invalid chain spec")
                }
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
                    if config.chain_spec.is_parachain() {
                        let partials = new_partial_parachain(&config, dev_service)?;
                        cmd.run(partials.client)
                    } else if config.chain_spec.is_solochain_evm() {
                        let partials = new_partial_solochain_evm(&config, dev_service)?;
                        cmd.run(partials.client)
                    } else {
                        unreachable!("Invalid chain spec")
                    }
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
                    let partials = if config.chain_spec.is_parachain() {
                        new_partial_parachain(&config, dev_service)?
                    } else if config.chain_spec.is_solochain_evm() {
                        new_partial_solochain_evm(&config, dev_service)?
                    } else {
                        unreachable!("Invalid chain spec")
                    };
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
            let mut fisherman_options = None;
            let runner = cli.create_runner(&cli.run.normalize())?;

            // If we have a provider config file
            if let Some(provider_config_file) = cli.provider_config_file {
                let config = config::read_config(&provider_config_file);
                if let Some(c) = config {
                    provider_options = Some(c.provider);
                    indexer_options = Some(c.indexer);
                    fisherman_options = Some(c.fisherman);
                };
            };

            // We then check cli (the cli doesn't allow to have both cli parameters and a config file so we should not have overlap here)
            if cli.provider_config.provider {
                provider_options = Some(cli.provider_config.provider_options());
            };

            if cli.indexer_config.indexer {
                indexer_options = cli.indexer_config.indexer_options();
            };

            if cli.fisherman_config.fisherman {
                fisherman_options = cli.fisherman_config.fisherman_options();
            };

            runner.run_node_until_exit(|config| async move {
                let hwbench = (!cli.no_hardware_benchmarks)
                    .then_some(config.database.path().map(|database_path| {
                        let _ = std::fs::create_dir_all(database_path);
                        sc_sysinfo::gather_hwbench(
                            Some(database_path),
                            &SUBSTRATE_REFERENCE_HARDWARE,
                        )
                    }))
                    .flatten();

                let para_id = chain_spec::Extensions::try_get(&*config.chain_spec)
                    .map(|e| e.para_id)
                    .ok_or("Could not find parachain ID in chain-spec.")?;

                let id = ParaId::from(para_id);

                info!(
                    "Is collating: {}",
                    if config.role.is_authority() {
                        "yes"
                    } else {
                        "no"
                    }
                );

                let default_backend = config.chain_spec.network_backend();
                let network_backend = config.network.network_backend.unwrap_or(default_backend);

                match network_backend {
                    sc_network::config::NetworkBackendType::Libp2p => {
                        if dev_service {
                            if config.chain_spec.is_parachain() {
                                crate::service::start_dev_parachain_node::<
                                    sc_network::NetworkWorker<_, _>,
                                >(
                                    config,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    hwbench,
                                    id,
                                    cli.run.sealing,
                                )
                                .await
                                .map_err(Into::into)
                            } else if config.chain_spec.is_solochain_evm() {
                                crate::service::start_dev_solochain_evm_node::<
                                    sc_network::NetworkWorker<_, _>,
                                >(
                                    config,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    hwbench,
                                    id,
                                    cli.run.sealing,
                                )
                                .await
                                .map_err(Into::into)
                            } else {
                                unreachable!("Invalid chain spec")
                            }
                        } else {
                            let collator_options = cli.run.collator_options();
                            let polkadot_cli = RelayChainCli::new(
                                &config,
                                [RelayChainCli::executable_name()]
                                    .iter()
                                    .chain(cli.relay_chain_args.iter()),
                            );
                            let tokio_handle = config.tokio_handle.clone();
                            let polkadot_config = SubstrateCli::create_configuration(
                                &polkadot_cli,
                                &polkadot_cli,
                                tokio_handle,
                            )
                            .map_err(|err| format!("Relay chain argument error: {}", err))?;

                            if config.chain_spec.is_parachain() {
                                crate::service::start_parachain_node::<
                                    sc_network::NetworkWorker<_, _>,
                                >(
                                    config,
                                    polkadot_config,
                                    collator_options,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    id,
                                    hwbench,
                                )
                                .await
                                .map(|r| r.0)
                                .map_err(Into::into)
                            } else if config.chain_spec.is_solochain_evm() {
                                crate::service::start_solochain_evm_node::<
                                    sc_network::NetworkWorker<_, _>,
                                >(
                                    config,
                                    polkadot_config,
                                    collator_options,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    id,
                                    hwbench,
                                )
                                .await
                                .map(|r| r.0)
                                .map_err(Into::into)
                            } else {
                                unreachable!("Invalid chain spec")
                            }
                        }
                    }
                    sc_network::config::NetworkBackendType::Litep2p => {
                        if dev_service {
                            if config.chain_spec.is_parachain() {
                                crate::service::start_dev_parachain_node::<
                                    sc_network::Litep2pNetworkBackend,
                                >(
                                    config,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    hwbench,
                                    id,
                                    cli.run.sealing,
                                )
                                .await
                                .map_err(Into::into)
                            } else if config.chain_spec.is_solochain_evm() {
                                crate::service::start_dev_solochain_evm_node::<
                                    sc_network::Litep2pNetworkBackend,
                                >(
                                    config,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    hwbench,
                                    id,
                                    cli.run.sealing,
                                )
                                .await
                                .map_err(Into::into)
                            } else {
                                unreachable!("Invalid chain spec")
                            }
                        } else {
                            let collator_options = cli.run.collator_options();
                            let polkadot_cli = RelayChainCli::new(
                                &config,
                                [RelayChainCli::executable_name()]
                                    .iter()
                                    .chain(cli.relay_chain_args.iter()),
                            );
                            let tokio_handle = config.tokio_handle.clone();
                            let polkadot_config = SubstrateCli::create_configuration(
                                &polkadot_cli,
                                &polkadot_cli,
                                tokio_handle,
                            )
                            .map_err(|err| format!("Relay chain argument error: {}", err))?;

                            if config.chain_spec.is_parachain() {
                                crate::service::start_parachain_node::<
                                    sc_network::Litep2pNetworkBackend,
                                >(
                                    config,
                                    polkadot_config,
                                    collator_options,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    id,
                                    hwbench,
                                )
                                .await
                                .map(|r| r.0)
                                .map_err(Into::into)
                            } else if config.chain_spec.is_solochain_evm() {
                                crate::service::start_solochain_evm_node::<
                                    sc_network::Litep2pNetworkBackend,
                                >(
                                    config,
                                    polkadot_config,
                                    collator_options,
                                    provider_options,
                                    indexer_options,
                                    fisherman_options.clone(),
                                    id,
                                    hwbench,
                                )
                                .await
                                .map(|r| r.0)
                                .map_err(Into::into)
                            } else {
                                unreachable!("Invalid chain spec")
                            }
                        }
                    }
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

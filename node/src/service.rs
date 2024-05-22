//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

// std
use std::{sync::Arc, time::Duration};

use codec::Encode;
use cumulus_client_cli::CollatorOptions;
use cumulus_client_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};

use file_manager::{in_memory::InMemoryFileStorage, traits::FileStorage};
use forest_manager::{
    in_memory::InMemoryForestStorage, rocksdb::RocksDBForestStorage, traits::ForestStorage,
};
use futures::{Stream, StreamExt};
use log::debug;
use polkadot_primitives::{BlakeTwo256, HeadData, ValidationCode};
use sc_consensus_manual_seal::consensus::aura::AuraConsensusDataProvider;
use shc_common::types::HasherOutT;
use sp_consensus_aura::Slot;
use sp_core::H256;
use sp_trie::{LayoutV1, TrieLayout};
use storage_hub_infra::actor::TaskSpawner;
// Local Runtime Types
use storage_hub_runtime::{
    opaque::{Block, Hash},
    RuntimeApi,
};

// Cumulus Imports
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;
use cumulus_client_service::{
    build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
    BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::{
    relay_chain::{well_known_keys as RelayChainWellKnownKeys, CollatorPair},
    ParaId,
};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

// Substrate Imports
use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use sc_client_api::{Backend, HeaderBackend};
use sc_consensus::{ImportQueue, LongestChain};
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{NetworkBlock, NetworkService};
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::Block as BlockT;
use substrate_prometheus_endpoint::Registry;

use crate::{
    cli::StorageLayer,
    services::{
        blockchain::BlockchainService,
        builder::{StorageHubBuilder, StorageLayerBuilder},
        file_transfer::configure_file_transfer_network,
        handler::StorageHubHandler,
    },
};
use crate::{
    cli::{self, ProviderType},
    command::ProviderOptions,
    services::blockchain::KEY_TYPE,
};

#[cfg(not(feature = "runtime-benchmarks"))]
type HostFunctions = (
    // TODO: change this to `cumulus_client_service::ParachainHostFunctions` once it is part of the next release
    sp_io::SubstrateHostFunctions,
    cumulus_client_service::storage_proof_size::HostFunctions,
);

#[cfg(feature = "runtime-benchmarks")]
type HostFunctions = (
    // TODO: change this to `cumulus_client_service::ParachainHostFunctions` once it is part of the next release
    sp_io::SubstrateHostFunctions,
    cumulus_client_service::storage_proof_size::HostFunctions,
    frame_benchmarking::benchmarking::HostFunctions,
);

pub(crate) type ParachainExecutor = WasmExecutor<HostFunctions>;

pub(crate) type ParachainClient = TFullClient<Block, RuntimeApi, ParachainExecutor>;

pub(crate) type ParachainBackend = TFullBackend<Block>;

pub(crate) type ParachainBlockImport =
    TParachainBlockImport<Block, Arc<ParachainClient>, ParachainBackend>;

pub(crate) type ParachainNetworkService = NetworkService<Block, <Block as BlockT>::Hash>;

type MaybeSelectChain = Option<sc_consensus::LongestChain<ParachainBackend, Block>>;

/// Assembly of PartialComponents (enough to run chain ops subcommands)
pub type Service = PartialComponents<
    ParachainClient,
    ParachainBackend,
    MaybeSelectChain,
    sc_consensus::DefaultImportQueue<Block>,
    sc_transaction_pool::FullPool<Block, ParachainClient>,
    (
        ParachainBlockImport,
        Option<Telemetry>,
        Option<TelemetryWorkerHandle>,
    ),
>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial(
    config: &Configuration,
    dev_service: bool,
) -> Result<Service, sc_service::Error> {
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let heap_pages = config
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = ParachainExecutor::builder()
        .with_execution_method(config.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.max_runtime_instances)
        .with_runtime_cache_size(config.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
            true,
        )?;
    let client = Arc::new(client);

    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let block_import = ParachainBlockImport::new(client.clone(), backend.clone());

    let import_queue = if dev_service {
        sc_consensus_manual_seal::import_queue(
            Box::new(client.clone()),
            &task_manager.spawn_essential_handle(),
            config.prometheus_registry(),
        )
    } else {
        build_import_queue(
            client.clone(),
            block_import.clone(),
            config,
            telemetry.as_ref().map(|telemetry| telemetry.handle()),
            &task_manager,
        )?
    };

    let select_chain = if dev_service {
        Some(LongestChain::new(backend.clone()))
    } else {
        None
    };

    Ok(PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain,
        other: (block_import, telemetry, telemetry_worker_handle),
    })
}

fn start_provider_tasks<T, FL, FS>(
    provider_options: ProviderOptions,
    sh_handler: StorageHubHandler<T, FL, FS>,
) where
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync + 'static,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    // Starting the tasks according to the provider type.
    match provider_options.provider_type {
        ProviderType::Bsp => sh_handler.start_bsp_tasks(),
        ProviderType::User => sh_handler.start_user_tasks(),
        _ => {}
    }
}

/// Start a development node with the given solo chain `Configuration`.
#[sc_tracing::logging::prefix_logs_with("Solo chain 💾")]
async fn start_dev_impl<FL, FS>(
    config: Configuration,
    provider_options: Option<ProviderOptions>,
    hwbench: Option<sc_sysinfo::HwBench>,
    para_id: ParaId,
    sealing: cli::Sealing,
) -> sc_service::error::Result<TaskManager>
where
    StorageHubBuilder<LayoutV1<BlakeTwo256>, FL, FS>: StorageLayerBuilder,
    FL: FileStorage<LayoutV1<BlakeTwo256>> + Send + Sync,
    FS: ForestStorage<LayoutV1<BlakeTwo256>> + Send + Sync + 'static,
    HasherOutT<LayoutV1<BlakeTwo256>>: TryFrom<[u8; 32]>,
{
    use async_io::Timer;
    use sc_consensus_manual_seal::{run_manual_seal, EngineCommand, ManualSealParams};

    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain: maybe_select_chain,
        transaction_pool,
        other: (_, mut telemetry, _),
    } = new_partial(&config, true)?;

    let signing_dev_key = config
        .dev_key_seed
        .clone()
        .expect("Dev key seed must be present in dev mode.");
    let keystore = keystore_container.keystore();

    // Initialise seed for signing transactions using blockchain service.
    // In dev mode we use a well known dev account.
    keystore
        .sr25519_generate_new(KEY_TYPE, Some(signing_dev_key.as_ref()))
        .expect("Invalid dev signing key provided.");

    let mut net_config = sc_network::config::FullNetworkConfiguration::new(&config.network);
    let collator = config.role.is_authority();
    let prometheus_registry = config.prometheus_registry().cloned();
    let select_chain = maybe_select_chain
        .expect("In `dev` mode, `new_partial` will return some `select_chain`; qed");

    // If we are a provider we update the network configuration with the file transfer protocol.
    let mut file_transfer_request_protocol = None;
    if provider_options.is_some() {
        file_transfer_request_protocol = Some(configure_file_transfer_network(
            client.clone(),
            &config,
            &mut net_config,
        ));
    }

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_params: None,
            block_relay: None,
        })?;

    if config.offchain_worker.enabled {
        use futures::FutureExt;

        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-work",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                keystore: Some(keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: network.clone(),
                is_validator: config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    let mut command_sink = None;

    let commands_stream: Box<dyn Stream<Item = EngineCommand<H256>> + Send + Sync + Unpin> =
        match sealing {
            cli::Sealing::Instant => {
                Box::new(
                    // This bit cribbed from the implementation of instant seal.
                    transaction_pool
                        .pool()
                        .validated_pool()
                        .import_notification_stream()
                        .map(|_| EngineCommand::SealNewBlock {
                            create_empty: false,
                            finalize: false,
                            parent_hash: None,
                            sender: None,
                        }),
                )
            }
            cli::Sealing::Manual => {
                let (sink, stream) = futures::channel::mpsc::channel(1000);
                // Keep a reference to the other end of the channel. It goes to the RPC.
                command_sink = Some(sink);
                Box::new(stream)
            }
            cli::Sealing::Interval(millis) => {
                if millis < 3000 {
                    log::info!("⚠️ Sealing interval is very short. Normally setting this to 6000 ms is recommended.");
                }

                Box::new(StreamExt::map(
                    Timer::interval(Duration::from_millis(millis)),
                    |_| EngineCommand::SealNewBlock {
                        create_empty: true,
                        finalize: false,
                        parent_hash: None,
                        sender: None,
                    },
                ))
            }
        };

    // If node is running as a Storage Provider, start building the StorageHubHandler using the StorageHubBuilder.
    let mut sh_builder = None;
    if provider_options.is_some() {
        // Start building the StorageHubHandler, if running as a provider.
        let task_spawner = TaskSpawner::new(task_manager.spawn_handle(), "generic");

        // Create builder for the StorageHubHandler.
        let mut storage_hub_builder =
            StorageHubBuilder::<LayoutV1<BlakeTwo256>, FL, FS>::new(task_spawner);

        // Add FileTransfer Service to the StorageHubHandler.
        let (file_transfer_request_protocol_name, file_transfer_request_receiver) =
            file_transfer_request_protocol
                .expect("FileTransfer request protocol should already be initialized.");
        storage_hub_builder
            .with_file_transfer(
                file_transfer_request_receiver,
                file_transfer_request_protocol_name,
                network.clone(),
            )
            .await;

        storage_hub_builder.setup_storage_layer();
        sh_builder = Some(storage_hub_builder);
    }

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                command_sink: command_sink.clone(),
                deny_unsafe,
            };

            crate::rpc::create_full(deps).map_err(Into::into)
        })
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config,
        keystore: keystore_container.keystore(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    // Finish building the StorageHubBuilder if node is running as a Storage Provider.
    if let Some(provider_options) = provider_options {
        let mut storage_hub_builder =
            sh_builder.expect("StorageHubBuilder should already be initialised.");

        // Spawn the Blockchain Service if node is running as a Storage Provider, now that
        // the rpc handlers has been created.
        storage_hub_builder
            .with_blockchain(client.clone(), Arc::new(rpc_handlers), keystore.clone())
            .await;

        // Getting the caller pub key used for the blockchain service, from the keystore.
        // Then add it to the StorageHub builder.
        let caller_pub_key = BlockchainService::caller_pub_key(keystore).0;
        storage_hub_builder.with_provider_pub_key(caller_pub_key);

        // Finally build the StorageHubBuilder and start the Provider tasks.
        let sh_handler = storage_hub_builder.build();
        start_provider_tasks(provider_options, sh_handler);
    }

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        // Here you can check whether the hardware meets your chains' requirements. Putting a link
        // in there and swapping out the requirements for your own are probably a good idea. The
        // requirements for a para-chain are dictated by its relay-chain.
        match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench) {
            Err(err) if collator => {
                log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
				err
			);
            }
            _ => {}
        }

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    if collator {
        let proposer = sc_basic_authorship::ProposerFactory::with_proof_recording(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        // aura import queue
        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
        let client_for_cidp = client.clone();

        task_manager.spawn_essential_handle().spawn_blocking(
            "authorship_task",
            Some("block-authoring"),
            run_manual_seal(ManualSealParams {
                block_import: client.clone(),
                env: proposer,
                client: client.clone(),
                pool: transaction_pool.clone(),
                commands_stream,
                select_chain,
                consensus_data_provider: Some(Box::new(AuraConsensusDataProvider::new(
                    client.clone(),
                ))),
                create_inherent_data_providers: move |block: Hash, ()| {
                    let current_para_block = client_for_cidp
                    .number(block)
                    .expect("Header lookup should succeed")
                    .expect("Header passed in as parent should be present in backend.");

                    let hash = client
                        .hash(current_para_block.saturating_sub(1))
                        .expect("Hash of the desired block must be present")
                        .expect("Hash of the desired block should exist");

                    let para_header = client
                        .expect_header(hash)
                        .expect("Expected parachain header should exist")
                        .encode();

                    let para_head_data = HeadData(para_header).encode();

                    let client_for_xcm = client_for_cidp.clone();

                    let para_head_key = RelayChainWellKnownKeys::para_head(para_id);
                    let relay_slot_key = RelayChainWellKnownKeys::CURRENT_SLOT.to_vec();

                    async move {
                        let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

						let relay_slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

                        let additional_keys = vec![(para_head_key, para_head_data), (relay_slot_key, Slot::from(u64::from(*relay_slot)).encode())];

                        let mocked_parachain = {
                            MockValidationDataInherentDataProvider {
                                current_para_block,
                                relay_offset: 1000,
                                relay_blocks_per_para_block: 2,
                                para_blocks_per_relay_epoch: 0,
                                relay_randomness_config: (),
                                xcm_config: MockXcmConfig::new(
                                    &*client_for_xcm,
                                    block,
                                    Default::default(),
                                    Default::default(),
                                ),
                                raw_downward_messages: vec![],
                                raw_horizontal_messages: vec![],
                                additional_key_values: Some(additional_keys),
                            }
                        };

                        Ok((relay_slot, mocked_parachain, timestamp))
                    }
                },
            }),
        );
    }

    log::info!("Development Service Ready");

    network_starter.start_network();
    Ok(task_manager)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("StorageHub 💾")]
async fn start_node_impl<FL, FS>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    provider_options: Option<ProviderOptions>,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient>)>
where
    StorageHubBuilder<LayoutV1<BlakeTwo256>, FL, FS>: StorageLayerBuilder,
    FL: FileStorage<LayoutV1<BlakeTwo256>> + Send + Sync,
    FS: ForestStorage<LayoutV1<BlakeTwo256>> + Send + Sync + 'static,
    HasherOutT<LayoutV1<BlakeTwo256>>: TryFrom<[u8; 32]>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial(&parachain_config, false)?;
    let (block_import, mut telemetry, telemetry_worker_handle) = params.other;
    let mut net_config =
        sc_network::config::FullNetworkConfiguration::new(&parachain_config.network);

    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;
    let keystore = params.keystore_container.keystore();

    // If we are a provider we update the network configuration with the file transfer protocol.
    let mut file_transfer_request_protocol = None;
    if provider_options.is_some() {
        file_transfer_request_protocol = Some(configure_file_transfer_network(
            client.clone(),
            &parachain_config,
            &mut net_config,
        ));
    }

    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
        hwbench.clone(),
    )
    .await
    .map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

    let validator = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        build_network(BuildNetworkParams {
            parachain_config: &parachain_config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            para_id,
            spawn_handle: task_manager.spawn_handle(),
            relay_chain_interface: relay_chain_interface.clone(),
            import_queue: params.import_queue,
            sybil_resistance_level: CollatorSybilResistance::Resistant, // because of Aura
        })
        .await?;

    if parachain_config.offchain_worker.enabled {
        use futures::FutureExt;

        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-work",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                keystore: Some(params.keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: network.clone(),
                is_validator: parachain_config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    // If node is running as a Storage Provider, start building the StorageHubHandler using the StorageHubBuilder.
    let mut sh_builder = None;
    if provider_options.is_some() {
        // Start building the StorageHubHandler, if running as a provider.
        let task_spawner = TaskSpawner::new(task_manager.spawn_handle(), "generic");

        // Create builder for the StorageHubHandler.
        let mut storage_hub_builder =
            StorageHubBuilder::<LayoutV1<BlakeTwo256>, FL, FS>::new(task_spawner);

        // Add FileTransfer Service to the StorageHubHandler.
        let (file_transfer_request_protocol_name, file_transfer_request_receiver) =
            file_transfer_request_protocol
                .expect("FileTransfer request protocol should already be initialized.");
        storage_hub_builder
            .with_file_transfer(
                file_transfer_request_receiver,
                file_transfer_request_protocol_name,
                network.clone(),
            )
            .await;

        storage_hub_builder.setup_storage_layer();
        sh_builder = Some(storage_hub_builder);
    }

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                command_sink: None,
                deny_unsafe,
            };

            crate::rpc::create_full(deps).map_err(Into::into)
        })
    };

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.keystore(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    // Finish building the StorageHubBuilder if node is running as a Storage Provider.
    if let Some(provider_options) = provider_options {
        let mut storage_hub_builder =
            sh_builder.expect("StorageHubBuilder should already be initialised.");

        // Spawn the Blockchain Service if node is running as a Storage Provider, now that
        // the rpc handlers has been created.
        storage_hub_builder
            .with_blockchain(client.clone(), Arc::new(rpc_handlers), keystore.clone())
            .await;

        // Getting the caller pub key used for the blockchain service, from the keystore.
        // Then add it to the StorageHub builder.
        let caller_pub_key = BlockchainService::caller_pub_key(keystore).0;
        storage_hub_builder.with_provider_pub_key(caller_pub_key);

        // Finally build the StorageHubBuilder and start the Provider tasks.
        let sh_handler = storage_hub_builder.build();
        start_provider_tasks(provider_options, sh_handler);
    }

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        // Here you can check whether the hardware meets your chains' requirements. Putting a link
        // in there and swapping out the requirements for your own are probably a good idea. The
        // requirements for a para-chain are dictated by its relay-chain.
        match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench) {
            Err(err) if validator => {
                log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.",
				err
			);
            }
            _ => {}
        }

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    let announce_block = {
        let sync_service = sync_service.clone();
        Arc::new(move |hash, data| sync_service.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    start_relay_chain_tasks(StartRelayChainTasksParams {
        client: client.clone(),
        announce_block: announce_block.clone(),
        para_id,
        relay_chain_interface: relay_chain_interface.clone(),
        task_manager: &mut task_manager,
        da_recovery_profile: if validator {
            DARecoveryProfile::Collator
        } else {
            DARecoveryProfile::FullNode
        },
        import_queue: import_queue_service,
        relay_chain_slot_duration,
        recovery_handle: Box::new(overseer_handle.clone()),
        sync_service: sync_service.clone(),
    })?;

    if validator {
        start_consensus(
            client.clone(),
            backend.clone(),
            block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            sync_service.clone(),
            params.keystore_container.keystore(),
            relay_chain_slot_duration,
            para_id,
            collator_key.expect("Command line arguments do not allow this. qed"),
            overseer_handle,
            announce_block,
        )?;
    }

    network_starter.start_network();

    Ok((task_manager, client))
}

/// Build the import queue for the parachain runtime.
fn build_import_queue(
    client: Arc<ParachainClient>,
    block_import: ParachainBlockImport,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error> {
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

    Ok(
        cumulus_client_consensus_aura::equivocation_import_queue::fully_verifying_import_queue::<
            sp_consensus_aura::sr25519::AuthorityPair,
            _,
            _,
            _,
            _,
        >(
            client,
            block_import,
            move |_, _| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp,
                    slot_duration,
                );

                Ok((slot, timestamp))
            },
            slot_duration,
            &task_manager.spawn_essential_handle(),
            config.prometheus_registry(),
            telemetry,
        ),
    )
}

fn start_consensus(
    client: Arc<ParachainClient>,
    backend: Arc<ParachainBackend>,
    block_import: ParachainBlockImport,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient>>,
    sync_oracle: Arc<SyncingService<Block>>,
    keystore: KeystorePtr,
    relay_chain_slot_duration: Duration,
    para_id: ParaId,
    collator_key: CollatorPair,
    overseer_handle: OverseerHandle,
    announce_block: Arc<dyn Fn(Hash, Option<Vec<u8>>) + Send + Sync>,
) -> Result<(), sc_service::Error> {
    use cumulus_client_consensus_aura::collators::lookahead::{self as aura, Params as AuraParams};

    // NOTE: because we use Aura here explicitly, we can use `CollatorSybilResistance::Resistant`
    // when starting the network.

    let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
        task_manager.spawn_handle(),
        client.clone(),
        transaction_pool,
        prometheus_registry,
        telemetry.clone(),
    );

    let proposer = Proposer::new(proposer_factory);

    let collator_service = CollatorService::new(
        client.clone(),
        Arc::new(task_manager.spawn_handle()),
        announce_block,
        client.clone(),
    );

    let params = AuraParams {
        create_inherent_data_providers: move |_, ()| async move { Ok(()) },
        block_import,
        para_client: client.clone(),
        para_backend: backend.clone(),
        relay_client: relay_chain_interface,
        code_hash_provider: move |block_hash| {
            client
                .code_at(block_hash)
                .ok()
                .map(|c| ValidationCode::from(c).hash())
        },
        sync_oracle,
        keystore,
        collator_key,
        para_id,
        overseer_handle,
        relay_chain_slot_duration,
        proposer,
        collator_service,
        authoring_duration: Duration::from_millis(1500),
        reinitialize: false,
    };

    let fut =
        aura::run::<Block, sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _, _, _, _>(
            params,
        );
    task_manager
        .spawn_essential_handle()
        .spawn("aura", None, fut);

    Ok(())
}

/// Start a development node.
pub async fn start_dev_node(
    config: Configuration,
    provider_options: Option<ProviderOptions>,
    hwbench: Option<sc_sysinfo::HwBench>,
    para_id: ParaId,
    sealing: cli::Sealing,
) -> sc_service::error::Result<TaskManager> {
    match provider_options {
        Some(provider_options) => match provider_options.storage_layer {
            StorageLayer::Memory => {
                start_dev_impl::<
                    InMemoryFileStorage<LayoutV1<BlakeTwo256>>,
                    InMemoryForestStorage<LayoutV1<BlakeTwo256>>,
                >(config, Some(provider_options), hwbench, para_id, sealing)
                .await
            }
            StorageLayer::RocksDB => {
                start_dev_impl::<
                    // TODO: Change this to RocksDB File Storage once it is implemented.
                    InMemoryFileStorage<LayoutV1<BlakeTwo256>>,
                    RocksDBForestStorage<LayoutV1<BlakeTwo256>>,
                >(config, Some(provider_options), hwbench, para_id, sealing)
                .await
            }
        },
        None => {
            start_dev_impl::<
                InMemoryFileStorage<LayoutV1<BlakeTwo256>>,
                InMemoryForestStorage<LayoutV1<BlakeTwo256>>,
            >(config, None, hwbench, para_id, sealing)
            .await
        }
    }
}

/// Start a parachain node.
pub async fn start_parachain_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    provider_options: Option<ProviderOptions>,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient>)> {
    match provider_options {
        Some(provider_options) => match provider_options.storage_layer {
            StorageLayer::Memory => {
                start_node_impl::<
                    InMemoryFileStorage<LayoutV1<BlakeTwo256>>,
                    InMemoryForestStorage<LayoutV1<BlakeTwo256>>,
                >(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    para_id,
                    hwbench,
                )
                .await
            }
            StorageLayer::RocksDB => {
                start_node_impl::<
                    // TODO: Change this to RocksDB File Storage once it is implemented.
                    InMemoryFileStorage<LayoutV1<BlakeTwo256>>,
                    RocksDBForestStorage<LayoutV1<BlakeTwo256>>,
                >(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    para_id,
                    hwbench,
                )
                .await
            }
        },
        None => {
            // In this case, it is not really important the types used for the storage layer, as
            // the node will not run as a provider.
            start_node_impl::<
                InMemoryFileStorage<LayoutV1<BlakeTwo256>>,
                InMemoryForestStorage<LayoutV1<BlakeTwo256>>,
            >(
                parachain_config,
                polkadot_config,
                collator_options,
                None,
                para_id,
                hwbench,
            )
            .await
        }
    }
}

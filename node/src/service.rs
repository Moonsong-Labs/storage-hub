//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

// std
use futures::{Stream, StreamExt};
use log::{error, info};
use shc_blockchain_service::capacity_manager::CapacityConfig;
use shc_client::builder::{FishermanOptions, IndexerOptions};
use shc_indexer_db::DbPool;
use shc_indexer_service::spawn_indexer_service;
use std::{cell::RefCell, path::PathBuf, sync::Arc, time::Duration};

use async_channel::Receiver;
use chrono::Utc;
use codec::Encode;
use cumulus_client_cli::CollatorOptions;
use cumulus_client_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};

use polkadot_primitives::{BlakeTwo256, HashT, HeadData};
use sc_consensus_manual_seal::consensus::aura::AuraConsensusDataProvider;
use shc_actors_framework::actor::TaskSpawner;
use shc_common::{traits::StorageEnableRuntime, types::*};
use shc_rpc::StorageHubClientRpcConfig;
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::Slot;
use sp_core::H256;

// Local Runtime Types
use shp_opaque::{Block, Hash};
use shr_solochain_evm::{
    apis::RuntimeApi as SolochainEvmRuntimeApi, Runtime as SolochainEvmRuntime,
};
use storage_hub_runtime::{apis::RuntimeApi as ParachainRuntimeApi, Runtime as ParachainRuntime};

// Cumulus Imports
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;
use cumulus_client_service::{
    build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
    BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::{
    relay_chain::{well_known_keys as RelayChainWellKnownKeys, CollatorPair, ValidationCode},
    ParaId,
};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

// Substrate Imports
use cumulus_primitives_core::CollectCollationInfo;
use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use polkadot_primitives::UpgradeGoAhead;
use sc_client_api::Backend;
use sc_consensus::{ImportQueue, LongestChain};
use sc_executor::{HeapAllocStrategy, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{
    config::IncomingRequest, service::traits::NetworkService, NetworkBackend, NetworkBlock,
    ProtocolName,
};
use sc_service::{Configuration, PartialComponents, RpcHandlers, TFullBackend, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sc_transaction_pool_api::TransactionPool;
use shc_client::{
    builder::{Buildable, StorageHubBuilder, StorageLayerBuilder},
    handler::{RunnableTasks, StorageHubHandler},
    types::{
        BspProvider, FishermanRole, InMemoryStorageLayer, MspProvider, NoStorageLayer,
        RocksDbStorageLayer, ShNodeType, ShRole, ShStorageLayer, UserRole,
    },
};
use shc_file_transfer_service::configure_file_transfer_network;
use sp_api::ProvideRuntimeApi;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::SaturatedConversion;
use substrate_prometheus_endpoint::Registry;

// Frontier / EVM imports (solochain)
use fc_consensus::FrontierBlockImport;
use fc_db::{self, DatabaseSource};
use fc_storage::{self, StorageOverride, StorageOverrideHandler};
use sc_consensus_babe::ImportQueueParams as BabeImportQueueParams;
use sc_consensus_grandpa::{self, SharedVoterState};
use sc_transaction_pool::BasicPool;

use crate::{
    cli::{self, ProviderType, StorageLayer},
    command::ProviderOptions,
};

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                        Generic Types over Runtime                                             ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

// Generic client type over Runtime
pub(crate) type StorageEnableClient<Runtime> =
    shc_common::types::ParachainClient<<Runtime as StorageEnableRuntime>::RuntimeApi>;

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                        StorageHub Parachain Types                                             ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

// Other generic types
pub(crate) type StorageEnableBackend = TFullBackend<Block>;
pub(crate) type StorageEnableSelectChain = sc_consensus::LongestChain<StorageEnableBackend, Block>;

pub(crate) type StorageEnableBlockImport<Runtime> =
    TParachainBlockImport<Block, Arc<StorageEnableClient<Runtime>>, StorageEnableBackend>;

/// Assembly of PartialComponents (enough to run chain ops subcommands)
pub type Service<Runtime> = PartialComponents<
    StorageEnableClient<Runtime>,
    StorageEnableBackend,
    Option<StorageEnableSelectChain>,
    sc_consensus::DefaultImportQueue<Block>,
    sc_transaction_pool::TransactionPoolHandle<Block, StorageEnableClient<Runtime>>,
    (
        StorageEnableBlockImport<Runtime>,
        Option<Telemetry>,
        Option<TelemetryWorkerHandle>,
    ),
>;

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                      StorageHub Solochain EVM Types                                           ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

// Solochain EVM specific types
type SolochainClient =
    sc_service::TFullClient<Block, SolochainEvmRuntimeApi, shc_common::types::ParachainExecutor>;
type SolochainBackend = TFullBackend<Block>;
type SolochainSelectChain = sc_consensus::LongestChain<SolochainBackend, Block>;

type SolochainPool = sc_transaction_pool::TransactionPoolHandle<Block, SolochainClient>;

/// Partial components returned by the Solochain EVM `new_partial_solochain_evm` path.
type SolochainService = sc_service::PartialComponents<
    SolochainClient,
    SolochainBackend,
    SolochainSelectChain,
    sc_consensus::DefaultImportQueue<Block>,
    SolochainPool,
    (
        sc_consensus_babe::BabeBlockImport<
            Block,
            SolochainClient,
            FrontierBlockImport<
                Block,
                sc_consensus_grandpa::GrandpaBlockImport<
                    SolochainBackend,
                    Block,
                    SolochainClient,
                    SolochainSelectChain,
                >,
                SolochainClient,
            >,
        >,
        sc_consensus_grandpa::LinkHalf<Block, SolochainClient, SolochainSelectChain>,
        sc_consensus_babe::BabeLink<Block>,
        Arc<fc_db::Backend<Block, SolochainClient>>,
        Arc<dyn StorageOverride<Block>>,
        Option<Telemetry>,
    ),
>;

fn frontier_database_dir(config: &Configuration, path: &str) -> std::path::PathBuf {
    config
        .base_path
        .config_dir(config.chain_spec.id())
        .join("frontier")
        .join(path)
}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                   StorageHub Client Setup Utilities                                           ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

/// Helper function to setup database pool
async fn setup_database_pool(database_url: String) -> Result<DbPool, sc_service::Error> {
    shc_indexer_db::setup_db_pool(database_url)
        .await
        .map_err(|e| sc_service::Error::Application(Box::new(e)))
}

/// Configure and spawn the indexer service.
async fn configure_and_spawn_indexer<Runtime: StorageEnableRuntime>(
    indexer_options: &Option<IndexerOptions>,
    task_manager: &TaskManager,
    client: Arc<StorageEnableClient<Runtime>>,
) -> Result<Option<DbPool>, sc_service::Error> {
    let indexer_options = match indexer_options {
        Some(config) => config,
        None => return Ok(None),
    };

    // Setup database pool
    let db_pool = setup_database_pool(indexer_options.database_url.clone()).await?;

    info!(
        "📊 Starting Indexer service (mode: {:?})",
        indexer_options.indexer_mode
    );

    let task_spawner = TaskSpawner::new(task_manager.spawn_handle(), "indexer-service");
    spawn_indexer_service::<Runtime>(
        &task_spawner,
        client.clone(),
        db_pool.clone(),
        indexer_options.indexer_mode,
    )
    .await;

    Ok(Some(db_pool))
}

async fn configure_and_spawn_fisherman<Runtime: StorageEnableRuntime>(
    fisherman_options: &Option<FishermanOptions>,
    indexer_config: &Option<IndexerOptions>,
    task_manager: &TaskManager,
    client: Arc<StorageEnableClient<Runtime>>,
    keystore: KeystorePtr,
    rpc_handlers: Arc<RpcHandlers>,
    rocksdb_root_path: impl Into<PathBuf>,
    network: Arc<dyn NetworkService>,
) -> Result<Option<DbPool>, sc_service::Error> {
    let fisherman_options = match fisherman_options {
        Some(fc) => fc,
        None => return Ok(None),
    };

    // Validate configuration compatibility with indexer if both are enabled
    if let Some(indexer_cfg) = indexer_config {
        if indexer_cfg.indexer_mode == shc_indexer_service::IndexerMode::Lite {
            return Err(sc_service::Error::Other(
                "Fisherman service cannot run with 'lite' indexer mode. Please use either 'full' or 'fishing' mode."
                    .to_string(),
            ));
        }
    }

    // Setup database pool for fisherman
    let db_pool = setup_database_pool(fisherman_options.database_url.clone()).await?;

    // Build StorageHubHandler for fisherman tasks
    let task_spawner = TaskSpawner::new(task_manager.spawn_handle(), "fisherman-service");
    let mut fisherman_builder =
        StorageHubBuilder::<FishermanRole, NoStorageLayer, Runtime>::new(task_spawner.clone());

    // Convert rocksdb_root_path to PathBuf first
    let rocksdb_path: PathBuf = rocksdb_root_path.into();

    // Setup blockchain service
    fisherman_builder
        .with_blockchain(
            client.clone(),
            keystore,
            rpc_handlers,
            rocksdb_path.clone(),
            false, // Not in maintenance mode
        )
        .await;

    // Set the indexer db pool
    fisherman_builder.with_indexer_db_pool(Some(db_pool.clone()));

    // Spawn the fisherman service
    fisherman_builder.with_fisherman(client.clone()).await;

    // All variables below are not needed for the fisherman service to operate but required by the StorageHubHandler
    // TODO: Refactor this once we have a proper setup to support role based StorageHubHandler builder
    fisherman_builder.setup_storage_layer(None);
    fisherman_builder.with_peer_manager(rocksdb_path);
    let (_sender, receiver) = async_channel::bounded(1);
    let protocol_name = ProtocolName::from("/storage-hub/file-transfer/1");
    fisherman_builder
        .with_file_transfer(receiver, protocol_name, network)
        .await;

    // Build the handler
    let mut fisherman_handler = fisherman_builder.build();

    // Run fisherman tasks
    fisherman_handler.run_tasks().await;

    Ok(Some(db_pool))
}

/// Initialize the StorageHubBuilder for the StorageHub node.
async fn init_sh_builder<R, S, Runtime: StorageEnableRuntime>(
    provider_options: &Option<ProviderOptions>,
    task_manager: &TaskManager,
    file_transfer_request_protocol: Option<(ProtocolName, Receiver<IncomingRequest>)>,
    network: Arc<dyn NetworkService>,
    keystore: KeystorePtr,
    client: Arc<StorageEnableClient<Runtime>>,
    indexer_options: Option<IndexerOptions>,
) -> Result<
    Option<(
        StorageHubBuilder<R, S, Runtime>,
        StorageHubClientRpcConfig<
            <(R, S) as ShNodeType<Runtime>>::FL,
            <(R, S) as ShNodeType<Runtime>>::FSH,
            Runtime,
        >,
    )>,
    sc_service::Error,
>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType<Runtime>,
    StorageHubBuilder<R, S, Runtime>: StorageLayerBuilder,
{
    let maybe_indexer_db_pool =
        configure_and_spawn_indexer::<Runtime>(&indexer_options, &task_manager, client.clone())
            .await?;

    match provider_options {
        Some(ProviderOptions {
            rpc_config,
            provider_type,
            storage_path,
            max_storage_capacity,
            jump_capacity,
            msp_charging_period,
            msp_charge_fees,
            msp_move_bucket,
            bsp_upload_file,
            bsp_move_bucket,
            bsp_charge_fees,
            bsp_submit_proof,
            blockchain_service,
            ..
        }) => {
            info!(
                "Starting as a Storage Provider. Storage path: {:?}, Max storage capacity: {:?}, Jump capacity: {:?}, MSP charging period: {:?}",
                storage_path, max_storage_capacity, jump_capacity, msp_charging_period,
            );

            // Start building the StorageHubHandler, if running as a provider.
            let task_spawner = TaskSpawner::new(task_manager.spawn_handle(), "sh-builder");
            let mut storage_hub_builder = StorageHubBuilder::<R, S, Runtime>::new(task_spawner);

            // Setup and spawn the File Transfer Service.
            let (file_transfer_request_protocol_name, file_transfer_request_receiver) =
                file_transfer_request_protocol
                    .expect("FileTransfer request protocol should already be initialised.");

            storage_hub_builder
                .with_file_transfer(
                    file_transfer_request_receiver,
                    file_transfer_request_protocol_name,
                    network.clone(),
                )
                .await;

            // Setup the `ShStorageLayer` and additional configuration parameters.
            storage_hub_builder
                .setup_storage_layer(storage_path.clone())
                .with_capacity_config(Some(CapacityConfig::new(
                    max_storage_capacity.unwrap_or_default().saturated_into(),
                    jump_capacity.unwrap_or_default().saturated_into(),
                )));

            storage_hub_builder.with_msp_charge_fees_config(msp_charge_fees.clone());
            storage_hub_builder.with_msp_move_bucket_config(msp_move_bucket.clone());
            storage_hub_builder.with_bsp_upload_file_config(bsp_upload_file.clone());
            storage_hub_builder.with_bsp_move_bucket_config(bsp_move_bucket.clone());
            storage_hub_builder.with_bsp_charge_fees_config(bsp_charge_fees.clone());
            storage_hub_builder.with_bsp_submit_proof_config(bsp_submit_proof.clone());

            // Setup specific configuration for the MSP node.
            if *provider_type == ProviderType::Msp {
                storage_hub_builder
                    .with_notify_period(*msp_charging_period)
                    .with_indexer_db_pool(maybe_indexer_db_pool);
            }

            if let Some(c) = blockchain_service {
                storage_hub_builder.with_blockchain_service_config(c.clone());
            }

            // Get the RPC configuration to use for this StorageHub node client.
            let storage_hub_client_rpc_config =
                storage_hub_builder.create_rpc_config(keystore, rpc_config.clone());

            Ok(Some((storage_hub_builder, storage_hub_client_rpc_config)))
        }
        None => Ok(None),
    }
}

/// Finish the StorageHubBuilder and run the tasks.
async fn finish_sh_builder_and_run_tasks<R, S, Runtime: StorageEnableRuntime>(
    mut sh_builder: StorageHubBuilder<R, S, Runtime>,
    client: Arc<StorageEnableClient<Runtime>>,
    rpc_handlers: RpcHandlers,
    keystore: KeystorePtr,
    rocksdb_root_path: impl Into<PathBuf>,
    maintenance_mode: bool,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    task_manager: &TaskManager,
    network: Arc<dyn NetworkService>,
) -> Result<(), sc_service::Error>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType<Runtime>,
    StorageHubBuilder<R, S, Runtime>: StorageLayerBuilder + Buildable<(R, S), Runtime>,
    StorageHubHandler<(R, S), Runtime>: RunnableTasks,
{
    let rocks_db_path = rocksdb_root_path.into();

    // Spawn fisherman service if enabled
    configure_and_spawn_fisherman::<Runtime>(
        &fisherman_options,
        &indexer_options,
        &task_manager,
        client.clone(),
        keystore.clone(),
        Arc::new(rpc_handlers.clone()),
        rocks_db_path.clone(),
        network.clone(),
    )
    .await?;

    // Spawn the Blockchain Service if node is running as a Storage Provider
    sh_builder
        .with_blockchain(
            client.clone(),
            keystore.clone(),
            Arc::new(rpc_handlers),
            rocks_db_path.clone(),
            maintenance_mode,
        )
        .await;

    // Initialize the BSP peer manager
    sh_builder.with_peer_manager(rocks_db_path.clone());

    // Build the StorageHubHandler
    let mut sh_handler = sh_builder.build();

    // Run StorageHub tasks according to the node role
    sh_handler.run_tasks().await;

    Ok(())
}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                                 StorageHub Parachain Node Setup Functions                                     ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

/// Start the StorageHub Parachain node in development mode.
///
/// This is the entrypoint function to launch a StorageHub Parachain node,
/// when running in development mode.
pub async fn start_dev_parachain_node<Network: NetworkBackend<OpaqueBlock, BlockHash>>(
    config: Configuration,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    hwbench: Option<sc_sysinfo::HwBench>,
    para_id: ParaId,
    sealing: cli::Sealing,
) -> sc_service::error::Result<TaskManager> {
    if let Some(provider_options) = provider_options {
        match (
            &provider_options.provider_type,
            &provider_options.storage_layer,
        ) {
            (&ProviderType::Bsp, &StorageLayer::Memory) => {
                start_dev_parachain_impl::<BspProvider, InMemoryStorageLayer, Network>(
                    config,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    hwbench,
                    para_id,
                    sealing,
                )
                .await
            }
            (&ProviderType::Bsp, &StorageLayer::RocksDB) => {
                start_dev_parachain_impl::<BspProvider, RocksDbStorageLayer, Network>(
                    config,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    hwbench,
                    para_id,
                    sealing,
                )
                .await
            }
            (&ProviderType::Msp, &StorageLayer::Memory) => {
                start_dev_parachain_impl::<MspProvider, InMemoryStorageLayer, Network>(
                    config,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    hwbench,
                    para_id,
                    sealing,
                )
                .await
            }
            (&ProviderType::Msp, &StorageLayer::RocksDB) => {
                start_dev_parachain_impl::<MspProvider, RocksDbStorageLayer, Network>(
                    config,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    hwbench,
                    para_id,
                    sealing,
                )
                .await
            }
            (&ProviderType::User, _) => {
                start_dev_parachain_impl::<UserRole, NoStorageLayer, Network>(
                    config,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    hwbench,
                    para_id,
                    sealing,
                )
                .await
            }
        }
    } else {
        // Start node without provider options which in turn will not start any storage hub related role services (e.g. Storage Provider, User)
        start_dev_parachain_impl::<UserRole, NoStorageLayer, Network>(
            config,
            None,
            indexer_options,
            fisherman_options,
            hwbench,
            para_id,
            sealing,
        )
        .await
    }
}

/// Start the StorageHub Parachain node.
///
/// This is the entrypoint function to launch a StorageHub Parachain node.
pub async fn start_parachain_node<Network: NetworkBackend<OpaqueBlock, BlockHash>>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<StorageEnableClient<ParachainRuntime>>)> {
    if let Some(provider_options) = provider_options {
        match (
            &provider_options.provider_type,
            &provider_options.storage_layer,
        ) {
            (&ProviderType::Bsp, &StorageLayer::Memory) => {
                start_parachain_node_impl::<BspProvider, InMemoryStorageLayer, Network>(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    para_id,
                    hwbench,
                )
                .await
            }
            (&ProviderType::Bsp, &StorageLayer::RocksDB) => {
                start_parachain_node_impl::<BspProvider, RocksDbStorageLayer, Network>(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    para_id,
                    hwbench,
                )
                .await
            }
            (&ProviderType::Msp, &StorageLayer::Memory) => {
                start_parachain_node_impl::<MspProvider, InMemoryStorageLayer, Network>(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    para_id,
                    hwbench,
                )
                .await
            }
            (&ProviderType::Msp, &StorageLayer::RocksDB) => {
                start_parachain_node_impl::<MspProvider, RocksDbStorageLayer, Network>(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    para_id,
                    hwbench,
                )
                .await
            }
            (&ProviderType::User, _) => {
                start_parachain_node_impl::<UserRole, NoStorageLayer, Network>(
                    parachain_config,
                    polkadot_config,
                    collator_options,
                    Some(provider_options),
                    indexer_options,
                    fisherman_options,
                    para_id,
                    hwbench,
                )
                .await
            }
        }
    } else {
        // Start node without provider options which in turn will not start any storage hub related role services (e.g. Storage Provider, User)
        start_parachain_node_impl::<UserRole, NoStorageLayer, Network>(
            parachain_config,
            polkadot_config,
            collator_options,
            None,
            indexer_options,
            fisherman_options,
            para_id,
            hwbench,
        )
        .await
    }
}

/// Create a new partial components for the StorageHub Parachain node.
///
/// This is the entrypoint function used when executing subcommands of the
/// StorageHub Parachain node.
pub fn new_partial_parachain(
    config: &Configuration,
    dev_service: bool,
) -> Result<Service<ParachainRuntime>, sc_service::Error> {
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
        .executor
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = ParachainExecutor::builder()
        .with_execution_method(config.executor.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.executor.max_runtime_instances)
        .with_runtime_cache_size(config.executor.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts_record_import::<Block, ParachainRuntimeApi, _>(
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

    // FIXME: The `config.transaction_pool.options` field is private, so for now use its default value
    // let transaction_pool = Arc::from(BasicPool::new_full(
    //     Default::default(),
    //     config.role.is_authority().into(),
    //     config.prometheus_registry(),
    //     task_manager.spawn_essential_handle(),
    //     client.clone(),
    // ));

    let transaction_pool = Arc::from(
        sc_transaction_pool::Builder::new(
            task_manager.spawn_essential_handle(),
            client.clone(),
            config.role.is_authority().into(),
        )
        .with_options(config.transaction_pool.clone())
        .with_prometheus(config.prometheus_registry())
        .build(),
    );

    let block_import =
        StorageEnableBlockImport::<ParachainRuntime>::new(client.clone(), backend.clone());

    let import_queue = if dev_service {
        sc_consensus_manual_seal::import_queue(
            Box::new(client.clone()),
            &task_manager.spawn_essential_handle(),
            config.prometheus_registry(),
        )
    } else {
        build_parachain_import_queue(
            client.clone(),
            block_import.clone(),
            config,
            telemetry.as_ref().map(|telemetry| telemetry.handle()),
            &task_manager,
        )
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

/// Start a development node with the given solo chain `Configuration`.
async fn start_dev_parachain_impl<R, S, Network>(
    config: Configuration,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    hwbench: Option<sc_sysinfo::HwBench>,
    para_id: ParaId,
    sealing: cli::Sealing,
) -> sc_service::error::Result<TaskManager>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType<ParachainRuntime>,
    StorageHubBuilder<R, S, ParachainRuntime>:
        StorageLayerBuilder + Buildable<(R, S), ParachainRuntime>,
    StorageHubHandler<(R, S), ParachainRuntime>: RunnableTasks,
    Network: sc_network::NetworkBackend<OpaqueBlock, BlockHash>,
{
    use async_io::Timer;
    use sc_consensus_manual_seal::{run_manual_seal, EngineCommand, ManualSealParams};

    // Check if we're in maintenance mode and build the dev node in maintenance mode if so
    let maintenance_mode = provider_options
        .as_ref()
        .map_or(false, |opts| opts.maintenance_mode);
    if maintenance_mode {
        log::info!("🛠️  Running dev node in maintenance mode");
        log::info!("🛠️  Network participation is disabled");
        log::info!("🛠️  Only storage management RPC methods are available");
        return start_dev_parachain_in_maintenance_mode::<R, S, Network>(
            config,
            provider_options,
            indexer_options,
            fisherman_options,
            hwbench,
        )
        .await;
    }

    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain: maybe_select_chain,
        transaction_pool,
        other: (_, mut telemetry, _),
    } = new_partial_parachain(&config, true)?;

    let signing_dev_key = config
        .dev_key_seed
        .clone()
        .expect("Dev key seed must be present in dev mode.");
    let keystore = keystore_container.keystore();

    // Initialise seed for signing transactions using blockchain service.
    // In dev mode we use a well known dev account.
    keystore
        .sr25519_generate_new(BCSV_KEY_TYPE, Some(signing_dev_key.as_ref()))
        .expect("Invalid dev signing key provided.");

    let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, Network>::new(
        &config.network,
        config
            .prometheus_config
            .as_ref()
            .map(|cfg| cfg.registry.clone()),
    );
    let collator = config.role.is_authority();
    let prometheus_registry = config.prometheus_registry().cloned();
    let select_chain = maybe_select_chain
        .expect("In `dev` mode, `new_partial` will return some `select_chain`; qed");

    // If we are a provider or fisherman we update the network configuration with the file transfer protocol.
    let mut file_transfer_request_protocol = None;
    if provider_options.is_some() || fisherman_options.is_some() {
        file_transfer_request_protocol =
            Some(configure_file_transfer_network::<_, ParachainRuntime>(
                client.clone(),
                &config,
                &mut net_config,
            ));
    }

    let metrics = Network::register_notification_metrics(
        config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,
            block_relay: None,
            metrics,
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
                network_provider: Arc::new(network.clone()),
                is_validator: config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })?
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
                    transaction_pool.import_notification_stream().map(|_| {
                        EngineCommand::SealNewBlock {
                            create_empty: false,
                            finalize: false,
                            parent_hash: None,
                            sender: None,
                        }
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
    let (sh_builder, maybe_storage_hub_client_rpc_config) =
        match init_sh_builder::<R, S, ParachainRuntime>(
            &provider_options,
            &task_manager,
            file_transfer_request_protocol,
            network.clone(),
            keystore.clone(),
            client.clone(),
            indexer_options.clone(),
        )
        .await?
        {
            Some((shb, rpc)) => (Some(shb), Some(rpc)),
            None => (None, None),
        };

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |_| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                maybe_storage_hub_client_config: maybe_storage_hub_client_rpc_config.clone(),
                command_sink: command_sink.clone(),
            };

            crate::rpc::create_full::<_, _, _, ParachainRuntime>(deps).map_err(Into::into)
        })
    };

    let base_path = config.base_path.path().to_path_buf().clone();

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config,
        keystore: keystore.clone(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    // Finish building the StorageHubBuilder if node is running as a Storage Provider.
    if let Some(_) = provider_options {
        finish_sh_builder_and_run_tasks(
            sh_builder.expect("StorageHubBuilder should already be initialised."),
            client.clone(),
            rpc_handlers,
            keystore.clone(),
            base_path,
            maintenance_mode,
            indexer_options,
            fisherman_options,
            &task_manager,
            network.clone(),
        )
        .await?;
    }

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        // Here you can check whether the hardware meets your chains' requirements. Putting a link
        // in there and swapping out the requirements for your own are probably a good idea. The
        // requirements for a para-chain are dictated by its relay-chain.
        match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench, false) {
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

    thread_local!(static TIMESTAMP: RefCell<u64> = RefCell::new(Utc::now().timestamp_millis().try_into().unwrap()));

    /// Provide a mock duration starting at Utc::now() in millisecond for timestamp inherent.
    /// Each call will increment timestamp by slot_duration making Aura think time has passed.
    struct MockTimestampInherentDataProvider;

    #[async_trait::async_trait]
    impl sp_inherents::InherentDataProvider for MockTimestampInherentDataProvider {
        async fn provide_inherent_data(
            &self,
            inherent_data: &mut sp_inherents::InherentData,
        ) -> Result<(), sp_inherents::Error> {
            TIMESTAMP.with(|x| {
                *x.borrow_mut() += storage_hub_runtime::SLOT_DURATION;
                inherent_data.put_data(sp_timestamp::INHERENT_IDENTIFIER, &*x.borrow())
            })
        }

        async fn try_handle_error(
            &self,
            _identifier: &sp_inherents::InherentIdentifier,
            _error: &[u8],
        ) -> Option<Result<(), sp_inherents::Error>> {
            // The pallet never reports error.
            None
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

                    let current_para_head = client_for_cidp
                        .header(block)
                        .expect("Header lookup should succeed")
                        .expect("Header passed in as parent should be present in backend.");

                    let should_send_go_ahead = match client_for_cidp
                        .runtime_api()
                        .collect_collation_info(block, &current_para_head)
                        {
                            Ok(info) => info.new_validation_code.is_some(),
                            Err(e) => {
                                error!("Failed to collect collation info: {:?}", e);
                                false
                            },
                        };

					let raw_para_head_data = HeadData(para_header);
					let para_head_data = raw_para_head_data.encode();

                    let client_for_xcm = client_for_cidp.clone();

                    let para_head_key = RelayChainWellKnownKeys::para_head(para_id);
                    let relay_slot_key = RelayChainWellKnownKeys::CURRENT_SLOT.to_vec();
                    let current_block_randomness_key = RelayChainWellKnownKeys::CURRENT_BLOCK_RANDOMNESS.to_vec();

                    async move {
                        let mut timestamp = 0u64;
                        // This allows us to create multiple blocks without considering the actual slot duration wait time. We increment the timestamp by slot_duration in inherent data.
                        TIMESTAMP.with(|x| {
                            timestamp = x.clone().take();
                        });

                        // If we don't increment the timestamp, we will hit a para slot and relay slot mismatch.
                        timestamp += storage_hub_runtime::SLOT_DURATION;

						let relay_slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							timestamp.into(),
							slot_duration,
						);

                        let current_block_randomness = BlakeTwo256::hash(timestamp.encode().as_slice());

                        let additional_keys = vec![
                            (para_head_key, para_head_data),
                            (relay_slot_key, Slot::from(u64::from(*relay_slot)).encode()),
                            (current_block_randomness_key, current_block_randomness.encode())
                        ];

                        let time = MockTimestampInherentDataProvider;

                        let mocked_parachain = {
                            MockValidationDataInherentDataProvider {
                                current_para_block,
								para_id,
								current_para_block_head: Some(raw_para_head_data),
                                relay_offset: 1000,
                                relay_blocks_per_para_block: 2,
                                para_blocks_per_relay_epoch: 0,
                                relay_randomness_config: (),
                                xcm_config: MockXcmConfig::new(
                                    &*client_for_xcm,
                                    block,
                                    Default::default(),
                                ),
                                raw_downward_messages: vec![],
                                raw_horizontal_messages: vec![],
                                additional_key_values: Some(additional_keys),
                                upgrade_go_ahead: should_send_go_ahead.then(|| {
                                    log::info!(
                                        "Detected pending validation code, sending go-ahead signal."
                                    );
                                    UpgradeGoAhead::GoAhead
                                }),
                            }
                        };

                        Ok((relay_slot, mocked_parachain, time))
                    }
                },
            }),
        );
    }

    log::info!("Development Service Ready");

    network_starter.start_network();
    Ok(task_manager)
}

async fn start_dev_parachain_in_maintenance_mode<R, S, Network>(
    config: Configuration,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<TaskManager>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType<ParachainRuntime>,
    StorageHubBuilder<R, S, ParachainRuntime>:
        StorageLayerBuilder + Buildable<(R, S), ParachainRuntime>,
    StorageHubHandler<(R, S), ParachainRuntime>: RunnableTasks,
    Network: sc_network::NetworkBackend<OpaqueBlock, BlockHash>,
{
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain: _maybe_select_chain,
        transaction_pool,
        other: (_, mut telemetry, _),
    } = new_partial_parachain(&config, true)?;

    let signing_dev_key = config
        .dev_key_seed
        .clone()
        .expect("Dev key seed must be present in dev mode.");
    let keystore = keystore_container.keystore();

    // Initialise seed for signing transactions using blockchain service.
    // In dev mode we use a well known dev account.
    keystore
        .sr25519_generate_new(BCSV_KEY_TYPE, Some(signing_dev_key.as_ref()))
        .expect("Invalid dev signing key provided.");

    let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, Network>::new(
        &config.network,
        config
            .prometheus_config
            .as_ref()
            .map(|cfg| cfg.registry.clone()),
    );

    // If we are a provider or fisherman we update the network configuration with the file transfer protocol.
    let mut file_transfer_request_protocol = None;
    if provider_options.is_some() || fisherman_options.is_some() {
        file_transfer_request_protocol =
            Some(configure_file_transfer_network::<_, ParachainRuntime>(
                client.clone(),
                &config,
                &mut net_config,
            ));
    }

    let metrics = Network::register_notification_metrics(
        config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,
            block_relay: None,
            metrics,
        })?;

    // No offchain workers in maintenance mode - intentionally omitted

    // Create command_sink for RPC
    let (command_sink, _) = futures::channel::mpsc::channel(1000);

    // If node is running as a Storage Provider, start building the StorageHubHandler using the StorageHubBuilder.
    let (sh_builder, maybe_storage_hub_client_rpc_config) =
        match init_sh_builder::<R, S, ParachainRuntime>(
            &provider_options,
            &task_manager,
            file_transfer_request_protocol,
            network.clone(),
            keystore.clone(),
            client.clone(),
            indexer_options.clone(),
        )
        .await?
        {
            Some((shb, rpc)) => (Some(shb), Some(rpc)),
            None => (None, None),
        };

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |_| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                maybe_storage_hub_client_config: maybe_storage_hub_client_rpc_config.clone(),
                command_sink: Some(command_sink.clone()),
            };

            crate::rpc::create_full::<_, _, _, ParachainRuntime>(deps).map_err(Into::into)
        })
    };

    let base_path = config.base_path.path().to_path_buf().clone();

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config,
        keystore: keystore.clone(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    // Finish building the StorageHubBuilder if node is running as a Storage Provider.
    if let Some(_) = provider_options {
        finish_sh_builder_and_run_tasks(
            sh_builder.expect("StorageHubBuilder should already be initialised."),
            client.clone(),
            rpc_handlers,
            keystore.clone(),
            base_path,
            true,
            indexer_options,
            fisherman_options,
            &task_manager,
            network.clone(),
        )
        .await?;
    }

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    // In maintenance mode, we intentionally don't start the manual sealing process
    // This means no block production will occur
    log::info!("🛠️  Dev node started in maintenance mode - block production is disabled");
    log::info!("🛠️  Manual sealing is disabled");
    log::info!("🛠️  Only RPC functionality is available");

    network_starter.start_network();
    Ok(task_manager)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
async fn start_parachain_node_impl<R, S, Network>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<StorageEnableClient<ParachainRuntime>>)>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType<ParachainRuntime>,
    StorageHubBuilder<R, S, ParachainRuntime>:
        StorageLayerBuilder + Buildable<(R, S), ParachainRuntime>,
    StorageHubHandler<(R, S), ParachainRuntime>: RunnableTasks,
    Network: NetworkBackend<OpaqueBlock, BlockHash>,
{
    // Check if we're in maintenance mode and build the node in maintenance mode if so
    let maintenance_mode = provider_options
        .as_ref()
        .map_or(false, |opts| opts.maintenance_mode);
    if maintenance_mode {
        log::info!("🛠️  Running dev node in maintenance mode");
        log::info!("🛠️  Network participation is disabled");
        log::info!("🛠️  Only storage management RPC methods are available");
        return start_parachain_node_in_maintenance_mode::<R, S, Network>(
            parachain_config,
            polkadot_config,
            collator_options,
            provider_options,
            indexer_options,
            fisherman_options,
            para_id,
            hwbench,
        )
        .await;
    }

    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial_parachain(&parachain_config, false)?;
    let (block_import, mut telemetry, telemetry_worker_handle) = params.other;
    let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, Network>::new(
        &parachain_config.network,
        parachain_config
            .prometheus_config
            .as_ref()
            .map(|cfg| cfg.registry.clone()),
    );

    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;
    let keystore = params.keystore_container.keystore();

    // If we are a provider we update the network configuration with the file transfer protocol.
    let mut file_transfer_request_protocol = None;
    if provider_options.is_some() {
        file_transfer_request_protocol =
            Some(configure_file_transfer_network::<_, ParachainRuntime>(
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
    .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

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
                network_provider: Arc::new(network.clone()),
                is_validator: parachain_config.role.is_authority(),
                enable_http_requests: false,
                custom_extensions: move |_| vec![],
            })?
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    // If node is running as a Storage Provider, start building the StorageHubHandler using the StorageHubBuilder.
    let (sh_builder, maybe_storage_hub_client_rpc_config) =
        match init_sh_builder::<R, S, ParachainRuntime>(
            &provider_options,
            &task_manager,
            file_transfer_request_protocol,
            network.clone(),
            keystore.clone(),
            client.clone(),
            indexer_options.clone(),
        )
        .await?
        {
            Some((shb, rpc)) => (Some(shb), Some(rpc)),
            None => (None, None),
        };

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |_| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                maybe_storage_hub_client_config: maybe_storage_hub_client_rpc_config.clone(),
                command_sink: None,
            };

            crate::rpc::create_full::<_, _, _, ParachainRuntime>(deps).map_err(Into::into)
        })
    };

    let base_path = parachain_config.base_path.path().to_path_buf().clone();

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: keystore.clone(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    // Finish building the StorageHubBuilder if node is running as a Storage Provider.
    if let Some(_) = provider_options {
        finish_sh_builder_and_run_tasks(
            sh_builder.expect("StorageHubBuilder should already be initialised."),
            client.clone(),
            rpc_handlers,
            keystore.clone(),
            base_path,
            maintenance_mode,
            indexer_options,
            fisherman_options,
            &task_manager,
            network.clone(),
        )
        .await?;
    }

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        // Here you can check whether the hardware meets your chains' requirements. Putting a link
        // in there and swapping out the requirements for your own are probably a good idea. The
        // requirements for a para-chain are dictated by its relay-chain.
        match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench, false) {
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
        start_parachain_consensus(
            client.clone(),
            backend.clone(),
            block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
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

async fn start_parachain_node_in_maintenance_mode<R, S, Network>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<StorageEnableClient<ParachainRuntime>>)>
where
    R: ShRole,
    S: ShStorageLayer,
    (R, S): ShNodeType<ParachainRuntime>,
    StorageHubBuilder<R, S, ParachainRuntime>:
        StorageLayerBuilder + Buildable<(R, S), ParachainRuntime>,
    StorageHubHandler<(R, S), ParachainRuntime>: RunnableTasks,
    Network: NetworkBackend<OpaqueBlock, BlockHash>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial_parachain(&parachain_config, false)?;
    let (_block_import, mut telemetry, telemetry_worker_handle) = params.other;

    // Create network configuration
    let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, Network>::new(
        &parachain_config.network,
        parachain_config
            .prometheus_config
            .as_ref()
            .map(|cfg| cfg.registry.clone()),
    );

    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;
    let keystore = params.keystore_container.keystore();

    // If we are a provider we update the network configuration with the file transfer protocol.
    let mut file_transfer_request_protocol = None;
    if provider_options.is_some() {
        file_transfer_request_protocol =
            Some(configure_file_transfer_network::<_, ParachainRuntime>(
                client.clone(),
                &parachain_config,
                &mut net_config,
            ));
    }

    // Create relay chain interface
    let (relay_chain_interface, _collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
        hwbench.clone(),
    )
    .await
    .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    let transaction_pool = params.transaction_pool.clone();

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

    // No need for offchain workers in maintenance mode

    // If node is running as a Storage Provider, start building the StorageHubHandler using the StorageHubBuilder.
    let (sh_builder, maybe_storage_hub_client_rpc_config) =
        match init_sh_builder::<R, S, ParachainRuntime>(
            &provider_options,
            &task_manager,
            file_transfer_request_protocol,
            network.clone(),
            keystore.clone(),
            client.clone(),
            indexer_options.clone(),
        )
        .await?
        {
            Some((shb, rpc)) => (Some(shb), Some(rpc)),
            None => (None, None),
        };

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |_| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                maybe_storage_hub_client_config: maybe_storage_hub_client_rpc_config.clone(),
                command_sink: None,
            };

            crate::rpc::create_full::<_, _, _, ParachainRuntime>(deps).map_err(Into::into)
        })
    };

    let base_path = parachain_config.base_path.path().to_path_buf().clone();

    let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: keystore.clone(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    // Finish building the StorageHubBuilder if node is running as a Storage Provider.
    if let Some(_) = provider_options {
        finish_sh_builder_and_run_tasks(
            sh_builder.expect("StorageHubBuilder should already be initialised."),
            client.clone(),
            rpc_handlers,
            keystore.clone(),
            base_path,
            true,
            indexer_options,
            fisherman_options,
            &task_manager,
            network.clone(),
        )
        .await?;
    }

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    // In maintenance mode, we don't need the relay chain tasks
    log::info!("🛠️  Skipping relay chain tasks initialization in maintenance mode");
    log::info!("🛠️  Block import and relay chain sync are disabled");

    // We still need to start the network to allow RPC connections
    network_starter.start_network();

    log::info!("🛠️  Node started in maintenance mode - only RPC functionality is available");

    Ok((task_manager, client))
}

/// Build the import queue for the parachain runtime.
fn build_parachain_import_queue(
    client: Arc<StorageEnableClient<ParachainRuntime>>,
    block_import: StorageEnableBlockImport<ParachainRuntime>,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> sc_consensus::DefaultImportQueue<Block> {
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

            Ok(timestamp)
        },
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
        telemetry,
    )
}

fn start_parachain_consensus(
    client: Arc<StorageEnableClient<ParachainRuntime>>,
    backend: Arc<StorageEnableBackend>,
    block_import: StorageEnableBlockImport<ParachainRuntime>,
    prometheus_registry: Option<&Registry>,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
    relay_chain_interface: Arc<dyn RelayChainInterface>,
    transaction_pool: Arc<
        sc_transaction_pool::TransactionPoolHandle<Block, StorageEnableClient<ParachainRuntime>>,
    >,
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
        keystore,
        collator_key,
        para_id,
        overseer_handle,
        relay_chain_slot_duration,
        proposer,
        collator_service,
        authoring_duration: Duration::from_millis(2000),
        reinitialize: false,
    };

    let fut = aura::run::<Block, sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _, _, _>(
        params,
    );
    task_manager
        .spawn_essential_handle()
        .spawn("aura", None, fut);

    Ok(())
}

//╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╗
//║                               StorageHub Solochain EVM Node Setup Functions                                   ║
//╚═══════════════════════════════════════════════════════════════════════════════════════════════════════════════╝

/// Start the StorageHub Solochain EVM node in development mode.
///
/// This is the entrypoint function to launch a StorageHub Solochain EVM node,
/// when running in development mode.
pub async fn start_dev_solochain_evm_node<Network: NetworkBackend<OpaqueBlock, BlockHash>>(
    config: Configuration,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    hwbench: Option<sc_sysinfo::HwBench>,
    para_id: ParaId,
    sealing: cli::Sealing,
) -> sc_service::error::Result<TaskManager> {
    todo!("Not implemented")
}

/// Start the StorageHub Solochain EVM node.
///
/// This is the entrypoint function to launch a StorageHub Solochain EVM node.
pub async fn start_solochain_evm_node<Network: NetworkBackend<OpaqueBlock, BlockHash>>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    provider_options: Option<ProviderOptions>,
    indexer_options: Option<IndexerOptions>,
    fisherman_options: Option<FishermanOptions>,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<StorageEnableClient<ParachainRuntime>>)> {
    todo!("Not implemented")
}

/// Create a new partial components for the StorageHub Solochain EVM node.
///
/// This is the entrypoint function used when executing subcommands of the
/// StorageHub Solochain EVM node.
pub fn new_partial_solochain_evm(
    config: &Configuration,
    dev_service: bool,
) -> Result<SolochainService, sc_service::Error> {
    // Telemetry
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

    // Wasm executor (reuse ParachainExecutor host functions)
    let heap_pages = config
        .executor
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static {
            extra_pages: h as _,
        });

    let executor = shc_common::types::ParachainExecutor::builder()
        .with_execution_method(config.executor.wasm_method)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .with_max_runtime_instances(config.executor.max_runtime_instances)
        .with_runtime_cache_size(config.executor.runtime_cache_size)
        .build();

    let (client, backend, keystore_container, mut task_manager) =
        sc_service::new_full_parts::<Block, SolochainEvmRuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;

    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    // Transaction pool (use builder to honor config)
    let transaction_pool = Arc::from(
        sc_transaction_pool::Builder::new(
            task_manager.spawn_essential_handle(),
            client.clone(),
            config.role.is_authority().into(),
        )
        .with_options(config.transaction_pool.clone())
        .with_prometheus(config.prometheus_registry())
        .build(),
    );

    // GRANDPA block import and link
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        512u32, // justification period
        &client,
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    // Frontier block import on top of GRANDPA
    let frontier_block_import =
        FrontierBlockImport::new(grandpa_block_import.clone(), client.clone());

    // BABE block import and link
    let (block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::configuration(&*client)?,
        frontier_block_import,
        client.clone(),
    )?;

    // Frontier storage override and backend
    let storage_override: Arc<dyn StorageOverride<Block>> =
        Arc::new(StorageOverrideHandler::<Block, _, _>::new(client.clone()));

    let frontier_backend: Arc<fc_db::Backend<Block, SolochainClient>> = {
        // Only Key-Value backend for now
        let db_settings = match config.database {
            sc_service::config::DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
                path: frontier_database_dir(config, "db"),
                cache_size: 0,
            },
            sc_service::config::DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
                path: frontier_database_dir(config, "paritydb"),
            },
            sc_service::config::DatabaseSource::Auto { .. } => DatabaseSource::Auto {
                rocksdb_path: frontier_database_dir(config, "db"),
                paritydb_path: frontier_database_dir(config, "paritydb"),
                cache_size: 0,
            },
            _ => DatabaseSource::RocksDb {
                path: frontier_database_dir(config, "db"),
                cache_size: 0,
            },
        };
        Arc::new(fc_db::Backend::KeyValue(Arc::new(fc_db::kv::Backend::<
            Block,
            SolochainClient,
        >::new(
            client.clone(),
            &fc_db::kv::DatabaseSettings {
                source: db_settings,
            },
        )?)))
    };

    // Import queue (BABE)
    let slot_duration = babe_link.config().slot_duration();
    let (import_queue, _babe_worker_handle) = sc_consensus_babe::import_queue(
        BabeImportQueueParams {
            link: babe_link.clone(),
            block_import: block_import.clone(),
            justification_import: Some(Box::new(grandpa_block_import.clone())),
            client: client.clone(),
            select_chain: select_chain.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
                Ok((slot, timestamp))
            },
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
        },
    )?;

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (
            block_import,
            grandpa_link,
            babe_link,
            frontier_backend,
            storage_override,
            telemetry,
        ),
    })
}

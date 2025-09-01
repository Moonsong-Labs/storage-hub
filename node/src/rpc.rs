//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use fc_rpc::TxPool;
use sc_consensus_manual_seal::{
    rpc::{ManualSeal, ManualSealApiServer},
    EngineCommand,
};
use sc_network_sync::SyncingService;
use sc_rpc::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use shc_client::types::FileStorageT;
use shc_common::{traits::StorageEnableRuntime, types::ParachainClient};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_rpc::{StorageHubClientApiServer, StorageHubClientRpc, StorageHubClientRpcConfig};
use shr_solochain_evm::configs::time;
use sp_api::ProvideRuntimeApi;
use sp_core::H256;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<P, FL, FS, Runtime>
where
    Runtime: StorageEnableRuntime,
{
    /// The client instance to use.
    pub client: Arc<ParachainClient<Runtime::RuntimeApi>>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// RPC configuration.
    pub maybe_storage_hub_client_config: Option<StorageHubClientRpcConfig<FL, FS, Runtime>>,
    /// Manual seal command sink
    pub command_sink: Option<futures::channel::mpsc::Sender<EngineCommand<H256>>>,
}

/// Instantiate all RPC extensions for the parachain runtime.
pub fn create_full_parachain<P, FL, FSH, Runtime>(
    deps: FullDeps<P, FL, FSH, Runtime>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    Runtime: StorageEnableRuntime,
    P: TransactionPool + Send + Sync + 'static,
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Send + Sync + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        maybe_storage_hub_client_config,
        command_sink,
    } = deps;

    io.merge(System::new(client.clone(), pool).into_rpc())?;
    io.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    if let Some(storage_hub_client_config) = maybe_storage_hub_client_config {
        io.merge(
            StorageHubClientRpc::<FL, FSH, Runtime, shc_common::types::OpaqueBlock>::new(
                client,
                storage_hub_client_config,
            )
            .into_rpc(),
        )?;
    }

    if let Some(command_sink) = command_sink {
        io.merge(
            // We provide the rpc handler with the sending end of the channel to allow the rpc
            // send EngineCommands to the background block authorship task.
            ManualSeal::new(command_sink).into_rpc(),
        )?;
    };

    // Deny unsafe RPCs.
    io.extensions_mut().insert(DenyUnsafe::Yes);

    Ok(io)
}

/// Deps for the solochain-evm RPC constructor (includes Frontier/EVM deps)
pub struct SolochainEvmDeps<P, FL, FS, Runtime, A>
where
    Runtime: StorageEnableRuntime,
    A: sc_transaction_pool::ChainApi<Block = shc_common::types::OpaqueBlock>,
{
    pub client: Arc<ParachainClient<Runtime::RuntimeApi>>,
    pub pool: Arc<P>,
    pub maybe_storage_hub_client_config: Option<StorageHubClientRpcConfig<FL, FS, Runtime>>,
    pub command_sink: Option<futures::channel::mpsc::Sender<EngineCommand<H256>>>,

    // Frontier deps
    pub network: Arc<dyn sc_network::service::traits::NetworkService>,
    pub sync: Arc<SyncingService<shc_common::types::OpaqueBlock>>,
    pub overrides: Arc<dyn fc_storage::StorageOverride<shc_common::types::OpaqueBlock>>,
    pub frontier_backend: Arc<dyn fc_api::Backend<shc_common::types::OpaqueBlock>>,
    pub graph: Arc<sc_transaction_pool::Pool<A>>,
    pub block_data_cache: Arc<fc_rpc::EthBlockDataCacheTask<shc_common::types::OpaqueBlock>>,
    pub filter_pool: Option<fc_rpc_core::types::FilterPool>,
    pub fee_history_cache: fc_rpc_core::types::FeeHistoryCache,
    pub fee_history_limit: u64,
    pub max_past_logs: u32,
    pub forced_parent_hashes: Option<std::collections::BTreeMap<H256, H256>>,
    pub is_authority: bool,
}

pub fn create_full_solochain_evm<P, FL, FSH, Runtime, A>(
    deps: SolochainEvmDeps<P, FL, FSH, Runtime, A>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    Runtime: StorageEnableRuntime,
    P: sc_transaction_pool_api::TransactionPool<Block = shc_common::types::OpaqueBlock> + 'static,
    FL: FileStorageT,
    FSH: ForestStorageHandler<Runtime> + Send + Sync + 'static,
    A: sc_transaction_pool::ChainApi<Block = shc_common::types::OpaqueBlock> + 'static,
    ParachainClient<Runtime::RuntimeApi>:
        ProvideRuntimeApi<shc_common::types::OpaqueBlock>
            + sc_client_api::HeaderBackend<shc_common::types::OpaqueBlock>
            + sc_client_api::UsageProvider<shc_common::types::OpaqueBlock>
            + sc_client_api::blockchain::HeaderMetadata<
                shc_common::types::OpaqueBlock,
                Error = sp_blockchain::Error,
            >
            + 'static,
    <ParachainClient<Runtime::RuntimeApi> as ProvideRuntimeApi<shc_common::types::OpaqueBlock>>::Api:
        fp_rpc::EthereumRuntimeRPCApi<shc_common::types::OpaqueBlock>
            + fp_rpc::ConvertTransactionRuntimeApi<shc_common::types::OpaqueBlock>
            + sp_block_builder::BlockBuilder<shc_common::types::OpaqueBlock>,
{
    // Frontier RPC traits
    use fc_rpc::{Eth, EthFilter, Net, Web3};
    use fc_rpc_core::{
        EthApiServer, EthFilterApiServer, NetApiServer, TxPoolApiServer, Web3ApiServer,
    };

    type Block = shc_common::types::OpaqueBlock;

    let SolochainEvmDeps {
        client,
        pool,
        maybe_storage_hub_client_config,
        command_sink,
        network,
        sync,
        overrides,
        frontier_backend,
        graph,
        block_data_cache,
        filter_pool,
        fee_history_cache,
        fee_history_limit,
        max_past_logs,
        forced_parent_hashes,
        is_authority,
    } = deps;

    // Start from base parachain RPC (System, TransactionPayment, StorageHub, ManualSeal, DenyUnsafe)
    let mut io = create_full_parachain::<P, FL, FSH, Runtime>(FullDeps {
        client: client.clone(),
        pool: pool.clone(),
        maybe_storage_hub_client_config: maybe_storage_hub_client_config,
        command_sink: command_sink,
    })?;

    // Frontier: Eth
    enum Never {}
    impl<T> fp_rpc::ConvertTransaction<T> for Never {
        fn convert_transaction(&self, _transaction: pallet_ethereum::Transaction) -> T {
            unreachable!()
        }
    }
    let convert_transaction: Option<Never> = None;

    let signers = Vec::new();
    let pending_consensus_data_provider: Option<
        Box<(dyn fc_rpc::pending::ConsensusDataProvider<_>)>,
    > = None;

    let pending_create_inherent_data_providers = move |_, _| async move {
        let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
        let slot =
            sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                sp_consensus_babe::SlotDuration::from_millis(time::SLOT_DURATION),
            );
        Ok((slot, timestamp))
    };

    io.merge(
        Eth::<_, _, _, _, _, _, _, ()>::new(
            client.clone(),
            pool.clone(),
            graph.clone(),
            convert_transaction,
            sync.clone(),
            signers,
            overrides.clone(),
            frontier_backend.clone(),
            is_authority,
            block_data_cache.clone(),
            fee_history_cache,
            fee_history_limit,
            10,
            forced_parent_hashes,
            pending_create_inherent_data_providers,
            pending_consensus_data_provider,
        )
        .into_rpc(),
    )?;

    if let Some(filter_pool) = filter_pool {
        io.merge(
            EthFilter::new(
                client.clone(),
                frontier_backend.clone(),
                graph.clone(),
                filter_pool,
                500_usize,
                max_past_logs,
                block_data_cache,
            )
            .into_rpc(),
        )?;
    }

    io.merge(Net::new(client.clone(), network, true).into_rpc())?;
    io.merge(Web3::new(client.clone()).into_rpc())?;

    let tx_pool = TxPool::new(client.clone(), graph.clone());
    io.merge(tx_pool.into_rpc())?;

    Ok(io)
}

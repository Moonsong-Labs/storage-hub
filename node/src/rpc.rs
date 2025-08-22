//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use sc_consensus_manual_seal::{
    rpc::{ManualSeal, ManualSealApiServer},
    EngineCommand,
};
use sc_rpc::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use shc_common::{traits::StorageEnableRuntime, types::ParachainClient};
use shc_forest_manager::traits::ForestStorageHandler;
use shc_rpc::{StorageHubClientApiServer, StorageHubClientRpc, StorageHubClientRpcConfig};
use sp_core::H256;

use shc_client::types::FileStorageT;

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

/// Instantiate all RPC extensions.
pub fn create_full<P, FL, FSH, Runtime>(
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

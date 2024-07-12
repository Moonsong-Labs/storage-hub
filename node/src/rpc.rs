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
use shc_common::types::HasherOutT;
use shc_file_manager::traits::FileStorage;
use shc_forest_manager::traits::ForestStorage;
use shc_rpc::StorageHubClientApiServer;
use shc_rpc::StorageHubClientRpc;
use shc_rpc::StorageHubClientRpcConfig;
use sp_core::H256;
use sp_trie::TrieLayout;
use storage_hub_runtime::{opaque::Block, AccountId, Balance, Nonce};

pub use sc_rpc::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, P, T, FL, FS> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// File Storage instance.
    pub maybe_storage_hub_client_config: Option<StorageHubClientRpcConfig<T, FL, FS>>,
    /// Manual seal command sink
    pub command_sink: Option<futures::channel::mpsc::Sender<EngineCommand<H256>>>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, T, FL, FS>(
    deps: FullDeps<C, P, T, FL, FS>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + HeaderMetadata<Block, Error = BlockChainError>
        + Send
        + Sync
        + 'static,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + Send + Sync + 'static,
    T: TrieLayout + Send + Sync + 'static,
    FL: FileStorage<T> + Send + Sync,
    FS: ForestStorage<T> + Send + Sync,
    HasherOutT<T>: TryFrom<[u8; 32]>,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut io = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        maybe_storage_hub_client_config,
        command_sink,
        deny_unsafe,
    } = deps;

    io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
    io.merge(TransactionPayment::new(client).into_rpc())?;

    if let Some(storage_hub_client_config) = maybe_storage_hub_client_config {
        io.merge(StorageHubClientRpc::new(storage_hub_client_config).into_rpc())?;
    }

    if let Some(command_sink) = command_sink {
        io.merge(
            // We provide the rpc handler with the sending end of the channel to allow the rpc
            // send EngineCommands to the background block authorship task.
            ManualSeal::new(command_sink).into_rpc(),
        )?;
    };

    Ok(io)
}

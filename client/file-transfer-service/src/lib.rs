use sc_client_api::BlockBackend;
use sc_network::service::traits::NetworkService;
use sc_network::ProtocolName;
use std::sync::Arc;
use std::time::Duration;

use sc_network::request_responses::IncomingRequest;
use sc_network::{config::FullNetworkConfiguration, request_responses::ProtocolConfig};
use sc_service::Configuration;
use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};
use shc_common::types::{BlockHash, OpaqueBlock, ParachainClient};

pub use self::handler::FileTransferService;

/// For defining the commands processed by the file transfer service.
pub mod commands;
/// For defining the events emitted by the file transfer service.
pub mod events;
/// For incoming provider requests.
pub mod handler;
/// For defining the provider requests protocol schema.
pub mod schema;

// TODO determine ideal max request/response sizes (we could technically specify here usize::MAX)
/// Max size of request packet. (1GB)
const MAX_REQUEST_PACKET_SIZE_BYTES: u64 = 1 * 1024 * 1024 * 1024;

// TODO determine ideal max request/response sizes (we could technically specify here usize::MAX)
/// Max size of response packet. (1GB)
const MAX_RESPONSE_PACKET_SIZE_BYTES: u64 = 1 * 1024 * 1024 * 1024;

/// Max number of queued requests.
const MAX_FILE_TRANSFER_REQUESTS_QUEUE: usize = 500;

/// Updates the network configuration with the file transfer request response protocol.
/// Returns the protocol name and the channel receiver to be used for reading requests.
pub fn configure_file_transfer_network<
    Network: sc_network::NetworkBackend<OpaqueBlock, BlockHash>,
>(
    client: Arc<ParachainClient>,
    parachain_config: &Configuration,
    net_config: &mut FullNetworkConfiguration<OpaqueBlock, BlockHash, Network>,
) -> (ProtocolName, async_channel::Receiver<IncomingRequest>) {
    let genesis_hash = client
        .block_hash(0u32.into())
        .ok()
        .flatten()
        .expect("Genesis block exists; qed");

    let (tx, request_receiver) = async_channel::bounded(MAX_FILE_TRANSFER_REQUESTS_QUEUE);

    let mut protocol_config =
        generate_protocol_config(genesis_hash, parachain_config.chain_spec.fork_id());
    protocol_config.inbound_queue = Some(tx);

    let request_response_config = Network::request_response_config(
        protocol_config.name.clone(),
        protocol_config.fallback_names.clone(),
        protocol_config.max_request_size,
        protocol_config.max_response_size,
        protocol_config.request_timeout,
        protocol_config.inbound_queue,
    );

    net_config.add_request_response_protocol(request_response_config);

    (protocol_config.name, request_receiver)
}

pub async fn spawn_file_transfer_service(
    task_spawner: &TaskSpawner,
    request_receiver: async_channel::Receiver<IncomingRequest>,
    protocol_name: ProtocolName,
    network: Arc<dyn NetworkService>,
) -> ActorHandle<FileTransferService> {
    let task_spawner = task_spawner
        .with_name("file-transfer-service")
        .with_group("network");

    let file_transfer_service = FileTransferService::new(protocol_name, request_receiver, network);

    let file_transfer_service_handle = task_spawner.spawn_actor(file_transfer_service);

    file_transfer_service_handle
}

/// Generate the provider requests protocol name from the genesis hash and fork id.
fn generate_protocol_name<Hash: AsRef<[u8]>>(genesis_hash: Hash, fork_id: Option<&str>) -> String {
    let genesis_hash = genesis_hash.as_ref();
    if let Some(fork_id) = fork_id {
        format!(
            "/{}/{}/storage-hub/provider/1",
            array_bytes::bytes2hex("", genesis_hash),
            fork_id
        )
    } else {
        format!(
            "/{}/storage-hub/provider/1",
            array_bytes::bytes2hex("", genesis_hash)
        )
    }
}

/// Generates a [`ProtocolConfig`] for the provider requests protocol, refusing incoming
/// requests.
pub fn generate_protocol_config<Hash: AsRef<[u8]>>(
    genesis_hash: Hash,
    fork_id: Option<&str>,
) -> ProtocolConfig {
    ProtocolConfig {
        name: generate_protocol_name(genesis_hash, fork_id).into(),
        fallback_names: Vec::new(),
        max_request_size: MAX_REQUEST_PACKET_SIZE_BYTES,
        max_response_size: MAX_RESPONSE_PACKET_SIZE_BYTES,
        request_timeout: Duration::from_secs(15),
        inbound_queue: None,
    }
}

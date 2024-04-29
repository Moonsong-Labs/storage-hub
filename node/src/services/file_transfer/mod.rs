use sc_client_api::BlockBackend;
use std::sync::Arc;
use std::time::Duration;

use crate::service::{ParachainClient, ParachainNetworkService};
use sc_network::request_responses::IncomingRequest;
use sc_network::{config::FullNetworkConfiguration, request_responses::ProtocolConfig};
use sc_service::Configuration;
use storage_hub_infra::actor::{ActorHandle, ActorSpawner, TaskSpawner};

pub use self::handler::FileTransferService;

/// For defining the events emitted by the file transfer service.
pub mod events;
/// For incoming provider requests.
mod handler;
/// For defining the provider requests protocol schema.
mod schema;

// TODO determine ideal max request/response sizes (we could technically specify here usize::MAX)
/// Max size of request packet. (1GB)
const MAX_REQUEST_PACKET_SIZE_BYTES: u64 = 1 * 1024 * 1024 * 1024;

// TODO determine ideal max request/response sizes (we could technically specify here usize::MAX)
/// Max size of response packet. (1GB)
const MAX_RESPONSE_PACKET_SIZE_BYTES: u64 = 1 * 1024 * 1024 * 1024;

/// Max number of queued requests.
const MAX_FILE_TRANSFER_REQUESTS_QUEUE: usize = 500;

/// Updates the network configuration with the file transfer request response protocol.
/// Returns the channel receiver to be used for reading requests.
pub fn configure_file_transfer_network(
    client: Arc<ParachainClient>,
    parachain_config: &Configuration,
    net_config: &mut FullNetworkConfiguration,
) -> async_channel::Receiver<IncomingRequest> {
    let genesis_hash = client
        .block_hash(0u32.into())
        .ok()
        .flatten()
        .expect("Genesis block exists; qed");

    let (tx, request_receiver) = async_channel::bounded(MAX_FILE_TRANSFER_REQUESTS_QUEUE);

    let mut protocol_config =
        generate_protocol_config(genesis_hash, parachain_config.chain_spec.fork_id());
    protocol_config.inbound_queue = Some(tx);

    net_config.add_request_response_protocol(protocol_config);

    request_receiver
}

pub async fn spawn_file_transfer_service(
    task_spawner: &TaskSpawner,
    request_receiver: async_channel::Receiver<IncomingRequest>,
    network: Arc<ParachainNetworkService>,
) -> ActorHandle<FileTransferService> {
    let task_spawner = task_spawner
        .with_name("file-transfer-service")
        .with_group("network");

    let file_transfer_service = FileTransferService::new(request_receiver, network);

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

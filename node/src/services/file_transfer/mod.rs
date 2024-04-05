use std::time::Duration;

use sc_network::{config::FullNetworkConfiguration, request_responses::ProtocolConfig};
use sc_service::{Configuration, SpawnTaskHandle};
use storage_hub_infra::actor::{ActorHandle, ActorSpawner, TaskSpawner};

use self::handler::FileTransferService;

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

pub async fn spawn_file_transfer_service<Hash: AsRef<[u8]>>(
    task_spawner: SpawnTaskHandle,
    genesis_hash: Hash,
    parachain_config: &Configuration,
    net_config: &mut FullNetworkConfiguration,
) -> ActorHandle<FileTransferService> {
    let task_spawner =
        TaskSpawner::new(task_spawner, "file-transfer-service").with_group("network");

    let (file_transfer_service, protocol_config) =
        FileTransferService::new(genesis_hash, parachain_config.chain_spec.fork_id());

    let file_transfer_service_handle = task_spawner.spawn_actor(file_transfer_service);

    net_config.add_request_response_protocol(protocol_config);

    file_transfer_service_handle
}

/// Generate the provider requests protocol name from the genesis hash and fork id.
fn generate_protocol_name<Hash: AsRef<[u8]>>(genesis_hash: Hash, fork_id: Option<&str>) -> String {
    let genesis_hash = genesis_hash.as_ref();
    if let Some(fork_id) = fork_id {
        format!(
            "/{}/{}/provider/1",
            array_bytes::bytes2hex("", genesis_hash),
            fork_id
        )
    } else {
        format!("/{}/provider/1", array_bytes::bytes2hex("", genesis_hash))
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

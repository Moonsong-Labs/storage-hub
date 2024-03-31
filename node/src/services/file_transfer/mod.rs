use std::sync::Arc;
use std::time::Duration;

use sc_network::{config::FullNetworkConfiguration, request_responses::ProtocolConfig};
use sc_service::{Configuration, TaskManager};
use storage_hub_runtime::opaque::Block;

use crate::service::ParachainClient;

use self::handler::ProviderRequestsHandler;

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

pub async fn spawn_file_transfer_service(
    task_manager: &TaskManager,
    client: Arc<ParachainClient>,
    pubsub_notification_sinks: Arc<
        shc_mapping_sync::StorageHubBlockNotificationSinks<
            shc_mapping_sync::StorageHubBlockNotification<Block>,
        >,
    >,
    parachain_config: &Configuration,
    net_config: &mut FullNetworkConfiguration,
) {
    let provider_requests_protocol_config = {
        // Allow both outgoing and incoming requests.
        let (handler, protocol_config) =
            ProviderRequestsHandler::new(parachain_config.chain_spec.fork_id(), client.clone());
        task_manager.spawn_handle().spawn(
            "provider-requests-handler",
            Some("networking"),
            handler.run(),
        );
        protocol_config
    };

    net_config.add_request_response_protocol(provider_requests_protocol_config);
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

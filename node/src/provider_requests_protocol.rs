use sc_network::request_responses::ProtocolConfig;

use std::time::Duration;

/// For incoming provider requests.
pub mod handler;

/// Generate the provider requests protocol name from the genesis hash and fork id.
fn generate_protocol_name<Hash: AsRef<[u8]>>(genesis_hash: Hash, fork_id: Option<&str>) -> String {
    let genesis_hash = genesis_hash.as_ref();
    if let Some(fork_id) = fork_id {
        format!(
            "/{}/{}/user-requests/1.0.0",
            array_bytes::bytes2hex("", genesis_hash),
            fork_id
        )
    } else {
        format!(
            "/{}/user-requests/1.0.0",
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
        max_request_size: 1 * 1024 * 1024,
        max_response_size: 16 * 1024 * 1024,
        request_timeout: Duration::from_secs(15),
        inbound_queue: None,
    }
}

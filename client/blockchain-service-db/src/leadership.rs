use diesel::ConnectionError;
use log::warn;
use serde::{Deserialize, Serialize};
use tokio_postgres::Client;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{make_rustls_config_from_env, DbSetupError, LOG_TARGET};

/// Public alias for the leadership connection client type.
///
/// This keeps the concrete client type encapsulated in this crate while allowing
/// downstream crates (e.g. `shc-blockchain-service`) to store and pass it around.
pub type LeadershipClient = Client;

/// Hardcoded advisory lock key for leader election.
///
/// All instances that share the same Postgres DB and keystore will contend on this key.
/// In future, this could be derived from the MSP/BSP ID.
pub const LEADERSHIP_LOCK_KEY: i64 = 1;

/// Advertised endpoints of a node.
///
/// Each node maintains its own advertised endpoints. When a node becomes the leader,
/// it submits these endpoints as leader metadata to the database so followers can discover them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeAdvertisedEndpoints {
    /// WebSocket RPC URL for blockchain queries and transactions
    /// Example: "ws://192.168.1.100:9944"
    pub rpc_url: String,

    /// Trusted file transfer server URL for efficient file uploads
    /// Example: "http://192.168.1.100:7070"
    pub trusted_file_transfer_server_url: String,
}

/// Open a dedicated, TLS-enabled Postgres connection for leadership purposes.
///
/// This connection is non-pooled and intended to live for the lifetime of the process.
/// It is suitable for acquiring and holding session-level advisory locks.
pub async fn open_leadership_connection(
    database_url: &str,
) -> Result<LeadershipClient, DbSetupError> {
    let rustls_config = make_rustls_config_from_env();
    let tls = MakeRustlsConnect::new(rustls_config);

    let (client, connection) = tokio_postgres::connect(database_url, tls)
        .await
        .map_err(|e| {
            DbSetupError::ConnectionError(ConnectionError::BadConnection(e.to_string()))
        })?;

    // Spawn the connection driver so it keeps running in the background.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!(
                target: LOG_TARGET,
                "Leadership connection closed with error: {}",
                e
            );
        } else {
            warn!(
                target: LOG_TARGET,
                "Leadership connection closed gracefully. Advisory locks (if any) were released."
            );
        }
    });

    Ok(client)
}

/// Try to acquire the leadership advisory lock.
///
/// Returns `Ok(true)` if the lock was obtained, `Ok(false)` if another session already holds it.
pub async fn try_acquire_leadership(
    client: &LeadershipClient,
    key: i64,
) -> Result<bool, DbSetupError> {
    let row = client
        .query_one("SELECT pg_try_advisory_lock($1)", &[&key])
        .await
        .map_err(|e| {
            DbSetupError::ConnectionError(ConnectionError::BadConnection(e.to_string()))
        })?;

    let acquired: bool = row.get(0);
    Ok(acquired)
}

/// Release the leadership advisory lock, if held by this session.
///
/// Returns `Ok(true)` if the lock was released, `Ok(false)` if it was not held.
/// This is optional as locks are automatically released when the connection closes.
pub async fn release_leadership(client: &LeadershipClient, key: i64) -> Result<bool, DbSetupError> {
    let row = client
        .query_one("SELECT pg_advisory_unlock($1)", &[&key])
        .await
        .map_err(|e| {
            DbSetupError::ConnectionError(ConnectionError::BadConnection(e.to_string()))
        })?;

    let released: bool = row.get(0);
    Ok(released)
}

/// Update the leader_info singleton table with the current leader's advertised endpoints.
///
/// This should only be called by the node holding the advisory lock.
/// The node submits its advertised endpoints as leader metadata for followers to discover.
pub async fn update_leader_info(
    client: &LeadershipClient,
    endpoints: &NodeAdvertisedEndpoints,
) -> Result<(), DbSetupError> {
    // Serialize to JSON using serde_json
    let metadata = serde_json::to_value(endpoints).map_err(|e| {
        DbSetupError::ConnectionError(ConnectionError::BadConnection(format!(
            "Failed to serialize NodeAdvertisedEndpoints to JSON: {}",
            e
        )))
    })?;

    // Update the singleton row (id=1) with the new metadata
    client
        .execute(
            "UPDATE leader_info SET metadata = $1 WHERE id = 1",
            &[&metadata],
        )
        .await
        .map_err(|e| {
            DbSetupError::ConnectionError(ConnectionError::BadConnection(format!(
                "Failed to update leader info: {}",
                e
            )))
        })?;

    Ok(())
}

/// Get the current leader's advertised endpoints from the leader_info singleton table.
///
/// Returns `Ok(Some(endpoints))` if leader info exists and is valid,
/// `Ok(None)` if no leader info is registered or metadata is empty,
/// or `Err` if the query fails or JSON is malformed.
///
/// # Example
/// ```no_run
/// use shc_blockchain_service_db::leadership::get_leader_info;
///
/// # async fn example(client: &tokio_postgres::Client) -> Result<(), Box<dyn std::error::Error>> {
/// if let Some(endpoints) = get_leader_info(client).await? {
///     println!("Leader RPC: {}", endpoints.rpc_url);
///     println!("Leader File Transfer: {}", endpoints.trusted_file_transfer_server_url);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn get_leader_info(
    client: &LeadershipClient,
) -> Result<Option<NodeAdvertisedEndpoints>, DbSetupError> {
    let rows = client
        .query("SELECT metadata FROM leader_info WHERE id = 1", &[])
        .await
        .map_err(|e| {
            DbSetupError::ConnectionError(ConnectionError::BadConnection(format!(
                "Failed to query leader info: {}",
                e
            )))
        })?;

    if rows.is_empty() {
        return Ok(None);
    }

    let metadata: serde_json::Value = rows[0].get(0);

    // If metadata is an empty object, consider it as no info
    if metadata.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        return Ok(None);
    }

    // Deserialize to NodeAdvertisedEndpoints using serde_json
    let endpoints = serde_json::from_value(metadata).map_err(|e| {
        DbSetupError::ConnectionError(ConnectionError::BadConnection(format!(
            "Failed to deserialize NodeAdvertisedEndpoints from JSON: {}",
            e
        )))
    })?;
    Ok(Some(endpoints))
}

use diesel::ConnectionError;
use log::warn;
use tokio_postgres::Client;
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::{make_rustls_config_from_env, DbSetupError, LOG_TARGET};

/// Hardcoded advisory lock key for leader election.
///
/// All instances that share the same Postgres DB and keystore will contend on this key.
/// In future, this could be derived from the MSP/BSP ID.
pub const LEADERSHIP_LOCK_KEY: i64 = 1;

/// Open a dedicated, TLS-enabled Postgres connection for leadership purposes.
///
/// This connection is non-pooled and intended to live for the lifetime of the process.
/// It is suitable for acquiring and holding session-level advisory locks.
pub async fn open_leadership_connection(database_url: &str) -> Result<Client, DbSetupError> {
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
pub async fn try_acquire_leadership(client: &Client, key: i64) -> Result<bool, DbSetupError> {
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
pub async fn release_leadership(client: &Client, key: i64) -> Result<bool, DbSetupError> {
    let row = client
        .query_one("SELECT pg_advisory_unlock($1)", &[&key])
        .await
        .map_err(|e| {
            DbSetupError::ConnectionError(ConnectionError::BadConnection(e.to_string()))
        })?;

    let released: bool = row.get(0);
    Ok(released)
}

//! SmartPool implementation for automatic test transaction management.
//!
//! ## Key Components
//! - [`SmartPool`] - Connection pool with automatic test transaction support
//!
//! ## Features
//! - Automatic test transactions in test mode (single connection)
//! - Normal pooling in production mode (32 connections)

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(not(test))]
use {
    diesel::{ConnectionError, ConnectionResult},
    diesel_async::pooled_connection::ManagerConfig,
    diesel_async::RunQueryDsl,
    futures::{future::BoxFuture, FutureExt},
    rustls::{
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        pki_types::{CertificateDer, ServerName, UnixTime},
        version, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    },
    rustls_pemfile::certs as load_pem_certs,
    rustls_platform_verifier::ConfigVerifierExt,
    std::{fs::File, io::BufReader, time::Duration},
    tracing::warn,
};

#[cfg(test)]
use diesel_async::AsyncConnection;
use diesel_async::{
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};

use super::error::RepositoryError;

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> =
    diesel_async::pooled_connection::bb8::PooledConnection<'a, AsyncPgConnection>;

/// Smart connection pool that automatically manages test transactions.
///
/// In test mode:
/// - Uses single connection to enable test transactions
/// - Automatically begins test transaction on first connection
/// - Transaction automatically rolls back when test ends
///
/// In production mode:
/// - Uses normal connection pooling with 32 connections
/// - No test transaction overhead
pub struct SmartPool {
    /// The underlying bb8 pool
    inner: Arc<DbPool>,

    /// Track whether test transaction has been initialized (test mode only)
    #[cfg(test)]
    test_tx_initialized: AtomicBool,
}

impl SmartPool {
    /// Create a new SmartPool with the given database URL.
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Result<Self, RepositoryError>` - The configured pool or error
    pub async fn new(database_url: &str) -> Result<Self, RepositoryError> {
        // Create the connection manager
        #[cfg(test)]
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        #[cfg(not(test))]
        let manager = {
            // Use TLS-aware custom setup only in non-test builds
            let mut manager_cfg = ManagerConfig::default();
            manager_cfg.custom_setup = Box::new(|config: &str| establish_connection(config));
            AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
                database_url,
                manager_cfg,
            )
        };

        // Configure pool based on compile mode
        #[cfg(test)]
        let pool = {
            // Single connection for test transactions
            Pool::builder()
                .max_size(1)
                .build(manager)
                .await
                .map_err(|e| RepositoryError::Pool(format!("Failed to create test pool: {}", e)))?
        };

        #[cfg(not(test))]
        let pool = {
            // Normal pool size and tuned settings for production
            let pool = Pool::builder()
                .max_size(32)
                .connection_timeout(Duration::from_secs(15))
                .idle_timeout(Some(Duration::from_secs(300)))
                .max_lifetime(Some(Duration::from_secs(3600)))
                .min_idle(Some(4))
                .build(manager)
                .await
                .map_err(|e| {
                    RepositoryError::Pool(format!("Failed to create production pool: {}", e))
                })?;

            // Perform immediate health-check to surface connection/TLS errors early
            {
                let mut conn = pool.get().await.map_err(|e| {
                    RepositoryError::Pool(format!("Failed to get connection: {}", e))
                })?;
                diesel::sql_query("SELECT 1")
                    .execute(&mut conn)
                    .await
                    .map_err(|e| RepositoryError::Pool(format!("Healthcheck failed: {}", e)))?;
            }

            pool
        };

        Ok(Self {
            inner: Arc::new(pool),
            #[cfg(test)]
            test_tx_initialized: AtomicBool::new(false),
        })
    }

    /// Get a connection from the pool.
    ///
    /// In test mode, this will automatically begin a test transaction
    /// on the first call, which will be rolled back when the test ends.
    ///
    /// # Returns
    /// * `Result<DbConnection, RepositoryError>` - Database connection or error
    pub async fn get(&self) -> Result<DbConnection<'_>, RepositoryError> {
        // Get connection from pool
        #[allow(unused_mut)]
        let mut conn = self
            .inner
            .get()
            .await
            .map_err(|e| RepositoryError::Pool(format!("Failed to get connection: {}", e)))?;

        #[cfg(test)]
        {
            if self
                .test_tx_initialized
                // initialize test transaction is not already initialized
                // if it was not initialized, it will be set as initialized
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // Begin test transaction that will rollback automatically
                conn.begin_test_transaction()
                    .await
                    .map_err(RepositoryError::Database)?;
            }
        }

        Ok(conn)
    }
}

// --- TLS setup and custom connection establishment ---

#[derive(Debug)]
#[cfg(not(test))]
struct NoCertificateVerification;

#[cfg(not(test))]
impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ED25519,
        ]
    }
}

#[cfg(not(test))]
fn make_rustls_config_from_env() -> ClientConfig {
    let insecure = std::env::var_os("SH_DB_TLS_INSECURE").is_some();
    let ca_file = std::env::var_os("SH_DB_TLS_CA_FILE");
    if insecure {
        // Accept any certificate and hostname. DO NOT use in production.
        let provider = rustls::crypto::ring::default_provider();
        let builder = rustls::ClientConfig::builder_with_provider(provider.into());
        let builder = builder
            .with_protocol_versions(&[&version::TLS13, &version::TLS12])
            .expect("valid TLS versions");
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
            .with_no_client_auth()
    } else if let Some(path) = ca_file {
        match File::open(&path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let certs_result: Result<Vec<_>, std::io::Error> =
                    load_pem_certs(&mut reader).collect();
                match certs_result {
                    Ok(pems) => {
                        let mut roots = RootCertStore::empty();
                        for cert in pems {
                            if let Err(err) = roots.add(cert) {
                                warn!(error = %err, "Failed to add certificate to root store");
                            }
                        }
                        let provider = rustls::crypto::ring::default_provider();
                        let builder = rustls::ClientConfig::builder_with_provider(provider.into());
                        let builder = builder
                            .with_protocol_versions(&[&version::TLS13, &version::TLS12])
                            .expect("valid TLS versions");
                        builder.with_root_certificates(roots).with_no_client_auth()
                    }
                    Err(err) => {
                        warn!(path = ?path, error = %err, "Failed to parse PEM certs, falling back to platform verifier");
                        ClientConfig::with_platform_verifier()
                    }
                }
            }
            Err(err) => {
                warn!(path = ?path, error = %err, "Failed to open CA file, falling back to platform verifier");
                ClientConfig::with_platform_verifier()
            }
        }
    } else {
        // Use system trust store and normal verification.
        ClientConfig::with_platform_verifier()
    }
}

#[cfg(not(test))]
fn establish_connection(config: &str) -> BoxFuture<'_, ConnectionResult<AsyncPgConnection>> {
    let fut = async {
        let rustls_config = make_rustls_config_from_env();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
        let (client, conn) = tokio_postgres::connect(config, tls)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

        AsyncPgConnection::try_from_client_and_connection(client, conn).await
    };
    fut.boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::indexer_db::test_helpers::setup_test_db;

    #[tokio::test]
    async fn create_and_get_connection() {
        let (_container, url) = setup_test_db(vec![], vec![]).await;

        let pool = SmartPool::new(&url).await.expect("able to create pool");

        pool.get().await.expect("able to get connection");

        assert!(
            pool.test_tx_initialized
                .fetch_and(true, std::sync::atomic::Ordering::SeqCst),
            "connection initialized with test_transaction"
        );
    }
}

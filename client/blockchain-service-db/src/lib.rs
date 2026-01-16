use std::{fs::File, io::BufReader, sync::Arc, time::Duration};

use diesel::prelude::*;
use diesel::{ConnectionError, ConnectionResult};
use diesel_async::{
    pooled_connection::{
        bb8::{Pool, PooledConnection},
        AsyncDieselConnectionManager, ManagerConfig,
    },
    AsyncPgConnection, RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use futures::{future::BoxFuture, FutureExt};
use log::{info, warn};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
    version, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use rustls_pemfile::certs as load_pem_certs;
use rustls_platform_verifier::ConfigVerifierExt;
use thiserror::Error;

pub mod leadership;
pub use leadership::NodeAdvertisedEndpoints;
pub mod models;
pub mod schema;
pub mod store;

pub(crate) const LOG_TARGET: &str = "shc-blockchain-service-db";

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbConnection<'a> = PooledConnection<'a, AsyncPgConnection>;

pub type AsyncPgPool = Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;
pub type AsyncPgPooled = PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>;

#[derive(Error, Debug)]
pub enum DbSetupError {
    #[error("Failed to connect to the database: {0}")]
    ConnectionError(#[from] ConnectionError),
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub async fn setup_db_pool(database_url: String) -> Result<DbPool, DbSetupError> {
    // Default ON; set SH_PENDING_DB_AUTO_MIGRATE to "false"/"0"/"off" to disable
    let should_auto_migrate = std::env::var("SH_PENDING_DB_AUTO_MIGRATE")
        .map(|v| {
            let v = v.to_lowercase();
            !(v == "0" || v == "false" || v == "off")
        })
        .unwrap_or(true);

    if should_auto_migrate {
        info!(target: LOG_TARGET, "👨‍💻 Running pending DB migrations...");
        // Run migrations synchronously in a blocking task before pool creation.
        let database_url_clone = database_url.clone();
        let migrate_result = tokio::task::spawn_blocking(move || {
            let mut conn = diesel::pg::PgConnection::establish(&database_url_clone)
                .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
            conn.run_pending_migrations(MIGRATIONS).map_err(|e| {
                ConnectionError::BadConnection(format!("failed to run migrations: {}", e))
            })?;
            Ok::<(), ConnectionError>(())
        })
        .await
        .map_err(|e| ConnectionError::BadConnection(format!("migration task join error: {}", e)))?;

        migrate_result?;
        info!(target: LOG_TARGET, "👨‍💻 Pending DB migrations completed");
    }

    let mut cfg = ManagerConfig::default();
    cfg.custom_setup = Box::new(|config: &str| establish_connection(config));
    let mgr = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(database_url, cfg);

    let pool = Pool::builder()
        .max_size(16)
        .connection_timeout(Duration::from_secs(15))
        .idle_timeout(Some(Duration::from_secs(300)))
        .max_lifetime(Some(Duration::from_secs(3600)))
        .min_idle(Some(4))
        .build(mgr)
        .await
        .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

    // Health-check immediately (surface PG/libpq errors now, not later):
    {
        let mut conn = pool
            .get()
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?; // Obeys connection_timeout above
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;
    }

    Ok(pool)
}

fn establish_connection(config: &str) -> BoxFuture<'_, ConnectionResult<AsyncPgConnection>> {
    let fut = async {
        // Build rustls config, optionally disabling verification for local testing.
        let rustls_config = make_rustls_config_from_env();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
        let (client, conn) = tokio_postgres::connect(config, tls)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

        AsyncPgConnection::try_from_client_and_connection(client, conn).await
    };
    fut.boxed()
}

#[derive(Debug)]
struct NoCertificateVerification;

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

pub(crate) fn make_rustls_config_from_env() -> ClientConfig {
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
                                warn!(
                                    target: LOG_TARGET,
                                    "Failed to add certificate to root store: {}",
                                    err
                                );
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
                        warn!(
                            target: LOG_TARGET,
                            "Failed to parse PEM certs from {:?}: {}. Falling back to platform verifier.",
                            path, err
                        );
                        ClientConfig::with_platform_verifier()
                    }
                }
            }
            Err(err) => {
                warn!(
                    target: LOG_TARGET,
                    "Failed to open CA file {:?}: {}. Falling back to platform verifier.",
                    path, err
                );
                ClientConfig::with_platform_verifier()
            }
        }
    } else {
        // Use system trust store and normal verification.
        ClientConfig::with_platform_verifier()
    }
}

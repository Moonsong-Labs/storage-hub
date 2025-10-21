//! StorageHub MSP Backend Binary
//!
//! Main entry point for the StorageHub MSP (Main Storage Provider) backend service.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
#[cfg(feature = "mocks")]
use sh_msp_backend_lib::data::{indexer_db::mock_repository::MockRepository, rpc::MockConnection};
use sh_msp_backend_lib::{
    api::create_app,
    config::{Config, LogFormat},
    constants::retry::get_retry_delay,
    data::{
        indexer_db::{client::DBClient, repository::postgres::Repository},
        rpc::{AnyRpcConnection, RpcConfig, StorageHubRpcClient, WsConnection},
        storage::{BoxedStorageWrapper, InMemoryStorage},
    },
    services::Services,
};
use tracing::{debug, info, warn};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "sh-msp-backend")]
#[command(about = "StorageHub MSP Backend Service", long_about = None)]
struct Args {
    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Override server host
    #[arg(long)]
    host: Option<String>,

    /// Override server port
    #[arg(short, long)]
    port: Option<u16>,

    /// Override log format (text, json, or auto)
    #[arg(long)]
    log_format: Option<String>,

    /// Override database URL
    #[arg(long)]
    database_url: Option<String>,

    /// Override RPC URL
    #[arg(long)]
    rpc_url: Option<String>,

    /// Override MSP callback URL
    #[arg(long)]
    msp_callback_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load the backend configuration
    let config = load_config()?;

    // Initialize tracing with the log format specified in the configuration
    initialize_logging(config.log_format);

    info!("Starting StorageHub Backend");

    let (host, port) = (config.host.clone(), config.port);
    info!(
        host = %host,
        port = port,
        log_format = ?config.log_format,
        "Configuration loaded"
    );
    debug!(target: "main", database_url = %config.database.url, "Database configuration");
    debug!(target: "main", rpc_url = %config.storage_hub.rpc_url, "RPC configuration");
    debug!(target: "main", msp_callback_url = %config.storage_hub.msp_callback_url, "MSP callback configuration");

    let memory_storage = InMemoryStorage::new();
    let storage = Arc::new(BoxedStorageWrapper::new(memory_storage));

    info!("Initializing services");
    let postgres_client = create_postgres_client(&config).await?;
    let rpc_client = create_rpc_client_with_retry(&config).await?;
    let services = Services::new(storage, postgres_client, rpc_client, config.clone()).await;
    info!("All services initialized successfully");

    // Start server
    let app = create_app(services);
    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .context("Failed to bind TCP listener")?;

    info!(host = %host, port = port, "Server listening");

    axum::serve(listener, app).await.context("Server error")?;

    info!("Shutting down StorageHub Backend");
    Ok(())
}

fn load_config() -> Result<Config> {
    let args = Args::parse();

    let mut config = match args.config {
        Some(path) => Config::from_file(&path)
            .with_context(|| format!("Failed to read config file: {}", path))?,
        None => {
            debug!(target: "main::load_config", "No config file specified, using defaults");
            Config::default()
        }
    };

    // Apply CLI overrides
    if let Some(host) = args.host {
        config.host = host;
    }
    if let Some(port) = args.port {
        config.port = port;
    }
    if let Some(log_format) = args.log_format {
        config.log_format = match log_format.to_lowercase().as_str() {
            "text" => LogFormat::Text,
            "json" => LogFormat::Json,
            "auto" => LogFormat::Auto,
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid log format: '{}'. Valid options: text, json, auto",
                    log_format
                ));
            }
        };
    }
    if let Some(database_url) = args.database_url {
        config.database.url = database_url;
    }
    if let Some(rpc_url) = args.rpc_url {
        config.storage_hub.rpc_url = rpc_url;
    }
    if let Some(msp_callback_url) = args.msp_callback_url {
        config.storage_hub.msp_callback_url = msp_callback_url;
    }

    Ok(config)
}

/// Initialize logging with the specified format
fn initialize_logging(log_format: LogFormat) {
    let env_filter = EnvFilter::from_default_env();
    let format = log_format.resolve();

    match format {
        LogFormat::Json => {
            // JSON logging using Bunyan format
            tracing_subscriber::registry()
                .with(env_filter)
                .with(JsonStorageLayer)
                .with(BunyanFormattingLayer::new(
                    "storage-hub-backend".to_string(),
                    std::io::stdout,
                ))
                .init();
        }
        LogFormat::Text => {
            // Human-readable text logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
        LogFormat::Auto => {
            // This should have been resolved, but handle it just in case
            let resolved = log_format.resolve();
            initialize_logging(resolved);
        }
    }
}

async fn create_postgres_client(config: &Config) -> Result<Arc<DBClient>> {
    debug!(target: "main::create_postgres_client", "Creating PostgreSQL client");

    #[cfg(feature = "mocks")]
    {
        if config.database.mock_mode {
            info!(mock_mode = true, "Using mock repository");

            let mock_repo = MockRepository::sample().await;
            let client = DBClient::new(Arc::new(mock_repo));

            // Test the connection (mock always succeeds)
            client
                .test_connection()
                .await
                .context("Failed to test mock connection")?;

            return Ok(Arc::new(client));
        }
    }

    // Initialize real repository for database access
    let repository = Repository::new(&config.database.url)
        .await
        .context("Failed to create repository with database connection")?;

    let client = DBClient::new(Arc::new(repository));

    // Test the connection
    client
        .test_connection()
        .await
        .context("Failed to connect to PostgreSQL")?;

    info!("Connected to PostgreSQL database");
    Ok(Arc::new(client))
}

async fn create_rpc_client(config: &Config) -> Result<Arc<StorageHubRpcClient>> {
    debug!(target: "main::create_rpc_client", "Creating RPC client");

    #[cfg(feature = "mocks")]
    {
        if config.storage_hub.mock_mode {
            info!(mock_mode = true, "Using mock RPC connection");

            let mock_conn = AnyRpcConnection::Mock(MockConnection::new());
            let client = StorageHubRpcClient::new(Arc::new(mock_conn));

            return Ok(Arc::new(client));
        }
    }

    let rpc_config = RpcConfig {
        url: config.storage_hub.rpc_url.clone(),
        timeout_secs: config.storage_hub.timeout_secs,
        max_concurrent_requests: config.storage_hub.max_concurrent_requests,
        verify_tls: config.storage_hub.verify_tls,
    };

    let ws_conn = WsConnection::new(rpc_config)
        .await
        .context("Failed to create RPC connection")?;

    let conn = AnyRpcConnection::Real(ws_conn);
    let client = StorageHubRpcClient::new(Arc::new(conn));

    info!("Connected to StorageHub RPC");
    Ok(Arc::new(client))
}

/// This function tries to create an RPC client and, if failing to do so (mainly caused by the MSP client
/// not being ready yet), it retries indefinitely with a stepped backoff strategy.
///
/// Note: Keep in mind that the failure to connect to the RPC is not always caused by the MSP client
/// not being ready yet, but could also occur due to a badly configured RPC URL. If this is the case,
/// the backend will keep retrying indefinitely but fail to start. Monitor the retry attempt count
/// in logs to detect potential configuration issues.
async fn create_rpc_client_with_retry(config: &Config) -> Result<Arc<StorageHubRpcClient>> {
    let mut attempt = 0;

    loop {
        match create_rpc_client(config).await {
            Ok(client) => return Ok(client),
            Err(e) => {
                // Calculate the retry delay before the next attempt based on the attempt number
                let delay_secs = get_retry_delay(attempt);
                warn!(
                    target: "main::create_rpc_client_with_retry",
                    attempt = attempt + 1,
                    delay_secs = delay_secs,
                    error = ?e,
                    "RPC not ready yet, retrying in {delay_secs} seconds",
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                attempt += 1;
            }
        }
    }
}

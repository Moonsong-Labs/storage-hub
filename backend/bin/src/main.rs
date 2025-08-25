//! StorageHub MSP Backend Binary
//!
//! Main entry point for the StorageHub MSP (Main Storage Provider) backend service.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
#[cfg(feature = "mocks")]
use sh_msp_backend_lib::data::{indexer_db::MockRepository, rpc::MockConnection};
use sh_msp_backend_lib::{
    api::create_app,
    config::Config,
    data::{
        indexer_db::{DBClient, Repository},
        rpc::{AnyRpcConnection, RpcConfig, StorageHubRpcClient, WsConnection},
        storage::{BoxedStorageWrapper, InMemoryStorage},
    },
    services::Services,
};
use tracing::{debug, info};
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

    /// Override database URL
    #[arg(long)]
    database_url: Option<String>,

    /// Override RPC URL
    #[arg(long)]
    rpc_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting StorageHub Backend");

    // Initialize services
    let config = load_config()?;
    info!("Server will run on {}:{}", config.host, config.port);

    let memory_storage = InMemoryStorage::new();
    let storage = Arc::new(BoxedStorageWrapper::new(memory_storage));

    let postgres_client = create_postgres_client(&config).await?;
    let rpc_client = create_rpc_client(&config).await?;
    let services = Services::new(storage, postgres_client, rpc_client);

    // Start server
    let app = create_app(services);
    let listener = tokio::net::TcpListener::bind((config.host.as_str(), config.port))
        .await
        .context("Failed to bind TCP listener")?;

    info!("Server listening on http://{}:{}", config.host, config.port);

    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}

fn load_config() -> Result<Config> {
    let args = Args::parse();

    let mut config = match args.config {
        Some(path) => Config::from_file(&path)
            .with_context(|| format!("Failed to read config file: {}", path))?,
        None => {
            debug!("No config file specified, using defaults");
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
    if let Some(database_url) = args.database_url {
        config.database.url = database_url;
    }
    if let Some(rpc_url) = args.rpc_url {
        config.storage_hub.rpc_url = rpc_url;
    }

    Ok(config)
}

async fn create_postgres_client(config: &Config) -> Result<Arc<DBClient>> {
    #[cfg(feature = "mocks")]
    {
        if config.database.mock_mode {
            info!("Using mock repository (mock_mode enabled)");

            let mock_repo = MockRepository::new();
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
    #[cfg(feature = "mocks")]
    {
        if config.storage_hub.mock_mode {
            info!("Using mock RPC connection (mock_mode enabled)");

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

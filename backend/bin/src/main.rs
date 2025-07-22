//! StorageHub Backend Binary
//!
//! Main entry point for the StorageHub backend service.
//! This binary initializes the service with configuration, sets up storage and database
//! connections, and starts the HTTP server.

use anyhow::{Context, Result};
use clap::Parser;
use sh_backend_lib::{
    api::create_app,
    config::Config,
    data::{
        postgres::{AnyDbConnection, DbConfig, PgConnection, PostgresClient, PostgresClientTrait},
        rpc::{AnyRpcConnection, RpcConfig, StorageHubRpcClient, StorageHubRpcTrait, WsConnection},
        storage::{BoxedStorageWrapper, InMemoryStorage},
    },
    services::Services,
};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(name = "sh-backend")]
#[command(about = "StorageHub Backend Service", long_about = None)]
struct Args {
    /// Config file path
    #[arg(short, long, default_value = "backend_config.toml")]
    config: String,
    
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

// WIP: Mock imports - postgres mocks commented out until diesel traits are fully implemented
#[cfg(feature = "mocks")]
use sh_backend_lib::data::{
    // postgres::{MockPostgresClient, MockDbConnection},
    rpc::MockConnection,
};

#[tokio::main]
async fn main() {
    // Initialize tracing/logging
    let filter = EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting StorageHub Backend");

    // Load configuration
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    info!(
        "Configuration loaded - Server will run on {}:{}",
        config.host, config.port
    );

    // Initialize storage
    let memory_storage = InMemoryStorage::new();
    let storage = Arc::new(BoxedStorageWrapper::new(memory_storage));
    info!("Initialized in-memory storage");

    // Initialize PostgreSQL client
    let postgres_client: Arc<dyn PostgresClientTrait> = match create_postgres_client(&config).await
    {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to initialize PostgreSQL client: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize RPC client
    let rpc_client: Arc<dyn StorageHubRpcTrait> = match create_rpc_client(&config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to initialize RPC client: {}", e);
            std::process::exit(1);
        }
    };

    // Create services
    let services = Services::new(storage, postgres_client, rpc_client);
    info!("Services initialized");

    // Create the application
    let app = create_app(services);

    // Start the server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("Server listening on http://{}", addr);
            listener
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    // Run the server
    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}

/// Load configuration from file with fallback to defaults
fn load_config() -> Result<Config> {
    let args = Args::parse();
    
    // Load base config
    let mut config = if args.config == "backend_config.toml" {
        // Default path behavior - use defaults if file doesn't exist
        match Config::from_file(&args.config) {
            Ok(config) => config,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("Config file not found, using defaults");
                Config::default()
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to read config file: {}", e)),
        }
    } else {
        // Explicit path - error if file doesn't exist
        Config::from_file(&args.config)
            .with_context(|| format!("Failed to read config file: {}", args.config))?
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

/// Create PostgreSQL client based on configuration
///
/// This function will create a connection first, then create the client.
/// It will return either a real PostgreSQL client or a mock client
/// depending on the configuration and available features.
async fn create_postgres_client(
    config: &Config,
) -> Result<Arc<dyn PostgresClientTrait>, Box<dyn std::error::Error>> {
    // WIP: Mock mode handling - commented out until diesel traits are fully implemented
    // #[cfg(feature = "mocks")]
    // {
    //     if config.database.mock_mode {
    //         info!("Using mock PostgreSQL connection (mock_mode enabled)");
    //         let mock_conn = AnyDbConnection::Mock(MockDbConnection::new());
    //         let client = PostgresClient::new(Arc::new(mock_conn));
    //         return Ok(Arc::new(client));
    //     }
    // }

    // Try to create real PostgreSQL connection
    let db_config = DbConfig::new(&config.database.url);
    match PgConnection::new(db_config).await {
        Ok(pg_conn) => {
            let conn = AnyDbConnection::Real(pg_conn);
            let client = PostgresClient::new(Arc::new(conn)).await;

            // Test the connection
            match client.test_connection().await {
                Ok(_) => {
                    info!("Connected to PostgreSQL at {}", config.database.url);
                    Ok(Arc::new(client))
                }
                Err(e) => {
                    error!("Failed to connect to PostgreSQL: {}", e);
                    Err(Box::new(e))
                }
            }
        }
        Err(e) => {
            error!("Failed to create PostgreSQL connection: {}", e);
            Err(e.into())
        }
    }
}

/// Create RPC client based on configuration
///
/// This function will create a connection first, then create the client.
/// It will return either a real RPC client or a mock client
/// depending on the configuration and available features.
async fn create_rpc_client(
    config: &Config,
) -> Result<Arc<dyn StorageHubRpcTrait>, Box<dyn std::error::Error>> {
    #[cfg(feature = "mocks")]
    {
        if config.storage_hub.mock_mode {
            info!("Using mock RPC connection (mock_mode enabled)");
            let mock_conn = AnyRpcConnection::Mock(MockConnection::new());
            let client = StorageHubRpcClient::new(Arc::new(mock_conn));
            return Ok(Arc::new(client));
        }
    }

    // Try to create real WebSocket connection
    let rpc_config = RpcConfig {
        url: config.storage_hub.rpc_url.clone(),
        timeout_secs: Some(30),
        max_concurrent_requests: Some(100),
        verify_tls: true,
    };
    match WsConnection::new(rpc_config).await {
        Ok(ws_conn) => {
            let conn = AnyRpcConnection::Real(ws_conn);
            let client = StorageHubRpcClient::new(Arc::new(conn));
            info!(
                "Connected to StorageHub RPC at {}",
                config.storage_hub.rpc_url
            );
            Ok(Arc::new(client))
        }
        Err(e) => {
            error!("Failed to create RPC connection: {}", e);

            #[cfg(feature = "mocks")]
            {
                if config.storage_hub.mock_mode {
                    info!("Using mock RPC connection (mock_mode enabled)");
                    let mock_conn = AnyRpcConnection::Mock(MockConnection::new());
                    let client = StorageHubRpcClient::new(Arc::new(mock_conn));
                    Ok(Arc::new(client))
                } else {
                    Err(e.into())
                }
            }

            #[cfg(not(feature = "mocks"))]
            {
                Err(e.into())
            }
        }
    }
}

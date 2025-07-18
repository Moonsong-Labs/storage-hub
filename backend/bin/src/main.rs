//! StorageHub Backend Binary
//!
//! Main entry point for the StorageHub backend service.
//! This binary initializes the service with configuration, sets up storage and database
//! connections, and starts the HTTP server.

use sh_backend_lib::{
    api::create_app,
    config::Config,
    data::{
        postgres::{PostgresClient, PostgresClientTrait},
        storage::{BoxedStorageWrapper, InMemoryStorage},
    },
    services::Services,
};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(feature = "mocks")]
use sh_backend_lib::mocks::MockPostgresClient;

#[tokio::main]
async fn main() {
    // Initialize tracing/logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sh_backend=debug,sh_backend_lib=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting StorageHub Backend");

    // Load configuration
    let config = match load_config().await {
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
    let postgres_client: Arc<dyn PostgresClientTrait> = match create_postgres_client(&config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to initialize PostgreSQL client: {}", e);
            std::process::exit(1);
        }
    };

    // Create services
    let services = Services::new(storage, postgres_client);
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
async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    const CONFIG_PATH: &str = "backend_config.toml";

    // Try to load from file first
    match Config::from_file(CONFIG_PATH) {
        Ok(config) => {
            info!("Configuration loaded from {}", CONFIG_PATH);
            Ok(config)
        }
        Err(e) => {
            info!(
                "Could not load config from {} ({}), using defaults",
                CONFIG_PATH, e
            );
            Ok(Config::default())
        }
    }
}

/// Create PostgreSQL client based on configuration
///
/// This function will return either a real PostgreSQL client or a mock client
/// depending on the configuration and available features.
async fn create_postgres_client(
    config: &Config,
) -> Result<Arc<dyn PostgresClientTrait>, Box<dyn std::error::Error>> {
    #[cfg(feature = "mocks")]
    {
        if config.database.mock_mode {
            info!("Using mock PostgreSQL client (mock_mode enabled)");
            return Ok(Arc::new(MockPostgresClient::new()));
        }
    }

    // Try to create real PostgreSQL client
    match PostgresClient::new(&config.database.url).await {
        Ok(client) => {
            // Test the connection
            match client.test_connection().await {
                Ok(_) => {
                    info!("Connected to PostgreSQL at {}", config.database.url);
                    Ok(Arc::new(client))
                }
                Err(e) => {
                    error!("Failed to connect to PostgreSQL: {}", e);
                    
                    #[cfg(feature = "mocks")]
                    {
                        info!("Falling back to mock PostgreSQL client");
                        return Ok(Arc::new(MockPostgresClient::new()));
                    }
                    
                    #[cfg(not(feature = "mocks"))]
                    {
                        Err(Box::new(e))
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to create PostgreSQL client: {}", e);
            
            #[cfg(feature = "mocks")]
            {
                info!("Falling back to mock PostgreSQL client");
                return Ok(Arc::new(MockPostgresClient::new()));
            }
            
            #[cfg(not(feature = "mocks"))]
            {
                Err(Box::new(e))
            }
        }
    }
}

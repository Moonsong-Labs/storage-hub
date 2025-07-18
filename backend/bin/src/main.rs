//! StorageHub Backend Binary
//!
//! Main entry point for the StorageHub backend service.
//! This binary initializes the service with configuration, sets up storage and database
//! connections, and starts the HTTP server.

use sh_backend_lib::{
    api::create_app,
    config::Config,
    data::{
        postgres::PostgresClient,
        storage::{BoxedStorageWrapper, InMemoryStorage},
    },
    services::Services,
};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
    let postgres_client = match PostgresClient::new(&config.database.url).await {
        Ok(client) => {
            // Test the connection
            match client.test_connection().await {
                Ok(_) => {
                    info!("Connected to PostgreSQL at {}", config.database.url);
                    Arc::new(client)
                }
                Err(e) => {
                    error!("Failed to connect to PostgreSQL: {}", e);
                    info!("Starting without PostgreSQL connection - some features may be unavailable");
                    // For now, we'll exit. In a real implementation, you might want to:
                    // - Use a mock client
                    // - Start with limited functionality
                    // - Retry connection periodically
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            error!("Failed to create PostgreSQL client: {}", e);
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

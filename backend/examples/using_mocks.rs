//! Example demonstrating how to use mocks in StorageHub backend
//!
//! Run with: cargo run --example using_mocks --features mocks

#[cfg(feature = "mocks")]
use sh_backend_lib::{
    api::create_app,
    config::Config,
    data::postgres::MockPostgresClient,
    data::{
        postgres::PostgresClientTrait,
        storage::{BoxedStorageWrapper, InMemoryStorage},
    },
    services::Services,
};
use std::sync::Arc;

#[cfg(not(feature = "mocks"))]
fn main() {
    println!("This example requires the 'mocks' feature to be enabled.");
    println!("Run with: cargo run --example using_mocks --features mocks");
}

#[cfg(feature = "mocks")]
#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("StorageHub Backend Mock Example");
    println!("================================\n");

    // Create mock configuration
    let config = Config {
        host: "127.0.0.1".to_string(),
        port: 8080,
        storage_hub: sh_backend_lib::config::StorageHubConfig {
            rpc_url: "ws://localhost:9944".to_string(),
            mock_mode: true,
        },
        database: sh_backend_lib::config::DatabaseConfig {
            url: "postgres://localhost:5432/storage_hub".to_string(),
            mock_mode: true,
        },
    };

    println!("Configuration:");
    println!("  Host: {}:{}", config.host, config.port);
    println!("  Database Mock Mode: {}", config.database.mock_mode);
    println!("  StorageHub Mock Mode: {}\n", config.storage_hub.mock_mode);

    // Initialize storage
    let storage = Arc::new(BoxedStorageWrapper::new(InMemoryStorage::new()));
    println!("✓ Initialized in-memory storage");

    // Initialize mock PostgreSQL client
    let postgres_client: Arc<dyn PostgresClientTrait> = Arc::new(MockPostgresClient::new());
    println!("✓ Initialized mock PostgreSQL client");

    // Test the mock client
    println!("\nTesting mock PostgreSQL client:");

    // Test connection
    match postgres_client.test_connection().await {
        Ok(_) => println!("  ✓ Connection test passed"),
        Err(e) => println!("  ✗ Connection test failed: {}", e),
    }

    // Test getting a file
    let file_key = vec![70, 71, 72, 73];
    match postgres_client.get_file_by_key(&file_key).await {
        Ok(file) => {
            println!("  ✓ Found test file:");
            println!("    - File key: {:?}", file.file_key);
            println!("    - Size: {} bytes", file.size);
            println!("    - Bucket ID: {}", file.bucket_id);
        }
        Err(e) => println!("  ✗ Failed to get file: {}", e),
    }

    // Test getting files by user
    let user_account = vec![50, 51, 52, 53];
    match postgres_client.get_files_by_user(&user_account, None).await {
        Ok(files) => {
            println!(
                "  ✓ Found {} files for user {:?}",
                files.len(),
                user_account
            );
        }
        Err(e) => println!("  ✗ Failed to get user files: {}", e),
    }

    // Create services
    let services = Services::new(storage, postgres_client);
    println!("\n✓ Services initialized");

    // Create the application
    let app = create_app(services);
    println!("✓ Application created");

    println!("\nMock backend is ready!");
    println!("In a real scenario, you would start the server with:");
    println!("  axum::serve(listener, app).await");

    println!("\nExample completed successfully!");
}

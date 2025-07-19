//! Example demonstrating how to use the refactored PostgresClient with connection abstraction
//!
//! This example shows how PostgresClient can work with both real PostgreSQL connections
//! and mock connections for testing.

use sh_backend_lib::data::postgres::{
    DbConfig, DbConnection, PgConnection, PostgresClient, PostgresClientTrait,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Using PostgresClient with a real PostgreSQL connection
    println!("Example 1: Real PostgreSQL Connection");
    println!("=====================================");
    
    // Configure the database connection
    let config = DbConfig::new("postgres://user:password@localhost:5432/storagehub")
        .with_max_connections(10)
        .with_connection_timeout(30)
        .with_idle_timeout(600);

    // Create a PostgreSQL connection pool
    match PgConnection::new(config).await {
        Ok(pg_conn) => {
            // Create the PostgresClient with the connection
            let client = PostgresClient::new(Arc::new(pg_conn));
            
            // Test the connection
            match client.test_connection().await {
                Ok(()) => println!("✓ Successfully connected to PostgreSQL"),
                Err(e) => println!("✗ Connection failed: {}", e),
            }
            
            // Example: Query a file by key
            let file_key = vec![1, 2, 3, 4];
            match client.get_file_by_key(&file_key).await {
                Ok(file) => println!("✓ Found file: {:?}", file.hash),
                Err(e) => println!("✗ File not found: {}", e),
            }
        }
        Err(e) => {
            println!("✗ Could not create connection: {}", e);
            println!("  (This is expected if PostgreSQL is not running)");
        }
    }

    // Example 2: Using PostgresClient with a mock connection for testing
    #[cfg(feature = "mocks")]
    {
        use sh_backend_lib::data::postgres::{MockDbConnection, MockTestData};
        use shc_indexer_db::models::{File, FileStorageRequestStep};
        
        println!("\nExample 2: Mock Connection for Testing");
        println!("======================================");
        
        // Create mock test data
        let mut mock_data = MockTestData::new();
        
        // Add a test file to the mock data
        let test_file = File {
            id: 1,
            file_key: vec![1, 2, 3, 4],
            account: vec![5, 6, 7, 8],
            bucket_id: 1,
            file_step: FileStorageRequestStep::Requested,
            hash: "QmTest123".to_string(),
            created_at: None,
            updated_at: None,
        };
        mock_data.add_file(test_file);
        
        // Create a mock connection with the test data
        let mock_conn = MockDbConnection::new_with_data(mock_data);
        
        // Create the PostgresClient with the mock connection
        // Note: The client doesn't know it's using a mock!
        let client = PostgresClient::new(Arc::new(mock_conn));
        
        // Test operations work the same way
        match client.test_connection().await {
            Ok(()) => println!("✓ Mock connection test passed"),
            Err(e) => println!("✗ Mock connection test failed: {}", e),
        }
        
        // Query the mock data
        match client.get_file_by_key(&[1, 2, 3, 4]).await {
            Ok(file) => println!("✓ Found mock file: {}", file.hash),
            Err(e) => println!("✗ Mock file not found: {}", e),
        }
        
        // Try to find a non-existent file
        match client.get_file_by_key(&[99, 99, 99, 99]).await {
            Ok(_) => println!("✗ Should not have found this file"),
            Err(_) => println!("✓ Correctly returned error for non-existent file"),
        }
    }

    println!("\nKey Benefits of the Refactored Design:");
    println!("======================================");
    println!("1. PostgresClient now accepts any DbConnection implementation");
    println!("2. The same client code works with both real and mock connections");
    println!("3. Business logic in PostgresClient can be tested without a database");
    println!("4. The public API (PostgresClientTrait) remains unchanged");
    println!("5. Mock connections can simulate errors for comprehensive testing");

    Ok(())
}
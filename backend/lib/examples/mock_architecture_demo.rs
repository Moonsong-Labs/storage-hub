//! Demonstration of the new mock architecture
//!
//! This example shows how the refactored architecture enables testing
//! production code paths with mock data sources.
//!
//! Run with: cargo run --example mock_architecture_demo --features mocks

#[cfg(feature = "mocks")]
use sh_backend_lib::{
    data::{
        postgres::{
            DbConnection, MockDbConnection, MockErrorConfig,
            PostgresClient, PostgresClientTrait, PaginationParams,
        },
        rpc::{
            MockConnectionBuilder, StorageHubRpcClient, StorageHubRpcTrait,
            ErrorMode,
        },
    },
};
use chrono::NaiveDateTime;
use serde_json::json;
use shc_indexer_db::models::{File, FileStorageRequestStep};
use std::sync::Arc;
use std::time::Instant;

#[cfg(not(feature = "mocks"))]
fn main() {
    println!("This example requires the 'mocks' feature to be enabled.");
    println!("Run with: cargo run --example mock_architecture_demo --features mocks");
}

#[cfg(feature = "mocks")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Mock Architecture Demonstration ===\n");

    // Demonstrate PostgresClient with mock connection
    demonstrate_postgres_mock().await?;
    
    println!("\n" + "=".repeat(50) + "\n");
    
    // Demonstrate RPC client with mock connection
    demonstrate_rpc_mock().await?;
    
    println!("\n" + "=".repeat(50) + "\n");
    
    // Demonstrate error simulation
    demonstrate_error_simulation().await?;
    
    println!("\n" + "=".repeat(50) + "\n");
    
    // Demonstrate integration
    demonstrate_integration().await?;
    
    println!("\n✅ Mock architecture successfully demonstrates:");
    println!("   - Testing production code paths with mock data");
    println!("   - Error simulation capabilities");
    println!("   - Performance testing with delays");
    println!("   - Integration between components");

    Ok(())
}

#[cfg(feature = "mocks")]
async fn demonstrate_postgres_mock() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. PostgresClient with Mock Connection");
    println!("   ------------------------------------");
    
    // Create mock connection
    let mock_conn = MockDbConnection::new();
    
    // Add test data
    let test_file = File {
        id: 0,
        account: vec![10, 20, 30],
        file_key: vec![40, 50, 60],
        bucket_id: 1,
        location: vec![70, 80, 90],
        fingerprint: vec![100, 110, 120],
        size: 2048,
        step: FileStorageRequestStep::Stored as i32,
        created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
    };
    mock_conn.add_test_file(test_file);
    
    // Create client - THIS IS THE SAME CLIENT CODE AS PRODUCTION
    let client = PostgresClient::new(mock_conn);
    
    // Test operations
    println!("   - Testing connection...");
    client.test_connection().await?;
    println!("     ✓ Connection successful");
    
    println!("   - Retrieving file by key [40, 50, 60]...");
    let file = client.get_file_by_key(&[40, 50, 60]).await?;
    println!("     ✓ Found file with size: {} bytes", file.size);
    
    println!("   - Testing pagination...");
    let paginated = client.get_files_by_user(
        &[10, 20, 30],
        Some(PaginationParams {
            limit: Some(10),
            offset: Some(0),
        }),
    ).await?;
    println!("     ✓ Retrieved {} files", paginated.len());
    
    println!("\n   💡 Key insight: The SAME PostgresClient implementation");
    println!("      works with both real and mock connections!");

    Ok(())
}

#[cfg(feature = "mocks")]
async fn demonstrate_rpc_mock() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. StorageHubRpcClient with Mock Connection");
    println!("   -----------------------------------------");
    
    // Create mock connection with predefined responses
    let mut builder = MockConnectionBuilder::new();
    
    builder.add_response(
        "storagehub_getFileMetadata",
        json!([[10, 20, 30]]),
        json!({
            "owner": [50, 60, 70],
            "bucket_id": [80, 90, 100],
            "location": [110, 120, 130],
            "fingerprint": [140, 150, 160],
            "size": 4096,
            "peer_ids": [[170, 180], [190, 200]]
        }),
    );
    
    builder.add_response(
        "chain_getBlockNumber",
        json!([]),
        json!(12345),
    );
    
    let mock_conn = Arc::new(builder.build().await?);
    
    // Create client - SAME CLIENT CODE AS PRODUCTION
    let client = StorageHubRpcClient::new(mock_conn);
    
    println!("   - Getting file metadata...");
    let metadata = client.get_file_metadata(&[10, 20, 30]).await?;
    if let Some(metadata) = metadata {
        println!("     ✓ File size: {} bytes", metadata.size);
        println!("     ✓ Peer count: {}", metadata.peer_ids.len());
    }
    
    println!("   - Getting block number...");
    let block_num = client.get_block_number().await?;
    println!("     ✓ Current block: #{}", block_num);
    
    println!("\n   💡 Key insight: Production RPC client code is tested");
    println!("      without needing a real blockchain node!");

    Ok(())
}

#[cfg(feature = "mocks")]
async fn demonstrate_error_simulation() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Error Simulation Capabilities");
    println!("   ------------------------------");
    
    // Database error simulation
    let mock_db = MockDbConnection::new();
    mock_db.set_error_config(MockErrorConfig {
        connection_error: Some("Database connection lost".to_string()),
        ..Default::default()
    });
    
    let db_client = PostgresClient::new(mock_db.clone());
    
    println!("   - Simulating database connection failure...");
    match db_client.test_connection().await {
        Err(e) => println!("     ✓ Error caught: {}", e),
        Ok(_) => println!("     ✗ Expected error but got success"),
    }
    
    // Reset error and add delay
    mock_db.set_error_config(MockErrorConfig {
        delay_ms: Some(200),
        ..Default::default()
    });
    
    println!("   - Simulating 200ms network delay...");
    let start = Instant::now();
    let _ = db_client.test_connection().await;
    let elapsed = start.elapsed();
    println!("     ✓ Operation took {}ms", elapsed.as_millis());
    
    // RPC error simulation
    let mock_rpc = MockConnectionBuilder::new().build().await?;
    mock_rpc.set_error_mode(ErrorMode::Timeout);
    
    let rpc_client = StorageHubRpcClient::new(Arc::new(mock_rpc));
    
    println!("   - Simulating RPC timeout...");
    match rpc_client.get_block_number().await {
        Err(e) => println!("     ✓ Timeout error: {}", e),
        Ok(_) => println!("     ✗ Expected timeout but got success"),
    }
    
    println!("\n   💡 Key insight: Complex error scenarios can be tested");
    println!("      without relying on unreliable external conditions!");

    Ok(())
}

#[cfg(feature = "mocks")]
async fn demonstrate_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Integration Between Components");
    println!("   --------------------------------");
    
    // Setup mock database
    let mock_db = MockDbConnection::new();
    let test_file = File {
        id: 0,
        account: vec![1, 2, 3],
        file_key: vec![10, 20, 30],
        bucket_id: 1,
        location: vec![40, 50, 60],
        fingerprint: vec![70, 80, 90],
        size: 8192,
        step: FileStorageRequestStep::Requested as i32,
        created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
    };
    mock_db.add_test_file(test_file);
    
    // Setup mock RPC
    let mut rpc_builder = MockConnectionBuilder::new();
    rpc_builder.add_response(
        "storagehub_getFileMetadata",
        json!([[10, 20, 30]]),
        json!({
            "owner": [1, 2, 3],
            "bucket_id": [100, 110, 120],
            "location": [40, 50, 60],
            "fingerprint": [70, 80, 90],
            "size": 8192,
            "peer_ids": [[200, 210], [220, 230]]
        }),
    );
    
    // Create clients
    let db_client = PostgresClient::new(mock_db);
    let rpc_client = StorageHubRpcClient::new(Arc::new(rpc_builder.build().await?));
    
    println!("   - Workflow: Get file from DB, then verify on-chain...");
    
    // Get from database
    let db_file = db_client.get_file_by_key(&[10, 20, 30]).await?;
    println!("     ✓ Retrieved file from database (size: {})", db_file.size);
    
    // Verify on blockchain
    let chain_metadata = rpc_client.get_file_metadata(&db_file.file_key).await?;
    if let Some(metadata) = chain_metadata {
        println!("     ✓ Verified on blockchain (size: {})", metadata.size);
        println!("     ✓ Data consistency: {}", 
            if metadata.size == db_file.size as u64 { "PASS" } else { "FAIL" });
    }
    
    println!("\n   💡 Key insight: Mock connections enable testing");
    println!("      complex workflows and integrations!");

    Ok(())
}
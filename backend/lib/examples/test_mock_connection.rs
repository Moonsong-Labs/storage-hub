//! Example demonstrating the mock connection usage

use sh_backend_lib::data::postgres::{DbConnection, MockDbConnection, MockErrorConfig};
use shc_indexer_db::models::{File, FileStorageRequestStep};
use chrono::NaiveDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing MockDbConnection...");

    // Create a new mock connection
    let mock_conn = MockDbConnection::new();

    // Test basic connection
    println!("Testing basic connection...");
    let conn = mock_conn.get_connection().await?;
    println!("✓ Connection obtained successfully");

    // Test health check
    println!("\nTesting health check...");
    assert!(mock_conn.is_healthy().await);
    println!("✓ Connection is healthy");

    // Add test data
    println!("\nAdding test data...");
    let test_file = File {
        id: 0, // Will be auto-assigned
        account: vec![1, 2, 3],
        file_key: vec![4, 5, 6],
        bucket_id: 1,
        location: vec![7, 8, 9],
        fingerprint: vec![10, 11, 12],
        size: 1024,
        step: FileStorageRequestStep::Stored as i32,
        created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
    };
    
    mock_conn.add_test_file(test_file);
    println!("✓ Test file added");

    // Verify test data
    {
        let data = mock_conn.get_test_data();
        assert_eq!(data.files.len(), 1);
        println!("✓ Test data verified: {} files", data.files.len());
    }

    // Test error simulation
    println!("\nTesting error simulation...");
    mock_conn.set_error_config(MockErrorConfig {
        connection_error: Some("Simulated failure".to_string()),
        ..Default::default()
    });

    let result = mock_conn.get_connection().await;
    assert!(result.is_err());
    println!("✓ Error simulation working");

    // Reset error config
    mock_conn.set_error_config(MockErrorConfig::default());

    // Test delay simulation
    println!("\nTesting delay simulation...");
    mock_conn.set_error_config(MockErrorConfig {
        delay_ms: Some(100),
        ..Default::default()
    });

    let start = std::time::Instant::now();
    let _conn = mock_conn.get_connection().await?;
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() >= 100);
    println!("✓ Delay simulation working ({}ms)", elapsed.as_millis());

    println!("\n✅ All tests passed!");

    Ok(())
}
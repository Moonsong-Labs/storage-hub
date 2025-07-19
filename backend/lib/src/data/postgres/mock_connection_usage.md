# MockDbConnection Usage Guide

The `MockDbConnection` provides a test implementation of the `DbConnection` trait that simulates database behavior for testing purposes.

## Features

- **In-memory test data storage**: Stores files, buckets, and MSPs in memory
- **Thread-safe**: Uses `Arc<Mutex<>>` for concurrent access
- **Error simulation**: Can simulate connection failures, timeouts, and query errors
- **Delay injection**: Can add artificial delays for performance testing
- **Auto-incrementing IDs**: Automatically assigns IDs to new entities
- **Default test data**: Comes with pre-populated MSP and bucket for quick testing

## Basic Usage

```rust
use sh_backend_lib::data::postgres::{DbConnection, MockDbConnection, PostgresClient};

// Create a mock connection
let mock_conn = MockDbConnection::new();

// Use it with PostgresClient
let client = PostgresClient::new(mock_conn);

// Now use the client normally
let files = client.get_files_by_user(&[1, 2, 3], None).await?;
```

## Adding Test Data

```rust
use shc_indexer_db::models::{File, Bucket, Msp};

let mock_conn = MockDbConnection::new();

// Add a test file
let file = File {
    id: 0,  // Will be auto-assigned
    account: vec![1, 2, 3],
    file_key: vec![4, 5, 6],
    bucket_id: 1,
    // ... other fields
};
mock_conn.add_test_file(file);

// Add a test bucket
let bucket = Bucket {
    id: 0,  // Will be auto-assigned
    msp_id: Some(1),
    // ... other fields
};
mock_conn.add_test_bucket(bucket);

// Add a test MSP
let msp = Msp {
    id: 0,  // Will be auto-assigned
    // ... other fields
};
mock_conn.add_test_msp(msp);
```

## Error Simulation

```rust
use sh_backend_lib::data::postgres::MockErrorConfig;

// Simulate connection failure
mock_conn.set_error_config(MockErrorConfig {
    connection_error: Some("Database unavailable".to_string()),
    ..Default::default()
});

// Simulate timeout
mock_conn.set_error_config(MockErrorConfig {
    timeout_error: true,
    ..Default::default()
});

// Simulate query error
mock_conn.set_error_config(MockErrorConfig {
    query_error: Some("Invalid query".to_string()),
    ..Default::default()
});

// Add delay to operations
mock_conn.set_error_config(MockErrorConfig {
    delay_ms: Some(500),  // 500ms delay
    ..Default::default()
});

// Reset to normal operation
mock_conn.set_error_config(MockErrorConfig::default());
```

## Health Check Simulation

```rust
// Set unhealthy state
mock_conn.set_healthy(false);
assert!(!mock_conn.is_healthy().await);

// Set healthy state
mock_conn.set_healthy(true);
assert!(mock_conn.is_healthy().await);
```

## Accessing Test Data

```rust
// Get direct access to test data for assertions
{
    let data = mock_conn.get_test_data();
    assert_eq!(data.files.len(), 5);
    assert_eq!(data.buckets.len(), 2);
    assert_eq!(data.msps.len(), 1);
}

// Clear all test data
mock_conn.clear_data();
```

## Default Test Data

The mock connection comes with pre-populated test data:

- **Default MSP** (ID: 1)
  - onchain_msp_id: `[1, 2, 3, 4]`
  - account: `[10, 11, 12, 13]`
  - value_prop: `[100, 101, 102]`

- **Default Bucket** (ID: 1)
  - msp_id: 1
  - account: hex encoded `[50, 51, 52, 53]`
  - onchain_bucket_id: `[30, 31, 32, 33]`
  - name: `[110, 111, 112, 113]`

## Integration Testing Example

```rust
#[tokio::test]
async fn test_file_operations() {
    let mock_conn = MockDbConnection::new();
    let client = PostgresClient::new(mock_conn.clone());

    // Add test data
    for i in 1..=10 {
        let file = create_test_file(i);
        mock_conn.add_test_file(file);
    }

    // Test pagination
    let page1 = client.get_files_by_user(
        &[1, 2, 3],
        Some(PaginationParams { limit: Some(5), offset: Some(0) })
    ).await.unwrap();
    assert_eq!(page1.len(), 5);

    // Simulate network latency
    mock_conn.set_error_config(MockErrorConfig {
        delay_ms: Some(100),
        ..Default::default()
    });

    // Test with simulated latency
    let start = std::time::Instant::now();
    let page2 = client.get_files_by_user(&[1, 2, 3], None).await.unwrap();
    assert!(start.elapsed().as_millis() >= 100);
}
```

## Notes

- The mock connection does not actually execute SQL queries
- It provides connection-level mocking, allowing the real `PostgresClient` logic to be tested
- Thread-safe for use in concurrent tests
- IDs are automatically assigned when set to 0
- All operations are performed in-memory
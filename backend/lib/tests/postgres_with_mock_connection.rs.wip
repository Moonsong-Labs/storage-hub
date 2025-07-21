//! Integration test demonstrating PostgresClient with MockDbConnection

#[cfg(feature = "mocks")]
mod tests {
    use sh_backend_lib::data::postgres::{
        DbConnection, MockDbConnection, MockErrorConfig, PostgresClient, PostgresClientTrait,
        PaginationParams,
    };
    use shc_indexer_db::models::{Bucket, File, FileStorageRequestStep, Msp};
    use chrono::NaiveDateTime;

    #[tokio::test]
    async fn test_postgres_client_with_mock_connection() {
        // Create mock connection with test data
        let mock_conn = MockDbConnection::new();

        // Add some test files
        for i in 1..=5 {
            let file = File {
                id: 0, // Will be auto-assigned
                account: vec![50, 51, 52, 53], // User account
                file_key: vec![70 + i, 71 + i, 72 + i, 73 + i],
                bucket_id: 1,
                location: vec![80 + i, 81 + i, 82 + i, 83 + i],
                fingerprint: vec![90 + i, 91 + i, 92 + i, 93 + i],
                size: 1024 * i as i64,
                step: FileStorageRequestStep::Stored as i32,
                created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000 + i as i64, 0).unwrap(),
                updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000 + i as i64, 0).unwrap(),
            };
            mock_conn.add_test_file(file);
        }

        // Create PostgresClient with mock connection
        let client = PostgresClient::new(mock_conn);

        // Test connection
        assert!(client.test_connection().await.is_ok());

        // Test getting files by user
        let user_files = client
            .get_files_by_user(&[50, 51, 52, 53], None)
            .await
            .unwrap();
        assert_eq!(user_files.len(), 5);

        // Test pagination
        let paginated_files = client
            .get_files_by_user(
                &[50, 51, 52, 53],
                Some(PaginationParams {
                    limit: Some(2),
                    offset: Some(1),
                }),
            )
            .await
            .unwrap();
        assert_eq!(paginated_files.len(), 2);

        // Test getting a specific file
        let file = client.get_file_by_key(&[71, 72, 73, 74]).await.unwrap();
        assert_eq!(file.size, 1024);

        // Test file not found
        let not_found = client.get_file_by_key(&[99, 99, 99, 99]).await;
        assert!(not_found.is_err());
    }

    #[tokio::test]
    async fn test_error_handling_with_mock_connection() {
        let mock_conn = MockDbConnection::new();

        // Configure connection errors
        mock_conn.set_error_config(MockErrorConfig {
            connection_error: Some("Database unavailable".to_string()),
            ..Default::default()
        });

        let client = PostgresClient::new(mock_conn.clone());

        // Test that operations fail with connection error
        let result = client.test_connection().await;
        assert!(result.is_err());

        // Reset error config
        mock_conn.set_error_config(MockErrorConfig::default());

        // Now operations should succeed
        assert!(client.test_connection().await.is_ok());
    }

    #[tokio::test]
    async fn test_timeout_simulation() {
        let mock_conn = MockDbConnection::new();

        // Configure timeout
        mock_conn.set_error_config(MockErrorConfig {
            timeout_error: true,
            ..Default::default()
        });

        let client = PostgresClient::new(mock_conn);

        // Operations should timeout
        let result = client.get_all_msps(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_msp_and_bucket_operations() {
        let mock_conn = MockDbConnection::new();

        // Add test MSP
        let test_msp = Msp {
            id: 0, // Will be auto-assigned (2)
            onchain_msp_id: vec![5, 6, 7, 8],
            account: vec![20, 21, 22, 23],
            value_prop: vec![200, 201, 202],
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        };
        mock_conn.add_test_msp(test_msp);

        // Add test bucket
        let test_bucket = Bucket {
            id: 0, // Will be auto-assigned (2)
            msp_id: Some(2),
            account: hex::encode(&[60, 61, 62, 63]),
            onchain_bucket_id: vec![40, 41, 42, 43],
            name: vec![120, 121, 122, 123],
            collection_id: None,
            private: true,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            merkle_root: vec![50, 51, 52, 53],
        };
        mock_conn.add_test_bucket(test_bucket);

        let client = PostgresClient::new(mock_conn);

        // Test getting all MSPs
        let msps = client.get_all_msps(None).await.unwrap();
        assert_eq!(msps.len(), 2); // Default + added

        // Test getting specific MSP
        let msp = client.get_msp_by_id(2).await.unwrap();
        assert_eq!(msp.onchain_msp_id, vec![5, 6, 7, 8]);

        // Test getting bucket
        let bucket = client.get_bucket_by_id(2).await.unwrap();
        assert_eq!(bucket.msp_id, Some(2));
        assert!(bucket.private);
    }

    #[tokio::test]
    async fn test_file_lifecycle() {
        let mock_conn = MockDbConnection::new();
        let client = PostgresClient::new(mock_conn);

        // Create a new file
        let new_file = File {
            id: 0,
            account: vec![100, 101, 102, 103],
            file_key: vec![200, 201, 202, 203],
            bucket_id: 1,
            location: vec![110, 111, 112, 113],
            fingerprint: vec![120, 121, 122, 123],
            size: 4096,
            step: FileStorageRequestStep::Requested as i32,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).unwrap(),
        };

        // Create file
        let created = client.create_file(new_file).await.unwrap();
        assert!(created.id > 0); // ID should be assigned

        // Update file step
        client
            .update_file_step(&[200, 201, 202, 203], FileStorageRequestStep::Stored)
            .await
            .unwrap();

        // Verify update
        let updated = client.get_file_by_key(&[200, 201, 202, 203]).await.unwrap();
        assert_eq!(updated.step, FileStorageRequestStep::Stored as i32);

        // Delete file
        client.delete_file(&[200, 201, 202, 203]).await.unwrap();

        // Verify deletion
        let deleted = client.get_file_by_key(&[200, 201, 202, 203]).await;
        assert!(deleted.is_err());
    }

    #[tokio::test]
    async fn test_delay_simulation_in_operations() {
        let mock_conn = MockDbConnection::new();

        // Configure 200ms delay
        mock_conn.set_error_config(MockErrorConfig {
            delay_ms: Some(200),
            ..Default::default()
        });

        let client = PostgresClient::new(mock_conn);

        // Measure operation time
        let start = std::time::Instant::now();
        let _ = client.get_all_msps(None).await;
        let elapsed = start.elapsed();

        // Should take at least 200ms due to delay
        assert!(elapsed.as_millis() >= 200);
    }
}
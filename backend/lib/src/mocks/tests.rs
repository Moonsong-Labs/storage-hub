//! Tests for mock implementations

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::data::postgres::{PaginationParams, PostgresClientTrait};
    use shc_indexer_db::models::FileStorageRequestStep;

    #[tokio::test]
    async fn test_mock_postgres_client() {
        let mock = MockPostgresClient::new();

        // Test connection
        assert!(mock.test_connection().await.is_ok());

        // Test getting a file by key
        let file_key = vec![70, 71, 72, 73];
        let file = mock.get_file_by_key(&file_key).await.unwrap();
        assert_eq!(file.file_key, file_key);
        assert_eq!(file.size, 1024);

        // Test getting files by user
        let user_account = vec![50, 51, 52, 53];
        let files = mock.get_files_by_user(&user_account, None).await.unwrap();
        assert_eq!(files.len(), 2);

        // Test pagination
        let pagination = PaginationParams {
            limit: Some(1),
            offset: Some(0),
        };
        let files = mock
            .get_files_by_user(&user_account, Some(pagination))
            .await
            .unwrap();
        assert_eq!(files.len(), 1);

        // Test getting non-existent file
        let missing_key = vec![99, 99, 99, 99];
        let result = mock.get_file_by_key(&missing_key).await;
        assert!(result.is_err());

        // Test update file step
        let result = mock
            .update_file_step(&file_key, FileStorageRequestStep::Stored)
            .await;
        assert!(result.is_ok());

        // Test getting bucket
        let bucket = mock.get_bucket_by_id(1).await.unwrap();
        assert_eq!(bucket.id, 1);
        assert_eq!(bucket.account, hex::encode(&[50, 51, 52, 53]));

        // Test getting MSP
        let msp = mock.get_msp_by_id(1).await.unwrap();
        assert_eq!(msp.id, 1);
        assert_eq!(msp.onchain_msp_id, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_mock_storage_hub_rpc() {
        use crate::mocks::rpc_mock::StorageHubRpcTrait;
        let mock = MockStorageHubRpc::new();

        // Test getting file metadata
        let file_key = vec![70, 71, 72, 73];
        let metadata = mock.get_file_metadata(&file_key).await.unwrap();
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.size, 1024);

        // Test getting bucket info
        let bucket_id = vec![30, 31, 32, 33];
        let bucket_info = mock.get_bucket_info(&bucket_id).await.unwrap();
        assert!(bucket_info.is_some());
        let bucket_info = bucket_info.unwrap();
        assert_eq!(bucket_info.capacity, 1_000_000_000);

        // Test block operations
        let block_num = mock.get_block_number().await.unwrap();
        assert_eq!(block_num, 1000);

        mock.advance_block(5);
        let block_num = mock.get_block_number().await.unwrap();
        assert_eq!(block_num, 1005);

        // Test submitting storage request
        let new_file_key = vec![80, 81, 82, 83];
        let tx_hash = mock
            .submit_storage_request(
                &new_file_key,
                &bucket_id,
                &[90, 91, 92, 93],
                &[100, 101, 102, 103],
                2048,
                vec![vec![60, 61, 62, 63]],
            )
            .await
            .unwrap();
        assert!(!tx_hash.is_empty());

        // Verify file was added
        let metadata = mock.get_file_metadata(&new_file_key).await.unwrap();
        assert!(metadata.is_some());
    }

    #[tokio::test]
    async fn test_mock_data_manipulation() {
        let mock = MockPostgresClient::new();

        // Clear all data
        mock.clear_data();

        // Verify no files exist
        let user_account = vec![50, 51, 52, 53];
        let files = mock.get_files_by_user(&user_account, None).await.unwrap();
        assert_eq!(files.len(), 0);

        // Add a test file
        use chrono::NaiveDateTime;
        use shc_indexer_db::models::File;

        let test_file = File {
            id: 100,
            account: vec![50, 51, 52, 53],
            file_key: vec![200, 201, 202, 203],
            bucket_id: 1,
            location: vec![210, 211, 212, 213],
            fingerprint: vec![220, 221, 222, 223],
            size: 4096,
            step: FileStorageRequestStep::Requested as i32,
            created_at: NaiveDateTime::from_timestamp_opt(1_700_002_000, 0).unwrap(),
            updated_at: NaiveDateTime::from_timestamp_opt(1_700_002_000, 0).unwrap(),
        };

        mock.add_test_file(test_file.clone());

        // Verify file was added
        let files = mock.get_files_by_user(&user_account, None).await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_key, test_file.file_key);
    }
}
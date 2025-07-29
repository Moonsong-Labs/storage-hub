//! Integration tests for the StorageHub RPC client

#[cfg(feature = "mocks")]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use sh_backend_lib::data::rpc::{
        AnyRpcConnection, MockConnection, RpcConfig, StorageHubRpcClient,
    };

    #[tokio::test]
    async fn test_client_with_mock_connection() {
        // Create a mock connection with predefined responses
        let mock_conn = MockConnection::new();

        // Setup response for get_file_metadata
        mock_conn.set_response(
            "storagehub_getFileMetadata",
            json!({
                "owner": [10, 20, 30],
                "bucket_id": [40, 50, 60],
                "location": [70, 80, 90],
                "fingerprint": [100, 110, 120],
                "size": 4096,
                "peer_ids": [[130, 140], [150, 160]]
            }),
        );

        // Setup response for get_block_number
        mock_conn.set_response("chain_getBlockNumber", json!(54321));

        // Wrap in AnyRpcConnection
        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));

        // Create the client
        let client = StorageHubRpcClient::new(connection);

        // Test get_file_metadata
        let metadata = client.get_file_metadata(&[1, 2, 3]).await.unwrap();
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        assert_eq!(metadata.owner, vec![10, 20, 30]);
        assert_eq!(metadata.size, 4096);
        assert_eq!(metadata.peer_ids.len(), 2);

        // Test get_block_number
        let block_number = client.get_block_number().await.unwrap();
        assert_eq!(block_number, 54321);
    }

    #[tokio::test]
    async fn test_client_with_multiple_operations() {
        let mock_conn = MockConnection::new();

        // Setup multiple responses
        mock_conn.set_response(
            "storagehub_getBucketInfo",
            json!({
                "owner": [8, 9, 10],
                "msp_id": [11, 12, 13],
                "root": [14, 15, 16],
                "user_peer_ids": [[17, 18], [19, 20], [21, 22]]
            }),
        );

        mock_conn.set_response(
            "storagehub_getProviderInfo",
            json!({
                "peer_id": [26, 27, 28],
                "root": [29, 30, 31],
                "capacity": 1000000,
                "data_used": 500000
            }),
        );

        mock_conn.set_response("storagehub_getStorageRequestStatus", json!("pending"));

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test multiple operations
        let bucket = client.get_bucket_info(&[5, 6, 7]).await.unwrap();
        assert!(bucket.is_some());
        let bucket = bucket.unwrap();
        assert_eq!(bucket.user_peer_ids.len(), 3);

        let provider = client.get_provider_info(&[23, 24, 25]).await.unwrap();
        assert!(provider.is_some());
        let provider = provider.unwrap();
        assert_eq!(provider.capacity, 1000000);
        assert_eq!(provider.data_used, 500000);

        let status = client
            .get_storage_request_status(&[32, 33, 34])
            .await
            .unwrap();
        assert_eq!(status, Some("pending".to_string()));
    }

    #[tokio::test]
    async fn test_submit_storage_request() {
        let mock_conn = MockConnection::new();

        // Setup response for submit_storage_request
        mock_conn.set_response("author_submitStorageRequest", json!("0xabc123def456"));

        // Setup response for get_transaction_receipt
        mock_conn.set_response(
            "storagehub_getTransactionReceipt",
            json!({
                "block_hash": [50, 60, 70],
                "block_number": 12345,
                "extrinsic_index": 3,
                "success": true
            }),
        );

        let connection = Arc::new(AnyRpcConnection::Mock(mock_conn));
        let client = StorageHubRpcClient::new(connection);

        // Test submit_storage_request
        let receipt = client
            .submit_storage_request(
                vec![1, 2, 3],
                vec![4, 5, 6],
                2048,
                vec![vec![7, 8], vec![9, 10]],
            )
            .await
            .unwrap();

        assert_eq!(receipt.block_number, 12345);
        assert_eq!(receipt.extrinsic_index, 3);
        assert!(receipt.success);
    }

    #[tokio::test]
    async fn test_websocket_connection_placeholder() {
        // This test demonstrates that we can switch between connection types
        // In production, this would use a real WebSocket connection
        use sh_backend_lib::data::rpc::{RpcConnectionError, WsConnection};

        let config = RpcConfig {
            url: "ws://localhost:9944".to_string(),
            ..Default::default()
        };

        // This should fail with a transport error in tests since no WebSocket server is running
        let result = WsConnection::new(config).await;
        assert!(result.is_err());

        // Verify it's a transport error
        match result.unwrap_err() {
            RpcConnectionError::Transport(msg) => {
                assert!(
                    msg.contains("Failed to connect"),
                    "Expected connection failure message"
                );
            }
            other => panic!("Expected Transport error, got: {:?}", other),
        }
    }
}

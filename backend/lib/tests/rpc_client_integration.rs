//! Integration tests for the StorageHub RPC client

#[cfg(feature = "mocks")]
mod tests {
    use sh_backend_lib::data::rpc::{
        StorageHubRpcClient, StorageHubRpcTrait, MockConnectionBuilder, 
        WsConnectionBuilder, RpcConnectionBuilder, RpcConfig,
    };
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_client_with_mock_connection() {
        // Create a mock connection with predefined responses
        let mut builder = MockConnectionBuilder::new();
        
        // Setup response for get_file_metadata
        builder.add_response(
            "storagehub_getFileMetadata",
            json!([[1, 2, 3]]),
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
        builder.add_response(
            "chain_getBlockNumber",
            json!([]),
            json!(54321),
        );
        
        // Build the mock connection
        let connection = Arc::new(builder.build().await.unwrap());
        
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
        let mut builder = MockConnectionBuilder::new();
        
        // Setup multiple responses
        builder.add_response(
            "storagehub_getBucketInfo",
            json!([[5, 6, 7]]),
            json!({
                "owner": [8, 9, 10],
                "msp_id": [11, 12, 13],
                "root": [14, 15, 16],
                "user_peer_ids": [[17, 18], [19, 20], [21, 22]]
            }),
        );
        
        builder.add_response(
            "storagehub_getProviderInfo",
            json!([[23, 24, 25]]),
            json!({
                "peer_id": [26, 27, 28],
                "root": [29, 30, 31],
                "capacity": 1000000,
                "data_used": 500000
            }),
        );
        
        builder.add_response(
            "storagehub_getStorageRequestStatus",
            json!([[32, 33, 34]]),
            json!("pending"),
        );
        
        let connection = Arc::new(builder.build().await.unwrap());
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
        
        let status = client.get_storage_request_status(&[32, 33, 34]).await.unwrap();
        assert_eq!(status, Some("pending".to_string()));
    }
    
    #[tokio::test]
    async fn test_submit_storage_request() {
        let mut builder = MockConnectionBuilder::new();
        
        // Setup response for submit_storage_request
        builder.add_response(
            "author_submitStorageRequest",
            json!({
                "location": [1, 2, 3],
                "fingerprint": [4, 5, 6],
                "size": 2048,
                "peer_ids": [[7, 8], [9, 10]]
            }),
            json!("0xabc123def456"),
        );
        
        // Setup response for get_transaction_receipt
        builder.add_response(
            "storagehub_getTransactionReceipt",
            json!(["0xabc123def456"]),
            json!({
                "block_hash": [50, 60, 70],
                "block_number": 12345,
                "extrinsic_index": 3,
                "success": true
            }),
        );
        
        let connection = Arc::new(builder.build().await.unwrap());
        let client = StorageHubRpcClient::new(connection);
        
        // Test submit_storage_request
        let receipt = client.submit_storage_request(
            vec![1, 2, 3],
            vec![4, 5, 6],
            2048,
            vec![vec![7, 8], vec![9, 10]],
        ).await.unwrap();
        
        assert_eq!(receipt.block_number, 12345);
        assert_eq!(receipt.extrinsic_index, 3);
        assert!(receipt.success);
    }
    
    #[tokio::test]
    #[should_panic(expected = "WS connection is not available in tests")]
    async fn test_websocket_connection_placeholder() {
        // This test demonstrates that we can switch between connection types
        // In production, this would use a real WebSocket connection
        let config = RpcConfig {
            url: "ws://localhost:9944".to_string(),
            ..Default::default()
        };
        
        let builder = WsConnectionBuilder::new(config);
        let _connection = builder.build().await.unwrap();
    }
}
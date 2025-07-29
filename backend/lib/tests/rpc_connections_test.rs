//! Integration tests for RPC connections

#[cfg(feature = "mocks")]
mod tests {
    use serde_json::Value;
    use sh_backend_lib::data::rpc::{MockConnection, RpcConnection};

    #[tokio::test]
    async fn test_mock_connection_with_storagehub_methods() {
        let conn = MockConnection::new();

        // Set up expected responses
        conn.set_response("storagehub_getFileMetadata", serde_json::json!({
            "owner": vec![0; 32],
            "bucket_id": vec![0; 32],
            "location": vec![0; 32],
            "fingerprint": vec![0; 32],
            "size": 1024,
            "peer_ids": []
        }));

        conn.set_response("storagehub_getBucketInfo", serde_json::json!({
            "owner": vec![0; 32],
            "msp_id": vec![0; 32],
            "root": vec![0; 32],
            "user_peer_ids": []
        }));

        conn.set_response("storagehub_getProviderInfo", serde_json::json!({
            "peer_id": vec![0; 32],
            "root": vec![0; 32],
            "capacity": 1000000,
            "data_used": 500000
        }));

        // Test StorageHub-specific methods
        let file_metadata: Value = conn.call("storagehub_getFileMetadata", ()).await.unwrap();
        assert_eq!(file_metadata["size"], 1024);

        let bucket_info: Value = conn.call("storagehub_getBucketInfo", ()).await.unwrap();
        assert!(bucket_info["user_peer_ids"].is_array());

        let provider_info: Value = conn.call("storagehub_getProviderInfo", ()).await.unwrap();
        assert_eq!(provider_info["capacity"], 1000000);
    }

    #[tokio::test]
    async fn test_mock_connection_error_simulation() {
        let conn = MockConnection::new();
        // Set up a successful response
        conn.set_response("system_health", serde_json::json!({"status": "healthy"}));

        // First call should succeed
        let _: Value = conn.call("system_health", ()).await.unwrap();

        // Call without a response set should return null
        let result: Value = conn.call("unknown_method", ()).await.unwrap();
        assert_eq!(result, serde_json::json!(null));
    }

    #[tokio::test]
    async fn test_ws_connection_config() {
        use sh_backend_lib::data::rpc::RpcConfig;

        // Just test that the config can be created
        // We can't actually connect without a real WebSocket server
        let config = RpcConfig {
            url: "ws://localhost:9944".to_string(),
            timeout_secs: Some(30),
            max_concurrent_requests: Some(100),
            verify_tls: true,
        };

        assert_eq!(config.url, "ws://localhost:9944");
        assert_eq!(config.timeout_secs, Some(30));
        assert_eq!(config.max_concurrent_requests, Some(100));
    }
}

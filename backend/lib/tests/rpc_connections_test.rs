//! Integration tests for RPC connections

#[cfg(feature = "mocks")]
mod tests {
    use sh_backend_lib::data::rpc::{
        MockConnection, MockConnectionBuilder, ErrorMode,
        WsConnectionBuilder, RpcConnection,
    };
    use serde_json::Value;

    #[tokio::test]
    async fn test_mock_connection_with_storagehub_methods() {
        let conn = MockConnectionBuilder::new()
            .build()
            .await
            .expect("Failed to build mock connection");

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
        let conn = MockConnectionBuilder::new()
            .with_error_mode(ErrorMode::FailAfterNCalls(1))
            .build()
            .await
            .expect("Failed to build mock connection");

        // First call should succeed
        let _: Value = conn.call("system_health", ()).await.unwrap();

        // Second call should fail
        let result: Result<Value, _> = conn.call("system_health", ()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ws_connection_builder() {
        // Just test that the builder can be created
        // We can't actually connect without a real WebSocket server
        let _builder = WsConnectionBuilder::new("ws://localhost:9944")
            .timeout_secs(30)
            .max_concurrent_requests(100);
    }
}
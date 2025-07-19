//! Example demonstrating how to use the StorageHub RPC client

use sh_backend_lib::data::rpc::{
    StorageHubRpcClient, StorageHubRpcTrait, WsConnectionBuilder,
    RpcConnectionBuilder, RpcConfig,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Using with a WebSocket connection (production)
    let config = RpcConfig {
        url: "ws://localhost:9944".to_string(),
        timeout_secs: Some(60),
        ..Default::default()
    };
    
    // Note: This would work in production but not in tests
    // let ws_builder = WsConnectionBuilder::new(config);
    // let ws_connection = Arc::new(ws_builder.build().await?);
    // let client = StorageHubRpcClient::new(ws_connection);
    
    // Example 2: Using with a mock connection (testing)
    #[cfg(feature = "mocks")]
    {
        use sh_backend_lib::data::rpc::MockConnectionBuilder;
        use serde_json::json;
        
        let mut mock_builder = MockConnectionBuilder::new();
        
        // Setup mock responses
        mock_builder.add_response(
            "chain_getBlockNumber",
            json!([]),
            json!(12345),
        );
        
        mock_builder.add_response(
            "storagehub_getFileMetadata",
            json!([[1, 2, 3]]),
            json!({
                "owner": [10, 11, 12],
                "bucket_id": [20, 21, 22],
                "location": [30, 31, 32],
                "fingerprint": [40, 41, 42],
                "size": 1024,
                "peer_ids": [[50, 51], [52, 53]]
            }),
        );
        
        let mock_connection = Arc::new(mock_builder.build().await?);
        let client = StorageHubRpcClient::new(mock_connection);
        
        // Use the client
        let block_number = client.get_block_number().await?;
        println!("Current block number: {}", block_number);
        
        let file_metadata = client.get_file_metadata(&[1, 2, 3]).await?;
        match file_metadata {
            Some(metadata) => {
                println!("File found!");
                println!("  Owner: {:?}", metadata.owner);
                println!("  Size: {} bytes", metadata.size);
                println!("  Peer count: {}", metadata.peer_ids.len());
            }
            None => {
                println!("File not found");
            }
        }
        
        // Submit a storage request
        let receipt = client.submit_storage_request(
            vec![100, 101, 102],  // location
            vec![200, 201, 202],  // fingerprint
            4096,                 // size
            vec![vec![1, 2], vec![3, 4]],  // peer_ids
        ).await;
        
        match receipt {
            Ok(r) => {
                println!("Storage request submitted successfully!");
                println!("  Block: {}", r.block_number);
                println!("  Success: {}", r.success);
            }
            Err(e) => {
                println!("Failed to submit storage request: {}", e);
            }
        }
    }
    
    Ok(())
}
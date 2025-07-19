//! Example to test RPC connections compilation

use sh_backend_lib::data::rpc::{
    RpcConnection, RpcConnectionBuilder,
    WsConnectionBuilder,
};

#[cfg(feature = "mocks")]
use sh_backend_lib::data::rpc::{
    MockConnectionBuilder, ErrorMode,
};

#[tokio::main]
async fn main() {
    println!("Testing RPC connections...");
    
    // Test WebSocket connection builder
    let ws_builder = WsConnectionBuilder::new("ws://localhost:9944")
        .timeout_secs(30)
        .max_concurrent_requests(100);
    
    println!("WebSocket connection builder created");
    
    #[cfg(feature = "mocks")]
    {
        // Test mock connection
        let mock_conn = MockConnectionBuilder::new()
            .with_error_mode(ErrorMode::None)
            .build()
            .await
            .expect("Failed to build mock connection");
        
        // Test a basic call
        let result: serde_json::Value = mock_conn
            .call("system_health", ())
            .await
            .expect("Failed to call system_health");
        
        println!("Mock connection test successful: {:?}", result);
    }
    
    println!("All tests passed!");
}
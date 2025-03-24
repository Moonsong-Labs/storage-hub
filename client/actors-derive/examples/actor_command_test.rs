use anyhow::Result;
use async_trait::async_trait;
use shc_actors_derive::{actor_command, command};
use shc_actors_framework::actor::ActorHandle;
use std::marker::PhantomData;

// Mock service for testing
pub struct TestService;

// Custom error type
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Test error occurred")]
    TestError,
}

// Define a command enum using the macro
#[actor_command(
    service = TestService,
    default_mode = "SyncAwait",
    default_error_type = TestError
)]
pub enum TestServiceCommand {
    // SyncAwait command (uses default)
    BasicCommand {
        value: String,
    },

    // AsyncAwait command with custom types
    #[command(mode = "AsyncAwait", success_type = Vec<u8>)]
    AsyncCommand {
        id: u32,
        data: Vec<u8>,
    },

    // FireAndForget command
    #[command(mode = "FireAndForget")]
    LogEvent {
        message: String,
        level: String,
    },
}

// Extension trait
#[async_trait]
pub trait TestServiceInterfaceExt {
    fn format_message(&self, value: &str) -> String;
}

#[async_trait]
impl<T> TestServiceInterfaceExt for ActorHandle<TestService<T>>
where
    T: Send + Sync + 'static,
{
    fn format_message(&self, value: &str) -> String {
        format!("Formatted: {}", value)
    }
}

#[tokio::main]
async fn main() {
    // Create fire-and-forget command
    let log_cmd = TestServiceCommand::LogEvent {
        message: "test".to_string(),
        level: "info".to_string(),
    };

    println!("Created commands:");
    println!("Basic command: {:?}", basic_cmd);
    println!("Async command: {:?}", async_cmd);
    println!("Log command: {:?}", log_cmd);
}

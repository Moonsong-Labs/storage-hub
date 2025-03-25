use anyhow::Result;
use async_trait::async_trait;
use shc_actors_derive::{actor_command, command};
use shc_actors_framework::actor::ActorHandle;
use std::marker::PhantomData;

// Test case 1: Simple generic type with multiple bounds
pub struct TestService<T: Send + Sync + 'static>;

// Test case 2: Service with no generics
pub struct BasicService;

// Test case 3: Service with multiple generics and complex bounds
pub struct ComplexService<T, U: Clone + Send, V: 'static> {
    t: PhantomData<T>,
    u: PhantomData<U>,
    v: PhantomData<V>,
}

// Custom error type
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Test error occurred")]
    TestError,
}

// Command for Test case 1
#[actor_command(
    service = TestService<T: Send + Sync + 'static>,
    default_mode = "SyncAwait",
    default_error_type = TestError
)]
pub enum TestServiceCommand {
    // SyncAwait command (uses default)
    #[command(success_type = ())]
    BasicCommand { value: String },

    // AsyncAwait command with custom types
    #[command(mode = "AsyncAwait", success_type = Vec<u8>)]
    AsyncCommand { id: u32, data: Vec<u8> },

    // FireAndForget command
    #[command(mode = "FireAndForget")]
    LogEvent { message: String, level: String },
}

// Command for Test case 2
#[actor_command(
    service = BasicService,
    default_mode = "SyncAwait"
)]
pub enum BasicServiceCommand {
    #[command(success_type = String)]
    Echo {
        message: String,
    },
    Ping,
}

// Command for Test case 3
#[actor_command(
    service = ComplexService<T, U: Clone + Send, V: 'static>,
    default_mode = "SyncAwait",
    default_error_type = TestError
)]
pub enum ComplexServiceCommand {
    #[command(mode = "AsyncAwait", success_type = String, inner_channel_type = tokio::sync::oneshot::Receiver<()>)]
    Process { value: String },

    #[command(mode = "AsyncAwait", success_type = Vec<u8>)]
    ComputeAsync { input: Vec<u8> },
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

// Extension trait for ComplexService
#[async_trait]
pub trait ComplexServiceInterfaceExt {
    fn transform<R>(&self, value: R) -> String
    where
        R: ToString + Send;
}

#[async_trait]
impl<T, U, V> ComplexServiceInterfaceExt for ActorHandle<ComplexService<T, U, V>>
where
    T: Send + Sync + 'static,
    U: Clone + Send + 'static,
    V: 'static,
{
    fn transform<R>(&self, value: R) -> String
    where
        R: ToString + Send,
    {
        format!("Transformed: {}", value.to_string())
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

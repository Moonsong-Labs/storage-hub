use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

use shc_actors_derive::actor_command;
use shc_actors_framework::actor::ActorHandle;

use crate::{handler::TelemetryService, types::TelemetryStrategy};

/// Commands that can be sent to the TelemetryService actor.
#[actor_command(
    service = TelemetryService,
    default_mode = "FireAndForget",
    default_inner_channel_type = tokio::sync::oneshot::Receiver,
)]
pub enum TelemetryServiceCommand {
    /// Queue a telemetry event for processing.
    /// This is fire-and-forget - the caller doesn't wait for confirmation.
    QueueEvent {
        event: JsonValue,
        strategy: TelemetryStrategy,
    },
}

/// Extension trait for TelemetryService command interface.
#[async_trait]
pub trait TelemetryServiceCommandInterfaceExt: TelemetryServiceCommandInterface {
    /// Queue a telemetry event using a typed event that implements TelemetryEvent.
    /// This is a convenience method that converts the event to JSON and extracts its strategy.
    async fn queue_typed_event<T>(&self, event: T) -> Result<()>
    where
        T: crate::types::TelemetryEvent + 'static + Send;
}

#[async_trait]
impl TelemetryServiceCommandInterfaceExt for ActorHandle<TelemetryService> {
    async fn queue_typed_event<T>(&self, event: T) -> Result<()>
    where
        T: crate::types::TelemetryEvent + 'static + Send,
    {
        let json = event.to_json();
        let strategy = event.strategy();
        
        // Fire and forget - we don't wait for the result
        self.queue_event(json, strategy).await;
        Ok(())
    }
}
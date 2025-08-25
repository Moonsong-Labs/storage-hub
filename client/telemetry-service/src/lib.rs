//! StorageHub Telemetry Service
//! 
//! This crate provides an actor-based telemetry service for the StorageHub client.
//! It uses a fire-and-forget pattern to ensure telemetry never blocks the main application.

pub mod commands;
pub mod handler;
pub mod telemetry;
pub mod types;

use std::sync::Arc;

use log::{error, info, warn};
use shc_actors_framework::actor::{ActorHandle, ActorSpawner, TaskSpawner};

pub use commands::{TelemetryServiceCommand, TelemetryServiceCommandInterface, TelemetryServiceCommandInterfaceExt};
pub use handler::TelemetryService;
pub use telemetry::events;
pub use types::{
    AxiomBackend, AxiomConfig, BaseTelemetryEvent, NoOpBackend, OverflowStrategy,
    TelemetryBackend, TelemetryConfig, TelemetryEvent, TelemetryStrategy,
};

use chrono::Utc;

/// Helper function to create a base telemetry event
pub fn create_base_event(event_type: &str, service: String, node_id: Option<String>) -> BaseTelemetryEvent {
    BaseTelemetryEvent {
        timestamp: Utc::now(),
        service,
        event_type: event_type.to_string(),
        node_id,
        correlation_id: None,
        span_id: None,
        parent_span_id: None,
    }
}

/// Spawn the telemetry service actor.
pub async fn spawn_telemetry_service(
    task_spawner: &TaskSpawner,
    service_name: String,
    node_id: Option<String>,
    axiom_token: Option<String>,
    axiom_dataset: Option<String>,
) -> Option<ActorHandle<TelemetryService>> {
    let task_spawner = task_spawner
        .with_name("telemetry-service")
        .with_group("telemetry");

    // Create backend based on configuration
    let backend: Arc<dyn TelemetryBackend> = match (axiom_token, axiom_dataset) {
        // Explicit configuration provided
        (Some(token), Some(dataset)) => {
            let axiom_config = AxiomConfig::new(token, dataset);
            match AxiomBackend::new(axiom_config) {
                Ok(backend) => {
                    info!("Axiom telemetry backend initialized successfully with provided config");
                    Arc::new(backend)
                }
                Err(e) => {
                    error!("Failed to initialize Axiom backend: {}", e);
                    Arc::new(NoOpBackend)
                }
            }
        }
        // Fall back to environment variables
        _ => {
            if let Some(axiom_config) = AxiomConfig::from_env() {
                match AxiomBackend::new(axiom_config) {
                    Ok(backend) => {
                        info!("Axiom telemetry backend initialized successfully from environment");
                        Arc::new(backend)
                    }
                    Err(e) => {
                        error!("Failed to initialize Axiom backend: {}", e);
                        Arc::new(NoOpBackend)
                    }
                }
            } else {
                warn!("Axiom configuration not found. Telemetry disabled.");
                return None;
            }
        }
    };

    // Don't spawn if backend is not enabled
    if !backend.is_enabled() {
        info!("Telemetry backend is not enabled, skipping telemetry service");
        return None;
    }

    let telemetry_service = TelemetryService::new(
        service_name,
        node_id,
        backend,
        TelemetryConfig::default(),
    );

    Some(task_spawner.spawn_actor(telemetry_service))
}
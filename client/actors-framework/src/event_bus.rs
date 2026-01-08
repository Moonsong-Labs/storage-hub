use anyhow::Result;
use sc_tracing::tracing::{error, info, warn};
use std::sync::Arc;
use tokio::sync::{broadcast, Semaphore};

use crate::{
    actor::{Actor, ActorHandle, TaskSpawner},
    constants::{MAX_PENDING_EVENTS, MAX_TASKS_SPAWNED_PER_QUEUE},
};
use shc_telemetry::{
    dec_gauge, inc_counter, observe_histogram, MetricsLink, STATUS_FAILURE, STATUS_SUCCESS,
};

pub trait EventBusMessage: Clone + Send + 'static {}

#[derive(Clone)]
pub struct EventBus<T: EventBusMessage> {
    sender: broadcast::Sender<T>,
}

impl<T: EventBusMessage> Default for EventBus<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: EventBusMessage + Clone> EventBus<T> {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(MAX_PENDING_EVENTS);
        Self { sender }
    }

    pub fn emit(&self, event: T) {
        // We log that there is no listener.
        match self.sender.send(event) {
            Ok(_) => {}
            Err(_) => {
                warn!("No listener for emitted event.");
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<T> {
        self.sender.subscribe()
    }
}

pub trait ProvidesEventBus<T: EventBusMessage> {
    fn event_bus(&self) -> &EventBus<T>;
}

pub trait EventHandler<E: EventBusMessage>: Clone + Send + 'static {
    /// Handle a single event.
    ///
    /// On success, returns a human-readable message that will be logged centrally by the
    /// event bus listener. This encourages each handler to provide a meaningful success
    /// description.
    fn handle_event(
        &mut self,
        event: E,
    ) -> impl std::future::Future<Output = Result<String>> + Send;

    fn subscribe_to_provider<EP: ProvidesEventBus<E>>(
        self,
        task_spawner: &TaskSpawner,
        provider: &EP,
        critical: bool,
        metrics_config: Option<EventMetricsConfig>,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
    {
        let receiver = provider.event_bus().subscribe();
        EventBusListener::new(
            task_spawner.clone(),
            self,
            receiver,
            critical,
            metrics_config,
        )
    }

    fn subscribe_to<A: Actor>(
        self,
        task_spawner: &TaskSpawner,
        actor_handle: &ActorHandle<A>,
        critical: bool,
        metrics_config: Option<EventMetricsConfig>,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
        <A as Actor>::EventBusProvider: ProvidesEventBus<E>,
    {
        self.subscribe_to_provider(
            task_spawner,
            &actor_handle.event_bus_provider,
            critical,
            metrics_config,
        )
    }
}

pub struct EventBusListener<T: EventBusMessage, E: EventHandler<T>> {
    spawner: TaskSpawner,
    receiver: broadcast::Receiver<T>,
    event_handler: E,
    semaphore: Arc<Semaphore>,
    // Indicate if the event is critical or not and if the receiver can drop it safely or have to panic.
    critical: bool,
    metrics_config: Option<EventMetricsConfig>,
}

impl<T: EventBusMessage, E: EventHandler<T> + Send + 'static> EventBusListener<T, E> {
    pub fn new(
        spawner: TaskSpawner,
        event_handler: E,
        receiver: broadcast::Receiver<T>,
        critical: bool,
        metrics_config: Option<EventMetricsConfig>,
    ) -> Self {
        Self {
            spawner: spawner.with_group("event-handler-worker"),
            event_handler,
            receiver,
            semaphore: Arc::new(Semaphore::new(MAX_TASKS_SPAWNED_PER_QUEUE)),
            critical,
            metrics_config,
        }
    }

    async fn run(&mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    // Record pending and start timer BEFORE spawning
                    if let Some(ref config) = self.metrics_config {
                        config.record_pending();
                    }
                    let start_time = std::time::Instant::now();

                    let metrics_config = self.metrics_config.clone();
                    let mut cloned_event_handler = self.event_handler.clone();
                    let permit = Arc::clone(&self.semaphore)
                        .acquire_owned()
                        .await
                        .expect("To acquire the permit");
                    self.spawner.spawn(async move {
                        let result = cloned_event_handler.handle_event(event).await;
                        let duration_secs = start_time.elapsed().as_secs_f64();

                        match result {
                            Ok(msg) => {
                                if let Some(ref config) = metrics_config {
                                    config.record_success(duration_secs);
                                }
                                info!("Task completed successfully: {}", msg);
                            }
                            Err(error) => {
                                if let Some(ref config) = metrics_config {
                                    config.record_failure(duration_secs);
                                }
                                warn!("Task ended with error: {:?}", error);
                            }
                        };
                        drop(permit);
                    });
                }
                Err(broadcast::error::RecvError::Lagged(_)) if self.critical => {
                    // If we have dropped critical events (critical events could be runtime events) we are panicking. The node can be in an incoherent state. The node must stop.
                    error!("CRITICAL❗️❗️ The receiver lagged behind for critical events and some events have been not been processed. (events type {})", std::any::type_name::<T>());
                    panic!("Some events have not been processed. The node could be an incoherent state.");
                }
                Err(broadcast::error::RecvError::Lagged(num_skipped_message)) => {
                    // If the receiver has dropped message from peers, it is not too bad. We are expecting it to retry.
                    // Dropping messages avoid filling the queue and spawning unbounded amount of task
                    warn!("The receiver lagged behind. Old messages are being overwritten by new ({} skipped message)", num_skipped_message);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Closing listener. No more active sender for this event bus.");
                    break;
                }
            }
        }
    }

    pub fn start(mut self) {
        let spawner = self.spawner.with_group("event-bus-listener");
        spawner.spawn(async move { self.run().await });
    }
}

/// Configuration for event metrics recording.
///
/// Holds the metrics link and the event name for recording event handler metrics.
#[derive(Clone)]
pub struct EventMetricsConfig {
    metrics: MetricsLink,
    event_name: &'static str,
}

impl EventMetricsConfig {
    /// Creates a new event metrics configuration.
    pub fn new(metrics: MetricsLink, event_name: &'static str) -> Self {
        Self {
            metrics,
            event_name,
        }
    }

    /// Record that an event handler has started (pending state).
    pub fn record_pending(&self) {
        inc_counter!(metrics: self.metrics.as_ref(), event_handler_pending, self.event_name);
    }

    /// Record that an event handler completed successfully.
    pub fn record_success(&self, duration_secs: f64) {
        dec_gauge!(metrics: self.metrics.as_ref(), event_handler_pending, self.event_name);
        inc_counter!(metrics: self.metrics.as_ref(), event_handler_total, labels: &[self.event_name, STATUS_SUCCESS]);
        observe_histogram!(metrics: self.metrics.as_ref(), event_handler_seconds, labels: &[self.event_name, STATUS_SUCCESS], duration_secs);
    }

    /// Record that an event handler failed.
    pub fn record_failure(&self, duration_secs: f64) {
        dec_gauge!(metrics: self.metrics.as_ref(), event_handler_pending, self.event_name);
        inc_counter!(metrics: self.metrics.as_ref(), event_handler_total, labels: &[self.event_name, STATUS_FAILURE]);
        observe_histogram!(metrics: self.metrics.as_ref(), event_handler_seconds, labels: &[self.event_name, STATUS_FAILURE], duration_secs);
    }
}

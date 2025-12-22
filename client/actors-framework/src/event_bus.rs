use anyhow::Result;
use sc_tracing::tracing::{error, info, warn};
use std::sync::Arc;
use tokio::sync::{broadcast, Semaphore};

use crate::{
    actor::{Actor, ActorHandle, TaskSpawner},
    constants::{MAX_PENDING_EVENTS, MAX_TASKS_SPAWNED_PER_QUEUE},
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

    fn subscribe_to_provider<EP: ProvidesEventBus<E>, M: LifecycleMetricRecorder>(
        self,
        task_spawner: &TaskSpawner,
        provider: &EP,
        critical: bool,
        metric_recorder: M,
    ) -> EventBusListener<E, Self, M>
    where
        Self: Sized + Send,
    {
        let receiver = provider.event_bus().subscribe();
        EventBusListener::new(
            task_spawner.clone(),
            self,
            receiver,
            critical,
            metric_recorder,
        )
    }

    fn subscribe_to<A: Actor, M: LifecycleMetricRecorder>(
        self,
        task_spawner: &TaskSpawner,
        actor_handle: &ActorHandle<A>,
        critical: bool,
        metric_recorder: M,
    ) -> EventBusListener<E, Self, M>
    where
        Self: Sized + Send,
        <A as Actor>::EventBusProvider: ProvidesEventBus<E>,
    {
        self.subscribe_to_provider(
            task_spawner,
            &actor_handle.event_bus_provider,
            critical,
            metric_recorder,
        )
    }
}

pub struct EventBusListener<
    T: EventBusMessage,
    E: EventHandler<T>,
    M: LifecycleMetricRecorder = NoOpMetricRecorder,
> {
    spawner: TaskSpawner,
    receiver: broadcast::Receiver<T>,
    event_handler: E,
    semaphore: Arc<Semaphore>,
    // Indicate if the event is critical or not and if the receiver can drop it safely or have to panic.
    critical: bool,
    metric_recorder: M,
}

impl<T: EventBusMessage, E: EventHandler<T> + Send + 'static, M: LifecycleMetricRecorder>
    EventBusListener<T, E, M>
{
    pub fn new(
        spawner: TaskSpawner,
        event_handler: E,
        receiver: broadcast::Receiver<T>,
        critical: bool,
        metric_recorder: M,
    ) -> Self {
        Self {
            spawner: spawner.with_group("event-handler-worker"),
            event_handler,
            receiver,
            semaphore: Arc::new(Semaphore::new(MAX_TASKS_SPAWNED_PER_QUEUE)),
            critical,
            metric_recorder,
        }
    }

    async fn run(&mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    // Record pending and start timer BEFORE spawning
                    self.metric_recorder.record_pending();
                    let start_time = std::time::Instant::now();

                    let metric_recorder = self.metric_recorder.clone();
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
                                metric_recorder.record_success(duration_secs);
                                info!("Task completed successfully: {}", msg);
                            }
                            Err(error) => {
                                metric_recorder.record_failure(duration_secs);
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

/// Trait for recording event handler lifecycle metrics.
/// Tracks: pending (event received), success/failure (handler result), and duration.
/// Implemented in downstream crates with concrete metrics.
pub trait LifecycleMetricRecorder: Clone + Send + Sync + 'static {
    /// Record that an event was received and handler is starting (pending state)
    fn record_pending(&self) {}

    /// Record that the handler completed successfully, with duration in seconds
    fn record_success(&self, _duration_secs: f64) {}

    /// Record that the handler failed, with duration in seconds
    fn record_failure(&self, _duration_secs: f64) {}
}

/// No-op implementation when metrics are disabled or not specified.
#[derive(Clone, Default)]
pub struct NoOpMetricRecorder;

impl LifecycleMetricRecorder for NoOpMetricRecorder {}

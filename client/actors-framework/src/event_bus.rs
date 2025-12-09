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

    fn subscribe_to_provider<EP: ProvidesEventBus<E>>(
        self,
        task_spawner: &TaskSpawner,
        provider: &EP,
        critical: bool,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
    {
        let receiver = provider.event_bus().subscribe();
        EventBusListener::new(task_spawner.clone(), self, receiver, critical)
    }

    fn subscribe_to<A: Actor>(
        self,
        task_spawner: &TaskSpawner,
        actor_handle: &ActorHandle<A>,
        critical: bool,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
        <A as Actor>::EventBusProvider: ProvidesEventBus<E>,
    {
        self.subscribe_to_provider(task_spawner, &actor_handle.event_bus_provider, critical)
    }
}

pub struct EventBusListener<T: EventBusMessage, E: EventHandler<T>> {
    spawner: TaskSpawner,
    receiver: broadcast::Receiver<T>,
    event_handler: E,
    semaphore: Arc<Semaphore>,
    // Indicate if the event is critical or not and if the receiver can drop it safely or have to panic.
    critical: bool,
}

impl<T: EventBusMessage, E: EventHandler<T> + Send + 'static> EventBusListener<T, E> {
    pub fn new(
        spawner: TaskSpawner,
        event_handler: E,
        receiver: broadcast::Receiver<T>,
        critical: bool,
    ) -> Self {
        Self {
            spawner: spawner.with_group("event-handler-worker"),
            event_handler,
            receiver,
            semaphore: Arc::new(Semaphore::new(MAX_TASKS_SPAWNED_PER_QUEUE)),
            critical,
        }
    }

    async fn run(&mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    let mut cloned_event_handler = self.event_handler.clone();
                    let permit = Arc::clone(&self.semaphore)
                        .acquire_owned()
                        .await
                        .expect("To acquire the permit");
                    self.spawner.spawn(async move {
                        match cloned_event_handler.handle_event(event).await {
                            Ok(msg) => {
                                info!("Task completed successfully: {}", msg);
                            }
                            Err(error) => {
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

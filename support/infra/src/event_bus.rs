use anyhow::Result;
use sc_tracing::tracing::warn;
use tokio::sync::broadcast;

use crate::{
    actor::{Actor, ActorHandle, TaskSpawner},
    constants::MAX_PENDING_EVENTS,
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
    fn handle_event(&self, event: E) -> impl std::future::Future<Output = Result<()>> + Send;

    fn subscribe_to_provider<EP: ProvidesEventBus<E>>(
        self,
        task_spawner: &TaskSpawner,
        provider: &EP,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
    {
        let receiver = provider.event_bus().subscribe();
        EventBusListener::new(task_spawner.clone(), self, receiver)
    }

    fn subscribe_to<A: Actor>(
        self,
        task_spawner: &TaskSpawner,
        actor_handle: &ActorHandle<A>,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
        <A as Actor>::EventBusProvider: ProvidesEventBus<E>,
    {
        self.subscribe_to_provider(task_spawner, &actor_handle.event_bus_provider)
    }
}

pub struct EventBusListener<T: EventBusMessage, E: EventHandler<T>> {
    spawner: TaskSpawner,
    receiver: broadcast::Receiver<T>,
    event_handler: E,
}

impl<T: EventBusMessage, E: EventHandler<T> + Send + 'static> EventBusListener<T, E> {
    pub fn new(spawner: TaskSpawner, event_handler: E, receiver: broadcast::Receiver<T>) -> Self {
        Self {
            spawner: spawner.with_group("event-handler-worker"),
            event_handler,
            receiver,
        }
    }

    async fn run(&mut self) {
        while let Ok(event) = self.receiver.recv().await {
            let cloned_event_handler = self.event_handler.clone();
            self.spawner.spawn(async move {
                match cloned_event_handler.handle_event(event).await {
                    Ok(_) => {}
                    Err(error) => {
                        warn!("Task ended with error: {:?}", error);
                    }
                }
            });
        }
    }

    pub fn start(mut self) {
        let spawner = self.spawner.with_group("event-bus-listener");
        spawner.spawn(async move { self.run().await });
    }
}

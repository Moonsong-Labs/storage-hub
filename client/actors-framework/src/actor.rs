use core::fmt::{self, Debug, Formatter};
use futures::prelude::*;

use crate::{
    constants::DEFAULT_ACTOR_COMMAND_QUEUE_WARNING_SIZE,
    event_bus::{EventBusMessage, ProvidesEventBus},
};

/// The [`Actor`] trait represents an actor, which runs on its own event loop and can handle messages.
/// The struct implementing this trait can be seen as the context of the actor, holding the internal
/// state or the shared data (through commands and queries).
pub trait Actor: Sized {
    /// The type of message that the actor can handle.
    /// Usually an enum that represents the different types of messages that the actor can receive.
    type Message: Send + Sized + 'static;

    /// The event loop associated with the actor.
    /// If no custom event loop is needed, the default [`EventLoop<Self>`] can be used.
    type EventLoop: ActorEventLoop<Self> + Send + 'static;

    /// The event bus provider associated with the actor. This struct will implement
    /// [`ProvidesEventBus`] for all events that will be emitted by the actor.
    /// If there are no events to be emitted, this can be set to `()`.
    type EventBusProvider: Clone + Send + 'static;

    /// Handles a message received by the actor.
    ///
    /// - `message` - The message to be handled.
    ///
    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Returns the event bus provider for the actor.
    fn get_event_bus_provider(&self) -> &Self::EventBusProvider;

    fn emit<E: EventBusMessage>(&self, event: E)
    where
        Self::EventBusProvider: ProvidesEventBus<E>,
    {
        self.get_event_bus_provider().event_bus().emit(event)
    }
}

/// Trait representing an event loop for an actor.
pub trait ActorEventLoop<T: Actor> {
    /// Creates a new instance of the event loop.
    ///
    /// - `actor` - The actor instance.
    /// - `receiver` - The receiver for the actor's messages.
    ///
    fn new(actor: T, receiver: sc_utils::mpsc::TracingUnboundedReceiver<T::Message>) -> Self;

    /// The event loop to be implemented. This function should run continuously, receiving and
    /// handling messages for the actor.
    /// To be spawned as a separate thread.
    fn run(self) -> impl std::future::Future<Output = ()> + Send;
}

/// A simple and generic event loop that handles messages for an actor.
/// If a custom event loop (i.e. to handle multiple queues or channels) is needed, you need to
/// implement the [`ActorEventLoop`] trait.
pub struct EventLoop<T: Actor> {
    receiver: sc_utils::mpsc::TracingUnboundedReceiver<T::Message>,
    actor: T,
}

/// Implements the [`ActorEventLoop`] trait for the [`EventLoop`] struct.
impl<T: Actor + Send> ActorEventLoop<T> for EventLoop<T> {
    fn new(actor: T, receiver: sc_utils::mpsc::TracingUnboundedReceiver<T::Message>) -> Self {
        Self { actor, receiver }
    }

    /// Simple event loop that runs continuously, receiving and handling messages for the actor.
    /// Stops after all senders are dropped.
    async fn run(mut self) {
        while let Some(message) = self.receiver.next().await {
            self.actor.handle_message(message).await;
        }
    }
}

/// Represents a handle to an actor.
#[derive(Debug)]
pub struct ActorHandle<T: Actor> {
    sender: sc_utils::mpsc::TracingUnboundedSender<T::Message>,
    pub(crate) event_bus_provider: T::EventBusProvider,
}

impl<T: Actor> ActorHandle<T> {
    /// Sends a message to the actor.
    ///
    /// This method sends a `message` of type `T::Message` to the actor associated with this handle.
    /// The message is sent asynchronously, and the method will await until the message is sent.
    ///
    pub async fn send(&self, message: T::Message) {
        self.sender.unbounded_send(message).expect("Actor is dead");
    }
}

/// Implements the `Clone` trait for all [`ActorHandle`]s.
/// We need to implement it manually because the compiler is not able to infer that we don't need
/// the `T: Clone` bound.
impl<T: Actor> Clone for ActorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            event_bus_provider: self.event_bus_provider.clone(),
        }
    }
}
pub trait ActorSpawner<T: Actor + Send + 'static> {
    fn spawn_actor(self, actor: T) -> ActorHandle<T>;
}

#[derive(Clone)]
pub struct TaskSpawner {
    spawner: sc_service::SpawnTaskHandle,
    name: &'static str,
    group: Option<&'static str>,
    queue_size_warning: usize,
}

impl Debug for TaskSpawner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskSpawner")
            .field("name", &self.name)
            .field("group", &self.group)
            .field("queue_size_warning", &self.queue_size_warning)
            .finish()
    }
}

impl TaskSpawner {
    pub fn new(spawner: sc_service::SpawnTaskHandle, name: &'static str) -> Self {
        Self {
            spawner,
            name,
            group: None,
            queue_size_warning: DEFAULT_ACTOR_COMMAND_QUEUE_WARNING_SIZE,
        }
    }

    pub fn with_queue_size_warning(&self, queue_size_warning: usize) -> Self {
        Self {
            queue_size_warning,
            ..self.clone()
        }
    }

    pub fn with_group(&self, group: &'static str) -> Self {
        Self {
            group: Some(group),
            ..self.clone()
        }
    }

    pub fn with_name(&self, name: &'static str) -> Self {
        Self {
            name,
            ..self.clone()
        }
    }

    pub fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.spawner.spawn(self.name, self.group, task);
    }
}

/// Implements the [`SpawnableActor`] trait for any type that implements the [`Actor`] trait.
impl<T: Actor + Send + 'static> ActorSpawner<T> for TaskSpawner {
    fn spawn_actor(self, actor: T) -> ActorHandle<T> {
        let (sender, receiver) =
            sc_utils::mpsc::tracing_unbounded(self.name, self.queue_size_warning);
        let event_bus_provider = actor.get_event_bus_provider().clone();
        let event_loop = T::EventLoop::new(actor, receiver);

        self.spawn(async move { event_loop.run().await });

        ActorHandle {
            sender,
            event_bus_provider,
        }
    }
}

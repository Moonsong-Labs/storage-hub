use anyhow::{anyhow, Result};
use std::fmt::Debug;
use tokio::sync::{broadcast, mpsc};

use crate::{FileChunk, FileKey, FileMetadata};

/// Storage interface to be implemented by the storage providers.
pub trait Storage: Clone + Send + Sync + 'static {
    /// Get metadata for a file.
    fn get_metadata(
        &self,
        key: &FileKey,
    ) -> impl std::future::Future<Output = Option<FileMetadata>> + Send;

    /// Set metadata for a file.
    fn set_metadata(
        &self,
        key: &FileKey,
        metadata: &FileMetadata,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Get a file chunk from storage.
    fn get_chunk(
        &self,
        key: &FileKey,
        chunk: u64,
    ) -> impl std::future::Future<Output = Option<FileChunk>> + Send;

    /// Write a file chunk in storage.
    fn write_chunk(
        &self,
        key: &str,
        chunk: u64,
        data: &FileChunk,
    ) -> impl std::future::Future<Output = ()> + Send;
}

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
    /// * `message` - The message to be handled.
    ///
    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Returns the event bus provider for the actor.
    fn get_event_bus_provider(&self) -> &Self::EventBusProvider;

    fn emit<E: EventBusMessage>(&self, event: E) -> Result<()>
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
    /// * `actor` - The actor instance.
    /// * `receiver` - The receiver for the actor's messages.
    ///
    fn new(actor: T, receiver: mpsc::Receiver<T::Message>) -> Self;

    /// The event loop to be implemented. This function should run continuously, receiving and
    /// handling messages for the actor.
    /// To be spawned as a separate thread.
    fn run(&mut self) -> impl std::future::Future<Output = ()> + Send;
}

/// A simple and generic event loop that handles messages for an actor.
/// If a custom event loop (i.e. to handle multiple queues or channels) is needed, you need to
/// implement the [`ActorEventLoop`] trait.
pub struct EventLoop<T: Actor> {
    receiver: mpsc::Receiver<T::Message>,
    actor: T,
}

/// Implements the [`ActorEventLoop`] trait for the [`EventLoop`] struct.
impl<T: Actor + Send> ActorEventLoop<T> for EventLoop<T> {
    fn new(actor: T, receiver: mpsc::Receiver<T::Message>) -> Self {
        Self { actor, receiver }
    }

    /// Simple event loop that runs continuously, receiving and handling messages for the actor.
    /// Stops after all senders are dropped.
    async fn run(&mut self) {
        while let Some(message) = self.receiver.recv().await {
            self.actor.handle_message(message).await;
        }
    }
}

/// Represents a handle to an actor.
#[derive(Debug)]
pub struct ActorHandle<T: Actor> {
    sender: mpsc::Sender<T::Message>,
    event_bus_provider: T::EventBusProvider,
}

impl<T: Actor> ActorHandle<T> {
    /// Sends a message to the actor.
    ///
    /// This method sends a `message` of type `T::Message` to the actor associated with this handle.
    /// The message is sent asynchronously, and the method will await until the message is sent.
    ///
    pub async fn send(&self, message: T::Message) {
        self.sender.send(message).await.expect("Actor is dead");
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

/// Helper trait to spawn an actor.
pub trait SpawnableActor: Actor {
    fn spawn(self) -> ActorHandle<Self>;
}

/// Implements the [`SpawnableActor`] trait for any type that implements the [`Actor`] trait.
impl<T: Actor + Send + 'static> SpawnableActor for T {
    fn spawn(self) -> ActorHandle<Self> {
        let (sender, receiver) = mpsc::channel(8);
        let event_bus_provider = self.get_event_bus_provider().clone();
        let mut event_loop = T::EventLoop::new(self, receiver);

        tokio::spawn(async move { event_loop.run().await });

        ActorHandle {
            sender,
            event_bus_provider,
        }
    }
}

pub trait EventBusMessage: Debug + Clone + Send + 'static {}

#[derive(Debug, Clone)]
pub struct EventBus<T: EventBusMessage> {
    sender: broadcast::Sender<T>,
}

impl<T: EventBusMessage> Default for EventBus<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: EventBusMessage> EventBus<T> {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(8);
        Self { sender }
    }

    pub fn emit(&self, event: T) -> Result<()> {
        self.sender
            .send(event)
            .map_err(|_| anyhow!("Failed to emit event"))
            .map(|_| ())
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
        provider: &EP,
    ) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
    {
        let receiver = provider.event_bus().subscribe();
        EventBusListener::new(self, receiver)
    }

    fn subscribe_to<A: Actor>(self, actor_handle: &ActorHandle<A>) -> EventBusListener<E, Self>
    where
        Self: Sized + Send,
        <A as Actor>::EventBusProvider: ProvidesEventBus<E>,
    {
        self.subscribe_to_provider(&actor_handle.event_bus_provider)
    }
}

pub struct EventBusListener<T: EventBusMessage, E: EventHandler<T>> {
    receiver: broadcast::Receiver<T>,
    event_handler: E,
}

impl<T: EventBusMessage, E: EventHandler<T> + Send + 'static> EventBusListener<T, E> {
    pub fn new(event_handler: E, receiver: broadcast::Receiver<T>) -> Self {
        Self {
            event_handler,
            receiver,
        }
    }

    async fn run(&mut self) {
        while let Ok(event) = self.receiver.recv().await {
            let cloned_event_handler = self.event_handler.clone();
            tokio::spawn(async move { cloned_event_handler.handle_event(event).await });
        }
    }

    pub fn start(mut self) {
        tokio::spawn(async move { self.run().await });
    }
}

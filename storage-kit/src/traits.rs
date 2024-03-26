use tokio::sync::mpsc;

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

/// The `Actor` trait represents an actor, which runs on its own event loop and can handle messages.
/// The struct implementing this trait can be seen as the context of the actor, holding the internal
/// state or the shared data (through commands and queries).
pub trait Actor: Sized {
    /// The type of message that the actor can handle.
    /// Usually an enum that represents the different types of messages that the actor can receive.
    type Message: Send + Sized + 'static;

    /// The event loop associated with the actor.
    /// If no custom event loop is needed, the default `EventLoop<Self>` can be used.
    type EventLoop: ActorEventLoop<Self> + Send + 'static;

    /// Handles a message received by the actor.
    ///
    /// * `message` - The message to be handled.
    ///
    fn handle_message(
        &mut self,
        message: Self::Message,
    ) -> impl std::future::Future<Output = ()> + std::marker::Send;
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
/// implement the `ActorEventLoop` trait.
pub struct EventLoop<T: Actor> {
    receiver: mpsc::Receiver<T::Message>,
    actor: T,
}

/// Implements the `ActorEventLoop` trait for the `EventLoop` struct.
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
#[derive(Debug, Clone)]
pub struct ActorHandle<T: Actor> {
    sender: mpsc::Sender<T::Message>,
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

/// Helper trait to spawn an actor.
pub trait SpawnableActor: Actor {
    fn spawn(self) -> ActorHandle<Self>;
}

/// Implements the `SpawnableActor` trait for any type that implements the `Actor` trait.
impl<T: Actor + Send + 'static> SpawnableActor for T {
    fn spawn(self) -> ActorHandle<Self> {
        let (sender, receiver) = mpsc::channel(8);
        let mut event_loop = EventLoop::<T>::new(self, receiver);

        tokio::spawn(async move { event_loop.run().await });

        ActorHandle { sender }
    }
}

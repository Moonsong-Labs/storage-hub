use tokio::sync::mpsc;

use crate::{FileChunk, FileKey, FileMetadata};

/// Storage interface to be implemented by the storage providers.
pub trait Storage: Clone + Send + Sync + 'static  {
    /// Get metadata for a file.
    fn get_metadata(&self, key: &FileKey) -> impl std::future::Future<Output = Option<FileMetadata>> + Send;

    /// Set metadata for a file.
    fn set_metadata(&self, key: &FileKey, metadata: &FileMetadata) -> impl std::future::Future<Output = ()> + Send;

    /// Get a file chunk from storage.
	fn get_chunk(&self, key: &FileKey, chunk: u64) -> impl std::future::Future<Output = Option<FileChunk>> + Send;

    /// Write a file chunk in storage.
    fn write_chunk(&self, key: &str, chunk: u64, data: &FileChunk) -> impl std::future::Future<Output = ()> + Send;
}

pub trait Actor: Sized {
    type Message: Send + Sized + 'static;
    type EventLoop: ActorEventLoop<Self> + Send + 'static;

    fn handle_message(&mut self, message: Self::Message) -> impl std::future::Future<Output = ()> + std::marker::Send;
}

pub trait ActorEventLoop<T: Actor> {
    fn new(actor: T, receiver: mpsc::Receiver<T::Message>) -> Self;

    fn run(&mut self) -> impl std::future::Future<Output = ()> + Send;
}

pub struct EventLoop<T: Actor> {
    receiver: mpsc::Receiver<T::Message>,
    actor: T,
}

impl<T: Actor + Send> ActorEventLoop<T> for EventLoop<T> {
    fn new(actor: T, receiver: mpsc::Receiver<T::Message>) -> Self {
        Self { actor, receiver }
    }

    async fn run(&mut self) {
        while let Some(message) = self.receiver.recv().await {
            self.actor.handle_message(message).await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActorHandle<T: Actor> {
    sender: mpsc::Sender<T::Message>,
}

impl <T: Actor> ActorHandle<T> {
    pub async fn send(&self, message: T::Message) {
        self.sender.send(message).await.expect("Actor is dead");
    }
}

pub trait SpawnableActor: Actor {
    fn spawn(self) -> ActorHandle<Self>;
}

impl<T: Actor + Send + 'static> SpawnableActor for T {
    fn spawn(self) -> ActorHandle<Self> {
        let (sender, receiver) = mpsc::channel(8);
        let mut event_loop = EventLoop::<T>::new(self, receiver);
        
        tokio::spawn(async move { event_loop.run().await });

        ActorHandle { sender }
    }
}

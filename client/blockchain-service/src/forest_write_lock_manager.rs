use crate::events::ForestWriteLockTaskData;
use shc_common::traits::StorageEnableRuntime;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Priority value for forest write lock tickets
#[derive(Clone, Debug)]
pub enum ForestWriteLockPriority {
    /// High priority request
    High = 0,
    /// Medium priority request
    Medium = 1,
    /// Low priority request
    Low = 2,
}

/// Ticket structure for managing forest write lock requests
#[derive(Clone, Debug)]
pub struct ForestWriteLockTicket<Runtime: StorageEnableRuntime> {
    /// Priority of the lock request
    pub priority: ForestWriteLockPriority,
    /// Data associated with the lock request ticket
    pub data: ForestWriteLockTaskData<Runtime>,
}

impl<Runtime: StorageEnableRuntime> ForestWriteLockTicket<Runtime> {
    /// Creates a new ForestWriteLockTicket with the given data and determines its priority
    /// Since MSPs and BSPs will have a separate queues, we can assign high priority to both types of requests
    pub fn new(data: ForestWriteLockTaskData<Runtime>) -> Self {
        Self {
            priority: match data {
                ForestWriteLockTaskData::SubmitProofRequest(_) => ForestWriteLockPriority::High,
                ForestWriteLockTaskData::MspRespondStorageRequest(_) => {
                    ForestWriteLockPriority::High
                }
                ForestWriteLockTaskData::ConfirmStoringRequest(_) => {
                    ForestWriteLockPriority::Medium
                }
                ForestWriteLockTaskData::StopStoringForInsolventUserRequest(_) => {
                    ForestWriteLockPriority::Low
                }
            },
            data,
        }
    }
}

#[derive(Debug)]
pub struct ForestWriteLockManager<Runtime: StorageEnableRuntime> {
    /// Semaphore to limit concurrent write operations
    pub lock: Arc<Semaphore>,
    /// Queue to manage pending write requests
    pub queue: Arc<VecDeque<ForestWriteLockTicket<Runtime>>>,
}

impl<Runtime: StorageEnableRuntime> ForestWriteLockManager<Runtime> {
    /// Creates a new ForestWriteLockManager instance
    /// - The lock is initialized with a single permit semaphore
    // TODO: Allow the queue to be pre-populated with existing tickets if necessary
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Semaphore::new(1)),
            queue: Arc::new(VecDeque::new()),
        }
    }

    /// Attempts to acquire the forest write lock or enqueues the request if the lock is not available
    pub async fn acquire(&mut self, data: ForestWriteLockTaskData<Runtime>) -> ForestWriteLockGuard<Runtime> {
        let ticket = ForestWriteLockTicket::new(data);

        if self.lock.available_permits() > 0 {
            let _permit = self.lock.acquire().await.unwrap();
        } else {
            self.enqueue(ticket);
        }

        ForestWriteLockGuard {
            manager: Arc::new(self.clone()),
        }
    }

    /// Checks if the forest write lock is currently available
    pub fn is_available(&self) -> bool {
        self.lock.available_permits() > 0
    }

    /// Enqueues a ticket based on its priority
    /// - High priority tickets are placed before any medium and low priority tickets
    /// - Medium priority tickets are placed before any low priority tickets
    /// - Low priority tickets are placed at the back of the queue
    fn enqueue(&mut self, ticket: ForestWriteLockTicket<Runtime>) {
        match ticket.priority {
            ForestWriteLockPriority::Low => self.queue.push_back(ticket),
            ForestWriteLockPriority::Medium => {
                let insert_pos = self
                    .queue
                    .iter()
                    .position(|item| matches!(item.priority, ForestWriteLockPriority::Low))
                    .unwrap_or(self.queue.len());
                self.queue.insert(insert_pos, ticket);
            }
            ForestWriteLockPriority::High => {
                let insert_pos = self
                    .queue
                    .iter()
                    .position(|item| {
                        matches!(
                            item.priority,
                            ForestWriteLockPriority::Medium | ForestWriteLockPriority::Low
                        )
                    })
                    .unwrap_or(self.queue.len());
                self.queue.insert(insert_pos, ticket);
            }
        }
    }
}
#[derive(Debug)]
pub struct ForestWriteLockGuard<Runtime: StorageEnableRuntime> {
    manager: Arc<ForestWriteLockManager<Runtime>>,
}
impl<Runtime: StorageEnableRuntime> Drop for ForestWriteLockGuard<Runtime> {
    /// Release the semaphore permit when the guard is dropped and acquire the next ticket if available
    fn drop(&mut self) {
        self.manager.lock.add_permits(1);
        // TODO: Implement logic to acquire the next ticket from the queue if available
    }
}

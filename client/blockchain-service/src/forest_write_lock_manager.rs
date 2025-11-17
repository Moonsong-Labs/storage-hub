use crate::events::ForestWriteLockTaskData;
use shc_common::traits::StorageEnableRuntime;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore};

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
    /// Identifier for the ticket
    pub id: String,
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
            id: format!("{:?}", data),
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

#[derive(Clone, Debug)]
pub struct ForestWriteLockManager<Runtime: StorageEnableRuntime> {
    /// Semaphore to limit concurrent write operations
    pub lock: Arc<Semaphore>,
    /// Queue to manage pending write requests
    pub queue: Arc<Mutex<VecDeque<ForestWriteLockTicket<Runtime>>>>,
    /// Current holder of the write lock
    pub current_holder: Arc<Mutex<Option<ForestWriteLockTicket<Runtime>>>>,
    /// Notify mechanism to signal when the lock becomes available
    pub notifier: Arc<Notify>,
}

impl<Runtime: StorageEnableRuntime> ForestWriteLockManager<Runtime> {
    /// Creates a new ForestWriteLockManager instance
    /// - The lock is initialized with a  max single permit semaphore
    // TODO: Allow the queue to be pre-populated with existing tickets if necessary
    pub fn new() -> Self {
        Self {
            lock: Arc::new(Semaphore::const_new(1)),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            current_holder: Arc::new(Mutex::new(None)),
            notifier: Arc::new(Notify::new()),
        }
    }

    /// Checks if the forest write lock is currently available
    pub fn is_available(&self) -> bool {
        self.lock.available_permits() > 0
    }

    /// Attempts to acquire the forest write lock or enqueues the request if the lock is not available
    /// - If the lock is available and the queue is empty, it is granted immediately
    /// - If the lock is unavailable, the request is enqueued based on its priority
    /// - If the lock is available but there are pending requests in the queue, the request is enqueued
    /// and the next ticket in the queue is granted the lock
    /// Returns a ForestWriteLockGuard that manages the lifetime of the acquired lock
    pub async fn acquire(
        &self,
        data: ForestWriteLockTaskData<Runtime>,
    ) -> ForestWriteLockGuard<Runtime> {
        let ticket = ForestWriteLockTicket::new(data);

        // Add the ticket to the priority queue first
        // TODO: Handle the edge case where the acquire request is already in the queue
        {
            self.enqueue(ticket.clone());
        }

        // If the ticket is at the front of the queue, try to acquire the lock
        // Otherwise, wait until it's our turn
        loop {
            let permit = self.lock.acquire().await.unwrap();

            let should_proceed = {
                let mut queue = self.queue.lock().unwrap();
                if let Some(next_task) = queue.front() {
                    if next_task.id == ticket.id {
                        queue.pop_front();
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if should_proceed {
                *self.current_holder.lock().unwrap() = Some(ticket);
                permit.forget(); // We'll manually release when guard is dropped
                break;
            } else {
                // Not our turn, wait for notification
                drop(permit);
                self.notifier.notified().await;
            }
        }

        ForestWriteLockGuard {
            manager: Arc::new(self.clone()),
        }
    }

    /// Enqueues a ticket based on its priority
    /// - High priority tickets are placed before any medium and low priority tickets
    /// - Medium priority tickets are placed before any low priority tickets
    /// - Low priority tickets are placed at the back of the queue
    fn enqueue(&self, ticket: ForestWriteLockTicket<Runtime>) {
        let mut queue = self.queue.lock().unwrap();
        match ticket.priority {
            ForestWriteLockPriority::Low => queue.push_back(ticket),
            ForestWriteLockPriority::Medium => {
                let insert_pos = queue
                    .iter()
                    .position(|item| matches!(item.priority, ForestWriteLockPriority::Low))
                    .unwrap_or(queue.len());
                queue.insert(insert_pos, ticket);
            }
            ForestWriteLockPriority::High => {
                let insert_pos = queue
                    .iter()
                    .position(|item| {
                        matches!(
                            item.priority,
                            ForestWriteLockPriority::Medium | ForestWriteLockPriority::Low
                        )
                    })
                    .unwrap_or(queue.len());
                queue.insert(insert_pos, ticket);
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
        // Release the permit back to the semaphore
        self.manager.lock.add_permits(1);
        // Clear the current holder
        *self.manager.current_holder.lock().unwrap() = None;
        // Notify next waiter
        self.manager.notifier.notify_one();
    }
}

// TODO: Implement unit tests for ForestWriteLockManager and ForestWriteLockGuard
// Verify that:
// a) With empty queue, acquire grants the lock immediately
// b) With non-empty queue, and free lock, the lock is granted to the next ticket in the queue
// c) With non-empty queue, and occupied lock, the ticket is enqueued correctly based
// d) Dropping the guard releases the lock and allows the next ticket to acquire it

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock, Semaphore};

use sc_tracing::tracing::{error, trace, warn};

use crate::handler::LOG_TARGET;

/// Priority value for forest root write operations.
/// Lower numbers indicate higher priority.
pub type PriorityValue = u64;

/// A ticket that represents a request for the forest root write lock
#[derive(Clone, Debug)]
pub struct ForestRootWriteTicket {
    /// The internal state of the ticket
    state: Arc<RwLock<TicketState>>,
    /// The priority of this ticket
    priority: PriorityValue,
    /// Unique identifier for this ticket
    id: usize,
    /// Reference to the manager that issued this ticket
    manager: Arc<ForestRootWriteLockManager>,
}

// Add ordering traits to ForestRootWriteTicket based on priority
impl PartialEq for ForestRootWriteTicket {
    fn eq(&self, other: &Self) -> bool {
        // Equal if same priority and same ticket id
        self.priority == other.priority && self.id == other.id
    }
}

impl Eq for ForestRootWriteTicket {}

impl PartialOrd for ForestRootWriteTicket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ForestRootWriteTicket {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare priorities (lower priority value = higher priority)
        self.priority
            .cmp(&other.priority)
            // Break ties with ID comparison for stability
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl Hash for ForestRootWriteTicket {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Use priority and ID for hashing
        self.priority.hash(state);
        self.id.hash(state);
    }
}

/// The state of a ticket
struct TicketState {
    /// Whether this ticket is currently active (has the lock)
    is_active: bool,
    /// A oneshot channel used to signal when this ticket can become active
    activation_tx: Option<oneshot::Sender<()>>,
    /// A oneshot channel used to receive the activation signal
    activation_rx: Option<oneshot::Receiver<()>>,
}

impl std::fmt::Debug for TicketState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TicketState")
            .field("is_active", &self.is_active)
            .field("activation_tx", &self.activation_tx.is_some())
            .field("activation_rx", &self.activation_rx.is_some())
            .finish()
    }
}

// Thread-safe counter for generating unique IDs
static NEXT_TICKET_ID: AtomicUsize = AtomicUsize::new(0);

fn get_next_ticket_id() -> usize {
    // Thread-safe increment and return
    NEXT_TICKET_ID.fetch_add(1, AtomicOrdering::Relaxed)
}

impl ForestRootWriteTicket {
    /// Create a new ticket with the given priority
    fn new(priority: PriorityValue, manager: &ForestRootWriteLockManager) -> Self {
        let (activation_tx, activation_rx) = oneshot::channel();

        Self {
            state: Arc::new(RwLock::new(TicketState {
                is_active: false,
                activation_tx: Some(activation_tx),
                activation_rx: Some(activation_rx),
            })),
            priority,
            id: get_next_ticket_id(),
            manager: Arc::new(manager.clone()),
        }
    }

    /// Acquire the lock for this ticket. This will wait until the ticket
    /// becomes active (is granted the lock) before returning.
    pub async fn lock(&self) -> ForestRootWriteGuard {
        let activation_rx = {
            let mut state = self.state.write().await;

            // If we're already active, we don't need to wait
            if state.is_active {
                return ForestRootWriteGuard {
                    ticket: self.clone(),
                };
            }

            // Take the activation receiver
            state
                .activation_rx
                .take()
                .expect("activation_rx should be present")
        };

        // Wait for the activation signal
        match activation_rx.await {
            Ok(()) => {
                // Mark this ticket as active
                let mut state = self.state.write().await;
                state.is_active = true;

                warn!(
                    target: LOG_TARGET,
                    "[LOCK_MANAGER] Ticket with priority {} acquired lock",
                    self.priority
                );

                ForestRootWriteGuard {
                    ticket: self.clone(),
                }
            }
            Err(e) => {
                error!(
                    target: LOG_TARGET,
                    "Failed to receive activation signal: {:?}", e
                );
                panic!("Failed to receive activation signal: {:?}", e);
            }
        }
    }

    /// Mark this ticket as inactive (release the lock)
    /// This is now mostly handled by the guard drop, but kept for explicit releases
    async fn release(&self) {
        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Releasing lock START");
        let mut state = self.state.write().await;
        if state.is_active {
            state.is_active = false;
            warn!(
                target: LOG_TARGET,
                "[LOCK_MANAGER] Ticket with priority {} released lock",
                self.priority
            );

            // Release the forgotten permit by adding it back to the semaphore
            if self
                .manager
                .forgotten_permits
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
                > 0
            {
                self.manager.lock.add_permits(1);
                warn!(target: LOG_TARGET, "[LOCK_MANAGER] Released forgotten permit");
            }

            // Notify the manager that this ticket has been released
            self.manager.notify_lock_released().await;
        }
        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Releasing lock END");
    }
}

/// A guard that automatically releases the lock when dropped
pub struct ForestRootWriteGuard {
    ticket: ForestRootWriteTicket,
}

impl Drop for ForestRootWriteGuard {
    fn drop(&mut self) {
        // Simply spawn a task to release the lock without blocking
        let ticket = self.ticket.clone();
        warn!(target: LOG_TARGET, "[LOCK_MANAGER] GUARD DROP: Spawning release task for ticket with priority {}", ticket.priority);
        tokio::spawn(async move {
            warn!(target: LOG_TARGET, "[LOCK_MANAGER] GUARD DROP: Starting release for ticket with priority {}", ticket.priority);
            ticket.release().await;
            warn!(target: LOG_TARGET, "[LOCK_MANAGER] GUARD DROP: Completed release for ticket with priority {}", ticket.priority);
        });
    }
}

/// Manages access to the forest root write lock
#[derive(Debug)]
pub struct ForestRootWriteLockManager {
    /// Semaphore used to control access to the lock (with 1 permit)
    lock: Arc<Semaphore>,
    /// Queue of tickets waiting for the lock, ordered by priority
    queue: Arc<RwLock<BTreeSet<ForestRootWriteTicket>>>,
    /// Whether a ticket is currently being processed
    processing: Arc<RwLock<bool>>,
    /// Counter to track how many permits have been forgotten and need manual release
    forgotten_permits: Arc<std::sync::atomic::AtomicUsize>,
}

impl ForestRootWriteLockManager {
    /// Create a new forest root write lock manager
    pub fn new() -> Self {
        Self {
            // Start with 1 permit (lock is available)
            lock: Arc::new(Semaphore::new(1)),
            queue: Arc::new(RwLock::new(BTreeSet::new())),
            processing: Arc::new(RwLock::new(false)),
            forgotten_permits: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Create a new ticket with the given priority
    pub async fn create_ticket(&self, priority: PriorityValue) -> ForestRootWriteTicket {
        let ticket = ForestRootWriteTicket::new(priority, self);

        // Add ticket to the queue
        {
            let mut queue = self.queue.write().await;
            queue.insert(ticket.clone());
        }

        // We don't automatically try to process tickets here anymore
        // External code should call process_next_ticket() when appropriate

        ticket
    }

    /// Try to process the next ticket in the queue
    pub async fn process_next_ticket(&self) {
        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Process next ticket START");
        // We use a loop to avoid recursion
        let mut should_continue = true;

        while should_continue {
            // Acquire the processing lock to ensure only one thread processes tickets
            let mut processing = self.processing.write().await;

            // If already processing, exit
            if *processing {
                should_continue = false;
                continue;
            }

            // Mark as processing
            *processing = true;

            // Release the lock before processing
            drop(processing);

            // Try to assign the lock to the next ticket
            self.try_assign_lock().await;

            // Mark processing as complete
            let mut processing = self.processing.write().await;
            *processing = false;

            // Check if we need to continue (set to false by default)
            should_continue = false;
        }
        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Process next ticket END");
    }

    /// Try to assign the lock to the next ticket
    async fn try_assign_lock(&self) {
        // Try to acquire the lock (non-blocking)
        let permit = match self.lock.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                // Lock is already taken
                warn!(target: LOG_TARGET, "[LOCK_MANAGER] Lock already taken");
                return;
            }
        };

        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Acquired lock");

        // Find the highest priority ticket
        let next_ticket = {
            let mut queue = self.queue.write().await;
            let queue_size = queue.len();
            warn!(target: LOG_TARGET, "[LOCK_MANAGER] Queue has {} tickets", queue_size);

            // Take the first ticket (highest priority due to BTreeSet ordering)
            if queue.is_empty() {
                None
            } else {
                // Remove and return the first (highest priority) ticket
                let ticket = queue.iter().next().cloned();
                if let Some(ref t) = ticket {
                    queue.remove(t);
                    warn!(target: LOG_TARGET, "[LOCK_MANAGER] Removed ticket with priority {} from queue", t.priority);
                }
                ticket
            }
        };

        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Next ticket: {:?}", next_ticket);

        match next_ticket {
            Some(ticket) => {
                // Get a mutable reference to the ticket's state
                let mut state = ticket.state.write().await;

                // Check if we can activate this ticket
                if let Some(tx) = state.activation_tx.take() {
                    // Send activation signal
                    drop(state); // Drop the lock first

                    if let Err(e) = tx.send(()) {
                        error!(
                            target: LOG_TARGET,
                            "Failed to send activation signal to ticket: {:?}", e
                        );
                        // Release the permit so another ticket can be processed
                        drop(permit);
                        return;
                    }

                    // Track that we've forgotten a permit
                    self.forgotten_permits
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // Deliberately forget the permit so it's not released until the ticket is released
                    std::mem::forget(permit);

                    trace!(
                        target: LOG_TARGET,
                        "Activated ticket with priority {}",
                        ticket.priority
                    );
                } else {
                    // Ticket can't be activated, release the permit
                    drop(permit);
                }
            }
            None => {
                // No tickets to process, release the permit
                drop(permit);
            }
        }
    }

    /// Notify the manager that a lock has been released
    async fn notify_lock_released(&self) {
        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Notify lock released START");

        // Use async spawning to prevent recursion that can cause stack overflow
        let manager = self.clone();
        tokio::spawn(async move {
            manager.process_next_ticket().await;
        });

        warn!(target: LOG_TARGET, "[LOCK_MANAGER] Notify lock released END");
    }

    /// Check if the lock is available without acquiring it
    pub fn is_lock_available(&self) -> bool {
        self.lock.available_permits() > 0
    }
}

impl Clone for ForestRootWriteLockManager {
    fn clone(&self) -> Self {
        Self {
            // Share the same semaphore - this is critical for proper lock coordination
            lock: Arc::clone(&self.lock),
            queue: Arc::clone(&self.queue),
            processing: Arc::clone(&self.processing),
            forgotten_permits: Arc::clone(&self.forgotten_permits),
        }
    }
}

impl Default for ForestRootWriteLockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_lock_manager_priority() {
        let manager = ForestRootWriteLockManager::new();

        // Create a channel to send results
        let (tx, mut rx) = mpsc::channel(10);

        // Create tickets with explicit priorities
        let low_ticket = manager
            .create_ticket(4) // Higher number = lower priority
            .await;
        let medium_ticket = manager
            .create_ticket(3) // Medium priority
            .await;
        let high_ticket = manager
            .create_ticket(0) // Lowest number = highest priority
            .await;

        // Process the tickets
        manager.process_next_ticket().await;

        // Spawn tasks to acquire locks in reverse priority order
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let _guard = low_ticket.lock().await;
            tx_clone.send("low").await.unwrap();
            // Hold the lock for a bit
            sleep(Duration::from_millis(100)).await;
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let _guard = medium_ticket.lock().await;
            tx_clone.send("medium").await.unwrap();
            // Hold the lock for a bit
            sleep(Duration::from_millis(100)).await;
        });

        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let _guard = high_ticket.lock().await;
            tx_clone.send("high").await.unwrap();
            // Hold the lock for a bit
            sleep(Duration::from_millis(100)).await;
        });

        // Collect results
        let mut results = Vec::new();
        for _ in 0..3 {
            let result = rx.recv().await.unwrap();
            results.push(result);
        }

        // The high priority task should be completed first,
        // followed by medium, then low
        assert_eq!(results, vec!["high", "medium", "low"]);
    }
}

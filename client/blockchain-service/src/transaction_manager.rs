use std::{collections::BTreeMap, time::Duration};

use anyhow::Result;
use codec::Encode;
use log::{debug, error, warn};
use sc_transaction_pool_api::TransactionStatus;
use sp_runtime::traits::{One, Saturating};
use tokio::time::Instant;

use crate::{
    handler::LOG_TARGET,
    types::{StatusToWait, WatchTransactionError},
};

/// Configuration for the transaction manager.
#[derive(Clone, Debug)]
pub struct TransactionManagerConfig {
    /// Maximum number of pending transactions to track.
    pub max_pending_transactions: u32,
    /// Number of blocks to wait before filling a nonce gap with a remark transaction.
    /// During this period, the system will try to use regular transactions to fill the gap.
    pub gap_fill_threshold_blocks: u32,
}

impl Default for TransactionManagerConfig {
    fn default() -> Self {
        Self {
            max_pending_transactions: 100,
            gap_fill_threshold_blocks: 10,
        }
    }
}

/// A pending transaction tracked by the manager.
#[derive(Clone, Debug)]
pub struct PendingTransaction<Hash, Call, BlockNumber> {
    /// Hash of the transaction.
    pub hash: Hash,
    /// The extrinsic call.
    pub call: Call,
    /// The tip used when submitting this transaction.
    pub tip: u128,
    /// Block number when transaction was submitted.
    pub submitted_at: BlockNumber,
    /// Latest status from the transaction watcher.
    pub latest_status: TransactionStatus<Hash, Hash>,
}

/// Information about nonce gaps detected in the transaction manager.
#[derive(Clone, Debug)]
pub struct NonceGap {
    /// The missing nonce.
    pub nonce: u32,
    /// How many blocks ago this gap was first detected.
    pub age_in_blocks: u32,
}

/// Transaction manager for tracking pending transactions and managing nonces.
///
/// Transaction state is managed via watcher events from Substrate's transaction pool.
/// This manager tracks nonces and detects gaps, but relies on watchers as the source
/// of truth for transaction lifecycle events (Ready, InBlock, Retracted, Finalized, etc.).
///
/// TODO: Make transaction manager state persistent across node restarts.
/// Currently, if the node restarts, we lose all tracking of pending transactions,
/// which nonces were used, and gap detection history.
/// The current mitigation is initializing the nonce counter from on-chain state,
/// which helps but doesn't fully solve the problem.
pub struct TransactionManager<Hash, Call, BlockNumber> {
    /// Configuration for the manager.
    pub config: TransactionManagerConfig,
    /// Map of nonce to pending transaction.
    pub(crate) pending: BTreeMap<u32, PendingTransaction<Hash, Call, BlockNumber>>,
    /// Map of nonce to block number when gap was first detected.
    detected_gaps: BTreeMap<u32, BlockNumber>,
    /// Map of nonce to status broadcast senders for subscription.
    /// Subscribers receive updates when the transaction status changes in the manager.
    status_subscribers: BTreeMap<u32, tokio::sync::watch::Sender<TransactionStatus<Hash, Hash>>>,
}

impl<Hash, Call, BlockNumber> TransactionManager<Hash, Call, BlockNumber>
where
    Hash: std::fmt::Debug + Clone + Eq + std::hash::Hash + Encode,
    Call: Clone + std::fmt::Debug,
    BlockNumber: Copy
        + std::fmt::Debug
        + Ord
        + Saturating
        + From<u32>
        + sp_runtime::traits::UniqueSaturatedInto<u32>
        + One
        + std::ops::Sub<Output = BlockNumber>,
{
    /// Create a new transaction manager with the given configuration.
    pub fn new(config: TransactionManagerConfig) -> Self {
        Self {
            config,
            pending: BTreeMap::new(),
            detected_gaps: BTreeMap::new(),
            status_subscribers: BTreeMap::new(),
        }
    }

    /// Track a newly submitted transaction.
    pub fn track_transaction(
        &mut self,
        nonce: u32,
        hash: Hash,
        call: Call,
        tip: u128,
        submitted_at: BlockNumber,
    ) -> Result<()> {
        if self.pending.len() >= self.config.max_pending_transactions as usize {
            warn!(
                target: LOG_TARGET,
                "Transaction manager is at capacity ({}), dropping oldest pending transaction",
                self.config.max_pending_transactions
            );
            // Remove the oldest pending transaction
            if let Some((&oldest_nonce, _)) = self.pending.iter().next() {
                self.pending.remove(&oldest_nonce);
            }
        }

        if let Some(existing) = self.pending.get(&nonce) {
            debug!(
                target: LOG_TARGET,
                "Replacing tracked transaction at nonce {} (old hash: {:?}, new hash: {:?}).",
                nonce,
                existing.hash,
                hash
            );
        } else {
            debug!(
                target: LOG_TARGET,
                "Tracking transaction with nonce {} and hash {:?}",
                nonce, hash
            );
        }

        let pending_tx = PendingTransaction {
            hash,
            call,
            tip,
            submitted_at,
            latest_status: TransactionStatus::Future,
        };

        self.pending.insert(nonce, pending_tx);

        // Create a new watch channel for status subscriptions
        // Even if we're replacing a transaction with the same nonce, the new transaction
        // needs its own fresh channel. The old transaction's watcher will receive an Usurped
        // event and clean up its own channel separately
        let (tx, _rx) = tokio::sync::watch::channel(TransactionStatus::Future);
        self.status_subscribers.insert(nonce, tx);

        // Clear gap tracking for this nonce (if it was previously detected as a gap)
        self.detected_gaps.remove(&nonce);

        Ok(())
    }

    /// Detect gaps in the nonce sequence.
    ///
    /// Returns a list of missing nonces between the on-chain nonce and the highest tracked nonce.
    ///
    /// The `local_nonce_counter` parameter is the highest nonce that has been locally assigned
    /// (not necessarily in a block or even in the manager). This allows detection of gaps even when the
    /// manager is empty (e.g., after a dropped transaction is cleaned up).
    pub fn detect_gaps(
        &mut self,
        on_chain_nonce: u32,
        local_nonce_counter: u32,
        current_block: BlockNumber,
    ) -> Vec<NonceGap> {
        let mut gaps = Vec::new();

        // Determine the highest nonce we need to check
        // This is the maximum of:
        // 1. The highest nonce in the manager (if any)
        // 2. The local nonce counter
        let max_nonce = self
            .pending
            .keys()
            .next_back()
            .copied()
            .unwrap_or(on_chain_nonce)
            .max(local_nonce_counter);

        // If max_nonce is less than or equal to on-chain nonce, there are no gaps to check
        if max_nonce <= on_chain_nonce {
            return gaps;
        }

        // Check for gaps in the sequence from on_chain_nonce to max_nonce
        for expected_nonce in on_chain_nonce..max_nonce {
            if !self.pending.contains_key(&expected_nonce) {
                let first_detected = self
                    .detected_gaps
                    .entry(expected_nonce)
                    .or_insert(current_block);

                let age = current_block.saturating_sub(*first_detected);
                let age_u32: u32 = age.unique_saturated_into();

                debug!(
                    target: LOG_TARGET,
                    "Detected nonce gap at {} (age: {} blocks)",
                    expected_nonce,
                    age_u32
                );

                gaps.push(NonceGap {
                    nonce: expected_nonce,
                    age_in_blocks: age_u32,
                });
            }
        }

        gaps
    }

    /// Remove a transaction from the manager completely.
    ///
    /// This removes both the pending transaction and any gap tracking history.
    /// Use this when a transaction is permanently replaced (e.g., Usurped).
    /// It also cleans up the status subscribers for the transaction.
    pub fn remove(&mut self, nonce: u32) {
        self.pending.remove(&nonce);
        self.detected_gaps.remove(&nonce);
        self.status_subscribers.remove(&nonce);
    }

    /// Remove a transaction from the manager but preserve gap tracking.
    ///
    /// This allows the gap detection system to remember when the gap was first detected,
    /// to execute the gap-filling logic later if needed.
    /// Use this for retriable terminal states (Invalid, Dropped) where we want to
    /// potentially fill the gap later.
    pub fn remove_pending_but_keep_gap(&mut self, nonce: u32) {
        self.pending.remove(&nonce);
        self.status_subscribers.remove(&nonce);
    }

    /// Clean up the stale nonce gaps in the transaction manager.
    ///
    /// Removes stale nonce gaps that have already been filled (either by us or externally).
    /// If a nonce gap is less than the on-chain nonce, it means the gap was filled,
    /// so we can remove it from tracking.
    pub fn cleanup_stale_nonce_gaps(&mut self, on_chain_nonce: u32) {
        let stale_gaps: Vec<u32> = self
            .detected_gaps
            .keys()
            .filter(|&&nonce| nonce < on_chain_nonce)
            .copied()
            .collect();

        for nonce in stale_gaps {
            debug!(
                target: LOG_TARGET,
                "Cleaning up filled gap for nonce {} (< on-chain nonce {})",
                nonce,
                on_chain_nonce
            );
            self.detected_gaps.remove(&nonce);
        }
    }

    /// Subscribe to status updates for a specific transaction.
    ///
    /// Returns a receiver that will get notified whenever the transaction status changes,
    /// or None if the transaction is not tracked in the manager.
    ///
    /// Multiple subscribers can wait for the same transaction without interfering with each other.
    pub fn subscribe_to_status(
        &mut self,
        nonce: u32,
    ) -> Option<tokio::sync::watch::Receiver<TransactionStatus<Hash, Hash>>> {
        // Get the watch sender for the transaction with the given nonce
        let Some(sender) = self.status_subscribers.get(&nonce) else {
            return None;
        };
        Some(sender.subscribe())
    }

    /// Notify all subscribers about a status change for a specific transaction.
    ///
    /// This should be called whenever a transaction's status is updated.
    /// It broadcasts the new status to all active subscribers.
    pub fn notify_status_change(&mut self, nonce: u32, status: TransactionStatus<Hash, Hash>) {
        if let Some(sender) = self.status_subscribers.get(&nonce) {
            // Send the status update. Ignore errors if all receivers were dropped.
            let _ = sender.send(status);
        }
    }
}

/// Wait for a transaction to reach a specific status using a status subscription receiver.
///
/// This is a helper function that waits for transaction status updates via a watch channel
/// and returns when the desired status is reached or a terminal failure occurs.
///
/// # Arguments
///
/// * `nonce` - The nonce of the transaction to wait for
/// * `status_receiver` - Watch receiver that provides transaction status updates
/// * `target_status` - The target status to wait for (InBlock or Finalized)
/// * `timeout` - Maximum time to wait before returning a timeout error
///
/// # Returns
///
/// Returns `Ok(Hash)` with the block hash if the transaction reaches the desired status.
/// Returns `Err` if:
/// - The transaction reaches a failure terminal state (Invalid, Dropped, Usurped, FinalityTimeout)
/// - The timeout is reached
pub async fn wait_for_transaction_status<Hash>(
    nonce: u32,
    mut status_receiver: tokio::sync::watch::Receiver<TransactionStatus<Hash, Hash>>,
    target_status: StatusToWait,
    timeout: Duration,
) -> Result<Hash, WatchTransactionError>
where
    Hash: std::fmt::Debug + Clone,
{
    let start_time = Instant::now();

    loop {
        // Check if timeout has been reached
        let elapsed = start_time.elapsed();
        if elapsed > timeout {
            error!(
                target: LOG_TARGET,
                "Timeout waiting for transaction with nonce {} to reach {:?}",
                nonce,
                target_status
            );
            return Err(WatchTransactionError::Timeout);
        }

        // Wait for a status change or timeout
        let wait_result =
            tokio::time::timeout(timeout.saturating_sub(elapsed), status_receiver.changed()).await;

        match wait_result {
            Ok(Ok(())) => {
                // Status changed, check the new status
                let current_status = status_receiver.borrow().clone();

                match &current_status {
                    // Success terminal states
                    TransactionStatus::InBlock((block_hash, _))
                        if matches!(target_status, StatusToWait::InBlock) =>
                    {
                        debug!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} reached InBlock state",
                            nonce
                        );
                        return Ok(block_hash.clone());
                    }
                    TransactionStatus::Finalized((block_hash, _))
                        if matches!(
                            target_status,
                            StatusToWait::InBlock | StatusToWait::Finalized
                        ) =>
                    {
                        debug!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} reached Finalized state",
                            nonce
                        );
                        return Ok(block_hash.clone());
                    }

                    // Failure terminal states
                    TransactionStatus::Invalid => {
                        error!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} is invalid",
                            nonce
                        );
                        return Err(WatchTransactionError::TransactionFailed {
                            dispatch_info: "Invalid".to_string(),
                            dispatch_error: "Transaction is invalid".to_string(),
                        });
                    }
                    TransactionStatus::Dropped => {
                        error!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} was dropped",
                            nonce
                        );
                        return Err(WatchTransactionError::TransactionFailed {
                            dispatch_info: "Dropped".to_string(),
                            dispatch_error: "Transaction was dropped from pool".to_string(),
                        });
                    }
                    TransactionStatus::Usurped(_) => {
                        error!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} was usurped",
                            nonce
                        );
                        return Err(WatchTransactionError::TransactionFailed {
                            dispatch_info: "Usurped".to_string(),
                            dispatch_error: "Transaction was usurped by another transaction"
                                .to_string(),
                        });
                    }
                    TransactionStatus::FinalityTimeout(_) => {
                        error!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} had finality timeout",
                            nonce
                        );
                        return Err(WatchTransactionError::TransactionFailed {
                            dispatch_info: "FinalityTimeout".to_string(),
                            dispatch_error: "Transaction had finality timeout".to_string(),
                        });
                    }

                    // Non-terminal states, keep waiting
                    _ => {
                        debug!(
                            target: LOG_TARGET,
                            "Transaction with nonce {} is in state {:?}, waiting for {:?}",
                            nonce,
                            current_status,
                            target_status
                        );
                    }
                }
            }
            Ok(Err(_)) => {
                // Channel closed, sender was dropped
                warn!(
                    target: LOG_TARGET,
                    "Status receiver channel closed, transaction may have been removed from manager"
                );
                return Err(WatchTransactionError::TransactionNotFound);
            }
            Err(_) => {
                // Timeout elapsed
                error!(
                    target: LOG_TARGET,
                    "Timeout waiting for transaction with nonce {} to reach {:?}",
                    nonce,
                    target_status
                );
                return Err(WatchTransactionError::Timeout);
            }
        }
    }
}

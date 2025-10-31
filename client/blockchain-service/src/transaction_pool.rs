use std::collections::BTreeMap;

use anyhow::Result;
use codec::Encode;
use log::{debug, warn};
use sc_transaction_pool_api::TransactionStatus;
use sp_runtime::traits::{One, Saturating};

use crate::handler::LOG_TARGET;

/// Configuration for the transaction pool.
#[derive(Clone, Debug)]
pub struct TransactionPoolConfig {
    /// Maximum number of pending transactions to track.
    pub max_pending_transactions: u32,
    /// Number of blocks to wait before filling a nonce gap with a remark transaction.
    /// During this period, the system will try to use regular transactions to fill the gap.
    pub gap_fill_threshold_blocks: u32,
}

impl Default for TransactionPoolConfig {
    fn default() -> Self {
        Self {
            max_pending_transactions: 100,
            gap_fill_threshold_blocks: 10,
        }
    }
}

/// A pending transaction tracked by the pool.
#[derive(Clone, Debug)]
pub struct PendingTransaction<Hash, Call, BlockNumber> {
    /// Hash of the transaction.
    pub hash: Hash,
    /// The extrinsic call (TODO: for re-submission if needed).
    pub call: Call,
    /// Block number when transaction was submitted.
    pub submitted_at: BlockNumber,
    /// Latest status from the transaction watcher.
    pub latest_status: TransactionStatus<Hash, Hash>,
}

/// Information about nonce gaps detected in the transaction pool.
#[derive(Clone, Debug)]
pub struct NonceGap {
    /// The missing nonce.
    pub nonce: u32,
    /// How many blocks ago this gap was first detected.
    pub age_in_blocks: u32,
}

/// Transaction pool for tracking pending transactions and managing nonces.
///
/// Transaction state is managed via watcher events from Substrate's transaction pool.
/// This pool tracks nonces and detects gaps, but relies on watchers as the source
/// of truth for transaction lifecycle events (Ready, InBlock, Retracted, Finalized, etc.).
///
/// TODO: Make transaction pool state persistent across node restarts.
/// Currently, if the node restarts, we lose all tracking of pending transactions,
/// which nonces were used, and gap detection history.
/// The current mitigation is initializing the nonce counter from on-chain state,
/// which helps but doesn't fully solve the problem.
pub struct TransactionPool<Hash, Call, BlockNumber> {
    /// Configuration for the pool.
    pub config: TransactionPoolConfig,
    /// Map of nonce to pending transaction.
    pub(crate) pending: BTreeMap<u32, PendingTransaction<Hash, Call, BlockNumber>>,
    /// Map of nonce to block number when gap was first detected.
    detected_gaps: BTreeMap<u32, BlockNumber>,
}

impl<Hash, Call, BlockNumber> TransactionPool<Hash, Call, BlockNumber>
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
    /// Create a new transaction pool with the given configuration.
    pub fn new(config: TransactionPoolConfig) -> Self {
        Self {
            config,
            pending: BTreeMap::new(),
            detected_gaps: BTreeMap::new(),
        }
    }

    /// Track a newly submitted transaction.
    pub fn track_transaction(
        &mut self,
        nonce: u32,
        hash: Hash,
        call: Call,
        submitted_at: BlockNumber,
    ) -> Result<()> {
        if self.pending.len() >= self.config.max_pending_transactions as usize {
            warn!(
                target: LOG_TARGET,
                "Transaction pool is at capacity ({}), dropping oldest pending transaction",
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
            submitted_at,
            latest_status: TransactionStatus::Future,
        };

        self.pending.insert(nonce, pending_tx);

        // Clear gap tracking for this nonce (if it was previously detected as a gap)
        self.detected_gaps.remove(&nonce);

        Ok(())
    }

    /// Detect gaps in the nonce sequence.
    ///
    /// Returns a list of missing nonces between the on-chain nonce and the highest tracked nonce.
    ///
    /// The `local_nonce_counter` parameter is the highest nonce that has been locally assigned
    /// (not necessarily submitted or in the pool). This allows detection of gaps even when the
    /// pool is empty (e.g., after a dropped transaction is cleaned up).
    pub fn detect_gaps(
        &mut self,
        on_chain_nonce: u32,
        local_nonce_counter: u32,
        current_block: BlockNumber,
    ) -> Vec<NonceGap> {
        let mut gaps = Vec::new();

        // Determine the highest nonce we need to check
        // This is the maximum of:
        // 1. The highest nonce in the pool (if any)
        // 2. The local nonce counter
        let max_nonce = self
            .pending
            .keys()
            .next_back()
            .copied()
            .unwrap_or(on_chain_nonce)
            .max(local_nonce_counter);

        // If max_nonce equals on-chain nonce, there are no gaps to check
        if max_nonce == on_chain_nonce {
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

    /// Remove a transaction from the pool completely.
    ///
    /// This removes both the pending transaction and any gap tracking history.
    /// Use this when a transaction is permanently replaced (e.g., Usurped).
    pub fn remove(&mut self, nonce: u32) {
        self.pending.remove(&nonce);
        self.detected_gaps.remove(&nonce);
    }

    /// Remove a transaction from pending but preserve gap tracking.
    ///
    /// This allows the gap detection system to remember when the gap was first detected,
    /// to execute the gap-filling logic later if needed.
    /// Use this for retriable terminal states (Invalid, Dropped) where we want to
    /// potentially fill the gap later.
    pub fn remove_pending_but_keep_gap(&mut self, nonce: u32) {
        self.pending.remove(&nonce);
    }

    /// Clean up the stale nonce gaps in the transaction pool.
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
}

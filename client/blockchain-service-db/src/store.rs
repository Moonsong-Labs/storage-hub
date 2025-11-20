use diesel::{dsl::sql, prelude::*, sql_types::Bool};
use diesel_async::RunQueryDsl;
use log::{debug, info, warn};
use sc_transaction_pool_api::TransactionStatus;

use crate::{models::NewPendingTransaction, schema::pending_transactions, DbPool, LOG_TARGET};

#[derive(Clone)]
pub struct PendingTxStore {
    pool: DbPool,
}

impl PendingTxStore {
    /// Create a new `PendingTxStore` backed by the provided asynchronous Diesel pool.
    ///
    /// This constructor does not perform any I/O. Connections are acquired lazily
    /// for each operation. The store is cheap to clone as it only holds the pool.
    ///
    /// - `pool`: Asynchronous database pool used for all CRUD operations.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert or update a pending transaction in the database with state `"future"`.
    ///
    /// This performs an atomic upsert keyed by the composite `(account_id, nonce)`:
    /// - If no row exists, a new row is inserted.
    /// - If a row exists, `hash`, `call_scale`, `state` and `creator_id` are updated.
    ///
    /// Parameters:
    /// - `account_id`: Account identifier (raw bytes) that owns the transaction.
    /// - `nonce`: Nonce of the transaction for the given account.
    /// - `hash`: Extrinsic hash (raw bytes).
    /// - `call_scale`: SCALE-encoded call bytes for the extrinsic.
    /// - `creator_id`: Logical identifier of the node/instance that created the record.
    ///
    /// Returns:
    /// - `Ok(())` on success.
    /// - `Err(diesel::result::Error)` if the database operation fails.
    pub async fn upsert_sent(
        &self,
        account_id: &[u8],
        nonce: i64,
        hash: &[u8],
        call_scale: &[u8],
        extrinsic_scale: &[u8],
        creator_id: &str,
    ) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;

        let new_row = NewPendingTransaction {
            account_id,
            nonce,
            hash,
            call_scale,
            extrinsic_scale,
            watched: true,
            state: "future",
            creator_id,
        };

        let mut conn = self.pool.get().await.unwrap();
        // Atomic upsert: insert or update on conflict
        let inserted: bool = diesel::insert_into(pt::pending_transactions)
            .values(&new_row)
            .on_conflict((pt::account_id, pt::nonce))
            .do_update()
            .set((
                pt::hash.eq(hash),
                pt::call_scale.eq(call_scale),
                pt::extrinsic_scale.eq(extrinsic_scale),
                pt::watched.eq(true),
                pt::state.eq("future"),
                pt::creator_id.eq(creator_id),
            ))
            // Detect whether the row was inserted (true) or updated (false)
            .returning(sql::<Bool>("xmax = 0"))
            .get_result(&mut conn)
            .await?;

        if !inserted {
            info!(
                target: LOG_TARGET,
                "🔄 Upserted existing pending tx (account ID: {:?}, nonce: {}) with new values",
                hex::encode(account_id), nonce
            );
        }

        Ok(())
    }

    /// Update the persisted state for a pending transaction from a watcher status.
    ///
    /// Behaviour:
    /// - If a row for `(account_id, nonce)` does not exist, a minimal row is inserted with:
    ///   - `hash` set to `tx_hash`
    ///   - empty `call_scale`
    ///   - `creator_id` from the `SH_NODE_INSTANCE_ID` environment variable, or `"local"` if unset
    /// - On conflict, only the `state` column is updated. If the hash of the transaction being
    ///   updated is different from the stored hash, a warning is logged.
    /// - Terminal states are logged at debug level
    ///
    /// Parameters:
    /// - `account_id`: Account identifier (raw bytes).
    /// - `nonce`: Transaction nonce.
    /// - `status`: Incoming `TransactionStatus` used to derive the DB state.
    /// - `tx_hash`: Known transaction hash (raw bytes) used when inserting a missing row.
    ///
    /// Returns:
    /// - `Ok(())` on success.
    /// - `Err(diesel::result::Error)` if the database operation fails.
    pub async fn update_state<Hash>(
        &self,
        account_id: &[u8],
        nonce: i64,
        status: &TransactionStatus<Hash, Hash>,
        tx_hash: &[u8],
    ) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let (state_str, is_terminal) = Self::status_to_db_state(status);

        let mut conn = self.pool.get().await.unwrap();

        // If the row doesn't exist, create minimal row with empty call_scale, extrinsic_scale, and creator_id from env/default.
        // TODO: Use this when we implement multiple instances of the same provider.
        let creator_id =
            std::env::var("SH_NODE_INSTANCE_ID").unwrap_or_else(|_| "local".to_string());
        let new_row = NewPendingTransaction {
            account_id,
            nonce,
            hash: tx_hash,
            call_scale: &[],
            extrinsic_scale: &[],
            watched: true,
            state: state_str,
            creator_id: &creator_id,
        };

        // Upsert state and fetch whether it was inserted as well as the current stored hash
        let (inserted, current_hash): (bool, Vec<u8>) =
            diesel::insert_into(pt::pending_transactions)
                .values(&new_row)
                .on_conflict((pt::account_id, pt::nonce))
                .do_update()
                .set(pt::state.eq(state_str))
                // Detect whether the row was inserted (true) or updated (false)
                .returning((sql::<Bool>("xmax = 0"), pt::hash))
                .get_result(&mut conn)
                .await?;

        // Case: Missing transaction, a new row was inserted. Logging a warning.
        if inserted {
            warn!(
                target: LOG_TARGET,
                "🆕 Missing row for update; inserted pending tx (account ID: {:?}, nonce: {}, state: {})",
                hex::encode(account_id),
                nonce,
                state_str
            );
        }

        // Case: There was an existing transaction with that nonce, but the hash is different. Logging a warning.
        if !inserted && current_hash != tx_hash {
            // Hash mismatch: trying to update state for a different tx than stored for (account, nonce)
            warn!(
                target: LOG_TARGET,
                "⚠️ Hash mismatch while updating pending tx state (account ID: {:?}, nonce: {}): stored hash {:?}, attempted hash {:?}",
                hex::encode(account_id),
                nonce,
                hex::encode(&current_hash),
                hex::encode(tx_hash)
            );
        }

        if is_terminal {
            debug!(
                target: LOG_TARGET,
                "Terminal state set for pending tx (account ID: {:?}, nonce: {}): {}",
                hex::encode(account_id),
                nonce,
                state_str
            );
        }

        // TODO: pg_notify on update so other nodes can mirror state
        Ok(())
    }

    /// Update watched flag for a specific `(account_id, nonce)`.
    pub async fn set_watched(
        &self,
        account_id: &[u8],
        nonce: i64,
        watched: bool,
    ) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        diesel::update(
            pt::pending_transactions.filter(pt::account_id.eq(account_id).and(pt::nonce.eq(nonce))),
        )
        .set(pt::watched.eq(watched))
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    /// Set the `watched` flag for **all** pending transactions in the table.
    ///
    /// This is primarily used on startup to reset watcher state before
    /// selectively re-marking rows as watched again.
    pub async fn set_watched_for_all(&self, watched: bool) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        diesel::update(pt::pending_transactions)
            .set(pt::watched.eq(watched))
            .execute(&mut conn)
            .await?;
        Ok(())
    }

    /// Bulk update the `watched` flag for a set of nonces belonging to a single account.
    ///
    /// This allows us to avoid one UPDATE per row when re-attaching watchers
    /// on startup.
    pub async fn set_watched_for_nonces(
        &self,
        account_id: &[u8],
        nonces: &[i64],
        watched: bool,
    ) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;

        if nonces.is_empty() {
            return Ok(());
        }

        let mut conn = self.pool.get().await.unwrap();
        diesel::update(
            pt::pending_transactions
                .filter(pt::account_id.eq(account_id).and(pt::nonce.eq_any(nonces))),
        )
        .set(pt::watched.eq(watched))
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    /// Remove a single pending transaction by `(account_id, nonce)`.
    ///
    /// This operation succeeds even if the target row does not exist (deleting zero rows).
    ///
    /// Parameters:
    /// - `account_id`: Account identifier (raw bytes).
    /// - `nonce`: Transaction nonce to delete.
    ///
    /// Returns:
    /// - `Ok(())` on success.
    /// - `Err(diesel::result::Error)` if the database operation fails.
    pub async fn remove(&self, account_id: &[u8], nonce: i64) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        diesel::delete(
            pt::pending_transactions.filter(pt::account_id.eq(account_id).and(pt::nonce.eq(nonce))),
        )
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    /// Load pending transactions for a given `account_id` filtered by a set of states.
    ///
    /// Returns rows ordered by nonce ascending.
    pub async fn load_for_account_with_states<Hash>(
        &self,
        account_id: &[u8],
        states: Vec<TransactionStatus<Hash, Hash>>,
    ) -> Result<Vec<crate::models::PendingTransactionRow>, diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        let states_vec: Vec<String> = states
            .iter()
            .map(|status| Self::status_to_db_state(status).0.to_string())
            .collect();
        let rows = pt::pending_transactions
            .filter(
                pt::account_id
                    .eq(account_id)
                    .and(pt::state.eq_any(states_vec)),
            )
            .order(pt::nonce.asc())
            .load::<crate::models::PendingTransactionRow>(&mut conn)
            .await?;
        Ok(rows)
    }

    /// Load rows for resubscribe flow, selecting only needed columns via Diesel DSL.
    pub async fn load_resubscribe_rows<Hash>(
        &self,
        account_id: &[u8],
        states: Vec<TransactionStatus<Hash, Hash>>,
    ) -> Result<Vec<crate::models::PendingResubscribeRow>, diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        let states_vec: Vec<String> = states
            .iter()
            .map(|status| Self::status_to_db_state(status).0.to_string())
            .collect();
        let rows = pt::pending_transactions
            .filter(
                pt::account_id
                    .eq(account_id)
                    .and(pt::state.eq_any(states_vec)),
            )
            .select((
                pt::account_id,
                pt::nonce,
                pt::extrinsic_scale,
                pt::call_scale,
                pt::state,
            ))
            .order(pt::nonce.asc())
            .load::<crate::models::PendingResubscribeRow>(&mut conn)
            .await?;
        Ok(rows)
    }

    /// Delete all pending transactions for `account_id` with `nonce` strictly below `nonce_threshold`.
    ///
    /// Each deletion is logged at debug level to aid in reconciliation and auditing.
    ///
    /// Parameters:
    /// - `account_id`: Account identifier (raw bytes).
    /// - `nonce_threshold`: Nonce threshold; all rows with `nonce < nonce_threshold` are removed.
    ///
    /// Returns:
    /// - `Ok(i64)` indicating the number of rows deleted.
    /// - `Err(diesel::result::Error)` if the database operation fails.
    pub async fn delete_below_nonce(
        &self,
        account_id: &[u8],
        nonce_threshold: i64,
    ) -> Result<i64, diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();

        let rows_to_delete = pt::pending_transactions
            .filter(
                pt::account_id
                    .eq(account_id)
                    .and(pt::nonce.lt(nonce_threshold)),
            )
            .load::<crate::models::PendingTransactionRow>(&mut conn)
            .await?;

        if !rows_to_delete.is_empty() {
            for row in &rows_to_delete {
                debug!(
                    target: LOG_TARGET,
                    "🗑️ Deleting pending tx (account ID: {:?}, nonce: {}, state: {}, threshold: {})",
                    hex::encode(&row.account_id),
                    row.nonce,
                    row.state,
                    nonce_threshold
                );
            }
        }

        let deleted = diesel::delete(
            pt::pending_transactions.filter(
                pt::account_id
                    .eq(account_id)
                    .and(pt::nonce.lt(nonce_threshold)),
            ),
        )
        .execute(&mut conn)
        .await
        .map(|deleted| deleted as i64)?;

        Ok(deleted)
    }

    /// Convert a `TransactionStatus` into its database state string and terminal flag.
    ///
    /// This helper maps the Substrate transaction lifecycle to StorageHub's
    /// persisted string states and indicates whether a state is terminal:
    /// - Non-terminal: `"future"`, `"ready"`, `"broadcast"`, `"in_block"`, `"retracted"`
    /// - Terminal: `"finalized"`, `"usurped"`, `"dropped"`, `"invalid"`, `"finality_timeout"`
    ///
    /// Returns:
    /// - `(state_str, is_terminal)` where `state_str` is the database value and `is_terminal`
    ///   signals whether no further updates are expected.
    pub fn status_to_db_state<Hash>(
        status: &TransactionStatus<Hash, Hash>,
    ) -> (&'static str, bool) {
        match status {
            TransactionStatus::Future => ("future", false),
            TransactionStatus::Ready => ("ready", false),
            TransactionStatus::Broadcast(_) => ("broadcast", false),
            TransactionStatus::InBlock(_) => ("in_block", false),
            TransactionStatus::Retracted(_) => ("retracted", false),
            TransactionStatus::Finalized(_) => ("finalized", true),
            TransactionStatus::Usurped(_) => ("usurped", true),
            TransactionStatus::Dropped => ("dropped", true),
            TransactionStatus::Invalid => ("invalid", true),
            TransactionStatus::FinalityTimeout(_) => ("finality_timeout", true),
        }
    }
}

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
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert_sent(
        &self,
        account_id: &[u8],
        nonce: i64,
        hash: &[u8],
        call_scale: &[u8],
        creator_id: &str,
    ) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;

        let new_row = NewPendingTransaction {
            account_id,
            nonce,
            hash,
            call_scale,
            state: "sent",
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
                pt::state.eq("sent"),
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

    /// Update state based on watcher status. If row doesn't exist, it will be created with minimal fields.
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

        // If the row doesn't exist, create minimal row with empty call_scale and creator_id from env/default.
        // TODO: Use this when we implement multiple instances of the same provider.
        let creator_id =
            std::env::var("SH_NODE_INSTANCE_ID").unwrap_or_else(|_| "local".to_string());
        let new_row = NewPendingTransaction {
            account_id,
            nonce,
            hash: tx_hash,
            call_scale: &[],
            state: state_str,
            creator_id: &creator_id,
        };

        let inserted: bool = diesel::insert_into(pt::pending_transactions)
            .values(&new_row)
            .on_conflict((pt::account_id, pt::nonce))
            .do_update()
            .set(pt::state.eq(state_str))
            .returning(sql::<Bool>("xmax = 0"))
            .get_result(&mut conn)
            .await?;

        if inserted {
            warn!(
                target: LOG_TARGET,
                "🆕 Missing row for update; inserted pending tx (account ID: {:?}, nonce: {}, state: {})",
                hex::encode(account_id),
                nonce,
                state_str
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

    pub async fn load_active(
        &self,
    ) -> Result<Vec<crate::models::PendingTransactionRow>, diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        let rows = pt::pending_transactions
            .filter(pt::state.eq_any(vec!["queued", "sent", "in_block"]))
            .load::<crate::models::PendingTransactionRow>(&mut conn)
            .await?;
        Ok(rows)
    }

    pub async fn delete_below_nonce(
        &self,
        account_id: &[u8],
        nonce_threshold: i64,
    ) -> Result<i64, diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
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

    /// Convert a TransactionStatus to a database state string.
    ///
    /// Returns a tuple of the database state string and a boolean indicating if the state is terminal.
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

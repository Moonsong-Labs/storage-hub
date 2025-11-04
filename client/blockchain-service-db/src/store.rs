use diesel::prelude::*;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;

use crate::{models::NewPendingTransaction, schema::pending_transactions, DbPool};

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
        nonce: i32,
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
        diesel::insert_into(pt::pending_transactions)
            .values(&new_row)
            .on_conflict((pt::account_id, pt::nonce))
            .do_nothing()
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn update_state(
        &self,
        account_id: &[u8],
        nonce: i32,
        state: &str,
    ) -> Result<(), diesel::result::Error> {
        use pending_transactions::dsl as pt;
        let mut conn = self.pool.get().await.unwrap();
        diesel::update(
            pt::pending_transactions.filter(pt::account_id.eq(account_id).and(pt::nonce.eq(nonce))),
        )
        .set(pt::state.eq(state))
        .execute(&mut conn)
        .await?;
        Ok(())
    }

    pub async fn remove(&self, account_id: &[u8], nonce: i32) -> Result<(), diesel::result::Error> {
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
        nonce_threshold: i32,
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
        .await?;
        Ok(deleted as i64)
    }
}

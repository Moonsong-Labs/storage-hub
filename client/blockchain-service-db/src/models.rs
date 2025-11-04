use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::schema::pending_transactions;

#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = pending_transactions)]
#[diesel(primary_key(account_id, nonce))]
pub struct PendingTransactionRow {
    pub account_id: Vec<u8>,
    pub nonce: i32,
    pub hash: Vec<u8>,
    pub call_scale: Vec<u8>,
    pub state: String,
    pub creator_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = pending_transactions)]
pub struct NewPendingTransaction<'a> {
    pub account_id: &'a [u8],
    pub nonce: i32,
    pub hash: &'a [u8],
    pub call_scale: &'a [u8],
    pub state: &'a str,
    pub creator_id: &'a str,
}



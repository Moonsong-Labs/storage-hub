use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::schema::pending_transactions;

#[derive(Debug, Clone, Queryable, Identifiable)]
#[diesel(table_name = pending_transactions)]
#[diesel(primary_key(account_id, nonce))]
pub struct PendingTransactionRow {
    pub account_id: Vec<u8>,
    pub nonce: i64,
    pub hash: Vec<u8>,
    pub call_scale: Option<Vec<u8>>,
    pub extrinsic_scale: Vec<u8>,
    pub watched: bool,
    pub state: String,
    pub creator_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = pending_transactions)]
pub struct NewPendingTransaction<'a> {
    pub account_id: &'a [u8],
    pub nonce: i64,
    pub hash: &'a [u8],
    pub call_scale: Option<&'a [u8]>,
    pub extrinsic_scale: &'a [u8],
    pub watched: bool,
    pub state: &'a str,
    pub creator_id: &'a str,
}

/// Minimal row for resubscribe flow, selected via Diesel projection.
#[derive(Debug, Clone, Queryable)]
pub struct PendingResubscribeRow {
    pub account_id: Vec<u8>,
    pub nonce: i64,
    pub extrinsic_scale: Vec<u8>,
    pub call_scale: Option<Vec<u8>>,
    pub state: String,
}

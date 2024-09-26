use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::multiaddress;

/// Table that holds the list of multiaddresses that the indexer is interested in.
#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = multiaddress)]
pub struct MultiAddress {
    pub id: i32,
    pub address: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::multiaddress, DbConnection};

/// Table that holds the list of multiaddresses that the indexer is interested in.
#[derive(Debug, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = multiaddress)]
pub struct MultiAddress {
    pub id: i64,
    pub address: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl MultiAddress {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        address: impl Into<Vec<u8>>,
    ) -> Result<Self, diesel::result::Error> {
        let multiaddress = diesel::insert_into(multiaddress::table)
            .values(multiaddress::address.eq(address.into()))
            .returning(MultiAddress::as_select())
            .get_result(conn)
            .await?;
        Ok(multiaddress)
    }
}

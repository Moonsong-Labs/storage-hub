use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::service_state, DbConnection};

/// A single record table that holds the state/metadata of the indexer service.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = service_state)]
pub struct ServiceState {
    pub id: i32,
    pub last_indexed_finalized_block: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl ServiceState {
    pub async fn get<'a>(conn: &mut DbConnection<'a>) -> Result<Self, diesel::result::Error> {
        service_state::table.first(conn).await
    }

    pub async fn update<'a>(
        conn: &mut DbConnection<'a>,
        last_indexed_finalized_block: i64,
    ) -> Result<Self, diesel::result::Error> {
        diesel::update(service_state::table)
            .filter(service_state::id.eq(1))
            .set(service_state::last_indexed_finalized_block.eq(last_indexed_finalized_block))
            .get_result(conn)
            .await
    }
}

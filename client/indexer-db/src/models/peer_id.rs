use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::peer_id, DbConnection};

/// Table that holds the list of peers that the indexer is interested in.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = peer_id)]
pub struct PeerId {
    pub id: i32,
    pub peer: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl PeerId {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        peer: impl Into<Vec<u8>>,
    ) -> Result<Self, diesel::result::Error> {
        let peer_id = diesel::insert_into(peer_id::table)
            .values(peer_id::peer.eq(peer.into()))
            .returning(PeerId::as_select())
            .get_result(conn)
            .await?;
        Ok(peer_id)
    }
}

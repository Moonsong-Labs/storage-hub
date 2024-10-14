use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::proofs, DbConnection};

/// A single record table that holds the state/metadata of the indexer service.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = proofs)]
pub struct Proofs {
    pub id: i32,
    pub provider_id: String,
    pub last_tick_proven: i64,
}

impl Proofs {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        provider_id: String,
        last_tick_proven: i64,
    ) -> Result<Self, diesel::result::Error> {
        let proofs = diesel::insert_into(proofs::table)
            .values((
                proofs::provider_id.eq(provider_id),
                proofs::last_tick_proven.eq(last_tick_proven),
            ))
            .returning(Proofs::as_select())
            .get_result(conn)
            .await?;

        Ok(proofs)
    }
    pub async fn get_by_provider_id<'a>(
        conn: &mut DbConnection<'a>,
        provider_id: String,
    ) -> Result<Self, diesel::result::Error> {
        let proof = proofs::table
            .filter(proofs::provider_id.eq(provider_id))
            .first(conn)
            .await?;
        Ok(proof)
    }

    pub async fn update_last_tick_proven<'a>(
        conn: &mut DbConnection<'a>,
        id: i32,
        last_tick_proven: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(proofs::table)
            .filter(proofs::id.eq(id))
            .set(proofs::last_tick_proven.eq(last_tick_proven))
            .execute(conn)
            .await?;
        Ok(())
    }
}

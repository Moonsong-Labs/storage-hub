use diesel::prelude::*;

use crate::{schema::proofs, DbConnection};

/// A single record table that holds the state/metadata of the indexer service.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = proofs)]
pub struct Proofs {
    pub id: i32,
    pub provider_id: String,
    pub proof: String,
}

impl Proofs {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        provider_id: String,
        proof: String,
    ) -> Result<Self, diesel::result::Error> {
        let proofs = diesel::insert_into(proofs::table)
            .values((proofs::provider_id.eq(provider_id), proofs::proof.eq(proof)))
            .returning(Proofs::as_select())
            .get_result(conn)
            .await?;

        Ok(proofs)
    }
    pub async fn get<'a>(conn: &mut DbConnection<'a>) -> Result<Self, diesel::result::Error> {
        proofs::table.first(conn).await
    }
}

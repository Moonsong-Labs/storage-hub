use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    models::multiaddress::MultiAddress,
    schema::{bsp, bsp_file, bsp_multiaddress},
    DbConnection,
};

/// Table that holds the BSPs.
/// The account is guaranteed to be unique across both MSPs and BSPs.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = bsp)]
pub struct Bsp {
    /// The ID of the BSP as stored in the database. For the runtime id, use `onchain_bsp_id`.
    pub id: i32,
    pub account: String,
    pub capacity: BigDecimal,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub onchain_bsp_id: String,
}

/// Association table between BSP and MultiAddress
#[derive(Debug, Queryable, Insertable, Associations)]
#[diesel(table_name = bsp_multiaddress)]
#[diesel(belongs_to(Bsp, foreign_key = bsp_id))]
#[diesel(belongs_to(MultiAddress, foreign_key = multiaddress_id))]
pub struct BspMultiAddress {
    pub bsp_id: i32,
    pub multiaddress_id: i32,
}

impl Bsp {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        capacity: BigDecimal,
        multiaddresses: Vec<MultiAddress>,
        onchain_bsp_id: String,
    ) -> Result<Self, diesel::result::Error> {
        let bsp = diesel::insert_into(bsp::table)
            .values((
                bsp::account.eq(account),
                bsp::capacity.eq(capacity),
                bsp::onchain_bsp_id.eq(onchain_bsp_id),
            ))
            .returning(Bsp::as_select())
            .get_result(conn)
            .await?;

        diesel::insert_into(bsp_multiaddress::table)
            .values(
                multiaddresses
                    .into_iter()
                    .map(|ma| {
                        (
                            bsp_multiaddress::bsp_id.eq(bsp.id),
                            bsp_multiaddress::multiaddress_id.eq(ma.id),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(conn)
            .await?;

        Ok(bsp)
    }

    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(bsp::table)
            .filter(bsp::account.eq(account))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn update_capacity<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        capacity: BigDecimal,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bsp::table)
            .filter(bsp::account.eq(account))
            .set(bsp::capacity.eq(capacity))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn get_by_onchain_bsp_id<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: String,
    ) -> Result<Self, diesel::result::Error> {
        let bsp = bsp::table
            .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id))
            .first(conn)
            .await?;
        Ok(bsp)
    }

    pub async fn get_by_account<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
    ) -> Result<Self, diesel::result::Error> {
        let bsp = bsp::table
            .filter(bsp::account.eq(account))
            .first(conn)
            .await?;
        Ok(bsp)
    }
}

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = bsp_file)]
pub struct BspFile {
    pub bsp_id: i32,
    pub file_id: i32,
}

impl BspFile {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        bsp_id: i32,
        file_id: i32,
    ) -> Result<(), diesel::result::Error> {
        diesel::insert_into(bsp_file::table)
            .values((bsp_file::bsp_id.eq(bsp_id), bsp_file::file_id.eq(file_id)))
            .execute(conn)
            .await?;
        Ok(())
    }
}

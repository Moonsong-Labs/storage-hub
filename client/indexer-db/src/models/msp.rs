use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    models::multiaddress::MultiAddress,
    schema::{msp, msp_multiaddress},
    DbConnection,
};

/// Table that holds the MSPs.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = msp)]
pub struct Msp {
    /// The ID of the MSP as stored in the database. For the runtime id, use `onchain_msp_id`.
    pub id: i64,
    pub account: String,
    pub capacity: BigDecimal,
    pub value_prop: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// The onchain MSP ID
    ///
    /// It's stored as a hex-encoded string
    pub onchain_msp_id: String,
}

/// Association table between MSP and MultiAddress
#[derive(Debug, Queryable, Insertable, Associations)]
#[diesel(table_name = msp_multiaddress)]
#[diesel(belongs_to(Msp, foreign_key = msp_id))]
#[diesel(belongs_to(MultiAddress, foreign_key = multiaddress_id))]
pub struct MspMultiAddress {
    pub msp_id: i64,
    pub multiaddress_id: i64,
}

impl Msp {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        capacity: BigDecimal,
        value_prop: String,
        multiaddresses: Vec<MultiAddress>,
        onchain_msp_id: String,
    ) -> Result<Self, diesel::result::Error> {
        let msp = diesel::insert_into(msp::table)
            .values((
                msp::account.eq(account),
                msp::capacity.eq(capacity),
                msp::value_prop.eq(value_prop),
                msp::onchain_msp_id.eq(onchain_msp_id),
            ))
            .returning(Msp::as_select())
            .get_result(conn)
            .await?;

        diesel::insert_into(msp_multiaddress::table)
            .values(
                multiaddresses
                    .into_iter()
                    .map(|ma| {
                        (
                            msp_multiaddress::msp_id.eq(msp.id),
                            msp_multiaddress::multiaddress_id.eq(ma.id),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(conn)
            .await?;

        Ok(msp)
    }

    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(msp::table)
            .filter(msp::account.eq(account))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn get_by_onchain_msp_id<'a>(
        conn: &mut DbConnection<'a>,
        onchain_msp_id: String,
    ) -> Result<Self, diesel::result::Error> {
        let msp = msp::table
            .filter(msp::onchain_msp_id.eq(onchain_msp_id))
            .first(conn)
            .await?;
        Ok(msp)
    }
}

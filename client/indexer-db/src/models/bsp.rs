use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    models::multiaddress::MultiAddress,
    schema::{bsp, bsp_file, bsp_multiaddress, file},
    DbConnection,
};

/// Table that holds the BSPs.
/// The account is guaranteed to be unique across both MSPs and BSPs.
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = bsp)]
pub struct Bsp {
    /// The ID of the BSP as stored in the database. For the runtime id, use `onchain_bsp_id`.
    pub id: i64,
    pub account: String,
    pub capacity: BigDecimal,
    pub stake: BigDecimal,
    pub last_tick_proven: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub onchain_bsp_id: String,
    pub merkle_root: Vec<u8>,
}

/// Association table between BSP and MultiAddress
#[derive(Debug, Queryable, Insertable, Associations)]
#[diesel(table_name = bsp_multiaddress)]
#[diesel(belongs_to(Bsp, foreign_key = bsp_id))]
#[diesel(belongs_to(MultiAddress, foreign_key = multiaddress_id))]
pub struct BspMultiAddress {
    pub bsp_id: i64,
    pub multiaddress_id: i64,
}

impl Bsp {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: String,
        capacity: BigDecimal,
        merkle_root: Vec<u8>,
        multiaddresses: Vec<MultiAddress>,
        onchain_bsp_id: String,
        stake: BigDecimal,
    ) -> Result<Self, diesel::result::Error> {
        let bsp = diesel::insert_into(bsp::table)
            .values((
                bsp::account.eq(account),
                bsp::capacity.eq(capacity),
                bsp::onchain_bsp_id.eq(onchain_bsp_id),
                bsp::merkle_root.eq(merkle_root),
                bsp::stake.eq(stake),
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

    pub async fn update_stake<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: String,
        stake: BigDecimal,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bsp::table)
            .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id))
            .set(bsp::stake.eq(stake))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn update_last_tick_proven<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: String,
        last_tick_proven: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bsp::table)
            .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id))
            .set(bsp::last_tick_proven.eq(last_tick_proven))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn update_merkle_root<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: String,
        merkle_root: Vec<u8>,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(bsp::table)
            .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id))
            .set(bsp::merkle_root.eq(merkle_root))
            .execute(conn)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = bsp_file)]
pub struct BspFile {
    pub bsp_id: i64,
    pub file_id: i64,
}

impl BspFile {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        bsp_id: i64,
        file_id: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::insert_into(bsp_file::table)
            .values((bsp_file::bsp_id.eq(bsp_id), bsp_file::file_id.eq(file_id)))
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
        onchain_bsp_id: String,
    ) -> Result<(), diesel::result::Error> {
        use diesel::dsl::exists;

        diesel::delete(bsp_file::table)
            .filter(exists(
                file::table
                    .filter(file::id.eq(bsp_file::file_id))
                    .filter(file::file_key.eq(file_key.as_ref())),
            ))
            .filter(exists(
                bsp::table
                    .filter(bsp::id.eq(bsp_file::bsp_id))
                    .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id)),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn get_bsps_for_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: &[u8],
    ) -> Result<Vec<String>, diesel::result::Error> {
        let bsp_ids: Vec<String> = bsp_file::table
            .inner_join(bsp::table.on(bsp_file::bsp_id.eq(bsp::id)))
            .inner_join(file::table.on(bsp_file::file_id.eq(file::id)))
            .filter(file::file_key.eq(file_key))
            .select(bsp::onchain_bsp_id)
            .load::<String>(conn)
            .await?;

        Ok(bsp_ids)
    }

    /// Get all file keys stored by a specific BSP
    pub async fn get_all_file_keys_for_bsp<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: &str,
    ) -> Result<Vec<Vec<u8>>, diesel::result::Error> {
        let file_keys: Vec<Vec<u8>> = bsp_file::table
            .inner_join(bsp::table.on(bsp_file::bsp_id.eq(bsp::id)))
            .inner_join(file::table.on(bsp_file::file_id.eq(file::id)))
            .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id))
            .select(file::file_key)
            .load::<Vec<u8>>(conn)
            .await?;

        Ok(file_keys)
    }
}

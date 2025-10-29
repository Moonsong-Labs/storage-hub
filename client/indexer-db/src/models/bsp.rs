use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    models::{
        file::{FileDeletionType, DEFAULT_BATCH_QUERY_LIMIT},
        multiaddress::MultiAddress,
        File,
    },
    schema::{bsp, bsp_file, bsp_multiaddress, file},
    types::OnchainBspId,
    DbConnection,
};

/// Table that holds the BSPs.
/// The account is guaranteed to be unique across both MSPs and BSPs.
#[derive(Debug, Clone, Queryable, Insertable, Selectable)]
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
    pub onchain_bsp_id: OnchainBspId,
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
        onchain_bsp_id: OnchainBspId,
        stake: BigDecimal,
    ) -> Result<Self, diesel::result::Error> {
        let bsp = diesel::insert_into(bsp::table)
            .values((
                bsp::account.eq(account),
                bsp::capacity.eq(capacity),
                bsp::onchain_bsp_id.eq(OnchainBspId::from(onchain_bsp_id)),
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
        onchain_bsp_id: OnchainBspId,
    ) -> Result<(), diesel::result::Error> {
        diesel::delete(bsp::table)
            .filter(bsp::onchain_bsp_id.eq(onchain_bsp_id))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete_by_account<'a>(
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
        onchain_bsp_id: OnchainBspId,
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
        onchain_bsp_id: OnchainBspId,
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
        onchain_bsp_id: OnchainBspId,
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
        onchain_bsp_id: OnchainBspId,
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
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::file;

        // Delete all BSP-file associations for the given file key
        let file_key = file_key.as_ref().to_vec();
        diesel::delete(bsp_file::table)
            .filter(
                bsp_file::file_id.eq_any(
                    file::table
                        .filter(file::file_key.eq(file_key))
                        .select(file::id),
                ),
            )
            .execute(conn)
            .await?;

        Ok(())
    }

    pub async fn delete_for_bsp<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
        onchain_bsp_id: OnchainBspId,
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

    // TODO: Add paging for performance
    pub async fn get_bsps_for_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: &[u8],
    ) -> Result<Vec<OnchainBspId>, diesel::result::Error> {
        let bsp_ids: Vec<OnchainBspId> = bsp_file::table
            .inner_join(bsp::table.on(bsp_file::bsp_id.eq(bsp::id)))
            .inner_join(file::table.on(bsp_file::file_id.eq(file::id)))
            .filter(file::file_key.eq(file_key))
            .select(bsp::onchain_bsp_id)
            .load::<OnchainBspId>(conn)
            .await?;

        Ok(bsp_ids)
    }

    // TODO: Add paging for performance
    // TODO: Make this and other db model functions return actual types instead of raw bytes for example
    pub async fn get_all_file_keys_for_bsp<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bsp_id: OnchainBspId,
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

    /// Get files pending deletion grouped by BSP.
    ///
    /// Queries the `bsp_file` association table joined with `file` and `bsp` tables
    /// to find all files pending deletion, filtered by deletion type, and groups them
    /// by their associated BSP ID.
    ///
    /// # Arguments
    /// * `deletion_type` - Filter for user deletions (with signature) or incomplete deletions (without signature)
    /// * `bsp_id` - Optional filter by specific BSP's onchain ID (returns only that BSP's files)
    /// * `limit` - Maximum number of files to return across all BSPs (default: 1000)
    /// * `offset` - Number of files to skip for pagination (default: 0)
    ///
    /// # Returns
    /// HashMap mapping BSP IDs ([`OnchainBspId`]) to vectors of files pending deletion for that BSP.
    ///
    /// # Note
    /// The limit/offset applies to the total number of files retrieved, not the number of BSPs.
    /// Files are ordered by BSP ID then file_key for consistent pagination.
    pub async fn get_files_pending_deletion_grouped_by_bsp<'a>(
        conn: &mut DbConnection<'a>,
        deletion_type: FileDeletionType,
        bsp_id: Option<OnchainBspId>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<HashMap<OnchainBspId, Vec<File>>, diesel::result::Error> {
        use crate::models::file::FileDeletionStatus;

        let limit = limit.unwrap_or(DEFAULT_BATCH_QUERY_LIMIT);
        let offset = offset.unwrap_or(0);

        let mut query = bsp_file::table
            .inner_join(file::table.on(bsp_file::file_id.eq(file::id)))
            .inner_join(bsp::table.on(bsp_file::bsp_id.eq(bsp::id)))
            .filter(file::deletion_status.eq(FileDeletionStatus::InProgress as i32))
            .into_boxed();

        // Filter by signature presence based on deletion type
        query = match deletion_type {
            FileDeletionType::User => query.filter(file::deletion_signature.is_not_null()),
            FileDeletionType::Incomplete => query.filter(file::deletion_signature.is_null()),
        };

        // Filter by specific BSP ID if provided
        if let Some(bsp_id) = bsp_id {
            query = query.filter(bsp::onchain_bsp_id.eq(bsp_id));
        }

        // Load both file and BSP ID
        let results: Vec<(File, OnchainBspId)> = query
            .select((File::as_select(), bsp::onchain_bsp_id))
            .order_by((
                bsp::onchain_bsp_id.asc(),
                file::deletion_requested_at.asc(),
                file::file_key.asc(),
            ))
            .limit(limit)
            .offset(offset)
            .load(conn)
            .await?;

        // Group files by BSP ID
        let mut grouped: HashMap<OnchainBspId, Vec<File>> = HashMap::new();
        for (file, bsp_id) in results {
            grouped.entry(bsp_id).or_insert_with(Vec::new).push(file);
        }

        Ok(grouped)
    }
}

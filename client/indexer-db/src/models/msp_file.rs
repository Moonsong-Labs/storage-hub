use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    schema::{bucket, file, msp, msp_file},
    types::OnchainMspId,
    DbConnection,
};

/// Association table between MSP and File
#[derive(Debug, Queryable, Insertable, Selectable, Associations)]
#[diesel(table_name = msp_file)]
#[diesel(belongs_to(super::Msp, foreign_key = msp_id))]
#[diesel(belongs_to(super::File, foreign_key = file_id))]
pub struct MspFile {
    pub msp_id: i64,
    // TODO: Why is this is a signed int?
    pub file_id: i64,
}

impl MspFile {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        msp_id: i64,
        file_id: i64,
    ) -> Result<(), diesel::result::Error> {
        diesel::insert_into(msp_file::table)
            .values((msp_file::msp_id.eq(msp_id), msp_file::file_id.eq(file_id)))
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Deletes MSP-file associations for a given file key and specific MSP
    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        file_key: &[u8],
        onchain_msp_id: OnchainMspId,
    ) -> Result<(), diesel::result::Error> {
        // First, get the database MSP ID from the onchain MSP ID
        let msp_db_id: i64 = msp::table
            .filter(msp::onchain_msp_id.eq(onchain_msp_id))
            .select(msp::id)
            .first(conn)
            .await?;

        // Get all file IDs for the given file key
        let file_ids: Vec<i64> = file::table
            .filter(file::file_key.eq(file_key))
            .select(file::id)
            .load(conn)
            .await?;

        // Log if we found multiple files with the same key
        if file_ids.len() > 1 {
            log::warn!(
                "Found {} files with the same file_key: {:?}. This is expected only if there was more than one storage request for the same file key. Will proceed to delete all associated file IDs with this key.",
                file_ids.len(),
                file_key
            );
        }

        // If no files found, nothing to delete
        if file_ids.is_empty() {
            log::debug!("No files found with file_key: {:?}", file_key);
            return Ok(());
        }

        // Delete MSP-file associations for all file IDs and the specific MSP
        let deleted_count = diesel::delete(msp_file::table)
            .filter(msp_file::file_id.eq_any(&file_ids))
            .filter(msp_file::msp_id.eq(msp_db_id))
            .execute(conn)
            .await?;

        log::debug!(
            "Deleted {} MSP-file associations for {} file(s) with file_key: {:?} and MSP: {}",
            deleted_count,
            file_ids.len(),
            file_key,
            onchain_msp_id
        );

        Ok(())
    }

    /// Deletes MSP-file association for the latest file record of the given file key and MSP.
    ///
    /// This is specifically used for cleaning up incomplete storage requests, where the latest
    /// file record is always the one associated with the incomplete request (since no other
    /// storage request can be issued while there's an incomplete one pending).
    pub async fn delete_latest_by_file_key<'a>(
        conn: &mut DbConnection<'a>,
        msp_id: i64,
        file_key: impl AsRef<[u8]>,
    ) -> Result<(), diesel::result::Error> {
        let file_key = file_key.as_ref();

        // Get the latest file record for this file key
        let file_id: Option<i64> = file::table
            .filter(file::file_key.eq(file_key))
            .order(file::created_at.desc())
            .select(file::id)
            .first(conn)
            .await
            .optional()?;

        let Some(file_id) = file_id else {
            log::debug!(
                "No file record found for file_key {:?}, nothing to delete",
                file_key
            );
            return Ok(());
        };

        let deleted_count = diesel::delete(msp_file::table)
            .filter(msp_file::file_id.eq(file_id))
            .filter(msp_file::msp_id.eq(msp_id))
            .execute(conn)
            .await?;

        log::debug!(
            "Deleted {} MSP-file association for file_id: {} and MSP: {}",
            deleted_count,
            file_id,
            msp_id
        );

        Ok(())
    }

    pub async fn delete_by_bucket<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: &[u8],
        msp_id: i64,
    ) -> Result<(), diesel::result::Error> {
        // Get all file IDs that match the onchain bucket ID, regardless of the DB bucket ID
        let file_ids: Vec<i64> = file::table
            .filter(file::onchain_bucket_id.eq(bucket_id))
            .select(file::id)
            .load(conn)
            .await?;

        if !file_ids.is_empty() {
            // Delete all msp_file associations for these files and this MSP
            diesel::delete(msp_file::table)
                .filter(msp_file::msp_id.eq(msp_id))
                .filter(msp_file::file_id.eq_any(file_ids))
                .execute(conn)
                .await?;
        }

        Ok(())
    }

    /// Creates MSP-file associations for all files in a bucket
    pub async fn create_for_bucket<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: &[u8],
        msp_id: i64,
    ) -> Result<usize, diesel::result::Error> {
        // Get all file IDs for this bucket
        // Only create MSP-file associations for files that are in the bucket's forest,
        // as files could have been deleted while the bucket was orphaned from a MSP.
        let file_ids: Vec<i64> = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(bucket::onchain_bucket_id.eq(bucket_id))
            .filter(file::is_in_bucket.eq(true))
            .select(file::id)
            .load(conn)
            .await?;

        if file_ids.is_empty() {
            log::debug!("No files found in bucket: {:?}", bucket_id);
            return Ok(0);
        }

        // Create MSP-file associations for all files
        let values: Vec<_> = file_ids
            .iter()
            .map(|&file_id| (msp_file::msp_id.eq(msp_id), msp_file::file_id.eq(file_id)))
            .collect();

        let created_count = diesel::insert_into(msp_file::table)
            .values(&values)
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;

        log::debug!(
            "Created {} MSP-file associations for bucket [{:?}] with MSP {}",
            created_count,
            bucket_id,
            msp_id
        );

        Ok(created_count)
    }

    pub async fn get_msp_for_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: &[u8],
    ) -> Result<Option<OnchainMspId>, diesel::result::Error> {
        let msp_id: Option<OnchainMspId> = file::table
            .filter(file::file_key.eq(file_key))
            .inner_join(msp_file::table.on(file::id.eq(msp_file::file_id)))
            .inner_join(msp::table.on(msp_file::msp_id.eq(msp::id)))
            .select(msp::onchain_msp_id)
            .first(conn)
            .await
            .optional()?;

        Ok(msp_id)
    }
}

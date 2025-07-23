use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{schema::msp_file, DbConnection};

/// Association table between MSP and File
#[derive(Debug, Queryable, Insertable, Selectable, Associations)]
#[diesel(table_name = msp_file)]
#[diesel(belongs_to(super::Msp, foreign_key = msp_id))]
#[diesel(belongs_to(super::File, foreign_key = file_id))]
pub struct MspFile {
    pub msp_id: i64,
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
            .returning(MspFile::as_select())
            .get_result(conn)
            .await?;
        Ok(())
    }

    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        file_key: &[u8],
        msp_onchain_id: String,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::{file, msp};

        // First get the file_id and msp_id
        let (file_id, msp_id): (i64, i64) = file::table
            .filter(file::file_key.eq(file_key))
            .inner_join(msp_file::table.on(file::id.eq(msp_file::file_id)))
            .inner_join(msp::table.on(msp_file::msp_id.eq(msp::id)))
            .filter(msp::onchain_msp_id.eq(msp_onchain_id))
            .select((file::id, msp::id))
            .first(conn)
            .await?;

        // Delete the association
        diesel::delete(msp_file::table)
            .filter(msp_file::msp_id.eq(msp_id))
            .filter(msp_file::file_id.eq(file_id))
            .execute(conn)
            .await?;

        Ok(())
    }

    pub async fn delete_by_bucket<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: &[u8],
        msp_id: i64,
    ) -> Result<(), diesel::result::Error> {
        use crate::schema::{bucket, file};

        // Get all file IDs for this bucket
        let file_ids: Vec<i64> = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(bucket::onchain_bucket_id.eq(bucket_id))
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
}

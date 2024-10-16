use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    models::PeerId,
    schema::{file, file_peer_id},
    DbConnection,
};

pub enum FileStorageRequestStep {
    Requested = 0,
    Fullfilled = 1,
    Expired = 2,
    Revoked = 3,
}

/// Table that holds the Files (both ongoing requests and completed).
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = file)]
pub struct File {
    /// The ID of the file as stored in the database. For the runtime id, use `onchain_bsp_id`.
    pub id: i32,
    pub account: String,
    pub file_key: Vec<u8>,
    pub bucket_id: i32,
    pub location: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub size: i64,
    /// The step this file is at. 0 = requested, 1 = fulfilled.
    pub step: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Association table between File and PeerId
#[derive(Debug, Queryable, Insertable, Associations)]
#[diesel(table_name = file_peer_id)]
#[diesel(belongs_to(File, foreign_key = file_id))]
#[diesel(belongs_to(PeerId, foreign_key = peer_id))]
pub struct FilePeerId {
    pub file_id: i32,
    pub peer_id: i32,
}

impl File {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: impl Into<String>,
        file_key: impl Into<Vec<u8>>,
        bucket_id: i32,
        location: impl Into<Vec<u8>>,
        fingerprint: impl Into<Vec<u8>>,
        size: i64,
        step: FileStorageRequestStep,
        peer_ids: Vec<PeerId>,
    ) -> Result<Self, diesel::result::Error> {
        let file = diesel::insert_into(file::table)
            .values((
                file::account.eq(account.into()),
                file::file_key.eq(file_key.into()),
                file::bucket_id.eq(bucket_id),
                file::location.eq(location.into()),
                file::fingerprint.eq(fingerprint.into()),
                file::size.eq(size),
                file::step.eq(step as i32),
            ))
            .returning(File::as_select())
            .get_result(conn)
            .await?;

        diesel::insert_into(file_peer_id::table)
            .values(
                peer_ids
                    .into_iter()
                    .map(|peer_id| {
                        (
                            file_peer_id::file_id.eq(file.id),
                            file_peer_id::peer_id.eq(peer_id.id),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(conn)
            .await?;

        Ok(file)
    }

    pub async fn get_by_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<Self, diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let file = file::table
            .filter(file::file_key.eq(file_key))
            .first::<Self>(conn)
            .await?;
        Ok(file)
    }

    pub async fn update_step<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
        step: FileStorageRequestStep,
    ) -> Result<(), diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        diesel::update(file::table)
            .filter(file::file_key.eq(file_key))
            .set(file::step.eq(step as i32))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn delete<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<(), diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        diesel::delete(file::table)
            .filter(file::file_key.eq(file_key))
            .execute(conn)
            .await?;
        Ok(())
    }
}

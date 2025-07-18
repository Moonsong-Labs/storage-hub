use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use sc_network::{Multiaddr, PeerId};

use shc_common::types::{FileMetadata, Fingerprint};

use crate::{
    models::MultiAddress,
    schema::{bucket, file, file_peer_id},
    DbConnection,
};

pub enum FileStorageRequestStep {
    Requested = 0,
    Stored = 1,
}

/// Table that holds the Files (both ongoing requests and completed).
#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = file)]
pub struct File {
    /// The ID of the file as stored in the database. For the runtime id, use `onchain_bsp_id`.
    pub id: i64,
    pub account: Vec<u8>,
    pub file_key: Vec<u8>,
    pub bucket_id: i64,
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
#[diesel(belongs_to(crate::models::PeerId, foreign_key = peer_id))]
pub struct FilePeerId {
    pub file_id: i64,
    pub peer_id: i64,
}

impl File {
    pub async fn create<'a>(
        conn: &mut DbConnection<'a>,
        account: impl Into<Vec<u8>>,
        file_key: impl Into<Vec<u8>>,
        bucket_id: i64,
        location: impl Into<Vec<u8>>,
        fingerprint: impl Into<Vec<u8>>,
        size: i64,
        step: FileStorageRequestStep,
        peer_ids: Vec<crate::models::PeerId>,
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

    pub async fn get_by_bucket_id<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: i64,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let files = file::table
            .filter(file::bucket_id.eq(bucket_id))
            .load(conn)
            .await?;
        Ok(files)
    }

    pub async fn get_by_onchain_bucket_id<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: Vec<u8>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let files = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .select(File::as_select())
            .load(conn)
            .await?;
        Ok(files)
    }

    pub async fn get_bsp_peer_ids(
        &self,
        conn: &mut DbConnection<'_>,
    ) -> Result<Vec<PeerId>, diesel::result::Error> {
        use crate::schema::{bsp, bsp_file, bsp_multiaddress, multiaddress};

        let peer_ids: Vec<PeerId> = bsp_file::table
            .filter(bsp_file::file_id.eq(self.id))
            .inner_join(bsp::table.on(bsp_file::bsp_id.eq(bsp::id)))
            .inner_join(bsp_multiaddress::table.on(bsp::id.eq(bsp_multiaddress::bsp_id)))
            .inner_join(
                multiaddress::table.on(multiaddress::id.eq(bsp_multiaddress::multiaddress_id)),
            )
            .select(multiaddress::all_columns)
            .distinct()
            .load::<MultiAddress>(conn)
            .await?
            .into_iter()
            .filter_map(|multiaddress| {
                Multiaddr::try_from(multiaddress.address)
                    .ok()
                    .and_then(|ma| PeerId::try_from_multiaddr(&ma))
            })
            .collect();

        Ok(peer_ids)
    }

    pub async fn get_user_files<'a>(
        conn: &mut DbConnection<'a>,
        user_account: impl AsRef<[u8]>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let account = user_account.as_ref().to_vec();
        let files = file::table
            .filter(file::account.eq(account))
            .load(conn)
            .await?;
        Ok(files)
    }

    pub async fn get_user_files_by_msp<'a>(
        conn: &mut DbConnection<'a>,
        user_account: impl AsRef<[u8]>,
        msp_id: i64,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let account = user_account.as_ref().to_vec();
        let files = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(file::account.eq(account))
            .filter(bucket::msp_id.eq(msp_id))
            .select(File::as_select())
            .load(conn)
            .await?;
        Ok(files)
    }

    pub async fn get_msp_peer_ids(
        &self,
        conn: &mut DbConnection<'_>,
    ) -> Result<Vec<PeerId>, diesel::result::Error> {
        use crate::schema::{msp, msp_file, msp_multiaddress, multiaddress};

        // Get MSP peer IDs through msp_file association
        let peer_ids: Vec<PeerId> = msp_file::table
            .filter(msp_file::file_id.eq(self.id))
            .inner_join(msp::table.on(msp_file::msp_id.eq(msp::id)))
            .inner_join(msp_multiaddress::table.on(msp::id.eq(msp_multiaddress::msp_id)))
            .inner_join(
                multiaddress::table.on(multiaddress::id.eq(msp_multiaddress::multiaddress_id)),
            )
            .select(multiaddress::all_columns)
            .distinct()
            .load::<MultiAddress>(conn)
            .await?
            .into_iter()
            .filter_map(|multiaddress| {
                Multiaddr::try_from(multiaddress.address)
                    .ok()
                    .and_then(|ma| PeerId::try_from_multiaddr(&ma))
            })
            .collect();

        Ok(peer_ids)
    }

    pub async fn get_msp_by_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<Option<crate::models::Msp>, diesel::result::Error> {
        use crate::schema::{msp, msp_file};

        let file_key = file_key.as_ref().to_vec();

        let msp = file::table
            .filter(file::file_key.eq(file_key))
            .inner_join(msp_file::table.on(file::id.eq(msp_file::file_id)))
            .inner_join(msp::table.on(msp_file::msp_id.eq(msp::id)))
            .select(msp::all_columns)
            .first::<crate::models::Msp>(conn)
            .await
            .optional()?;

        Ok(msp)
    }
}

impl File {
    pub fn to_file_metadata(&self, onchain_bucket_id: Vec<u8>) -> Result<FileMetadata, String> {
        FileMetadata::new(
            self.account.clone(),
            onchain_bucket_id,
            self.location.clone(),
            self.size as u64,
            Fingerprint::from(self.fingerprint.as_slice()),
        )
        .map_err(|_| "Invalid file metadata".to_string())
    }
}

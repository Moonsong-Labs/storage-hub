use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use sc_network::{Multiaddr, PeerId};

use shc_common::types::{FileMetadata, Fingerprint};

use crate::{
    models::MultiAddress,
    schema::{bucket, file, file_peer_id, msp_file},
    DbConnection,
};

/// Default limit for single file queries (user or incomplete deletions)
pub(crate) const DEFAULT_FILE_QUERY_LIMIT: i64 = 100;

/// Default limit for batch queries that group by BSP or Bucket
pub(crate) const DEFAULT_BATCH_QUERY_LIMIT: i64 = 1000;

pub enum FileStorageRequestStep {
    Requested = 0,
    Stored = 1,
    Expired = 2,
}

impl TryFrom<i32> for FileStorageRequestStep {
    type Error = i32;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Requested),
            1 => Ok(Self::Stored),
            2 => Ok(Self::Expired),
            _ => Err(v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDeletionStatus {
    None = 0,
    InProgress = 1,
}

/// Type of file deletion based on presence of user signature.
///
/// Used to distinguish between user-initiated deletions (which require signatures)
/// and automated incomplete storage cleanup (which does not).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDeletionType {
    /// User-initiated deletion (has signature)
    User,
    /// Automated incomplete storage cleanup (no signature)
    Incomplete,
}

/// Table that holds the Files (both ongoing requests and completed).
#[derive(Debug, Clone, Queryable, Insertable, Selectable)]
#[diesel(table_name = file)]
pub struct File {
    /// The ID of the file as stored in the database. There's a dedicated field for the `file_key`.
    pub id: i64,
    /// Owner of the file.
    pub account: Vec<u8>,
    pub file_key: Vec<u8>,
    pub bucket_id: i64,
    pub onchain_bucket_id: Vec<u8>,
    pub location: Vec<u8>,
    pub fingerprint: Vec<u8>,
    pub size: i64,
    /// The step this file is at. 0 = requested, 1 = fulfilled.
    pub step: i32,
    /// Deletion status of the file. NULL = normal, 1 = deletion in progress.
    pub deletion_status: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    /// User signature from [`FileDeletionRequested`](pallet_file_system::Event::FileDeletionRequested) event,
    /// stored as SCALE-encoded bytes.
    ///
    /// Required by fisherman nodes to construct valid proofs for the `delete_file` extrinsic.
    /// Must be decoded using [`codec::Decode`] before use to avoid double-encoding.
    ///
    /// NULL when file is deleted through automated processes (e.g., [`StorageRequestRevoked`](pallet_file_system::Event::StorageRequestRevoked)).
    pub deletion_signature: Option<Vec<u8>>,
    /// Timestamp when file was marked for deletion.
    ///
    /// Set automatically when [`deletion_status`] becomes [`FileDeletionStatus::InProgress`] via either
    /// [`FileDeletionRequested`](pallet_file_system::Event::FileDeletionRequested) or
    /// [`IncompleteStorageRequest`](pallet_file_system::Event::IncompleteStorageRequest) events.
    ///
    /// Used for FIFO ordering of deletion processing by fisherman nodes.
    pub deletion_requested_at: Option<NaiveDateTime>,
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
        onchain_bucket_id: impl Into<Vec<u8>>,
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
                file::onchain_bucket_id.eq(onchain_bucket_id.into()),
                file::location.eq(location.into()),
                file::fingerprint.eq(fingerprint.into()),
                file::size.eq(size),
                file::step.eq(step as i32),
                file::deletion_status.eq(None::<i32>),
                file::deletion_signature.eq(None::<Vec<u8>>),
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

    pub async fn update_deletion_status<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
        status: FileDeletionStatus,
        signature: Option<Vec<u8>>,
    ) -> Result<(), diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let status_value = match status {
            FileDeletionStatus::None => None,
            FileDeletionStatus::InProgress => Some(1),
        };

        // When marking for deletion, set current timestamp; otherwise clear it
        match status {
            FileDeletionStatus::InProgress => {
                diesel::update(file::table)
                    .filter(file::file_key.eq(file_key))
                    .set((
                        file::deletion_status.eq(status_value),
                        file::deletion_signature.eq(signature),
                        file::deletion_requested_at.eq(diesel::dsl::now),
                    ))
                    .execute(conn)
                    .await?;
            }
            FileDeletionStatus::None => {
                diesel::update(file::table)
                    .filter(file::file_key.eq(file_key))
                    .set((
                        file::deletion_status.eq(status_value),
                        file::deletion_signature.eq(signature),
                        file::deletion_requested_at.eq(None::<chrono::NaiveDateTime>),
                    ))
                    .execute(conn)
                    .await?;
            }
        }
        Ok(())
    }

    /// Check if file has any MSP associations
    pub async fn has_msp_associations<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::msp_file;

        let file_key = file_key.as_ref().to_vec();

        // Get file ID
        let file_id: Option<i64> = file::table
            .filter(file::file_key.eq(&file_key))
            .select(file::id)
            .first(conn)
            .await
            .optional()?;

        if let Some(file_id) = file_id {
            let count: i64 = msp_file::table
                .filter(msp_file::file_id.eq(file_id))
                .count()
                .get_result(conn)
                .await?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Check if file has any BSP associations
    pub async fn has_bsp_associations<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::bsp_file;

        let file_key = file_key.as_ref().to_vec();

        // Get file ID
        let file_id: Option<i64> = file::table
            .filter(file::file_key.eq(&file_key))
            .select(file::id)
            .first(conn)
            .await
            .optional()?;

        if let Some(file_id) = file_id {
            let count: i64 = bsp_file::table
                .filter(bsp_file::file_id.eq(file_id))
                .count()
                .get_result(conn)
                .await?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Delete file only if it has no provider associations (orphaned)
    pub async fn delete_if_orphaned<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<bool, diesel::result::Error> {
        let file_key = file_key.as_ref();

        // Check if file has any associations
        let has_msp = Self::has_msp_associations(conn, file_key).await?;
        let has_bsp = Self::has_bsp_associations(conn, file_key).await?;

        if !has_msp && !has_bsp {
            // No associations, delete the file
            Self::delete(conn, file_key).await?;
            log::info!("Deleted orphaned file with key: {:?}", file_key);
            Ok(true)
        } else {
            log::debug!(
                "File with key {:?} still has associations (MSP: {}, BSP: {}), keeping it",
                file_key,
                has_msp,
                has_bsp
            );
            Ok(false)
        }
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

    /// Get all files belonging to a specific user account
    ///
    /// # Example
    /// ```ignore
    /// use sp_runtime::AccountId32;
    ///
    /// let user: AccountId32 = /* ... */;
    /// let files = File::get_user_files(&mut conn, user.as_ref()).await?;
    /// ```
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

    /// Get all files belonging to a specific user account and stored by a specific MSP
    ///
    /// # Example
    /// ```ignore
    /// use sp_runtime::AccountId32;
    ///
    /// let user: AccountId32 = /* ... */;
    /// let msp_id: i64 = /* ... */;
    /// let files = File::get_user_files_by_msp(&mut conn, user.as_ref(), msp_id).await?;
    /// ```
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

    pub async fn get_all_file_keys_for_bucket<'a>(
        conn: &mut DbConnection<'a>,
        onchain_bucket_id: &[u8],
    ) -> Result<Vec<Vec<u8>>, diesel::result::Error> {
        let file_keys: Vec<Vec<u8>> = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .select(file::file_key)
            .load::<Vec<u8>>(conn)
            .await?;

        Ok(file_keys)
    }

    /// Get all files pending user deletion (deletion_status = InProgress with signature).
    ///
    /// Returns files that were marked for deletion via [`FileDeletionRequested`] events,
    /// which include user signatures required for proof construction.
    ///
    /// # Arguments
    /// * `bucket_id` - Optional filter by specific bucket's onchain ID
    /// * `limit` - Maximum number of results to return (default: 100)
    /// * `offset` - Number of results to skip for pagination (default: 0)
    pub async fn get_pending_user_deletions<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: Option<&[u8]>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let limit = limit.unwrap_or(DEFAULT_FILE_QUERY_LIMIT);
        let offset = offset.unwrap_or(0);

        let mut query = file::table
            .filter(file::deletion_status.eq(FileDeletionStatus::InProgress as i32))
            .filter(file::deletion_signature.is_not_null())
            .into_boxed();

        // Filter by bucket ID if provided
        if let Some(bucket_id) = bucket_id {
            query = query.filter(file::onchain_bucket_id.eq(bucket_id));
        }

        let files = query
            .order_by(file::deletion_requested_at.asc())
            .limit(limit)
            .offset(offset)
            .load(conn)
            .await?;
        Ok(files)
    }

    /// Get all files pending incomplete storage deletion (deletion_status = InProgress without signature).
    ///
    /// Returns files that were marked for deletion via [`IncompleteStorageRequest`] events,
    /// which do not include user signatures.
    ///
    /// # Arguments
    /// * `bucket_id` - Optional filter by specific bucket's onchain ID
    /// * `limit` - Maximum number of results to return (default: 100)
    /// * `offset` - Number of results to skip for pagination (default: 0)
    pub async fn get_pending_incomplete_deletions<'a>(
        conn: &mut DbConnection<'a>,
        bucket_id: Option<&[u8]>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let limit = limit.unwrap_or(DEFAULT_FILE_QUERY_LIMIT);
        let offset = offset.unwrap_or(0);

        let mut query = file::table
            .filter(file::deletion_status.eq(FileDeletionStatus::InProgress as i32))
            .filter(file::deletion_signature.is_null())
            .into_boxed();

        // Filter by bucket ID if provided
        if let Some(bucket_id) = bucket_id {
            query = query.filter(file::onchain_bucket_id.eq(bucket_id));
        }

        let files = query
            .order_by(file::deletion_requested_at.asc())
            .limit(limit)
            .offset(offset)
            .load(conn)
            .await?;
        Ok(files)
    }

    /// Get files pending deletion grouped by bucket.
    ///
    /// Queries files with `deletion_status = InProgress` that have actual MSP associations
    /// (i.e., files that the MSP accepted and confirmed storing) and groups them by their
    /// `onchain_bucket_id`. The deletion type determines whether to include files
    /// with or without user signatures.
    ///
    /// **Important**: This function only returns files where the MSP actually accepted the
    /// storage request and created an `msp_file` association. Files that have a bucket_id
    /// but were never accepted by the MSP will NOT be included in the results.
    ///
    /// # Arguments
    /// * `deletion_type` - Filter for user deletions (with signature) or incomplete deletions (without signature)
    /// * `bucket_id` - Optional filter by specific bucket's onchain ID (returns only that bucket's files)
    /// * `limit` - Maximum number of files to return across all buckets (default: 1000)
    /// * `offset` - Number of files to skip for pagination (default: 0)
    ///
    /// # Returns
    /// HashMap mapping bucket IDs (as `Vec<u8>`) to vectors of files pending deletion in that bucket.
    /// Only includes files with actual MSP storage associations.
    ///
    /// # Note
    /// The limit/offset applies to the total number of files retrieved, not the number of buckets.
    /// Files are ordered by bucket_id then file_key for consistent pagination.
    pub async fn get_files_pending_deletion_grouped_by_bucket<'a>(
        conn: &mut DbConnection<'a>,
        deletion_type: FileDeletionType,
        bucket_id: Option<&[u8]>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<HashMap<Vec<u8>, Vec<Self>>, diesel::result::Error> {
        let limit = limit.unwrap_or(DEFAULT_BATCH_QUERY_LIMIT);
        let offset = offset.unwrap_or(0);

        let mut query = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .inner_join(msp_file::table.on(file::id.eq(msp_file::file_id)))
            .filter(file::deletion_status.eq(FileDeletionStatus::InProgress as i32))
            .into_boxed();

        // Filter by signature presence based on deletion type
        query = match deletion_type {
            FileDeletionType::User => query.filter(file::deletion_signature.is_not_null()),
            FileDeletionType::Incomplete => query.filter(file::deletion_signature.is_null()),
        };

        // Filter by specific bucket ID if provided
        if let Some(bucket_id) = bucket_id {
            query = query.filter(file::onchain_bucket_id.eq(bucket_id));
        }

        let files: Vec<Self> = query
            .select(File::as_select())
            .order_by((
                file::onchain_bucket_id.asc(),
                file::deletion_requested_at.asc(),
                file::file_key.asc(),
            ))
            .limit(limit)
            .offset(offset)
            .load(conn)
            .await?;

        // Group files by onchain_bucket_id
        let mut grouped: HashMap<Vec<u8>, Vec<Self>> = HashMap::new();
        for file in files {
            grouped
                .entry(file.onchain_bucket_id.clone())
                .or_insert_with(Vec::new)
                .push(file);
        }

        Ok(grouped)
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

use std::collections::HashMap;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use sc_network::{Multiaddr, PeerId};

use shc_common::types::{FileMetadata, Fingerprint};

use crate::{
    models::{Bucket, MultiAddress},
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
    Revoked = 3,
    Rejected = 4,
}

impl TryFrom<i32> for FileStorageRequestStep {
    type Error = i32;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Requested),
            1 => Ok(Self::Stored),
            2 => Ok(Self::Expired),
            3 => Ok(Self::Revoked),
            4 => Ok(Self::Rejected),
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
    /// Whether the file is currently in the bucket's forest.
    ///
    /// Updated based on [`MutationsApplied`](pallet_proofs_dealer::Event::MutationsApplied) events:
    /// - Set to `true` when an `Add` mutation is applied for this file in the bucket
    /// - Set to `false` when a `Remove` mutation is applied for this file in the bucket
    pub is_in_bucket: bool,
    /// Block hash where the file was created.
    ///
    /// Contains the block hash where the `NewStorageRequest` event was emitted.
    pub block_hash: Vec<u8>,
    /// Transaction hash that created this file (for EVM-originated storage requests).
    ///
    /// Contains the Ethereum transaction hash from `pallet_ethereum::Event::Executed` if the storage
    /// request was created via an EVM transaction. NULL for native Substrate transactions.
    pub tx_hash: Option<Vec<u8>>,
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
        block_hash: Vec<u8>,
        tx_hash: Option<Vec<u8>>,
        is_in_bucket: bool,
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
                file::is_in_bucket.eq(is_in_bucket),
                file::block_hash.eq(block_hash),
                file::tx_hash.eq(tx_hash),
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

    /// Get all file records for a given file key.
    ///
    /// There can be multiple file records for a given file key if there were
    /// multiple storage requests for the same file key.
    pub async fn get_by_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<Vec<Self>, diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let file_records = file::table
            .filter(file::file_key.eq(file_key))
            .load::<Self>(conn)
            .await?;
        Ok(file_records)
    }

    /// Get the most recently created file record for a given file key.
    ///
    /// Returns error if there are no records for the given key.
    pub async fn get_latest_by_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<Self, diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let file_record: Self = file::table
            .filter(file::file_key.eq(file_key))
            .order(file::created_at.desc())
            .first(conn)
            .await?;
        Ok(file_record)
    }

    /// Get the oldest file record for a given file key.
    ///
    /// Returns error if there are no records for the given key.
    pub async fn get_oldest_by_file_key<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<Self, diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let file_record: Self = file::table
            .filter(file::file_key.eq(file_key))
            .order(file::created_at.asc())
            .first(conn)
            .await?;
        Ok(file_record)
    }

    /// Check if any file record with the given file key is currently in the bucket forest.
    ///
    /// This is useful when creating new file records for repeated storage requests
    /// to inherit the bucket membership status from previous requests, since for example if the
    /// MSP was already storing the file key, the `MutationsApplied` event won't be emitted for it
    /// so if we default `is_in_bucket` to false it would be incorrectly marked as not in the bucket.
    pub async fn is_file_key_in_bucket<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<bool, diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let count: i64 = file::table
            .filter(file::file_key.eq(file_key))
            .filter(file::is_in_bucket.eq(true))
            .count()
            .get_result(conn)
            .await?;
        Ok(count > 0)
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
        file_id: i64,
    ) -> Result<(), diesel::result::Error> {
        // Delete the file by its ID
        diesel::delete(file::table)
            .filter(file::id.eq(file_id))
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

    /// Check if file has any BSP associations
    pub async fn has_bsp_associations<'a>(
        conn: &mut DbConnection<'a>,
        file_id: i64,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::bsp_file;

        let count: i64 = bsp_file::table
            .filter(bsp_file::file_id.eq(file_id))
            .count()
            .get_result(conn)
            .await?;
        Ok(count > 0)
    }

    /// Check if a file has any MSP associations
    ///
    /// TODO: This check is not used for now, but should be used in the future to prevent the
    /// indexer from trying to delete a file that still has associations and getting stuck.
    pub async fn has_msp_associations<'a>(
        conn: &mut DbConnection<'a>,
        file_id: i64,
    ) -> Result<bool, diesel::result::Error> {
        let count: i64 = msp_file::table
            .filter(msp_file::file_id.eq(file_id))
            .count()
            .get_result(conn)
            .await?;
        Ok(count > 0)
    }

    /// Delete file only if it has no BSP associations and is not in the bucket forest.
    /// The flag [`is_in_bucket`](File::is_in_bucket) is set to false or true based on the [`MutationsApplied`] event emitted by the proofs dealer pallet for catch all.
    /// Returns true if all files associated with the file key were deleted, false if any still has associations.
    pub async fn delete_if_orphaned<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
    ) -> Result<(), diesel::result::Error> {
        let file_key = file_key.as_ref();

        // Check if file is still in bucket forest or has BSP associations
        let file_records: Vec<Self> = file::table
            .filter(file::file_key.eq(file_key))
            .load(conn)
            .await?;

        // For each found file record, check if it has any BSP or MSP associations, and delete it if it doesn't
        let mut deleted_all = true;
        for file_record in file_records {
            let has_bsp = Self::has_bsp_associations(conn, file_record.id).await?;
            let is_in_bucket = file_record.is_in_bucket;

            // Check if file has an active storage request that's not marked for deletion
            // This prevents race conditions where deletion events arrive before confirmation events
            let has_active_storage_request = file_record.step
                == FileStorageRequestStep::Requested as i32
                && file_record.deletion_status.is_none();

            if !is_in_bucket && !has_bsp && !has_active_storage_request {
                Self::delete(conn, file_record.id).await?;
                log::debug!(
                    "Deleted orphaned file key: {:?} and id: {:?}",
                    file_record.file_key,
                    file_record.id
                );
            } else {
                log::debug!(
                		"File with key {:?} and id {:?} still has storage (in_bucket: {}, BSP: {}, active_request: {}), keeping it",
                		file_record.file_key,
                		file_record.id,
                		is_in_bucket,
                		has_bsp,
                		has_active_storage_request,
            		);
                deleted_all = false;
            }
        }

        log::debug!(
            "Deleted all files associated with file key: {:?}: {}",
            file_key,
            deleted_all
        );

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
        is_in_bucket: Option<bool>,
    ) -> Result<Vec<Vec<u8>>, diesel::result::Error> {
        let mut query = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
            .filter(bucket::onchain_bucket_id.eq(onchain_bucket_id))
            .into_boxed();

        // Filter by is_in_bucket if provided
        if let Some(is_in_bucket_value) = is_in_bucket {
            query = query.filter(file::is_in_bucket.eq(is_in_bucket_value));
        }

        let file_keys: Vec<Vec<u8>> = query.select(file::file_key).load::<Vec<u8>>(conn).await?;

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
    /// Queries files with `deletion_status = InProgress` and groups them by their
    /// `onchain_bucket_id`. The deletion type determines whether to include files
    /// with or without user signatures.
    ///
    /// Returns all files in buckets regardless of MSP acceptance status, since bucket
    /// deletions are concerned with the bucket's forest state, not MSP storage associations.
    ///
    /// # Arguments
    /// * `deletion_type` - Filter for user deletions (with signature) or incomplete deletions (without signature)
    /// * `bucket_id` - Optional filter by specific bucket's onchain ID (returns only that bucket's files)
    /// * `is_in_bucket` - Optional filter by whether files are in the bucket's forest (None = all files)
    /// * `limit` - Maximum number of files to return across all buckets (default: 1000)
    /// * `offset` - Number of files to skip for pagination (default: 0)
    ///
    /// # Returns
    /// HashMap mapping bucket IDs (as `Vec<u8>`) to vectors of files pending deletion in that bucket.
    ///
    /// # Note
    /// The limit/offset applies to the total number of files retrieved, not the number of buckets.
    /// Files are ordered by bucket_id then file_key for consistent pagination.
    pub async fn get_files_pending_deletion_grouped_by_bucket<'a>(
        conn: &mut DbConnection<'a>,
        deletion_type: FileDeletionType,
        bucket_id: Option<&[u8]>,
        is_in_bucket: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<HashMap<Vec<u8>, Vec<Self>>, diesel::result::Error> {
        let limit = limit.unwrap_or(DEFAULT_BATCH_QUERY_LIMIT);
        let offset = offset.unwrap_or(0);

        let mut query = file::table
            .inner_join(bucket::table.on(file::bucket_id.eq(bucket::id)))
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

        // Filter by `is_in_bucket` if provided
        if let Some(is_in_bucket_value) = is_in_bucket {
            query = query.filter(file::is_in_bucket.eq(is_in_bucket_value));
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

    /// Update the bucket membership status for a file.
    ///
    /// Updates `is_in_bucket` based on mutations applied to the bucket's forest.
    /// The file is identified by both `file_key` and `onchain_bucket_id`.
    ///
    /// This updates all file records with the same file key (for cases where there were
    /// multiple storage requests for the same file).
    pub async fn update_bucket_membership<'a>(
        conn: &mut DbConnection<'a>,
        file_key: impl AsRef<[u8]>,
        onchain_bucket_id: impl AsRef<[u8]>,
        is_in_bucket: bool,
    ) -> Result<(), diesel::result::Error> {
        let file_key = file_key.as_ref().to_vec();
        let onchain_bucket_id = onchain_bucket_id.as_ref().to_vec();

        // Update all file records with this file key to have the new bucket membership status
        diesel::update(file::table)
            .filter(file::file_key.eq(&file_key))
            .filter(file::onchain_bucket_id.eq(&onchain_bucket_id))
            .set(file::is_in_bucket.eq(is_in_bucket))
            .execute(conn)
            .await?;

        // Update the bucket stats to reflect the change in bucket membership
        Bucket::sync_stats(conn, onchain_bucket_id).await?;

        Ok(())
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

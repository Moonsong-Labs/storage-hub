use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::error;

use shc_indexer_db::models::File as DBFile;
use shp_types::Hash;
use shc_indexer_db::models::{File as DBFile, FileStorageRequestStep};

use crate::models::buckets::FileTree;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
    /// Indicates that the file's storage request has not yet been fulfilled by the requested MSP
    InProgress,
    /// Indicates that the file's storage request has been fulfilled and the file is generally available with the requested replication target criteria met
    Ready,
    /// Indicates that the file's storage request has not been fulfilled completely but is still with the MSP
    Expired,
    /// Indicates that the file's has been marked for deletion and will be removed from the MSP soon
    DeletionInProgress,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    #[serde(rename = "fileKey")]
    pub file_key: String,
    #[serde(serialize_with = "crate::utils::serde::hex_string")]
    pub fingerprint: [u8; 32],
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub location: String,
    pub size: u64,
    #[serde(rename = "isPublic")]
    pub is_public: bool,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: DateTime<Utc>,
    pub status: FileStatus,
}

impl FileInfo {
    pub fn status_from_db(db: &DBFile) -> FileStatus {
        db.deletion_status
            .map(|_| FileStatus::DeletionInProgress)
            .unwrap_or_else(|| match FileStorageRequestStep::try_from(db.step) {
                Ok(FileStorageRequestStep::Requested) => FileStatus::InProgress,
                Ok(FileStorageRequestStep::Stored) => FileStatus::Ready,
                Ok(FileStorageRequestStep::Expired) => FileStatus::Expired,
                Err(step) => {
                    error!(step, "Unsupported File's StorageRequest step");
                    unreachable!("unknown storage request step #{step} present in Indexer DB")
                }
            })
    }

    pub fn from_db(db: &DBFile, is_public: bool) -> Self {
        Self {
            file_key: hex::encode(&db.file_key),
            fingerprint: Hash::from_slice(&db.fingerprint).to_fixed_bytes(),
            bucket_id: hex::encode(&db.onchain_bucket_id),
            // TODO: determine if lossy conversion is acceptable here
            location: String::from_utf8_lossy(&db.location).into_owned(),
            size: db.size as u64,
            is_public,
            uploaded_at: db.updated_at.and_utc(),
            status: Self::status_from_db(&db),
        }
    }

    pub fn fingerprint_hexstr(&self) -> String {
        hex::encode(&self.fingerprint)
    }
}

#[derive(Debug, Serialize)]
pub struct DistributeResponse {
    pub status: String,
    #[serde(rename = "fileKey")]
    pub file_key: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FileListResponse {
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub tree: FileTree,
}

#[derive(Debug, Serialize)]
pub struct FileUploadResponse {
    pub status: String,
    #[serde(rename = "fileKey")]
    pub file_key: String,
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub fingerprint: String,
    pub location: String,
}

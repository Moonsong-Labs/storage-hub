use chrono::{DateTime, Utc};
use serde::Serialize;

use shc_indexer_db::models::File as DBFile;

use crate::models::buckets::FileTree;

#[derive(Debug, Serialize)]
pub struct FileInfo {
    #[serde(rename = "fileKey")]
    pub file_key: String,
    pub fingerprint: String,
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub location: String,
    pub size: u64,
    #[serde(rename = "isPublic")]
    pub is_public: bool,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: DateTime<Utc>,
}

impl FileInfo {
    pub fn from_db(db: &DBFile, is_public: bool) -> Self {
        Self {
            file_key: hex::encode(&db.file_key),
            fingerprint: hex::encode(&db.fingerprint),
            bucket_id: hex::encode(&db.onchain_bucket_id),
            // TODO: determine if lossy conversion is acceptable here
            location: String::from_utf8_lossy(&db.location).into_owned(),
            size: db.size as u64,
            is_public,
            uploaded_at: db.updated_at.and_utc(),
        }
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
    pub files: Vec<FileTree>,
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

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::buckets::FileEntry;

#[derive(Debug, Serialize)]
pub struct FileInfo {
    #[serde(rename = "fileKey")]
    pub file_key: String,
    pub fingerprint: String,
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub name: String,
    pub location: String,
    pub size: u64,
    #[serde(rename = "isPublic")]
    pub is_public: bool,
    #[serde(rename = "uploadedAt")]
    pub uploaded_at: DateTime<Utc>,
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
    pub files: Vec<FileEntry>,
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

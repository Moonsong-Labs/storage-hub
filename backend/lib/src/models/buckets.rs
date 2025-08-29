use serde::Serialize;

use shc_indexer_db::models::Bucket as DBBucket;

#[derive(Debug, Serialize)]
pub struct Bucket {
    /// The onchain bucket identifier (hex string)
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub name: String,
    /// The merkle root of the bucket (hex string)
    pub root: String,
    #[serde(rename = "isPublic")]
    pub is_public: bool,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    #[serde(rename = "valuePropId")]
    pub value_prop_id: String,
    #[serde(rename = "fileCount")]
    pub file_count: u64,
}

impl Bucket {
    pub fn from_db(db: &DBBucket, size_bytes: u64, file_count: u64) -> Self {
        Self {
            bucket_id: hex::encode(&db.onchain_bucket_id),
            // TODO: determine if lossy conversion is acceptable here
            name: String::from_utf8_lossy(&db.name).into_owned(),
            root: hex::encode(&db.merkle_root),
            is_public: !db.private,
            size_bytes,
            // TODO: the value_prop_id is not stored by the indexer, it's discarded
            // see [index_file_system_event](client/indexer-service/src/handler.rs:async fn index_file_system_event)
            value_prop_id: "unknown".to_owned(),
            file_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeFile {
    pub size_bytes: u64,
    pub file_key: String,
}

#[derive(Debug, Serialize)]
pub struct FileTreeFolder {
    pub children: Vec<FileTreeEntry>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FileTreeEntry {
    File(FileTreeFile),
    Folder(FileTreeFolder),
}

impl FileTreeEntry {
    pub fn file(&self) -> Option<&FileTreeFile> {
        match self {
            Self::File(file) => Some(file),
            _ => None,
        }
    }

    pub fn folder(&self) -> Option<&FileTreeFolder> {
        match self {
            Self::Folder(folder) => Some(folder),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FileTree {
    pub name: String,

    #[serde(flatten)]
    pub entry: FileTreeEntry,
}

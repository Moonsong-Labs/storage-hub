use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Bucket {
    #[serde(rename = "bucketId")]
    pub bucket_id: String,
    pub name: String,
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

#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sizeBytes")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "fileKey")]
    pub file_key: Option<String>,
}

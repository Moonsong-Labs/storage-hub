use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use shc_indexer_db::models::{Bucket as DBBucket, File as DBFile};

use crate::models::files::{FileInfo, FileStatus};

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

#[derive(Debug, Serialize)]
pub struct ListBucketsResponse {
    pub buckets: Vec<Bucket>,
    #[serde(rename = "totalBuckets")]
    pub total_buckets: String,
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
            value_prop_id: db.value_prop_id.clone(),
            file_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTreeFile {
    pub size_bytes: u64,
    pub file_key: String,
    pub status: FileStatus,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FileTreeEntryKind {
    File(FileTreeFile),
    Folder,
}

#[derive(Debug, Serialize)]
pub struct FileTreeEntry {
    pub name: String,

    #[serde(flatten)]
    pub kind: FileTreeEntryKind,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub struct FileTree {
    pub name: String,

    pub children: Vec<FileTreeEntry>,
}

impl FileTree {
    /// Create a file tree containing only direct children of the specified path
    ///
    /// ## Business Rules for File Location Handling
    ///
    /// - **Root is implicit/optional**: `/folder/file.txt` and `folder/file.txt` both create
    ///   a `folder` folder with `file.txt` as its child
    /// - **Duplicate slashes are collapsed**: `//file.txt`, `////file.txt`, and `/file.txt`
    ///   all become the same entry named `file.txt` under the root folder
    /// - **Trailing slashes are trimmed**: `file.txt/` and `file.txt` are both displayed
    ///   as `file.txt` in the name (these would be separate entries in the database)
    pub fn from_files_filtered(files: Vec<DBFile>, filter_path: &str) -> Self {
        let mut children_map: BTreeMap<String, Vec<FileTreeEntryKind>> = BTreeMap::new();

        let prefix_to_match = Self::normalize_path(filter_path);

        for file in files {
            let location = String::from_utf8_lossy(&file.location);

            // Normalize the file location using the same rules
            let normalized_location = Self::normalize_path(&location);

            // Determine if this file is under the filter path
            let relative_path = if prefix_to_match.is_empty() {
                // We're at root - everything is relative to root
                normalized_location.clone()
            } else if normalized_location == prefix_to_match {
                // The location exactly matches the filter path - it's a file at this level
                // Extract just the filename
                normalized_location
                    .rsplit('/')
                    .next()
                    .unwrap_or(&normalized_location)
                    .to_string()
            } else if let Some(remaining) =
                normalized_location.strip_prefix(&format!("{}/", prefix_to_match))
            {
                // The location is under the filter path
                remaining.to_string()
            } else {
                // Not under this path
                continue;
            };

            // Get the first segment - this is the direct child
            let first_segment = relative_path.split('/').next().unwrap_or("");
            if first_segment.is_empty() {
                continue; // Skip empty segments
            }

            // Check if this is a file or folder by looking for more segments
            let is_file = !relative_path.contains('/');

            if is_file {
                // This is a direct file under the path
                children_map
                    .entry(first_segment.to_string())
                    .or_default()
                    .push(FileTreeEntryKind::File(FileTreeFile {
                        size_bytes: file.size as u64,
                        file_key: hex::encode(&file.file_key),
                        status: FileInfo::status_from_db(&file),
                        uploaded_at: file.updated_at.and_utc(),
                    }));
            } else {
                // This is a folder (has more segments after the first)
                // We only want to create the folder entry once, not recurse into it
                let entries = children_map.entry(first_segment.to_string()).or_default();

                // Only add folder entry if we don't already have one
                if !entries
                    .iter()
                    .any(|e| matches!(e, FileTreeEntryKind::Folder))
                {
                    entries.push(FileTreeEntryKind::Folder);
                }
            }
        }

        // Convert the map to a FileTree structure
        let children = Self::map_vec_to_children(children_map);

        // Use the last segment of the path as the name, or "/" for root
        let name = if prefix_to_match.is_empty() {
            "/".to_string()
        } else {
            prefix_to_match
                .rsplit('/')
                .next()
                .unwrap_or(&prefix_to_match) // fallback to original
                .to_string()
        };

        FileTree { name, children }
    }

    /// Normalize a path according to the business rules:
    /// - Remove leading slash (root is implicit)
    /// - Trim trailing slashes
    /// - Collapse duplicate slashes
    fn normalize_path(path: &str) -> String {
        // Collapse duplicate slashes, trim leading/trailing slashes
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        segments.join("/")
    }

    /// Convert a map of vectors back to children vector
    /// Each name can have multiple entries (files with same normalized name)
    fn map_vec_to_children(map: BTreeMap<String, Vec<FileTreeEntryKind>>) -> Vec<FileTreeEntry> {
        map.into_iter()
            .flat_map(|(name, entries)| {
                entries.into_iter().map(move |kind| FileTreeEntry {
                    name: name.clone(),
                    kind,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::test_utils::random_bytes_32;

    fn test_file_with_location_key_and_size(location: &str, file_key: &[u8], size: i64) -> DBFile {
        DBFile {
            id: 1,
            account: vec![],
            file_key: file_key.to_vec(),
            bucket_id: 1,
            onchain_bucket_id: vec![],
            location: location.as_bytes().to_vec(),
            fingerprint: vec![],
            size,
            step: 0,
            deletion_status: None,
            deletion_signature: None,
            deletion_requested_at: None,
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            is_in_bucket: false,
            block_hash: vec![0u8; 32], // Placeholder block hash for test data
            tx_hash: None,             // No transaction hash for test data
        }
    }

    #[test]
    fn normalize_path_root() {
        assert_eq!(FileTree::normalize_path("/"), "");
        assert_eq!(FileTree::normalize_path(""), "");
    }

    #[test]
    fn normalize_path_duplicate_slashes() {
        assert_eq!(FileTree::normalize_path("//file.txt"), "file.txt");
        assert_eq!(FileTree::normalize_path("////file.txt"), "file.txt");
        assert_eq!(
            FileTree::normalize_path("/path//to///file.txt"),
            "path/to/file.txt"
        );
    }

    #[test]
    fn normalize_path_trailing_slashes() {
        assert_eq!(FileTree::normalize_path("file.txt/"), "file.txt");
        assert_eq!(FileTree::normalize_path("/folder/"), "folder");
        assert_eq!(
            FileTree::normalize_path("folder/subfolder/"),
            "folder/subfolder"
        );
    }

    #[test]
    fn normalize_path_leading_slash() {
        assert_eq!(
            FileTree::normalize_path("/folder/file.txt"),
            "folder/file.txt"
        );
        assert_eq!(
            FileTree::normalize_path("folder/file.txt"),
            "folder/file.txt"
        );
    }

    #[test]
    fn normalize_path_combined() {
        assert_eq!(
            FileTree::normalize_path("///folder//file.txt///"),
            "folder/file.txt"
        );
    }

    #[test]
    fn business_rules_root_optional() {
        let folder = "folder";
        // Test that /folder/file.txt and folder/file.txt produce the same result
        let file_key = random_bytes_32();
        let files1 = vec![test_file_with_location_key_and_size(
            &format!("/{folder}/file.txt"),
            &file_key,
            100,
        )];
        let files2 = vec![test_file_with_location_key_and_size(
            &format!("{folder}/file.txt"),
            &file_key,
            100,
        )];

        let tree1 = FileTree::from_files_filtered(files1, folder);
        let tree2 = FileTree::from_files_filtered(files2, folder);

        // Both should have the same structure
        assert_eq!(tree1.children.len(), 1, "Should have 1 file");
        assert_eq!(
            tree1.children.len(),
            tree2.children.len(),
            "Should both trees have the same number of files"
        );
        assert_eq!(
            tree1.children[0].name, tree2.children[0].name,
            "Should both trees have the same children name"
        );
    }

    #[test]
    fn business_rules_duplicate_slashes_collapsed() {
        let file1 = "file1.txt";
        let file2 = "file2.txt";
        let file3 = "file3.txt";

        // Test that multiple slashes are collapsed to single path
        let files = vec![
            test_file_with_location_key_and_size(&format!("//{file1}"), &random_bytes_32(), 100),
            test_file_with_location_key_and_size(&format!("////{file2}"), &random_bytes_32(), 200),
            test_file_with_location_key_and_size(&format!("/{file3}"), &random_bytes_32(), 300),
        ];

        let tree = FileTree::from_files_filtered(files, "/");

        // All three files should be at root level despite different slash counts
        assert_eq!(tree.children.len(), 3);

        let names = tree
            .children
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&file1));
        assert!(names.contains(&file2));
        assert!(names.contains(&file3));
    }

    #[test]
    fn business_rules_trailing_slashes_trimmed() {
        // Test that trailing slashes are trimmed
        let key1 = random_bytes_32();
        let key2 = random_bytes_32();
        let files = vec![
            test_file_with_location_key_and_size("file.txt/", &key1, 100),
            test_file_with_location_key_and_size("file.txt", &key2, 200),
        ];

        let tree = FileTree::from_files_filtered(files, "/");

        // Both should appear as "file.txt" (2 separate entries with the same name)
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].name, "file.txt");
        assert_eq!(tree.children[1].name, "file.txt");

        // Verify they are different files
        if let FileTreeEntryKind::File(file1) = &tree.children[0].kind {
            assert_eq!(file1.file_key, hex::encode(&key1));
            assert_eq!(file1.size_bytes, 100);
        }
        if let FileTreeEntryKind::File(file2) = &tree.children[1].kind {
            assert_eq!(file2.file_key, hex::encode(&key2));
            assert_eq!(file2.size_bytes, 200);
        }
    }

    #[test]
    fn file_tree_filtered_at_root() {
        let files = vec![
            test_file_with_location_key_and_size("/path/to/file/foo.txt", &random_bytes_32(), 100),
            test_file_with_location_key_and_size("/path/to/file/bar.txt", &random_bytes_32(), 200),
            test_file_with_location_key_and_size(
                "/path/to/another/thing.txt",
                &random_bytes_32(),
                300,
            ),
            test_file_with_location_key_and_size("/a/different/file.txt", &random_bytes_32(), 400),
            test_file_with_location_key_and_size("/root_file.txt", &random_bytes_32(), 500),
        ];

        // Test root path (should show only direct children: "path", "a", and "root_file.txt")
        let tree = FileTree::from_files_filtered(files.clone(), "/");
        assert_eq!(tree.name, "/");
        assert_eq!(tree.children.len(), 3);

        // Check for "a" folder
        assert!(tree
            .children
            .iter()
            .any(|c| c.name == "a" && matches!(c.kind, FileTreeEntryKind::Folder)));

        // Check for "path" folder
        assert!(tree
            .children
            .iter()
            .any(|c| c.name == "path" && matches!(c.kind, FileTreeEntryKind::Folder)));

        // Check for "root_file.txt" file
        assert!(tree
            .children
            .iter()
            .any(|c| c.name == "root_file.txt" && matches!(c.kind, FileTreeEntryKind::File(_))));

        // Also test with empty string (should be same as "/")
        let tree2 = FileTree::from_files_filtered(files, "");
        assert_eq!(tree2.name, "/");
        assert_eq!(tree2.children.len(), 3);
    }

    #[test]
    fn file_tree_filtered_at_specific_path() {
        let files = vec![
            test_file_with_location_key_and_size("/path/to/file/foo.txt", &random_bytes_32(), 100),
            test_file_with_location_key_and_size("/path/to/file/bar.txt", &random_bytes_32(), 200),
            test_file_with_location_key_and_size(
                "/path/to/another/thing.txt",
                &random_bytes_32(),
                300,
            ),
            test_file_with_location_key_and_size("/path/direct_file.txt", &random_bytes_32(), 600),
        ];

        // Test "/path" - should show "to" folder and "direct_file.txt"
        let tree = FileTree::from_files_filtered(files.clone(), "/path");
        assert_eq!(tree.name, "path");

        assert_eq!(tree.children.len(), 2);

        // Check for "to" folder (should be empty since we don't recurse)
        let to_entry = tree
            .children
            .iter()
            .find(|c| c.name == "to")
            .expect("should have an entry named 'to'");
        assert!(
            matches!(to_entry.kind, FileTreeEntryKind::Folder),
            "'to' should be a folder"
        );

        // Check for "direct_file.txt"
        let file_entry = tree
            .children
            .iter()
            .find(|c| c.name == "direct_file.txt")
            .unwrap();
        if let FileTreeEntryKind::File(file) = &file_entry.kind {
            assert_eq!(file.size_bytes, 600);
        } else {
            panic!("'direct_file.txt' should be a file");
        }
    }

    #[test]
    fn file_tree_filtered_at_deeper_path() {
        let files = vec![
            test_file_with_location_key_and_size("/path/to/file/foo.txt", &random_bytes_32(), 100),
            test_file_with_location_key_and_size("/path/to/file/bar.txt", &random_bytes_32(), 200),
            test_file_with_location_key_and_size(
                "/path/to/another/thing.txt",
                &random_bytes_32(),
                300,
            ),
            test_file_with_location_key_and_size("/path/to/direct.txt", &random_bytes_32(), 700),
        ];

        // Test "/path/to" - should show "file" folder, "another" folder, and "direct.txt"
        let tree = FileTree::from_files_filtered(files, "/path/to");
        assert_eq!(tree.name, "to");
        assert_eq!(tree.children.len(), 3);

        // Check for "file" folder (should be empty since we don't recurse)
        let file_folder = tree.children.iter().find(|c| c.name == "file").unwrap();
        assert!(matches!(file_folder.kind, FileTreeEntryKind::Folder));

        // Check for "another" folder
        let another_folder = tree.children.iter().find(|c| c.name == "another").unwrap();
        assert!(matches!(another_folder.kind, FileTreeEntryKind::Folder));

        // Check for "direct.txt" file
        let direct_file = tree
            .children
            .iter()
            .find(|c| c.name == "direct.txt")
            .unwrap();

        if let FileTreeEntryKind::File(file) = &direct_file.kind {
            assert_eq!(file.size_bytes, 700);
        } else {
            panic!("'direct.txt' should be a file");
        }
    }
}

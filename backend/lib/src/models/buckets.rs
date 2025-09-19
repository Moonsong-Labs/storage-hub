use std::collections::BTreeMap;

use serde::Serialize;

use shc_indexer_db::models::{Bucket as DBBucket, File as DBFile};

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
    pub children: Vec<FileTree>,
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

impl FileTree {
    /// Convert a list of files into a hierarchical file tree structure
    ///
    /// Applies the same normalization rules as `from_files_filtered`
    pub fn from_files(files: Vec<DBFile>) -> Self {
        // Use a BTreeMap to maintain consistent ordering
        let mut root_map: BTreeMap<String, FileTreeEntry> = BTreeMap::new();

        for file in files {
            // Convert location from Vec<u8> to String
            let location = String::from_utf8_lossy(&file.location);

            // Normalize the path and split into segments
            let normalized = Self::normalize_path(&location);
            let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();

            if segments.is_empty() {
                continue;
            }

            // Build the path recursively
            Self::insert_file_into_tree(&mut root_map, &segments, &file);
        }

        // Convert the map to a FileTree structure
        let children = Self::map_to_children(root_map);

        FileTree {
            name: "/".to_string(),
            entry: FileTreeEntry::Folder(FileTreeFolder { children }),
        }
    }

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
        let mut children_map: BTreeMap<String, Vec<FileTreeEntry>> = BTreeMap::new();

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
                    .or_insert_with(Vec::new)
                    .push(FileTreeEntry::File(FileTreeFile {
                        size_bytes: file.size as u64,
                        file_key: hex::encode(&file.file_key),
                    }));
            } else {
                // This is a folder (has more segments after the first)
                // We only want to create the folder entry once, not recurse into it
                let entries = children_map
                    .entry(first_segment.to_string())
                    .or_insert_with(Vec::new);

                // Only add folder entry if we don't already have one
                if !entries
                    .iter()
                    .any(|e| matches!(e, FileTreeEntry::Folder(_)))
                {
                    entries.push(FileTreeEntry::Folder(FileTreeFolder {
                        children: Vec::new(), // Empty children since we don't recurse
                    }));
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

        FileTree {
            name,
            entry: FileTreeEntry::Folder(FileTreeFolder { children }),
        }
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

    /// Recursively insert a file into the tree structure
    fn insert_file_into_tree(
        map: &mut BTreeMap<String, FileTreeEntry>,
        segments: &[&str],
        file: &DBFile,
    ) {
        if segments.is_empty() {
            return;
        }

        let (first, rest) = segments.split_first().unwrap();

        if rest.is_empty() {
            // This is the file itself
            map.insert(
                first.to_string(),
                FileTreeEntry::File(FileTreeFile {
                    size_bytes: file.size as u64,
                    file_key: hex::encode(&file.file_key),
                }),
            );
        } else {
            // This is a folder - get or create it
            let entry = map.entry(first.to_string()).or_insert_with(|| {
                FileTreeEntry::Folder(FileTreeFolder {
                    children: Vec::new(),
                })
            });

            // Recursively process the rest of the path
            if let FileTreeEntry::Folder(folder) = entry {
                // Take ownership of children to avoid cloning
                let children = std::mem::take(&mut folder.children);
                let mut child_map = Self::children_to_map(children);
                Self::insert_file_into_tree(&mut child_map, rest, file);
                folder.children = Self::map_to_children(child_map);
            }
        }
    }

    /// Convert children vector to a map for easier manipulation
    fn children_to_map(children: Vec<FileTree>) -> BTreeMap<String, FileTreeEntry> {
        children
            .into_iter()
            .map(|child| (child.name, child.entry))
            .collect()
    }

    /// Convert a map back to children vector
    fn map_to_children(map: BTreeMap<String, FileTreeEntry>) -> Vec<FileTree> {
        map.into_iter()
            .map(|(name, entry)| FileTree { name, entry })
            .collect()
    }

    /// Convert a map of vectors back to children vector
    /// Each name can have multiple entries (files with same normalized name)
    fn map_vec_to_children(map: BTreeMap<String, Vec<FileTreeEntry>>) -> Vec<FileTree> {
        map.into_iter()
            .flat_map(|(name, entries)| {
                entries.into_iter().map(move |entry| FileTree {
                    name: name.clone(),
                    entry,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::mock_utils::random_bytes_32;

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
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
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
        // Test that /folder/file.txt and folder/file.txt produce the same result
        let file_key = random_bytes_32();
        let files1 = vec![test_file_with_location_key_and_size(
            "/folder/file.txt",
            &file_key,
            100,
        )];
        let files2 = vec![test_file_with_location_key_and_size(
            "folder/file.txt",
            &file_key,
            100,
        )];

        let tree1 = FileTree::from_files(files1);
        let tree2 = FileTree::from_files(files2);

        // Both should have the same structure
        if let FileTreeEntry::Folder(folder1) = &tree1.entry {
            if let FileTreeEntry::Folder(folder2) = &tree2.entry {
                assert_eq!(folder1.children.len(), folder2.children.len());
                assert_eq!(folder1.children[0].name, folder2.children[0].name);
            }
        }
    }

    #[test]
    fn business_rules_duplicate_slashes_collapsed() {
        // Test that multiple slashes are collapsed to single path
        let files = vec![
            test_file_with_location_key_and_size("//file1.txt", &random_bytes_32(), 100),
            test_file_with_location_key_and_size("////file2.txt", &random_bytes_32(), 200),
            test_file_with_location_key_and_size("/file3.txt", &random_bytes_32(), 300),
        ];

        let tree = FileTree::from_files_filtered(files, "/");

        if let FileTreeEntry::Folder(folder) = &tree.entry {
            // All three files should be at root level despite different slash counts
            assert_eq!(folder.children.len(), 3);

            let names: Vec<String> = folder.children.iter().map(|c| c.name.clone()).collect();
            assert!(names.contains(&"file1.txt".to_string()));
            assert!(names.contains(&"file2.txt".to_string()));
            assert!(names.contains(&"file3.txt".to_string()));
        }
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

        if let FileTreeEntry::Folder(folder) = &tree.entry {
            // Both should appear as "file.txt" (2 separate entries with the same name)
            assert_eq!(folder.children.len(), 2);
            assert_eq!(folder.children[0].name, "file.txt");
            assert_eq!(folder.children[1].name, "file.txt");

            // Verify they are different files
            if let FileTreeEntry::File(file1) = &folder.children[0].entry {
                assert_eq!(file1.file_key, hex::encode(&key1));
                assert_eq!(file1.size_bytes, 100);
            }
            if let FileTreeEntry::File(file2) = &folder.children[1].entry {
                assert_eq!(file2.file_key, hex::encode(&key2));
                assert_eq!(file2.size_bytes, 200);
            }
        }
    }

    #[test]
    fn file_tree_from_files_basic() {
        let key1 = random_bytes_32();
        let key2 = random_bytes_32();
        let key3 = random_bytes_32();
        let key4 = random_bytes_32();
        let files = vec![
            test_file_with_location_key_and_size("/path/to/file/foo.txt", &key1, 100),
            test_file_with_location_key_and_size("/path/to/file/bar.txt", &key2, 200),
            test_file_with_location_key_and_size("/path/to/another/thing.txt", &key3, 300),
            test_file_with_location_key_and_size("/a/different/file.txt", &key4, 400),
        ];

        let tree = FileTree::from_files(files);

        // Check root is named "/"
        assert_eq!(tree.name, "/");

        // Check root is a folder
        if let FileTreeEntry::Folder(root_folder) = &tree.entry {
            // Root should have 2 children: "path" and "a"
            assert_eq!(root_folder.children.len(), 2);

            // Find "a" folder (it comes first in BTreeMap ordering)
            let a_entry = root_folder
                .children
                .iter()
                .find(|child| child.name == "a")
                .expect("Should have 'a' folder");

            if let FileTreeEntry::Folder(a_folder) = &a_entry.entry {
                assert_eq!(a_folder.children.len(), 1);

                // Check "different" folder
                let different_entry = &a_folder.children[0];
                assert_eq!(different_entry.name, "different");

                if let FileTreeEntry::Folder(different_folder) = &different_entry.entry {
                    assert_eq!(different_folder.children.len(), 1);

                    // Check file.txt
                    let file_entry = &different_folder.children[0];
                    assert_eq!(file_entry.name, "file.txt");

                    if let FileTreeEntry::File(file) = &file_entry.entry {
                        assert_eq!(file.size_bytes, 400);
                        assert_eq!(file.file_key, hex::encode(&key4));
                    } else {
                        panic!("'file.txt' should be a file");
                    }
                } else {
                    panic!("'different' should be a folder");
                }
            } else {
                panic!("'a' should be a folder");
            }

            // Find "path" folder
            let path_entry = root_folder
                .children
                .iter()
                .find(|child| child.name == "path")
                .expect("Should have 'path' folder");

            if let FileTreeEntry::Folder(path_folder) = &path_entry.entry {
                assert_eq!(path_folder.children.len(), 1);

                // Check nested structure for verification
                let to_entry = &path_folder.children[0];
                assert_eq!(to_entry.name, "to");

                if let FileTreeEntry::Folder(to_folder) = &to_entry.entry {
                    assert_eq!(to_folder.children.len(), 2);

                    // Should have "another" and "file" folders
                    let has_another = to_folder.children.iter().any(|c| c.name == "another");
                    let has_file = to_folder.children.iter().any(|c| c.name == "file");
                    assert!(has_another, "Should have 'another' folder");
                    assert!(has_file, "Should have 'file' folder");
                } else {
                    panic!("'to' should be a folder");
                }
            } else {
                panic!("'path' should be a folder");
            }
        } else {
            panic!("Root should be a folder");
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

        if let FileTreeEntry::Folder(folder) = &tree.entry {
            assert_eq!(folder.children.len(), 3);

            // Check for "a" folder
            assert!(folder
                .children
                .iter()
                .any(|c| c.name == "a" && matches!(c.entry, FileTreeEntry::Folder(_))));

            // Check for "path" folder
            assert!(folder
                .children
                .iter()
                .any(|c| c.name == "path" && matches!(c.entry, FileTreeEntry::Folder(_))));

            // Check for "root_file.txt" file
            assert!(folder
                .children
                .iter()
                .any(|c| c.name == "root_file.txt" && matches!(c.entry, FileTreeEntry::File(_))));
        } else {
            panic!("Root should be a folder");
        }

        // Also test with empty string (should be same as "/")
        let tree2 = FileTree::from_files_filtered(files, "");
        assert_eq!(tree2.name, "/");

        if let FileTreeEntry::Folder(folder2) = &tree2.entry {
            assert_eq!(folder2.children.len(), 3);
        }
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

        if let FileTreeEntry::Folder(folder) = &tree.entry {
            assert_eq!(folder.children.len(), 2);

            // Check for "to" folder (should be empty since we don't recurse)
            let to_entry = folder.children.iter().find(|c| c.name == "to").unwrap();
            if let FileTreeEntry::Folder(to_folder) = &to_entry.entry {
                assert_eq!(to_folder.children.len(), 0); // No recursion
            } else {
                panic!("'to' should be a folder");
            }

            // Check for "direct_file.txt"
            let file_entry = folder
                .children
                .iter()
                .find(|c| c.name == "direct_file.txt")
                .unwrap();
            if let FileTreeEntry::File(file) = &file_entry.entry {
                assert_eq!(file.size_bytes, 600);
            } else {
                panic!("'direct_file.txt' should be a file");
            }
        } else {
            panic!("Result should be a folder");
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

        if let FileTreeEntry::Folder(folder) = &tree.entry {
            assert_eq!(folder.children.len(), 3);

            // Check for "file" folder (should be empty since we don't recurse)
            let file_folder = folder.children.iter().find(|c| c.name == "file").unwrap();
            assert!(matches!(file_folder.entry, FileTreeEntry::Folder(_)));

            // Check for "another" folder
            let another_folder = folder
                .children
                .iter()
                .find(|c| c.name == "another")
                .unwrap();
            assert!(matches!(another_folder.entry, FileTreeEntry::Folder(_)));

            // Check for "direct.txt" file
            let direct_file = folder
                .children
                .iter()
                .find(|c| c.name == "direct.txt")
                .unwrap();
            if let FileTreeEntry::File(file) = &direct_file.entry {
                assert_eq!(file.size_bytes, 700);
            } else {
                panic!("'direct.txt' should be a file");
            }
        } else {
            panic!("Result should be a folder");
        }
    }
}

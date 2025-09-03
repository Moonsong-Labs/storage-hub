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
pub struct FileEntry {
    pub name: String,

    #[serde(flatten)]
    pub entry: FileTreeEntry,
}

impl FileTree {
    /// Convert a list of files into a hierarchical file tree structure
    pub fn from_files(files: Vec<DBFile>) -> Self {
        // Use a BTreeMap to maintain consistent ordering
        let mut root_map: BTreeMap<String, FileTreeEntry> = BTreeMap::new();

        for file in files {
            // Convert location from Vec<u8> to String
            let location = String::from_utf8_lossy(&file.location);

            // Split the location into segments, removing empty ones
            let segments: Vec<&str> = location.split('/').filter(|s| !s.is_empty()).collect();

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
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;

    fn test_file_with_location_key_and_size(location: &str, file_key: &str, size: i64) -> DBFile {
        DBFile {
            id: 1,
            account: vec![],
            file_key: file_key.as_bytes().to_vec(),
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
    fn file_tree_from_files() {
        let files = vec![
            test_file_with_location_key_and_size("/path/to/file/foo.txt", "key1", 100),
            test_file_with_location_key_and_size("/path/to/file/bar.txt", "key2", 200),
            test_file_with_location_key_and_size("/path/to/another/thing.txt", "key3", 300),
            test_file_with_location_key_and_size("/a/different/file.txt", "key4", 400),
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
                        assert_eq!(file.file_key, hex::encode(b"key4"));
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
}

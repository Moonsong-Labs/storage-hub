export default {
  types: {
    FileMetadata: {
      owner: "Vec<u8>",
      bucket_id: "Vec<u8>",
      location: "Vec<u8>",
      file_size: "u64",
      fingerprint: "[u8; 32]"
    },
    IncompleteFileStatus: {
      file_metadata: "FileMetadata",
      stored_chunks: "u64",
      total_chunks: "u64"
    },
    SaveFileToDisk: {
      _enum: {
        FileNotFound: null,
        Success: "FileMetadata",
        IncompleteFile: "IncompleteFileStatus"
      }
    }
  },
  rpc: {
    loadFileInStorage: {
      description:
        "Load a file in the local storage. This is the first step when uploading a file.",
      params: [
        {
          name: "file_path",
          type: "String"
        },
        {
          name: "location",
          type: "String"
        },
        {
          name: "owner",
          type: "AccountId32"
        },
        {
          name: "bucket_id",
          type: "H256"
        }
      ],
      type: "FileMetadata"
    },
    saveFileToDisk: {
      description: "Save a file from the local storage to the disk.",
      params: [
        {
          name: "file_key",
          type: "H256"
        },
        {
          name: "file_path",
          type: "String"
        }
      ],
      type: "SaveFileToDisk"
    },
    getForestRoot: {
      description: "Get the root of the forest trie.",
      params: [],
      type: "H256"
    },
    rotateBcsvKeys: {
      description: "Rotate (generate and insert) new keys of BCSV type for the Blockchain Service.",
      params: [
        {
          name: "seed",
          type: "String"
        }
      ],
      type: "String"
    }
  }
};

export const rpcDefinitions = {
  storagehubclient: {
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
      params: [
        {
          name: "key",
          type: "Option<String>"
        }
      ],
      type: "H256"
    },
    isFileInForest: {
      description: "Check if a file is in the forest.",
      params: [
        {
          name: "key",
          type: "Option<String>"
        },
        {
          name: "file_key",
          type: "H256"
        }
      ],
      type: "bool"
    },
    isFileInFileStorage: {
      description: "Check if a file is in the file storage.",
      params: [
        {
          name: "file_key",
          type: "H256"
        }
      ],
      type: "bool"
    },
    getFileMetadata: {
      description: "Get the metadata of a file.",
      params: [
        {
          name: "key",
          type: "Option<String>"
        },
        {
          name: "file_key",
          type: "H256"
        }
      ],
      type: "Option<FileMetadata>"
    },
    insertBcsvKeys: {
      description: "Generate and insert new keys of type BCSV into the keystore.",
      params: [
        {
          name: "seed",
          type: "Option<String>"
        }
      ],
      type: "String"
    },
    removeBcsvKeys: {
      description: "Remove keys of BCSV type for the Blockchain Service.",
      params: [
        {
          name: "keystore_path",
          type: "String"
        }
      ],
      type: "()"
    }
  }
};
//# sourceMappingURL=rpc.js.map

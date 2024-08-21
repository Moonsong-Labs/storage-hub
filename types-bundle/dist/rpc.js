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
      params: [],
      type: "H256"
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

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
      type: "LoadFileInStorageResult"
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
          name: "forest_key",
          type: "Option<H256>"
        }
      ],
      type: "Option<H256>"
    },
    isFileInForest: {
      description: "Check if a file is in the forest.",
      params: [
        {
          name: "forest_key",
          type: "Option<H256>"
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
      type: "GetFileFromFileStorageResult"
    },
    getFileMetadata: {
      description: "Get the metadata of a file from the Forest storage.",
      params: [
        {
          name: "forest_key",
          type: "Option<H256>"
        },
        {
          name: "file_key",
          type: "H256"
        }
      ],
      type: "Option<FileMetadata>"
    },
    generateForestProof: {
      description:
        "Generate a SCALE-encoded proof for a group of file keys that might or might not be in the forest.",
      params: [
        {
          name: "forest_key",
          type: "Option<H256>"
        },
        {
          name: "challenged_file_keys",
          type: "Vec<H256>"
        }
      ],
      type: "Vec<u8>"
    },
    generateProof: {
      description:
        "Generate a SCALE-encoded proof for a group of file keys that might or might not be in the forest, alongside their key proofs.",
      params: [
        {
          name: "provider_id",
          type: "H256"
        },
        {
          name: "seed",
          type: "H256"
        },
        {
          name: "checkpoint_challenges",
          type: "Option<Vec<CheckpointChallenge>>"
        }
      ],
      type: "Vec<u8>"
    },
    generateFileKeyProofBspConfirm: {
      description:
        "Generate a SCALE-encoded proof for a file key to allow a BSP to confirm storing it.",
      params: [
        {
          name: "bsp_id",
          type: "H256"
        },
        {
          name: "file_key",
          type: "H256"
        }
      ],
      type: "Vec<u8>"
    },
    generateFileKeyProofMspAccept: {
      description:
        "Generate a SCALE-encoded proof for a file key to allow a MSP to accept storing it.",
      params: [
        {
          name: "msp_id",
          type: "H256"
        },
        {
          name: "file_key",
          type: "H256"
        }
      ],
      type: "Vec<u8>"
    }
  },
  storagehubprovider: {
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
    },
    addToExcludeList: {
      description:
        "Add key to exclude list. Exclude type can be `file`, `user`, `bucket` and `fingerprint`.",
      params: [
        {
          name: "key",
          type: "H256"
        },
        {
          name: "exclude_type",
          type: "String"
        }
      ],
      type: "()"
    },
    removeFromExcludeList: {
      description: "Remove key from exclude list",
      params: [
        {
          name: "key",
          type: "H256"
        },
        {
          name: "exclude_type",
          type: "String"
        }
      ],
      type: "()"
    }
  }
};
//# sourceMappingURL=rpc.js.map

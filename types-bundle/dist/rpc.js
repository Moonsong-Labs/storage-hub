export const rpcDefinitions = {
    storagehubclient: {
        loadFileInStorage: {
            description: "Load a file in the local storage. This is the first step when uploading a file.",
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
                    name: "owner_account_id_hex",
                    type: "String"
                },
                {
                    name: "bucket_id",
                    type: "H256"
                }
            ],
            type: "LoadFileInStorageResult"
        },
        removeFilesFromFileStorage: {
            description: "Remove a list of files from the file storage. Useful when doing manual maintenance.",
            params: [
                {
                    name: "file_keys",
                    type: "Vec<H256>"
                }
            ],
            type: "()"
        },
        removeFilesWithPrefixFromFileStorage: {
            description: "Remove all files under a prefix from the file storage. Useful when doing manual maintenance.",
            params: [
                {
                    name: "prefix",
                    type: "H256"
                }
            ],
            type: "()"
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
        addFilesToForestStorage: {
            description: "Add files to the forest storage. Useful when doing manual maintenance.",
            params: [
                {
                    name: "forest_key",
                    type: "Option<H256>"
                },
                {
                    name: "metadata_of_files_to_add",
                    type: "Vec<FileMetadata>"
                }
            ],
            type: "AddFilesToForestStorageResult"
        },
        removeFilesFromForestStorage: {
            description: "Remove files from the forest storage. Useful when doing manual maintenance.",
            params: [
                {
                    name: "forest_key",
                    type: "Option<H256>"
                },
                {
                    name: "file_keys",
                    type: "Vec<H256>"
                }
            ],
            type: "RemoveFilesFromForestStorageResult"
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
            description: "Generate a SCALE-encoded proof for a group of file keys that might or might not be in the forest.",
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
            description: "Generate a SCALE-encoded proof for a group of file keys that might or might not be in the forest, alongside their key proofs.",
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
            description: "Generate a SCALE-encoded proof for a file key to allow a BSP to confirm storing it.",
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
            description: "Generate a SCALE-encoded proof for a file key to allow a MSP to accept storing it.",
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
        },
        addToExcludeList: {
            description: "Add key to exclude list. Exclude type can be `file`, `user`, `bucket` and `fingerprint`.",
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
        },
        isFileKeyExpected: {
            description: "Check whether this node is currently expecting to receive the given file key",
            params: [
                {
                    name: "file_key",
                    type: "H256"
                }
            ],
            type: "bool"
        },
        receiveBackendFileChunks: {
            description: "Send a FileKeyProof with one or more chunks to be processed locally by the node",
            params: [
                {
                    name: "file_key",
                    type: "H256"
                },
                {
                    name: "file_key_proof",
                    type: "Vec<u8>"
                }
            ],
            type: "Vec<u8>"
        },
        getProviderId: {
            description: "Get the provider ID of the current node, if any.",
            params: [],
            type: "RpcProviderId"
        },
        getValuePropositions: {
            description: "Get the value propositions of the node if it's an MSP; otherwise a NotAnMsp/Error enum.",
            params: [],
            type: "GetValuePropositionsResult"
        },
        getCurrentPricePerGigaUnitPerTick: {
            description: "Get the current price per giga unit per tick from the payment streams pallet",
            params: [],
            type: "u128"
        }
    }
};
//# sourceMappingURL=rpc.js.map
declare const _default: {
    types: {
        FileMetadata: {
            owner: string;
            bucket_id: string;
            location: string;
            file_size: string;
            fingerprint: string;
        };
        IncompleteFileStatus: {
            file_metadata: string;
            stored_chunks: string;
            total_chunks: string;
        };
        SaveFileToDisk: {
            _enum: {
                FileNotFound: null;
                Success: string;
                IncompleteFile: string;
            };
        };
        BackupStorageProviderId: string;
        StorageData: string;
        MerklePatriciaRoot: string;
        ChunkId: string;
        BackupStorageProvider: {
            capacity: string;
            data_used: string;
            multiaddresses: string;
            root: string;
            last_capacity_change: string;
            owner_account: string;
            payment_account: string;
        };
        GetBspInfoError: {
            _enum: {
                BspNotRegistered: null;
                InternalApiError: null;
            };
        };
        QueryFileEarliestVolunteerBlockError: {
            _enum: {
                FailedToEncodeFingerprint: null;
                FailedToEncodeBsp: null;
                ThresholdArithmeticError: null;
                StorageRequestNotFound: null;
                InternalError: null;
            };
        };
        QueryBspConfirmChunksToProveForFileError: {
            _enum: {
                StorageRequestNotFound: null;
                InternalError: null;
            };
        };
    };
    runtime: import("@polkadot/types/types").DefinitionsCall;
    rpc: {
        loadFileInStorage: {
            description: string;
            params: {
                name: string;
                type: string;
            }[];
            type: string;
        };
        saveFileToDisk: {
            description: string;
            params: {
                name: string;
                type: string;
            }[];
            type: string;
        };
        getForestRoot: {
            description: string;
            params: never[];
            type: string;
        };
        rotateBcsvKeys: {
            description: string;
            params: {
                name: string;
                type: string;
            }[];
            type: string;
        };
    };
};
export default _default;

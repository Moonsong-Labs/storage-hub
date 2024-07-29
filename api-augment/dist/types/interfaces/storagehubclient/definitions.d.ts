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
    };
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

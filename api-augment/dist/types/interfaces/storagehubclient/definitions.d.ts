declare const _default: {
    types: {
        FileMetadata: {
            owner: string;
            bucket_id: string;
            location: string;
            size: string;
            fingerprint: string;
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
        getForestRoot: {
            description: string;
            params: never[];
            type: string;
        };
    };
};
export default _default;

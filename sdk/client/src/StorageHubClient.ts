export interface StorageHubClientOptions {
    endpoint?: string;
}

export class StorageHubClient {
    public readonly endpoint?: string;

    private constructor(opts: StorageHubClientOptions) {
        this.endpoint = opts.endpoint;
    }

    static async connect(opts: StorageHubClientOptions = {}): Promise<StorageHubClient> {
        // For now no network connection; just return instance
        return new StorageHubClient(opts);
    }
} 
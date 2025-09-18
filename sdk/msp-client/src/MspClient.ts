import type { HttpClientConfig } from "@storagehub-sdk/core";
import { FileMetadata, FileTrie, HttpClient, initWasm } from "@storagehub-sdk/core";
import type {
  Bucket,
  DownloadOptions,
  DownloadResult,
  FileInfo,
  FileListResponse,
  GetFilesOptions,
  HealthStatus,
  InfoResponse,
  NonceResponse,
  StatsResponse,
  UploadOptions,
  UploadReceipt,
  ValueProp,
  VerifyResponse
} from "./types.js";

export class MspClient {
  public readonly config: HttpClientConfig;
  private readonly http: HttpClient;
  public token?: string;

  private constructor(config: HttpClientConfig, http: HttpClient) {
    this.config = config;
    this.http = http;
  }

  static async connect(config: HttpClientConfig): Promise<MspClient> {
    if (!config?.baseUrl) throw new Error("MspClient.connect: baseUrl is required");

    const http = new HttpClient({
      baseUrl: config.baseUrl,
      ...(config.timeoutMs !== undefined && { timeoutMs: config.timeoutMs }),
      ...(config.defaultHeaders !== undefined && { defaultHeaders: config.defaultHeaders }),
      ...(config.fetchImpl !== undefined && { fetchImpl: config.fetchImpl })
    });

    return new MspClient(config, http);
  }

  getHealth(options?: { signal?: AbortSignal }): Promise<HealthStatus> {
    return this.http.get<HealthStatus>("/health", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Get general MSP information */
  getInfo(options?: { signal?: AbortSignal }): Promise<InfoResponse> {
    return this.http.get<InfoResponse>("/info", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Get MSP statistics */
  getStats(options?: { signal?: AbortSignal }): Promise<StatsResponse> {
    return this.http.get<StatsResponse>("/stats", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Get available value propositions */
  getValuePropositions(options?: { signal?: AbortSignal }): Promise<ValueProp[]> {
    return this.http.get<ValueProp[]>("/value-props", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  // Auth endpoints:

  /** Request a SIWE-style nonce message for the given address and chainId */
  getNonce(
    address: string,
    chainId: number,
    options?: { signal?: AbortSignal }
  ): Promise<NonceResponse> {
    return this.http.post<NonceResponse>("/auth/nonce", {
      body: { address, chainId },
      headers: { "Content-Type": "application/json" },
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Verify signed message and receive JWT token */
  verify(
    message: string,
    signature: string,
    options?: { signal?: AbortSignal }
  ): Promise<VerifyResponse> {
    return this.http.post<VerifyResponse>("/auth/verify", {
      body: { message, signature },
      headers: { "Content-Type": "application/json" },
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Store token to be sent on subsequent protected requests */
  setToken(token: string): void {
    this.token = token;
  }

  /** Merge Authorization header when token is present */
  private withAuth(headers?: Record<string, string>): Record<string, string> | undefined {
    if (!this.token) return headers;
    return { ...(headers ?? {}), Authorization: `Bearer ${this.token}` };
  }

  // Bucket endpoints:

  /** List all buckets for the current authenticateduser */
  listBuckets(options?: { signal?: AbortSignal }): Promise<Bucket[]> {
    const headers = this.withAuth();
    return this.http.get<Bucket[]>("/buckets", {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {})
    });
  }

  /** Get a specific bucket's metadata by its bucket ID */
  getBucket(bucketId: string, options?: { signal?: AbortSignal }): Promise<Bucket> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}`;
    return this.http.get<Bucket>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {})
    });
  }

  /** Gets the list of files and folders under the specified path for a bucket. If no path is provided, it returns the files and folders found at root. */
  getFiles(bucketId: string, options?: GetFilesOptions): Promise<FileListResponse> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}/files`;
    return this.http.get<FileListResponse>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
      ...(options?.path ? { query: { path: options.path.replace(/^\/+/, "") } } : {})
    });
  }

  /** Get metadata for a file in a bucket by fileKey */
  getFileInfo(
    bucketId: string,
    fileKey: string,
    options?: { signal?: AbortSignal }
  ): Promise<FileInfo> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}/info/${encodeURIComponent(fileKey)}`;
    type FileInfoWire = Omit<FileInfo, "uploadedAt"> & { uploadedAt: string };
    return this.http
      .get<FileInfoWire>(path, {
        ...(headers ? { headers } : {}),
        ...(options?.signal ? { signal: options.signal } : {})
      })
      .then((wire): FileInfo => ({ ...wire, uploadedAt: new Date(wire.uploadedAt) }));
  }

  // File endpoints:

  /**
   * Upload a file to a bucket with a specific key.
   *
   * Always uses multipart/form-data upload with both file data and encoded FileMetadata.
   * The file data is loaded into memory to create the multipart request.
   *
   */
  async uploadFile(
    bucketId: string,
    fileKey: string,
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | unknown,
    owner: string,
    location: string,
    _options?: UploadOptions
  ): Promise<UploadReceipt> {
    void _options;

    await initWasm();

    const backendPath = `/buckets/${encodeURIComponent(bucketId)}/upload/${encodeURIComponent(fileKey)}`;
    const authHeaders = this.withAuth();

    // Convert the file to a blob and get its size
    const fileBlob = await this.coerceToFormPart(file);
    const fileSize = fileBlob.size;

    // Compute the fingerprint first
    // TODO: We should instead use FileManager here and use its `getFingerprint` method.
    // This would allow us to remove the `initWasm` call at the top and to stream the file
    // instead of loading it into memory as a blob.
    const fingerprint = await this.computeFileFingerprint(fileBlob);

    // Create the FileMetadata instance
    const metadata = await this.formFileMetadata(
      owner,
      bucketId,
      location,
      fingerprint,
      BigInt(fileSize)
    );

    // Compute the file key and ensure it matches the provided file key
    const computedFileKey = await this.computeFileKey(metadata);
    const expectedFileKeyBytes = this.hexToBytes(fileKey);
    if (
      computedFileKey.length !== expectedFileKeyBytes.length ||
      !computedFileKey.every((byte, index) => byte === expectedFileKeyBytes[index])
    ) {
      throw new Error(
        `Computed file key ${computedFileKey.toString()} does not match provided file key ${expectedFileKeyBytes.toString()}`
      );
    }

    // Encode the file metadata
    const encodedMetadata = metadata.encode();

    // Create the multipart form with both the file and its metadata
    const form = new FormData();
    const fileMetadataBlob = new Blob([new Uint8Array(encodedMetadata)], {
      type: "application/octet-stream"
    });
    form.append("file_metadata", fileMetadataBlob, "file_metadata");
    form.append("file", fileBlob, "file");

    const res = await this.http.put<UploadReceipt>(
      backendPath,
      authHeaders
        ? { body: form as unknown as BodyInit, headers: authHeaders }
        : { body: form as unknown as BodyInit }
    );
    return res;
  }

  private async coerceToFormPart(
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | unknown
  ): Promise<Blob> {
    if (typeof Blob !== "undefined" && file instanceof Blob) return file;
    if (file instanceof Uint8Array) return new Blob([file.buffer as ArrayBuffer]);
    if (typeof ArrayBuffer !== "undefined" && file instanceof ArrayBuffer) return new Blob([file]);

    // Handle ReadableStream by reading it into memory
    if (file instanceof ReadableStream) {
      const reader = file.getReader();
      const chunks: Uint8Array[] = [];
      let totalLength = 0;

      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          if (value) {
            chunks.push(value);
            totalLength += value.length;
          }
        }
      } finally {
        reader.releaseLock();
      }

      // Combine all chunks into a single Uint8Array
      const combined = new Uint8Array(totalLength);
      let offset = 0;
      for (const chunk of chunks) {
        combined.set(chunk, offset);
        offset += chunk.length;
      }

      return new Blob([combined], { type: "application/octet-stream" });
    }

    return new Blob([file as BlobPart], { type: "application/octet-stream" });
  }

  private async computeFileFingerprint(fileBlob: Blob): Promise<Uint8Array> {
    const trie = new FileTrie();
    const fileBytes = new Uint8Array(await fileBlob.arrayBuffer());

    // Process the file in 1KB chunks (matching CHUNK_SIZE from constants)
    const CHUNK_SIZE = 1024;
    let offset = 0;

    while (offset < fileBytes.length) {
      const end = Math.min(offset + CHUNK_SIZE, fileBytes.length);
      const chunk = fileBytes.slice(offset, end);
      trie.push_chunk(chunk);
      offset = end;
    }

    return trie.get_root();
  }

  async formFileMetadata(
    owner: string,
    bucketId: string,
    location: string,
    fingerprint: Uint8Array,
    size: bigint
  ): Promise<FileMetadata> {
    const ownerBytes = this.hexToBytes(owner);
    const bucketIdBytes = this.hexToBytes(bucketId);
    const locationBytes = new TextEncoder().encode(location);
    await initWasm();
    return new FileMetadata(ownerBytes, bucketIdBytes, locationBytes, size, fingerprint);
  }

  hexToBytes(hex: string): Uint8Array {
    if (!hex) {
      throw new Error("hex string cannot be empty");
    }

    const cleanHex = hex.startsWith("0x") ? hex.slice(2) : hex;

    if (cleanHex.length % 2 !== 0) {
      throw new Error("hex string must have an even number of characters");
    }

    if (!/^[0-9a-fA-F]*$/.test(cleanHex)) {
      throw new Error("hex string contains invalid characters");
    }

    return new Uint8Array(cleanHex.match(/.{2}/g)?.map((byte) => Number.parseInt(byte, 16)) || []);
  }

  async computeFileKey(fileMetadata: FileMetadata): Promise<Uint8Array> {
    await initWasm();
    return fileMetadata.getFileKey();
  }

  /** Download a file by key. */
  async downloadByKey(fileKey: string, options?: DownloadOptions): Promise<DownloadResult> {
    const path = `/download/${encodeURIComponent(fileKey)}`;
    const baseHeaders: Record<string, string> = { Accept: "*/*" };
    if (options?.range) {
      const { start, end } = options.range;
      const rangeValue = `bytes=${start}-${end ?? ""}`;
      baseHeaders.Range = rangeValue;
    }
    const headers = this.withAuth(baseHeaders);
    const res = await this.http.getRaw(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {})
    });

    if (!res.body) {
      throw new Error("Response body is null - unable to create stream");
    }

    const contentType = res.headers.get("content-type");
    const contentRange = res.headers.get("content-range");
    const contentLengthHeader = res.headers.get("content-length");
    const parsedLength = contentLengthHeader !== null ? Number(contentLengthHeader) : undefined;
    const contentLength =
      typeof parsedLength === "number" && Number.isFinite(parsedLength) ? parsedLength : null;

    return {
      stream: res.body,
      status: res.status,
      contentType,
      contentRange,
      contentLength
    };
  }

  /** Download a file by its location path under a bucket. */
  async downloadByLocation(
    bucketId: string,
    filePath: string,
    options?: DownloadOptions
  ): Promise<DownloadResult> {
    const normalized = filePath.replace(/^\/+/, "");
    const encodedPath = normalized.split("/").map(encodeURIComponent).join("/");
    const path = `/buckets/${encodeURIComponent(bucketId)}/download/path/${encodedPath}`;
    const baseHeaders: Record<string, string> = { Accept: "*/*" };
    if (options?.range) {
      const { start, end } = options.range;
      const rangeValue = `bytes=${start}-${end ?? ""}`;
      baseHeaders.Range = rangeValue;
    }
    const headers = this.withAuth(baseHeaders);
    const res = await this.http.getRaw(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {})
    });

    if (!res.body) {
      throw new Error("Response body is null - unable to create stream");
    }

    const contentType = res.headers.get("content-type");
    const contentRange = res.headers.get("content-range");
    const contentLengthHeader = res.headers.get("content-length");
    const parsedLength = contentLengthHeader !== null ? Number(contentLengthHeader) : undefined;
    const contentLength =
      typeof parsedLength === "number" && Number.isFinite(parsedLength) ? parsedLength : null;

    return {
      stream: res.body,
      status: res.status,
      contentType,
      contentRange,
      contentLength
    };
  }
}

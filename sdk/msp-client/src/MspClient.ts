import type {
  DownloadOptions,
  DownloadResult,
  HealthStatus,
  NonceResponse,
  UploadOptions,
  UploadReceipt,
  VerifyResponse,
  Bucket,
  FileListResponse,
  GetFilesOptions,
} from './types.js';
import type { HttpClientConfig } from '@storagehub-sdk/core';
import { HttpClient } from '@storagehub-sdk/core';

export class MspClient {
  public readonly config: HttpClientConfig;
  private readonly http: HttpClient;
  private token?: string;

  private constructor(config: HttpClientConfig, http: HttpClient) {
    this.config = config;
    this.http = http;
  }

  static async connect(config: HttpClientConfig): Promise<MspClient> {
    if (!config?.baseUrl) throw new Error('MspClient.connect: baseUrl is required');

    const http = new HttpClient({
      baseUrl: config.baseUrl,
      ...(config.timeoutMs !== undefined && { timeoutMs: config.timeoutMs }),
      ...(config.defaultHeaders !== undefined && { defaultHeaders: config.defaultHeaders }),
      ...(config.fetchImpl !== undefined && { fetchImpl: config.fetchImpl }),
    });

    return new MspClient(config, http);
  }

  getHealth(options?: { signal?: AbortSignal }): Promise<HealthStatus> {
    return this.http.get<HealthStatus>('/health', {
      ...(options?.signal !== undefined && { signal: options.signal }),
    });
  }

  // Auth endpoints:

  /** Request a SIWE-style nonce message for the given address and chainId */
  getNonce(
    address: string,
    chainId: number,
    options?: { signal?: AbortSignal },
  ): Promise<NonceResponse> {
    return this.http.post<NonceResponse>('/auth/nonce', {
      body: { address, chainId },
      headers: { 'Content-Type': 'application/json' },
      ...(options?.signal !== undefined && { signal: options.signal }),
    });
  }

  /** Verify signed message and receive JWT token */
  verify(
    message: string,
    signature: string,
    options?: { signal?: AbortSignal },
  ): Promise<VerifyResponse> {
    return this.http.post<VerifyResponse>('/auth/verify', {
      body: { message, signature },
      headers: { 'Content-Type': 'application/json' },
      ...(options?.signal !== undefined && { signal: options.signal }),
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
    return this.http.get<Bucket[]>('/buckets', {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
    });
  }

  /** Get a specific bucket's metadata by its bucket ID */
  getBucket(bucketId: string, options?: { signal?: AbortSignal }): Promise<Bucket> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}`;
    return this.http.get<Bucket>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
    });
  }

  /** Gets the list of files and folders under the specified path for a bucket. If no path is provided, it returns the files and folders found at root. */
  getFiles(bucketId: string, options?: GetFilesOptions): Promise<FileListResponse> {
    const headers = this.withAuth();
    const path = `/buckets/${encodeURIComponent(bucketId)}/files`;
    return this.http.get<FileListResponse>(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
      ...(options?.path ? { query: { path: options.path.replace(/^\/+/, '') } } : {}),
    });
  }

  // File endpoints:

  /**
   * Upload a file to a bucket for a specific fileKey using multipart/form-data.
   *
   * This matches the backend's current expectation of a FormData field named
   * "file" sent via PUT to `/buckets/:bucketId/upload/:fileKey`.
   *
   * Accepted `file` types depend on the environment. In browsers, pass a
   * Blob/File or ArrayBuffer/Uint8Array. In Node 18+/23 with fetch, Node
   * Readable streams are also accepted by FormData.
   */
  async uploadFile(
    bucketId: string,
    fileKey: string,
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | unknown,
    _options?: UploadOptions,
  ): Promise<UploadReceipt> {
    void _options;
    const form = new FormData();

    const part = this.coerceToFormPart(file);
    form.append('file', part as unknown as Blob);

    const path = `/buckets/${encodeURIComponent(bucketId)}/upload/${encodeURIComponent(fileKey)}`;
    const authHeaders = this.withAuth();
    const res = await this.http.put<UploadReceipt>(
      path,
      authHeaders
        ? { body: form as unknown as BodyInit, headers: authHeaders }
        : { body: form as unknown as BodyInit },
    );
    return res;
  }

  private coerceToFormPart(
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | unknown,
  ): Blob | unknown {
    if (typeof Blob !== 'undefined' && file instanceof Blob) return file;
    if (file instanceof Uint8Array) return new Blob([file]);
    if (typeof ArrayBuffer !== 'undefined' && file instanceof ArrayBuffer) return new Blob([file]);
    // In Node environments, FormData accepts streams; pass-through as-is
    return file;
  }

  /** Download a file by bucket and key. */
  async downloadByKey(
    bucketId: string,
    fileKey: string,
    options?: DownloadOptions,
  ): Promise<DownloadResult> {
    const path = `/buckets/${encodeURIComponent(bucketId)}/download/${encodeURIComponent(fileKey)}`;
    const baseHeaders: Record<string, string> = { Accept: '*/*' };
    if (options?.range) {
      const { start, end } = options.range;
      const rangeValue = `bytes=${start}-${end ?? ''}`;
      baseHeaders.Range = rangeValue;
    }
    const headers = this.withAuth(baseHeaders);
    const res = await this.http.getRaw(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
    });

    if (!res.body) {
      throw new Error('Response body is null - unable to create stream');
    }

    const contentType = res.headers.get('content-type');
    const contentRange = res.headers.get('content-range');
    const contentLengthHeader = res.headers.get('content-length');
    const parsedLength = contentLengthHeader !== null ? Number(contentLengthHeader) : undefined;
    const contentLength =
      typeof parsedLength === 'number' && Number.isFinite(parsedLength) ? parsedLength : null;

    return {
      stream: res.body,
      status: res.status,
      contentType,
      contentRange,
      contentLength,
    };
  }

  /** Download a file by its location path under a bucket. */
  async downloadByLocation(
    bucketId: string,
    filePath: string,
    options?: DownloadOptions,
  ): Promise<DownloadResult> {
    const normalized = filePath.replace(/^\/+/, '');
    const encodedPath = normalized.split('/').map(encodeURIComponent).join('/');
    const path = `/buckets/${encodeURIComponent(bucketId)}/download/path/${encodedPath}`;
    const baseHeaders: Record<string, string> = { Accept: '*/*' };
    if (options?.range) {
      const { start, end } = options.range;
      const rangeValue = `bytes=${start}-${end ?? ''}`;
      baseHeaders.Range = rangeValue;
    }
    const headers = this.withAuth(baseHeaders);
    const res = await this.http.getRaw(path, {
      ...(headers ? { headers } : {}),
      ...(options?.signal ? { signal: options.signal } : {}),
    });

    if (!res.body) {
      throw new Error('Response body is null - unable to create stream');
    }

    const contentType = res.headers.get('content-type');
    const contentRange = res.headers.get('content-range');
    const contentLengthHeader = res.headers.get('content-length');
    const parsedLength = contentLengthHeader !== null ? Number(contentLengthHeader) : undefined;
    const contentLength =
      typeof parsedLength === 'number' && Number.isFinite(parsedLength) ? parsedLength : null;

    return {
      stream: res.body,
      status: res.status,
      contentType,
      contentRange,
      contentLength,
    };
  }
}

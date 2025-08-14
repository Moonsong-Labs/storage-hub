import { HttpClient } from '@storagehub-sdk/core';
import type { HttpClientConfig } from '@storagehub-sdk/core';
import type { HealthStatus, UploadOptions, UploadReceipt, NonceResponse } from './types';

export class MspClient {
  public readonly config: HttpClientConfig;
  private readonly http: HttpClient;

  private constructor(config: HttpClientConfig, http: HttpClient) {
    this.config = config;
    this.http = http;
  }

  static async connect(config: HttpClientConfig): Promise<MspClient> {
    if (!config?.baseUrl) throw new Error('MspClient.connect: baseUrl is required');

    const http = new HttpClient({
      baseUrl: config.baseUrl,
      timeoutMs: config.timeoutMs,
      defaultHeaders: config.defaultHeaders,
      fetchImpl: config.fetchImpl,
    });

    return new MspClient(config, http);
  }

  getHealth(options?: { signal?: AbortSignal }): Promise<HealthStatus> {
    return this.http.get<HealthStatus>('/health', { signal: options?.signal });
  }

  /** Request a SIWE-style nonce message for the given address and chainId */
  getNonce(address: string, chainId: number, options?: { signal?: AbortSignal }): Promise<NonceResponse> {
    return this.http.post<NonceResponse>('/auth/nonce', {
      body: { address, chainId },
      signal: options?.signal,
      headers: { 'Content-Type': 'application/json' },
    });
  }

  /**
   * Upload a file to a bucket for a specific fileKey using multipart/form-data.
   *
   * This matches the backend's current expectation of a FormData field named
   * "file" sent via PUT to `/buckets/:bucketId/:fileKey/upload`.
   *
   * Accepted `file` types depend on the environment. In browsers, pass a
   * Blob/File or ArrayBuffer/Uint8Array. In Node 18+/23 with fetch, Node
   * Readable streams are also accepted by FormData.
   */
  async uploadFile(
    bucketId: string,
    fileKey: string,
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | any,
    _options?: UploadOptions,
  ): Promise<UploadReceipt | any> {
    const form = new FormData();

    const part = this.coerceToFormPart(file);
    form.append('file', part as any);

    const path = `/buckets/${encodeURIComponent(bucketId)}/${encodeURIComponent(fileKey)}/upload`;
    const res = await this.http.put<any>(path, {
      body: form as unknown as BodyInit,
    });
    return res;
  }

  private coerceToFormPart(
    file: Blob | ArrayBuffer | Uint8Array | ReadableStream<Uint8Array> | any,
  ): Blob | any {
    if (typeof Blob !== 'undefined' && file instanceof Blob) return file;
    if (file instanceof Uint8Array) return new Blob([file]);
    if (typeof ArrayBuffer !== 'undefined' && file instanceof ArrayBuffer) return new Blob([file]);
    // In Node environments, FormData accepts streams; pass-through as-is
    return file;
  }
}

import { HttpClient } from '@storagehub-sdk/core';
import type { HttpClientConfig } from '@storagehub-sdk/core';
import type { HealthStatus } from './types';

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
}

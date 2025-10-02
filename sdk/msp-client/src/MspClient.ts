import type {
  Bucket,
  FileInfo,
  HealthStatus,
  InfoResponse,
  StatsResponse,
  UploadOptions,
  UploadReceipt,
  ValueProp
} from "./types.js";
import type { HttpClientConfig } from "@storagehub-sdk/core";
import { FileMetadata, FileTrie, HttpClient, initWasm } from "@storagehub-sdk/core";
import type { MspClientContext } from "./context.js";
import { AuthModule } from "./modules/auth.js";
import { BucketsModule } from "./modules/buckets.js";
import { ModuleBase } from "./base.js";
import { FilesModule } from "./modules/files.js";

export class MspClient extends ModuleBase {
  public readonly config: HttpClientConfig;
  private readonly http: HttpClient;
  private readonly context: MspClientContext;
  public readonly auth: AuthModule;
  public readonly buckets: BucketsModule;
  public readonly files: FilesModule;

  private constructor(config: HttpClientConfig, http: HttpClient) {
    const context: MspClientContext = { config, http };
    super(context);
    this.config = config;
    this.http = http;
    this.context = context;
    this.auth = new AuthModule(this.context);
    this.buckets = new BucketsModule(this.context);
    this.files = new FilesModule(this.context);
  }

  static async connect(config: HttpClientConfig): Promise<MspClient> {
    if (!config?.baseUrl) throw new Error("MspClient.connect: baseUrl is required");

    const http = new HttpClient({
      baseUrl: config.baseUrl,
      ...(config.timeoutMs !== undefined && { timeoutMs: config.timeoutMs }),
      ...(config.defaultHeaders !== undefined && {
        defaultHeaders: config.defaultHeaders
      }),
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

}

import type { HttpClientConfig } from "@storagehub-sdk/core";
import { HttpClient } from "@storagehub-sdk/core";
import type { MspClientContext } from "./context.js";
import { AuthModule } from "./modules/auth.js";
import { BucketsModule } from "./modules/buckets.js";
import { ModuleBase } from "./base.js";
import { FilesModule } from "./modules/files.js";
import { InfoModule } from "./modules/info.js";

export class MspClient extends ModuleBase {
  public readonly config: HttpClientConfig;
  private readonly context: MspClientContext;
  public readonly auth: AuthModule;
  public readonly buckets: BucketsModule;
  public readonly files: FilesModule;
  public readonly info: InfoModule;

  private constructor(config: HttpClientConfig, http: HttpClient) {
    const context: MspClientContext = { config, http };
    super(context);
    this.config = config;
    this.context = context;
    this.auth = new AuthModule(this.context);
    this.buckets = new BucketsModule(this.context);
    this.files = new FilesModule(this.context);
    this.info = new InfoModule(this.context);
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
}

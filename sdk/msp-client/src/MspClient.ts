import type { HttpClientConfig } from "@storagehub-sdk/core";
import { HttpClient } from "@storagehub-sdk/core";
import type { MspClientContext } from "./context.js";
import { AuthModule } from "./modules/auth.js";
import { BucketsModule } from "./modules/buckets.js";
import { ModuleBase } from "./base.js";
import { FilesModule } from "./modules/files.js";
import { InfoModule } from "./modules/info.js";
import type { SessionProvider } from "./types.js";

export class MspClient extends ModuleBase {
  public readonly config: HttpClientConfig;
  private readonly context: MspClientContext;
  public readonly auth: AuthModule;
  public readonly buckets: BucketsModule;
  public readonly files: FilesModule;
  public readonly info: InfoModule;

  private constructor(
    config: HttpClientConfig,
    http: HttpClient,
    sessionProvider: SessionProvider
  ) {
    const context: MspClientContext = { config, http };
    super(context, sessionProvider);
    this.config = config;
    this.context = context;
    this.auth = new AuthModule(this.context, sessionProvider);
    this.buckets = new BucketsModule(this.context, sessionProvider);
    this.files = new FilesModule(this.context, sessionProvider);
    this.info = new InfoModule(this.context, sessionProvider);
  }

  static async connect(
    config: HttpClientConfig,
    sessionProvider: SessionProvider
  ): Promise<MspClient> {
    if (!config?.baseUrl) throw new Error("MspClient.connect: baseUrl is required");

    const http = new HttpClient({
      baseUrl: config.baseUrl,
      ...(config.timeoutMs !== undefined && { timeoutMs: config.timeoutMs }),
      ...(config.defaultHeaders !== undefined && {
        defaultHeaders: config.defaultHeaders
      }),
      ...(config.fetchImpl !== undefined && { fetchImpl: config.fetchImpl })
    });

    if (!sessionProvider) throw new Error("MspClient.connect: sessionProvider is required");
    return new MspClient(config, http, sessionProvider);
  }
}

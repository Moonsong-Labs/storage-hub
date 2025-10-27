import type { HttpClient, HttpClientConfig } from "@storagehub-sdk/core";

export interface MspClientContext {
  config: HttpClientConfig;
  http: HttpClient;
}

import type { Session } from "./types.js";
import type { HttpClient, HttpClientConfig } from "@storagehub-sdk/core";

export interface MspClientContext {
  config: HttpClientConfig;
  http: HttpClient;
  session?: Session;
}



export interface MspClientOptions {
  endpoint?: string;
}

export class MspClient {
  public readonly endpoint: string | undefined;

  private constructor(opts: MspClientOptions) {
    this.endpoint = opts.endpoint;
  }

  static async connect(opts: MspClientOptions = {}): Promise<MspClient> {
    // For now no network connection; just return instance
    return new MspClient(opts);
  }
}

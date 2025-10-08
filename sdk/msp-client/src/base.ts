import type { MspClientContext } from "./context.js";

export abstract class ModuleBase {
  protected readonly ctx: MspClientContext;

  constructor(ctx: MspClientContext) {
    this.ctx = ctx;
  }

  protected withAuth(headers?: Record<string, string>): Record<string, string> | undefined {
    const token = this.ctx.session?.token;
    return token ? { ...(headers ?? {}), Authorization: `Bearer ${token}` } : headers;
  }
}

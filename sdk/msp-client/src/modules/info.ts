import type { MspClientContext } from "../context.js";
import { ModuleBase } from "../base.js";
import type { HealthStatus, InfoResponse, StatsResponse, ValueProp } from "../types.js";

export class InfoModule extends ModuleBase {
  constructor(ctx: MspClientContext) {
    super(ctx);
  }

  getHealth(options?: { signal?: AbortSignal }): Promise<HealthStatus> {
    return this.ctx.http.get<HealthStatus>("/health", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Get general MSP information */
  getInfo(options?: { signal?: AbortSignal }): Promise<InfoResponse> {
    return this.ctx.http.get<InfoResponse>("/info", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Get MSP statistics */
  getStats(options?: { signal?: AbortSignal }): Promise<StatsResponse> {
    return this.ctx.http.get<StatsResponse>("/stats", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }

  /** Get available value propositions */
  getValuePropositions(options?: { signal?: AbortSignal }): Promise<ValueProp[]> {
    return this.ctx.http.get<ValueProp[]>("/value-props", {
      ...(options?.signal !== undefined && { signal: options.signal })
    });
  }
}



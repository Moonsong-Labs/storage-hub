import { ModuleBase } from "../base.js";
import type {
  HealthStatus,
  InfoResponse,
  PaymentStreamsResponse,
  StatsResponse,
  ValueProp
} from "../types.js";

export class InfoModule extends ModuleBase {
  getHealth(signal?: AbortSignal): Promise<HealthStatus> {
    return this.ctx.http.get<HealthStatus>("/health", {
      ...(signal ? { signal } : {})
    });
  }

  /** Get general MSP information */
  getInfo(signal?: AbortSignal): Promise<InfoResponse> {
    return this.ctx.http.get<InfoResponse>("/info", {
      ...(signal ? { signal } : {})
    });
  }

  /** Get MSP statistics */
  getStats(signal?: AbortSignal): Promise<StatsResponse> {
    return this.ctx.http.get<StatsResponse>("/stats", {
      ...(signal ? { signal } : {})
    });
  }

  /** Get available value propositions */
  getValuePropositions(signal?: AbortSignal): Promise<ValueProp[]> {
    return this.ctx.http.get<ValueProp[]>("/value-props", {
      ...(signal ? { signal } : {})
    });
  }

  /** Get payment streams for current authenticated user */
  async getPaymentStreams(signal?: AbortSignal): Promise<PaymentStreamsResponse> {
    const headers = await this.withAuth();
    return this.ctx.http.get<PaymentStreamsResponse>("/payment_streams", {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });
  }
}

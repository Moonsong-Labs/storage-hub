import { ModuleBase } from "../base.js";
import type {
  HealthStatus,
  InfoResponse,
  PaymentStreamsResponse,
  StatsResponse,
  ValueProp
} from "../types.js";
import { ensure0xPrefix } from "@storagehub-sdk/core";

export class InfoModule extends ModuleBase {
  getHealth(signal?: AbortSignal): Promise<HealthStatus> {
    return this.ctx.http.get<HealthStatus>("/health", {
      ...(signal ? { signal } : {})
    });
  }

  /** Get general MSP information */
  async getInfo(signal?: AbortSignal): Promise<InfoResponse> {
    const wire = await this.ctx.http.get<{
      client: string;
      version: string;
      mspId: string;
      multiaddresses: string[];
      ownerAccount: string;
      paymentAccount: string;
      status: string;
      activeSince: number;
      uptime: string;
    }>("/info", {
      ...(signal ? { signal } : {})
    });

    return {
      client: wire.client,
      version: wire.version,
      mspId: ensure0xPrefix(wire.mspId),
      multiaddresses: wire.multiaddresses,
      ownerAccount: ensure0xPrefix(wire.ownerAccount), // Ensure 0x prefix (backend has it, but TypeScript needs guarantee)
      paymentAccount: ensure0xPrefix(wire.paymentAccount), // Ensure 0x prefix (backend has it, but TypeScript needs guarantee)
      status: wire.status,
      activeSince: wire.activeSince,
      uptime: wire.uptime
    };
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

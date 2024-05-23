import { createClient, type PolkadotClient } from "polkadot-api";
import { WebSocketProvider } from "polkadot-api/ws-provider/node";
import { relaychain, storagehub } from "@polkadot-api/descriptors";

export type TypesBundle = typeof storagehub | typeof relaychain;

// TODO add method for waiting for blocks instead of time
export const waitForChain = async (
  client: PolkadotClient,
  options?: {
    timeoutMs?: number;
    blocks?: number;
  }
) => {
  process.stdout.write(
    `Waiting a maximum of ${
      options?.timeoutMs || 60_000 / 1000
    } seconds for ${await client._request("system_chain", [])} chain to be ready...`
  );
  const startTime = performance.now();

  const startingHeight = (await client.getBlockHeader()).number;
  for (;;) {
    try {
      const blockHeight = (await client.getBlockHeader()).number;
      if (blockHeight - startingHeight > (options?.blocks || 0)) {
        process.stdout.write("✅\n");
        break;
      }
    } catch (e) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    if (performance.now() - startTime > (options?.timeoutMs || 60_000)) {
      throw new Error("Timeout waiting for chain to be ready");
    }
  }
};

export const getZombieClients = async (
  params = { relayWs: "ws://127.0.0.1:31000", shWs: "ws://127.0.0.1:32000" }
) => {
  const relayClient = createClient(WebSocketProvider(params.relayWs));
  const relayApi = relayClient.getTypedApi(relaychain);
  const relayRT = await relayApi.runtime.latest();

  const shClient = createClient(WebSocketProvider(params.shWs));
  const storageApi = shClient.getTypedApi(storagehub);
  const storageRT = await storageApi.runtime.latest();

  return { relayApi, relayRT, relayClient, storageApi, storageRT, shClient };
};

import { createClient, type PolkadotClient } from "polkadot-api";
import { WebSocketProvider } from "polkadot-api/ws-provider/node";
import { relaychain, storagehub } from "@polkadot-api/descriptors";

export type TypesBundle = typeof relaychain | typeof storagehub;

export const getClient = async (endpoint: string, typesBundle: TypesBundle) => {
  const client = createClient(WebSocketProvider(endpoint));
  const api = client.getTypedApi(typesBundle);
  const rt = await api.runtime.latest();
  return { api, rt, client };
};

export const waitForChain = async (client: PolkadotClient, timeout = 120000) => {
  console.log(`Waiting a maximum of ${timeout / 1000} seconds for chain to be ready...`);
  const startTime = performance.now();

  for (;;) {
    try {
      const blockHeight = (await client.getBlockHeader()).number;
      if (blockHeight > 0) {
        console.log(`Chain is ready at block height ${blockHeight}`);
        break;
      }
    } catch (e) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    if (performance.now() - startTime > timeout) {
      throw new Error("Timeout waiting for chain to be ready");
    }
  }
};

export const getZombieClients = async () => {
  const {
    api: relayApi,
    rt: relayRT,
    client: relayClient,
  } = await getClient("ws://127.0.0.1:39459", relaychain);
  const {
    api: storageApi,
    rt: storageRT,
    client: shClient,
  } = await getClient("ws://127.0.0.1:42933", storagehub);

  return { relayApi, relayRT, relayClient, storageApi, storageRT, shClient };
};

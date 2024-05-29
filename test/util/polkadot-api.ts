import { createClient, FixedSizeBinary, type PolkadotClient, type TypedApi } from "polkadot-api";
import { WebSocketProvider } from "polkadot-api/ws-provider/node";
import { relaychain, storagehub } from "@polkadot-api/descriptors";

export type TypesBundle = typeof storagehub | typeof relaychain;

type StorageHubApi = TypedApi<typeof storagehub>;

export const waitForRandomness = async (api: StorageHubApi, timeoutMs = 60_000) => {
  process.stdout.write("Waiting for randomness...");

  const waitForValueOrTimeout = (timeoutMs: number): Promise<[FixedSizeBinary<32>, number]> => {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("Timeout"));
      }, timeoutMs);

      let valueCount = 0;
      const subscription = api.query.Randomness.LatestOneEpochAgoRandomness.watchValue(
        "best"
      ).subscribe((value) => {
        valueCount++;

        if (!value) {
          subscription.unsubscribe();
          reject(new Error("Randomness value is undefined"));
        }

        if (valueCount === 2) {
          if (!value) {
            throw new Error("Randomness value is undefined");
          }
          clearTimeout(timeout);
          subscription.unsubscribe();
          resolve(value);
        }
      });
    });
  };

  try {
    const [randomness, blockHeight] = await waitForValueOrTimeout(timeoutMs);
    if (blockHeight && randomness) {
      process.stdout.write("✅\n");
      return { blockHeight, randomness };
    }
    process.stdout.write("❌\n");
    console.error("Timeout reached without receiving a value.");
  } catch (error) {
    process.stdout.write("❌\n");
    console.error("An error occurred:", error);
  }
};

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

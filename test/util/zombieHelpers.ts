import "@storagehub/api-augment";
import assert from "node:assert";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { sleep } from "./timer";

export type ZombieClients = Promise<{
  [Symbol.asyncDispose]: () => Promise<void>;
  relayApi: ApiPromise;
  storageApi: ApiPromise;
}>;

export const getZombieClients = async (options: {
  relayWs?: string;
  shWs?: string;
}): ZombieClients => {
  const relayWsProvider = new WsProvider(options.relayWs);
  const relayApi = await ApiPromise.create({ provider: relayWsProvider, noInitWarn: true });
  const shWsProvider = new WsProvider(options.shWs);
  const shApi = await ApiPromise.create({ provider: shWsProvider, noInitWarn: true });

  return {
    [Symbol.asyncDispose]: async () => {
      await relayApi.disconnect();
      await shApi.disconnect();
    },
    relayApi,
    storageApi: shApi
  };
};

export const waitForChain = async (
  api: ApiPromise,
  options?: {
    timeoutMs?: number;
    blocks?: number;
  }
) => {
  const startTime = performance.now();

  process.stdout.write(
    `Waiting a maximum of ${
      options?.timeoutMs || 60_000 / 1000
    } seconds for ${await api.rpc.system.chain()} chain to be ready...`
  );
  const startingHeight = (await api.rpc.chain.getHeader()).number.toNumber();

  for (;;) {
    try {
      const blockHeight = (await api.rpc.chain.getHeader()).number.toNumber();
      if (blockHeight - startingHeight > (options?.blocks || 0)) {
        process.stdout.write("✅\n");
        break;
      }
      await sleep(1000);
    } catch (_e) {
      await sleep(1000);
    }

    assert(
      performance.now() - startTime < (options?.timeoutMs || 60_000),
      "Timeout waiting for chain to be ready"
    );
  }
};

export const waitForRandomness = async (api: ApiPromise, timeoutMs = 60_000) => {
  process.stdout.write("Waiting for randomness...");

  const waitForValueOrTimeout = (timeoutMs: number) => {
    return new Promise(async (resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("Timeout"));
      }, timeoutMs);

      let valueCount = 0;
      const unsub = await api.query.randomness.latestOneEpochAgoRandomness((data) => {
        valueCount++;
        if (!data) {
          unsub();
          reject(new Error("Randomness value is undefined"));
        }
        if (valueCount === 2) {
          assert(data, "Randomness value is undefined");
          clearTimeout(timeout);
          unsub();
          resolve(data);
        }
      });
    });
  };

  try {
    const result = await waitForValueOrTimeout(timeoutMs);
    if (result) {
      process.stdout.write("✅\n");
      return result;
    }
    process.stdout.write("❌\n");
    console.error("Timeout reached without receiving a value.");
  } catch (error) {
    process.stdout.write("❌\n");
    console.error("An error occurred:", error);
  }
};

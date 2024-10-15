import { after, before, describe, it, afterEach, beforeEach } from "node:test";
import { runInitialisedFullNet, runSimpleFullNet } from "./helpers";
import { cleardownTest, launchEventEmitter, ShConsts } from "..";
import type { FullNetContext, Initialised, TestNetConfig, TestNetContext, TestOptions } from "../types";
import { ShTestApi, type EnrichedShApi } from "../test-api";


/**
 * Implementation of the describeBspNet function.
 * @param title The title of the test suite.
 * @param args Additional arguments (either tests function or options and tests function).
 */
export async function describeMspNet<
  T extends [(context: FullNetContext) => void] | [TestOptions, (context: FullNetContext) => void]
>(title: string, ...args: T): Promise<void> {
  const options = args.length === 2 ? args[0] : {};
  const tests = args.length === 2 ? args[1] : args[0];

  const fullNetConfigCases = pickConfig(options);

  for (const fullNetConfig of fullNetConfigCases) {
    fullNetConfig.capacity = options.capacity;
    fullNetConfig.bspStartingWeight = options.bspStartingWeight;
    fullNetConfig.extrinsicRetryTimeout = options.extrinsicRetryTimeout;

    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`FullNet: ${title} (${fullNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      let userApiPromise: Promise<EnrichedShApi>;
      let bspApiPromise: Promise<EnrichedShApi>;
      let mspApiPromise: Promise<EnrichedShApi>;
      let responseListenerPromise: ReturnType<typeof launchFullNetwork>;

      before(async () => {
        // Create a promise which captures a response from the launchNetwork function
        responseListenerPromise = new Promise((resolve) => {
          launchEventEmitter.once("networkLaunched", resolve);
        });
        // Launch the network
        const launchResponse = await launchFullNetwork(
          {
            ...fullNetConfig,
            toxics: options?.toxics
          },
          options?.initialised
        );
        launchEventEmitter.emit("networkLaunched", launchResponse);

        userApiPromise = ShTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
        bspApiPromise = ShTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);
        mspApiPromise = ShTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.msp.port}`);
      });

      after(async () => {
        await cleardownTest({
          api: [await userApiPromise, await bspApiPromise, await mspApiPromise],
          keepNetworkAlive: options?.keepAlive
        });
        if (options?.keepAlive) {
          if (fullNetConfigCases.length > 1) {
            console.error(
              `test run configured for multiple bspNetConfigs, only ${JSON.stringify(
                fullNetConfig
              )} will be kept alive`
            );
          }
          console.log("🩺 Info:  Test run configured to keep FullNet alive");
          console.log("ℹ️ Hint: close network with:   pnpm docker:stop:fullnet  ");
          process.exit(0);
        }
      });

      const context = {
        it,
        createUserApi: () => userApiPromise,
        createBspApi: () => bspApiPromise,
        createMspApi: () => mspApiPromise,
        createApi: (endpoint) => ShTestApi.create(endpoint),
        bspNetConfig: fullNetConfig,
        before,
        after,
        afterEach,
        beforeEach,
        getLaunchResponse: () => responseListenerPromise
      } satisfies FullNetContext;

      tests(context);
    });
  }
}

export const launchFullNetwork = async (
  config: TestNetConfig,
  initialised: boolean | "multi" = false
): Promise<Initialised | undefined> => {
  if (initialised === "multi") {
    throw new Error("multi initialisation not supported for fullNet");
  }

  if (initialised) {
    return await runInitialisedFullNet(config);
  }

  await runSimpleFullNet(config);
  return undefined;
};

  const pickConfig = (options: TestOptions) => {
    return options.networkConfig === "all"
      ? [
          // "ALL" network config
          { noisy: false, rocksdb: false },
          { noisy: false, rocksdb: true }
        ]
      : options.networkConfig === "standard"
        ? [
            // "STANDARD" network config
            { noisy: false, rocksdb: false }
          ]
        : options.networkConfig === "noisy"
          ? [{ noisy: true, rocksdb: false }]
          : typeof options.networkConfig === "object"
            ? options.networkConfig
            : // default config is same as "ALL"
              [
                { noisy: false, rocksdb: false },
                { noisy: false, rocksdb: true }
              ];
  };
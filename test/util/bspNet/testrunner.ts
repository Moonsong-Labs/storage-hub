import { EventEmitter } from "node:events";
import { after, before, describe, it, afterEach, beforeEach } from "node:test";
import { cleardownTest, verifyContainerFreshness } from "./helpers";
import { BspNetTestApi, type EnrichedBspApi } from "./test-api";
import type {
  BspNetConfig,
  BspNetContext,
  FullNetContext,
  Initialised,
  TestOptions
} from "./types";
import * as ShConsts from "./consts";
import { NetworkLauncher } from "../netLaunch";

export const launchEventEmitter = new EventEmitter();

/**
 * Describes a set of BspNet tests.
 * @param title The title of the test suite.
 * @param tests A function that defines the tests using the provided context.
 */
export function describeBspNet(
  title: string,
  tests: (context: BspNetContext) => void
): Promise<void>;

/**
 * Describes a set of BspNet tests with additional options.
 * @param title The title of the test suite.
 * @param options Configuration options for the test suite.
 * @param tests A function that defines the tests using the provided context.
 */
export function describeBspNet(
  title: string,
  options: TestOptions,
  tests: (context: BspNetContext) => void
): Promise<void>;

/**
 * Implementation of the describeBspNet function.
 * @param title The title of the test suite.
 * @param args Additional arguments (either tests function or options and tests function).
 */
export async function describeBspNet<
  T extends [(context: BspNetContext) => void] | [TestOptions, (context: BspNetContext) => void]
>(title: string, ...args: T): Promise<void> {
  await verifyContainerFreshness();

  const options = args.length === 2 ? args[0] : {};
  const tests = args.length === 2 ? args[1] : args[0];

  const bspNetConfigCases = pickConfig(options);

  for (const bspNetConfig of bspNetConfigCases) {
    bspNetConfig.capacity = options.capacity;
    bspNetConfig.bspStartingWeight = options.bspStartingWeight;
    bspNetConfig.extrinsicRetryTimeout = options.extrinsicRetryTimeout;

    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`BSPNet: ${title} (${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      let userApiPromise: Promise<EnrichedBspApi>;
      let bspApiPromise: Promise<EnrichedBspApi>;
      let responseListenerPromise: ReturnType<typeof launchNetwork>;

      before(async () => {
        // Create a promise which captures a response from the launchNetwork function
        responseListenerPromise = new Promise((resolve) => {
          launchEventEmitter.once("networkLaunched", resolve);
        });
        // Launch the network
        const launchResponse = await NetworkLauncher.create("bspnet", {
          ...bspNetConfig,
          toxics: options?.toxics,
          initialised: options?.initialised
        });
        launchEventEmitter.emit("networkLaunched", launchResponse);

        userApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
        bspApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);
      });

      after(async () => {
        console.log("🧹 Cleaning up test resources");
        await cleardownTest({
          api: [await userApiPromise, await bspApiPromise],
          keepNetworkAlive: options?.keepAlive
        });

        if (options?.keepAlive) {
          if (bspNetConfigCases.length > 1) {
            console.error(
              `test run configured for multiple bspNetConfigs, only ${JSON.stringify(
                bspNetConfig
              )} will be kept alive`
            );
          }
          console.log("🩺 Info:  Test run configured to keep BSPNet alive");
          console.log("ℹ️ Hint: close network with:   pnpm docker:stop:bspnet  ");
          process.exit(0);
        }
      });

      const context = {
        it,
        createUserApi: () => userApiPromise,
        createBspApi: () => bspApiPromise,
        createApi: (endpoint) => BspNetTestApi.create(endpoint),
        bspNetConfig,
        before,
        after,
        afterEach,
        beforeEach,
        getLaunchResponse: () => responseListenerPromise
      } satisfies BspNetContext;

      tests(context);
    });
  }
}

/**
 * Implementation of the describeMspNet function.
 * @param title The title of the test suite.
 * @param args Additional arguments (either tests function or options and tests function).
 */
export async function describeMspNet<
  T extends [(context: FullNetContext) => void] | [TestOptions, (context: FullNetContext) => void]
>(title: string, ...args: T): Promise<void> {
  await verifyContainerFreshness();

  const options = args.length === 2 ? args[0] : {};
  const tests = args.length === 2 ? args[1] : args[0];

  const fullNetConfigCases = pickConfig(options);

  for (const fullNetConfig of fullNetConfigCases) {
    fullNetConfig.capacity = options.capacity;
    fullNetConfig.bspStartingWeight = options.bspStartingWeight;
    fullNetConfig.extrinsicRetryTimeout = options.extrinsicRetryTimeout;

    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`FullNet: ${title} (${fullNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      let userApiPromise: Promise<EnrichedBspApi>;
      let bspApiPromise: Promise<EnrichedBspApi>;
      let mspApiPromise: Promise<EnrichedBspApi>;
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

        userApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
        bspApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);
        mspApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.msp1.port}`);
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
        createApi: (endpoint) => BspNetTestApi.create(endpoint),
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

export const launchNetwork = async (
  config: BspNetConfig,
  initialised: boolean | "multi" = false
) => {
  return initialised === "multi"
    ? await NetworkLauncher.create("bspnet", { ...config, initialised: "multi" })
    : initialised === true
      ? await NetworkLauncher.create("bspnet", { ...config, initialised: true })
      : await NetworkLauncher.create("bspnet", config);
};

export const launchFullNetwork = async (
  config: BspNetConfig,
  initialised: boolean | "multi" = false
): Promise<Initialised | undefined> => {
  if (initialised === "multi") {
    throw new Error("multi initialisation not supported for fullNet");
  }

  if (initialised) {
    // return await runInitialisedFullNet(config);
    return (await NetworkLauncher.create("fullnet", { ...config, initialised: true })) as any;
  }

  // await runSimpleFullNet(config);
  await NetworkLauncher.create("fullnet", config);
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

import { EventEmitter } from "node:events";
import { after, afterEach, before, beforeEach, describe, it } from "node:test";
import { createSqlClient, verifyContainerFreshness } from "..";
import { NetworkLauncher } from "../netLaunch";
import {
  type DynamicNetworkContext,
  launchNetworkFromTopology,
  type NetworkTopology,
  ConsoleProgressReporter
} from "../netLaunch";
import * as ShConsts from "./consts";
import { cleardownTest } from "./helpers";
import { BspNetTestApi, type EnrichedBspApi } from "./test-api";
import type { BspNetContext, FullNetContext, TestOptions } from "./types";

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
  const options = args.length === 2 ? args[0] : {};
  const tests = args.length === 2 ? args[1] : args[0];

  const bspNetConfigCases = pickConfig(options);

  for (const bspNetConfig of bspNetConfigCases) {
    bspNetConfig.capacity = options.capacity;
    bspNetConfig.bspStartingWeight = options.bspStartingWeight;
    bspNetConfig.extrinsicRetryTimeout = options.extrinsicRetryTimeout;
    bspNetConfig.logLevel = options.logLevel;

    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`BSPNet: ${title} (${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      let userApiPromise: Promise<EnrichedBspApi>;
      let bspApiPromise: Promise<EnrichedBspApi>;
      let responseListenerPromise: ReturnType<typeof NetworkLauncher.create>;

      before(async () => {
        await verifyContainerFreshness();

        responseListenerPromise = new Promise((resolve) => {
          launchEventEmitter.once("networkLaunched", resolve);
        });

        const launchResponse = await NetworkLauncher.create("fullnet", {
          ...bspNetConfig,
          toxics: options?.toxics,
          initialised: options?.initialised,
          big_file: options?.big_file,
          runtimeType: options?.runtimeType
        });
        launchEventEmitter.emit("networkLaunched", launchResponse);

        userApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
        bspApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);
      });

      after(async () => {
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
 *
 * TODO: Add a new docker container service in compose to run a standalone indexer node (right now the user node runs the indexer)
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
    fullNetConfig.indexer = options.indexer;
    fullNetConfig.indexerMode = options.indexerMode;
    fullNetConfig.standaloneIndexer = options.standaloneIndexer;
    fullNetConfig.fisherman = options.fisherman;
    fullNetConfig.backend = options.backend;
    fullNetConfig.fishermanIncompleteSyncMax = options.fishermanIncompleteSyncMax;
    fullNetConfig.fishermanIncompleteSyncPageSize = options.fishermanIncompleteSyncPageSize;
    fullNetConfig.logLevel = options.logLevel;

    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`FullNet: ${title} (${fullNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      let userApiPromise: Promise<EnrichedBspApi>;
      let bspApiPromise: Promise<EnrichedBspApi>;
      let msp1ApiPromise: Promise<EnrichedBspApi>;
      let msp2ApiPromise: Promise<EnrichedBspApi>;
      let fishermanApiPromise: Promise<EnrichedBspApi> | undefined;
      let indexerApiPromise: Promise<EnrichedBspApi> | undefined;
      let responseListenerPromise: ReturnType<typeof NetworkLauncher.create>;

      before(async () => {
        await verifyContainerFreshness();

        responseListenerPromise = new Promise((resolve) => {
          launchEventEmitter.once("networkLaunched", resolve);
        });
        const launchResponse = await NetworkLauncher.create("fullnet", {
          ...fullNetConfig,
          toxics: options?.toxics,
          initialised: options?.initialised,
          big_file: options?.big_file,
          runtimeType: options?.runtimeType,
          pendingTxDb: options?.pendingTxDb
        });
        launchEventEmitter.emit("networkLaunched", launchResponse);

        userApiPromise = BspNetTestApi.create(
          `ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`,
          options?.runtimeType
        );
        bspApiPromise = BspNetTestApi.create(
          `ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`,
          options?.runtimeType
        );
        msp1ApiPromise = BspNetTestApi.create(
          `ws://127.0.0.1:${ShConsts.NODE_INFOS.msp1.port}`,
          options?.runtimeType
        );
        msp2ApiPromise = BspNetTestApi.create(
          `ws://127.0.0.1:${ShConsts.NODE_INFOS.msp2.port}`,
          options?.runtimeType
        );

        // Create fisherman API if fisherman is enabled
        if (fullNetConfig.fisherman) {
          fishermanApiPromise = BspNetTestApi.create(
            `ws://127.0.0.1:${ShConsts.NODE_INFOS.fisherman.port}`,
            options?.runtimeType
          );

          // Ensure fisherman node is ready and synced
          const userApi = await userApiPromise;
          const fishermanApi = await fishermanApiPromise;
          await userApi.wait.nodeCatchUpToChainTip(fishermanApi);
        }

        // Create indexer API if standalone indexer is enabled
        if (fullNetConfig.standaloneIndexer && fullNetConfig.indexer) {
          indexerApiPromise = BspNetTestApi.create(
            `ws://127.0.0.1:${ShConsts.NODE_INFOS.indexer.port}`,
            options?.runtimeType
          );

          // Ensure indexer node is ready and synced
          const userApi = await userApiPromise;
          const indexerApi = await indexerApiPromise;
          await userApi.wait.nodeCatchUpToChainTip(indexerApi);
        }
      });

      after(async () => {
        const apis = [
          await userApiPromise,
          await bspApiPromise,
          await msp1ApiPromise,
          await msp2ApiPromise
        ];

        if (fishermanApiPromise) {
          apis.push(await fishermanApiPromise);
        }

        if (indexerApiPromise) {
          apis.push(await indexerApiPromise);
        }

        await cleardownTest({
          api: apis,
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
        createMsp1Api: () => msp1ApiPromise,
        createMsp2Api: () => msp2ApiPromise,
        createFishermanApi: fullNetConfig.fisherman
          ? () => fishermanApiPromise as Promise<EnrichedBspApi>
          : undefined,
        createIndexerApi:
          fullNetConfig.standaloneIndexer && fullNetConfig.indexer
            ? () => indexerApiPromise as Promise<EnrichedBspApi>
            : undefined,
        createApi: (endpoint) => BspNetTestApi.create(endpoint, options?.runtimeType),
        createSqlClient: () => createSqlClient(),
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

/**
 * Dynamic network context for topology-based tests.
 */
export interface DynamicNetworkTestContext {
  it: typeof it;
  network: DynamicNetworkContext;
  before: typeof before;
  after: typeof after;
  beforeEach: typeof beforeEach;
  afterEach: typeof afterEach;
}

/**
 * Describes a test suite using dynamic network topology.
 *
 * Creates test networks with arbitrary numbers of BSPs, MSPs, and fishermen.
 * See topology.ts for infrastructure architecture.
 *
 * @example
 * ```ts
 * describeNetwork(
 *   "100 BSP scale test",
 *   { bsps: 100, msps: 2, fishermen: 1 },
 *   { timeout: 300000 },
 *   (ctx) => {
 *     ctx.it("all BSPs can volunteer", async () => {
 *       const results = await ctx.network.mapBsps(async (api, index) => {
 *         return api.sealBlock(api.tx.fileSystem.bspVolunteer(...));
 *       });
 *       assert.equal(results.length, 100);
 *     });
 *   }
 * );
 * ```
 */
export async function describeNetwork(
  title: string,
  topology: NetworkTopology,
  options: TestOptions,
  tests: (context: DynamicNetworkTestContext) => void
): Promise<void> {
  const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

  describeFunc(`DynamicNetwork: ${title}`, { timeout: options?.timeout }, () => {
    let network: DynamicNetworkContext | undefined;

    before(async () => {
      await verifyContainerFreshness();

      const verbose = process.env.SH_TEST_VERBOSE === "1";
      const bspCount = typeof topology.bsps === "number" ? topology.bsps : topology.bsps.length;
      const mspCount = typeof topology.msps === "number" ? topology.msps : topology.msps.length;
      const fishCount =
        typeof topology.fishermen === "number" ? topology.fishermen : topology.fishermen.length;

      console.log(`\n=== 🧪 DynamicNetwork: ${title} (${bspCount} BSPs, ${mspCount} MSPs) ===`);

      if (verbose) {
        const configTable = [
          { option: "bsps", value: bspCount },
          { option: "msps", value: mspCount },
          { option: "fishermen", value: fishCount },
          {
            option: "runtimeType",
            value: options?.runtimeType ?? "parachain"
          }
        ];
        console.table(configTable);
      }

      network = await launchNetworkFromTopology(topology, {
        runtimeType: options?.runtimeType ?? "parachain",
        progressReporter: verbose ? new ConsoleProgressReporter() : undefined
      });
    });

    after(async () => {
      if (network && !options?.keepAlive) {
        await network.cleanup();
      } else if (options?.keepAlive) {
        process.exit(0);
      }
    });

    const context: DynamicNetworkTestContext = {
      it,
      get network() {
        if (!network) {
          throw new Error("Network not initialized - before() hook may have failed");
        }
        return network;
      },
      before,
      after,
      beforeEach,
      afterEach
    };

    tests(context);
  });
}

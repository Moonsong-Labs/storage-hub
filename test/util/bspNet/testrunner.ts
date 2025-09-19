import { EventEmitter } from "node:events";
import { after, afterEach, before, beforeEach, describe, it } from "node:test";
import { createSqlClient, verifyContainerFreshness, closeAllSqlClients } from "..";
import { NetworkLauncher } from "../netLaunch";
import * as ShConsts from "./consts";
import { cleardownTest } from "./helpers";
import { BspNetTestApi, type EnrichedBspApi } from "./test-api";
import type { BspNetContext, FullNetContext, TestOptions } from "./types";

export const launchEventEmitter = new EventEmitter();

// Track test execution count for debugging CI issues
let testExecutionCount = 0;

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
          runtimeType: options?.runtimeType
        });
        launchEventEmitter.emit("networkLaunched", launchResponse);

        userApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
        bspApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);
      });

      after(async () => {
        console.log("[TEST] Starting BSPNet test cleanup...");

        // First close all SQL clients to free database connections
        console.log("[TEST] Closing SQL clients...");
        await closeAllSqlClients();

        const apis: EnrichedBspApi[] = [];

        // Only try to resolve APIs if they were actually created
        try {
          if (userApiPromise) {
            const api = await userApiPromise;
            if (api) apis.push(api);
          }
          if (bspApiPromise) {
            const api = await bspApiPromise;
            if (api) apis.push(api);
          }
        } catch (_e) {
          console.log("[TEST] Some APIs were not initialized, skipping their cleanup");
        }

        if (apis.length > 0) {
          console.log(`[TEST] Disconnecting ${apis.length} API(s) and cleaning up containers...`);
          await cleardownTest({
            api: apis,
            keepNetworkAlive: options?.keepAlive
          });
        } else {
          console.log("[TEST] No APIs to disconnect, cleaning up containers only...");
          await cleardownTest({
            api: [],
            keepNetworkAlive: options?.keepAlive
          });
        }

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

        console.log("[TEST] BSPNet test cleanup complete");
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
    fullNetConfig.fisherman = options.fisherman;
    fullNetConfig.backend = options.backend;

    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`FullNet: ${title} (${fullNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      let userApiPromise: Promise<EnrichedBspApi>;
      let bspApiPromise: Promise<EnrichedBspApi>;
      let msp1ApiPromise: Promise<EnrichedBspApi>;
      let msp2ApiPromise: Promise<EnrichedBspApi>;
      let fishermanApiPromise: Promise<EnrichedBspApi> | undefined;
      let responseListenerPromise: ReturnType<typeof NetworkLauncher.create>;

      before(async () => {
        testExecutionCount++;
        console.log(`[TEST-TRACKER] Starting test execution #${testExecutionCount}`);
        console.log(`[TEST-TRACKER] Test suite: ${title}`);
        console.log(
          `[TEST-TRACKER] Config: ${JSON.stringify({
            fisherman: fullNetConfig.fisherman,
            indexer: fullNetConfig.indexer,
            indexerMode: fullNetConfig.indexerMode,
            rocksdb: fullNetConfig.rocksdb
          })}`
        );

        await verifyContainerFreshness();

        responseListenerPromise = new Promise((resolve) => {
          launchEventEmitter.once("networkLaunched", resolve);
        });
        const launchResponse = await NetworkLauncher.create("fullnet", {
          ...fullNetConfig,
          toxics: options?.toxics,
          initialised: options?.initialised,
          runtimeType: options?.runtimeType
        });
        launchEventEmitter.emit("networkLaunched", launchResponse);

        console.log("[TEST-SETUP] Creating API connections for test suite...");
        console.log(
          `[TEST-SETUP] Test configuration: indexer=${fullNetConfig.indexer}, fisherman=${fullNetConfig.fisherman}, indexerMode=${fullNetConfig.indexerMode}`
        );

        console.log("[TEST-SETUP] Creating user API connection...");
        userApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

        console.log("[TEST-SETUP] Creating BSP API connection...");
        bspApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);

        console.log("[TEST-SETUP] Creating MSP-1 API connection...");
        msp1ApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.msp1.port}`);

        console.log("[TEST-SETUP] Creating MSP-2 API connection...");
        msp2ApiPromise = BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.msp2.port}`);

        // Create fisherman API if fisherman is enabled
        if (fullNetConfig.fisherman) {
          console.log("[TEST-SETUP] Creating Fisherman API connection...");
          fishermanApiPromise = BspNetTestApi.create(
            `ws://127.0.0.1:${ShConsts.NODE_INFOS.fisherman.port}`
          );
        }

        console.log(
          "[TEST-SETUP] All API connections initiated, waiting for connections to establish..."
        );
      });

      after(async () => {
        console.log("[TEST] Starting FullNet test cleanup...");

        // First close all SQL clients to free database connections
        console.log("[TEST] Closing SQL clients...");
        await closeAllSqlClients();

        // Add a small delay for CI to allow connections to close properly
        if (process.env.CI === "true" && fullNetConfig.fisherman) {
          console.log(
            "[TEST-CLEANUP] CI environment detected with fisherman - adding cleanup delay..."
          );
          console.log("[TEST-CLEANUP] This helps prevent resource exhaustion in subsequent tests");
          await new Promise((resolve) => setTimeout(resolve, 2000));
        }

        const apis: EnrichedBspApi[] = [];

        // Only try to resolve APIs if they were actually created
        console.log("[TEST-CLEANUP] Collecting API connections for cleanup...");
        try {
          if (userApiPromise) {
            console.log("[TEST-CLEANUP] Resolving user API...");
            const api = await userApiPromise;
            if (api) apis.push(api);
          }
          if (bspApiPromise) {
            console.log("[TEST-CLEANUP] Resolving BSP API...");
            const api = await bspApiPromise;
            if (api) apis.push(api);
          }
          if (msp1ApiPromise) {
            console.log("[TEST-CLEANUP] Resolving MSP-1 API...");
            const api = await msp1ApiPromise;
            if (api) apis.push(api);
          }
          if (msp2ApiPromise) {
            console.log("[TEST-CLEANUP] Resolving MSP-2 API...");
            const api = await msp2ApiPromise;
            if (api) apis.push(api);
          }
          if (fishermanApiPromise) {
            console.log("[TEST-CLEANUP] Resolving Fisherman API...");
            const api = await fishermanApiPromise;
            if (api) apis.push(api);
          }
          console.log(`[TEST-CLEANUP] Collected ${apis.length} API connection(s) for cleanup`);
        } catch (_e) {
          console.log("[TEST-CLEANUP] Some APIs were not initialized, skipping their cleanup");
        }

        if (apis.length > 0) {
          console.log(`[TEST] Disconnecting ${apis.length} API(s) and cleaning up containers...`);
          await cleardownTest({
            api: apis,
            keepNetworkAlive: options?.keepAlive
          });
        } else {
          console.log("[TEST] No APIs to disconnect, cleaning up containers only...");
          await cleardownTest({
            api: [],
            keepNetworkAlive: options?.keepAlive
          });
        }

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

        console.log("[TEST] FullNet test cleanup complete");
        console.log(`[TEST-TRACKER] Completed test execution #${testExecutionCount}`);
        console.log("[TEST-TRACKER] ==========================================\n");
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
        createApi: (endpoint) => BspNetTestApi.create(endpoint),
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

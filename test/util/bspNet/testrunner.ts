import { after, before, describe, it } from "node:test";
import { createApiObject } from "./api";
import { NODE_INFOS } from "./consts";
import {
  cleardownTest,
  runInitialisedBspsNet,
  runMultipleInitialisedBspsNet,
  runSimpleBspNet
} from "./helpers";
import type { BspNetApi, BspNetConfig, BspNetContext, TestOptions } from "./types";
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

  const bspNetConfigCases: BspNetConfig[] =
    options.networkConfig === "all"
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

  for (const bspNetConfig of bspNetConfigCases) {
    const describeFunc = options?.only ? describe.only : options?.skip ? describe.skip : describe;

    describeFunc(`BSPNet: ${title} (${bspNetConfig.rocksdb ? "RocksDB" : "MemoryDB"})`, () => {
      // TODO - maybe make the below awaited and always connected?
      let userApiPromise: Promise<BspNetApi>;
      let bspApiPromise: Promise<BspNetApi>;
      let launchResponse: Awaited<
        | ReturnType<typeof runSimpleBspNet>
        | ReturnType<typeof runInitialisedBspsNet>
        | ReturnType<typeof runMultipleInitialisedBspsNet>
      >;

      before(async () => {
        // Launch the network
        launchResponse =
          options?.initialised === "multi"
            ? await runMultipleInitialisedBspsNet(bspNetConfig)
            : options?.initialised === true
              ? await runInitialisedBspsNet(bspNetConfig)
              : await runSimpleBspNet(bspNetConfig);

        // Create API Connectors
        userApiPromise = createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
        bspApiPromise = createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
      });

      after(async () => {
        await cleardownTest({ api: await userApiPromise, keepNetworkAlive: options?.keepAlive });
        await (await bspApiPromise).disconnect();
        if (options?.keepAlive) {
          if (bspNetConfigCases.length > 1) {
            console.error(
              `test run configured for multiple bspNetConfigs, only ${JSON.stringify(bspNetConfig)} will be kept alive`
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
        bspNetConfig,
        before,
        after,
        launchResponse
      } satisfies BspNetContext;

      tests(context);
    });
  }
}

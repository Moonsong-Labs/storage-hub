import { after, afterEach, before, beforeEach, describe, it } from "node:test";
import { verifyContainerFreshness } from "../..";
import {
  type DynamicNetworkContext,
  launchNetworkFromTopology,
  type NetworkTopology,
  ConsoleProgressReporter
} from ".";
import type { TestOptions } from "../../bspNet/types";

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

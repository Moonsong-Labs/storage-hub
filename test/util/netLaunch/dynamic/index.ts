/**
 * Dynamic network topology-based launcher.
 *
 * This module provides all the components needed to launch large-scale test networks
 * with arbitrary numbers of BSPs, MSPs, fishermen, and users.
 *
 * Main exports:
 * - `launchNetworkFromTopology` - Launch a network from a topology specification
 * - `DynamicNetworkContext` - Context for interacting with launched networks
 * - `NetworkTopology` - Topology type definitions
 *
 * @example
 * ```ts
 * import { describeNetwork } from "./testrunner";
 *
 * describeNetwork(
 *   "10 BSP scale test",
 *   { bsps: 10, msps: 2, fishermen: 1 },
 *   { timeout: 300000 },
 *   (ctx) => {
 *     ctx.it("all BSPs are connected", async () => {
 *       // ...
 *     });
 *   }
 * );
 * ```
 */

// Core topology types
export * from "./topology";

// Dynamic network launcher and context
export * from "./dynamicLauncher";

// Connection pool for lazy API connections
export * from "./connectionPool";

// Progress reporting
export * from "./progressReporter";

// Key generation (re-export types that may be needed)
export {
  generateNodeIdentity,
  injectKeys,
  fetchProviderId
} from "./keyGenerator";
export type { GeneratedIdentity, HexString } from "./keyGenerator";

// Port allocation (re-export for potential customization)
export { PortAllocator } from "./portAllocator";
export type { Ports, PortAllocatorConfig } from "./portAllocator";

// Test runner for dynamic network tests
export * from "./testrunner";

// Service generation types (useful for advanced customization)
export type {
  DockerService,
  NodeIdentityInfo,
  NodeIdentities,
  BootnodeInfo,
  ServiceGeneratorConfig
} from "./serviceGenerator";
export { generateComposeServices } from "./serviceGenerator";

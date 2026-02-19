# Dynamic Integration Test Network

This module provides a topology-driven framework for launching ephemeral Docker networks in integration tests. You declare _what_ your network should look like; the framework handles container orchestration, key injection, provider registration, and teardown.

It replaces the need to hand-edit Docker Compose templates for one-off test configurations and makes it straightforward to test scenarios that require large or unusual numbers of nodes.

## Quick Start

```ts
import { describeNetwork } from "../../util/netLaunch/dynamic/testrunner";

await describeNetwork(
  "my feature test",
  { bsps: 3, msps: 1, fishermen: 0 },
  { timeout: 120000 },
  (ctx) => {
    ctx.it("BSPs can volunteer", async () => {
      const api = await ctx.network.getBlockProducerApi();
      // ... seal blocks, query state, assert events
    });
  }
);
```

## Declaring a Topology

```ts
interface NetworkTopology {
  bsps:       number | NodeConfig[];
  msps:       number | NodeConfig[];
  fishermen:  number | NodeConfig[];
  users?:     number | NodeConfig[];
  collators?: number;
}
```

The simplest form uses counts:

```ts
{ bsps: 10, msps: 2, fishermen: 1 }
```

To configure individual nodes, pass an array of `NodeConfig` objects:

```ts
interface NodeConfig {
  capacity?:       bigint;
  rocksdb?:        boolean;
  additionalArgs?: string[];
}
```

Mixed example — two BSPs with different capacities:

```ts
{
  bsps: [
    { capacity: 1n * 1024n ** 3n },
    { capacity: 512n * 1024n ** 2n }
  ],
  msps: 1,
  fishermen: 0
}
```

### Constraints

- At least 1 BSP, 1 MSP, and 1 User are required.
- Fishermen may be 0.
- Validation throws at test start if the topology is invalid.

## Infrastructure Architecture

The framework enforces a fixed infrastructure layout per node type:

| Node type  | Containers spawned                                  |
|------------|-----------------------------------------------------|
| BSP        | 1 — the BSP node itself                             |
| MSP        | 3 — Postgres → Indexer → MSP node                  |
| Fisherman  | 3 — Postgres → Indexer → Fisherman node             |
| User       | 1 — the user node                                   |

BSPs and Users never have a database or indexer. MSPs and Fishermen always have a dedicated Postgres + Indexer pair each.

**BSP-0 is the bootnode.** All other containers specify `--bootnodes` pointing at BSP-0's P2P address, consistent with the static bspnet/fullnet setup.

**user-0 is the conventional block producer.** Chain setup (funding, runtime params, provider registration) runs through user-0 after it starts, consistent with bspnet/fullnet tests where `sh-user` is used for `block.seal()`. Use `ctx.network.getBlockProducerApi()` (which returns user-0) for all `block.seal()` calls.

## Bootstrap Sequence

`launchNetworkFromTopology` starts containers in a specific order to satisfy dependency constraints:

1. **BSP-0** — becomes the bootnode; waits for `💤 Idle` log.
2. **Postgres** for all MSPs and Fishermen — waits for `database system is ready to accept connections`.
3. **Indexers** for all MSPs and Fishermen — waits for `💤 Idle`.
4. **BSP-1..N** — each gets keys injected and is registered on-chain via `forceBspSignUp`.
5. **MSP containers** — keys injected; registration happens after chain setup.
6. **User containers** — keys injected.
7. **Chain setup** via BSP-0: `preFundAccounts`, `setupRuntimeParams`, MSP registration (`forceMspSignUp`), provider ID fetch.
8. **Fisherman containers** — started last after the chain is ready.

## DynamicNetworkContext API

The `ctx.network` object passed to your test function exposes:

### Getting API connections

```ts
ctx.network.getBspApi(index: number): Promise<EnrichedBspApi>
ctx.network.getMspApi(index: number): Promise<EnrichedBspApi>
ctx.network.getFishermanApi(index: number): Promise<EnrichedBspApi>
ctx.network.getUserApi(index: number): Promise<EnrichedBspApi>
ctx.network.getBlockProducerApi(): Promise<EnrichedBspApi>
```

Connections are created lazily on first access and pooled with LRU eviction (max 50 open connections by default).

### Iterating BSPs

```ts
// Sequential, for side-effectful operations
await ctx.network.forEachBsp(async (api, index) => { ... });

// Collect results
const results = await ctx.network.mapBsps(async (api, index) => { ... });
```

### Node counts

```ts
ctx.network.bspCount
ctx.network.mspCount
ctx.network.fishermanCount
ctx.network.userCount
```

### Identities and provider IDs

```ts
ctx.network.getBspIdentity(index): NodeIdentityInfo
ctx.network.getMspIdentity(index): NodeIdentityInfo
ctx.network.getFishermanIdentity(index): NodeIdentityInfo
ctx.network.getUserIdentity(index): NodeIdentityInfo

ctx.network.getBspProviderId(index): HexString
ctx.network.getMspProviderId(index): HexString
```

The `identity.keyring` field is a `KeyringPair` for signing transactions:

```ts
const userIdentity = ctx.network.getUserIdentity(0);
await api.block.seal({
  calls: [...],
  signer: userIdentity.identity.keyring
});
```

## TestOptions

```ts
interface TestOptions {
  timeout?:     number;
  only?:        boolean;
  skip?:        boolean;
  keepAlive?:   boolean;
  runtimeType?: "parachain" | "solochain";
}
```

## Environment Variables

| Variable           | Effect                                                      |
|--------------------|-------------------------------------------------------------|
| `SH_TEST_VERBOSE=1`| Enables `ConsoleProgressReporter` (phase timings, per-node readiness) and prints a config table at startup. |

## Concrete Example

From `test/suites/integration/benchmark/msp-distribute-file-multi-bsp.test.ts`:

```ts
const BSP_COUNT = 10;

await describeNetwork(
  `MSP distributes files to ${BSP_COUNT} BSPs`,
  { bsps: BSP_COUNT, msps: 1, fishermen: 0 },
  { timeout: 600000 },
  (ctx) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let blockProducerApi: EnrichedBspApi;

    ctx.before(async () => {
      userApi = await ctx.network.getUserApi(0);
      mspApi = await ctx.network.getMspApi(0);
      blockProducerApi = await ctx.network.getBlockProducerApi();
    });

    ctx.it("all BSPs receive the file", async () => {
      const mspProviderId = ctx.network.getMspProviderId(0);
      const userIdentity = ctx.network.getUserIdentity(0);

      // Create bucket, issue storage request, wait for MSP acceptance...
      // Then verify every BSP stored the file:
      for (let i = 0; i < BSP_COUNT; i++) {
        const bspApi = await ctx.network.getBspApi(i);
        await bspApi.wait.fileStorageComplete(fileKey);
      }
    });
  }
);
```

## Module Map

| File                  | Responsibility                                                                      |
|-----------------------|-------------------------------------------------------------------------------------|
| `testrunner.ts`       | `describeNetwork` — developer entry point; wraps `describe` with lifecycle hooks.   |
| `topology.ts`         | `NetworkTopology`, `NodeConfig` types; `normalizeTopology`, `validateTopology`.     |
| `dynamicLauncher.ts`  | `launchNetworkFromTopology`; `DynamicNetworkContext` class.                         |
| `serviceGenerator.ts` | Generates Docker Compose service definitions for all node types.                    |
| `connectionPool.ts`   | `LazyConnectionPool` — on-demand WS connections with LRU eviction.                 |
| `keyGenerator.ts`     | `generateNodeIdentity`, `injectKeys`, `fetchProviderId`.                            |
| `portAllocator.ts`    | Sequential port allocation across all node types to avoid collisions.               |
| `progressReporter.ts` | `ConsoleProgressReporter` / `SilentProgressReporter` interfaces.                   |
| `index.ts`            | Re-exports everything from all sub-modules.                                         |

/**
 * Dynamic network launcher for topology-based networks.
 *
 * Orchestrates the startup of large-scale test networks with:
 * - Sequential startup phases (collators → providers → monitors)
 * - Automatic key injection and provider registration
 * - Fail-fast error handling with internal retries
 * - Progress reporting for long-running operations
 *
 * Usage pattern (mirrors NetworkLauncher):
 *
 * ```ts
 * const network = await launchNetworkFromTopology({
 *   bsps: 5,
 *   msps: 2,
 *   fishermen: 1
 * });
 *
 * // Use methods on the network context
 * const api = await network.getBspApi(0);
 * await network.preFundAccounts(api);
 * await network.setupRuntimeParams(api);
 *
 * // Cleanup when done
 * await network.cleanup();
 * ```
 */

import fs from "node:fs";
import path from "node:path";
import * as compose from "docker-compose";
import tmp from "tmp";
import type { EnrichedBspApi } from "../../bspNet/test-api";
import { BspNetTestApi } from "../../bspNet/test-api";
import { LazyConnectionPool } from "./connectionPool";
import { fetchProviderId, type HexString } from "./keyGenerator";
import { SilentProgressReporter, PhaseTimer, type ProgressReporter } from "./progressReporter";
import {
  generateComposeServices,
  type BootnodeInfo,
  type NodeIdentities,
  type NodeIdentityInfo
} from "./serviceGenerator";
import { normalizeTopology, validateTopology, type NetworkTopology } from "./topology";
import { getContainerIp, getContainerPeerId } from "../../bspNet/helpers";
import { waitForLog } from "../../bspNet/docker";
import { sendCustomRpc } from "../../rpc";
import { sleep } from "../../timer";
import { CAPACITY_512 } from "../../bspNet/consts";
import { injectKeys } from "./keyGenerator";
import { cleanupEnvironment } from "../../helpers";
import { BaseNetworkContext } from "../baseContext";
import yaml from "yaml";

const WORLD_WRITABLE_MODE = 0o777;
const AURA_SEAL_RETRY_DELAY_MS = 6500;
const AURA_SEAL_MAX_RETRIES = 3;
const NODE_IDLE_TIMEOUT_MS = 30000;

function ensureWorldWritableDirectory(dirPath: string): void {
  fs.mkdirSync(dirPath, {
    recursive: true,
    mode: WORLD_WRITABLE_MODE
  });
  // mkdir mode is affected by process umask (e.g. 0022 => 0755), so chmod explicitly.
  fs.chmodSync(dirPath, WORLD_WRITABLE_MODE);
}

function stringifyError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function isAuraCreateBlockTrap(error: unknown): boolean {
  const message = stringifyError(error);
  return (
    message.includes("pallet_aura") ||
    message.includes("Slot must") ||
    message.includes("Execution aborted due to trap") ||
    message.includes("wasm `unreachable` instruction executed")
  );
}

async function waitForNodeTipSyncWithoutSealing(
  nodeSyncedApi: EnrichedBspApi,
  nodeBehindApi: EnrichedBspApi,
  timeoutMs = 60000,
  pollMs = 500
): Promise<void> {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    const [syncedHeader, behindHeader] = await Promise.all([
      nodeSyncedApi.rpc.chain.getHeader(),
      nodeBehindApi.rpc.chain.getHeader()
    ]);

    if (syncedHeader.hash.eq(behindHeader.hash)) {
      return;
    }

    await sleep(pollMs);
  }

  const [syncedHeader, behindHeader] = await Promise.all([
    nodeSyncedApi.rpc.chain.getHeader(),
    nodeBehindApi.rpc.chain.getHeader()
  ]);

  throw new Error(
    `Node did not passively catch up to chain tip within ${timeoutMs}ms ` +
      `(synced #${syncedHeader.number.toString()} ${syncedHeader.hash.toString()}, ` +
      `behind #${behindHeader.number.toString()} ${behindHeader.hash.toString()})`
  );
}

async function syncNodeTip(
  nodeSyncedApi: EnrichedBspApi,
  nodeBehindApi: EnrichedBspApi,
  timeoutMs = 120000
): Promise<void> {
  await waitForNodeTipSyncWithoutSealing(nodeSyncedApi, nodeBehindApi, timeoutMs);
}

async function runWithAuraRecovery<T>(
  initialApi: EnrichedBspApi,
  runtimeType: "parachain" | "solochain",
  rpcPort: number,
  operationName: string,
  operation: (api: EnrichedBspApi) => Promise<T>,
  maxRetries = AURA_SEAL_MAX_RETRIES
): Promise<{ api: EnrichedBspApi; result: T }> {
  let api = initialApi;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const result = await operation(api);
      return { api, result };
    } catch (error) {
      if (!isAuraCreateBlockTrap(error) || attempt === maxRetries) {
        throw error;
      }

      const compactReason = stringifyError(error).split("\n")[0];
      console.warn(
        `[DynamicLauncher] ${operationName} hit Aura trap on ws://127.0.0.1:${rpcPort} ` +
          `(attempt ${attempt}/${maxRetries}): ${compactReason}`
      );

      try {
        await api.disconnect();
      } catch {
        // Ignore disconnect errors when recovering from a failed seal attempt.
      }

      await sleep(AURA_SEAL_RETRY_DELAY_MS * attempt);
      api = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`, runtimeType);
    }
  }

  throw new Error(`Unreachable: failed to execute ${operationName} after recovery attempts`);
}

async function waitForNodeIdle(
  nodeType: NodeIdentityInfo["nodeType"],
  index: number,
  timeout = NODE_IDLE_TIMEOUT_MS
): Promise<void> {
  await waitForLog({
    containerName: `storage-hub-sh-${nodeType}-${index}`,
    searchString: "💤 Idle",
    timeout
  });
}

async function registerProvidersWithSetupProducer(
  producerApi: EnrichedBspApi,
  providerNodes: NodeIdentityInfo[],
  runtimeType: "parachain" | "solochain",
  producerRpcPort: number
): Promise<EnrichedBspApi> {
  let currentProducerApi = producerApi;

  for (const nodeInfo of providerNodes) {
    ({ api: currentProducerApi } = await runWithAuraRecovery(
      currentProducerApi,
      runtimeType,
      producerRpcPort,
      `register-${nodeInfo.nodeType}-${nodeInfo.index}`,
      (api) => registerProvider(api, nodeInfo)
    ));

    const nodeApi = await BspNetTestApi.create(`ws://127.0.0.1:${nodeInfo.ports.rpc}`, runtimeType);
    try {
      await syncNodeTip(currentProducerApi, nodeApi);
      await fetchProviderId(nodeApi, nodeInfo.identity);
    } finally {
      await nodeApi.disconnect();
    }
  }

  return currentProducerApi;
}

export interface DynamicNetworkConfig {
  runtimeType?: "parachain" | "solochain";
  image?: string;
  progressReporter?: ProgressReporter;
}

export class DynamicNetworkContext extends BaseNetworkContext {
  private _runtimeType: "parachain" | "solochain";

  constructor(
    private connectionPool: LazyConnectionPool,
    private identities: NodeIdentities,
    private composeFile: string,
    runtimeType: "parachain" | "solochain" = "parachain",
    private keystoreTempDir?: tmp.DirResult
  ) {
    super();
    this._runtimeType = runtimeType;
  }

  get runtimeType(): "parachain" | "solochain" {
    return this._runtimeType;
  }

  /**
   * Returns all dynamically generated account addresses for pre-funding.
   */
  protected getAccountsToFund(_api: EnrichedBspApi): string[] {
    const addresses: string[] = [];
    for (const bsp of this.identities.bsps) {
      addresses.push(bsp.identity.keyring.address);
    }
    for (const msp of this.identities.msps) {
      addresses.push(msp.identity.keyring.address);
    }
    for (const fisherman of this.identities.fishermen) {
      addresses.push(fisherman.identity.keyring.address);
    }
    for (const user of this.identities.users) {
      addresses.push(user.identity.keyring.address);
    }
    return addresses;
  }

  /**
   * Gets API connection for a BSP node (lazy loaded).
   */
  async getBspApi(index: number): Promise<EnrichedBspApi> {
    return this.connectionPool.getOrCreate(`bsp-${index}`);
  }

  /**
   * Gets API connection for an MSP node (lazy loaded).
   */
  async getMspApi(index: number): Promise<EnrichedBspApi> {
    return this.connectionPool.getOrCreate(`msp-${index}`);
  }

  /**
   * Gets API connection for a fisherman node (lazy loaded).
   */
  async getFishermanApi(index: number): Promise<EnrichedBspApi> {
    return this.connectionPool.getOrCreate(`fisherman-${index}`);
  }

  /**
   * Gets API connection for a user node (lazy loaded).
   */
  async getUserApi(index: number): Promise<EnrichedBspApi> {
    return this.connectionPool.getOrCreate(`user-${index}`);
  }

  /**
   * Gets API connection for the block producer node (user-0).
   *
   * By convention, user-0 is used for block sealing, consistent with bspnet/fullnet
   * tests where sh-user is the block-sealing node.
   *
   * @returns EnrichedBspApi connected to user-0
   */
  async getBlockProducerApi(): Promise<EnrichedBspApi> {
    return this.connectionPool.getOrCreate("user-0");
  }

  /**
   * Executes a function for each BSP sequentially.
   */
  async forEachBsp(fn: (api: EnrichedBspApi, index: number) => Promise<void>): Promise<void> {
    for (let i = 0; i < this.identities.bsps.length; i++) {
      const api = await this.getBspApi(i);
      await fn(api, i);
    }
  }

  /**
   * Maps a function over all BSPs, collecting results.
   */
  async mapBsps<T>(fn: (api: EnrichedBspApi, index: number) => Promise<T>): Promise<T[]> {
    const results: T[] = [];
    for (let i = 0; i < this.identities.bsps.length; i++) {
      const api = await this.getBspApi(i);
      results.push(await fn(api, i));
    }
    return results;
  }

  /**
   * Cleanup all connections and resources.
   *
   * Cleans up in order:
   * 1. API connections (WebSocket)
   * 2. Docker containers (via docker-compose down)
   * 3. Ephemeral keystore directory (if created)
   */
  async cleanup(): Promise<void> {
    await this.connectionPool.cleanup();

    // Align dynamic-network cleanup with other integration suites:
    // export container logs into /tmp/bsp-logs-* before teardown.
    try {
      await cleanupEnvironment();
    } catch {
      // Cleanup errors are non-critical
    }

    try {
      await compose.down({
        cwd: path.resolve(process.cwd(), "..", "docker"),
        config: this.composeFile,
        log: false
      });
    } catch {
      // Cleanup errors are non-critical
    }

    if (this.keystoreTempDir) {
      try {
        this.keystoreTempDir.removeCallback();
      } catch {
        try {
          fs.rmSync(this.keystoreTempDir.name, {
            recursive: true,
            force: true
          });
        } catch {
          // Cleanup errors are non-critical
        }
      }
    }
  }

  get bspCount(): number {
    return this.identities.bsps.length;
  }

  get mspCount(): number {
    return this.identities.msps.length;
  }

  get fishermanCount(): number {
    return this.identities.fishermen.length;
  }

  get userCount(): number {
    return this.identities.users.length;
  }

  getBspIdentity(index: number) {
    return this.identities.bsps[index];
  }
  getMspIdentity(index: number) {
    return this.identities.msps[index];
  }
  getFishermanIdentity(index: number) {
    return this.identities.fishermen[index];
  }
  getUserIdentity(index: number) {
    return this.identities.users[index];
  }

  /**
   * Gets the provider ID for a BSP.
   *
   * @param index - BSP index (0-based)
   * @returns The BSP's provider ID as a hex string
   * @throws Error if BSP not found or provider ID not set
   */
  getBspProviderId(index: number): HexString {
    const identity = this.identities.bsps[index];
    if (!identity) {
      throw new Error(`BSP ${index} not found`);
    }
    if (!identity.identity.providerId) {
      throw new Error(`BSP ${index} provider ID not set - was the provider registered?`);
    }
    // PolkadotJS serializes as {"bsp":"0x..."}
    const parsed = JSON.parse(identity.identity.providerId);
    if (!parsed.bsp) {
      throw new Error(`BSP ${index} has unexpected provider ID format`);
    }
    return parsed.bsp as HexString;
  }

  /**
   * Gets the provider ID for an MSP.
   *
   * @param index - MSP index (0-based)
   * @returns The MSP's provider ID as a hex string
   * @throws Error if MSP not found or provider ID not set
   */
  getMspProviderId(index: number): HexString {
    const identity = this.identities.msps[index];
    if (!identity) {
      throw new Error(`MSP ${index} not found`);
    }
    if (!identity.identity.providerId) {
      throw new Error(`MSP ${index} provider ID not set - was the provider registered?`);
    }
    // PolkadotJS serializes as {"msp":"0x..."}
    const parsed = JSON.parse(identity.identity.providerId);
    if (!parsed.msp) {
      throw new Error(`MSP ${index} has unexpected provider ID format`);
    }
    return parsed.msp as HexString;
  }
}

/**
 * Launches a network from a topology specification.
 *
 * Bootstrap order:
 * 1. BSP-0 first (becomes bootnode, needed for indexers to connect)
 * 2. Postgres instances for all MSPs and Fishermen
 * 3. Indexer containers for all MSPs and Fishermen (connect to BSP bootnode)
 * 4. Remaining BSP nodes (BSP-1 through BSP-N)
 * 5. MSP nodes (connect to BSP bootnode, depend on their indexer)
 * 6. User nodes (connect to BSP bootnode)
 * 7. Fisherman nodes (connect to BSP bootnode, depend on their indexer)
 *
 * @param topology - Network topology defining node counts and configurations
 * @param config - Base network configuration
 * @returns Context object for interacting with the launched network
 */
export async function launchNetworkFromTopology(
  topology: NetworkTopology,
  config: DynamicNetworkConfig = {}
): Promise<DynamicNetworkContext> {
  validateTopology(topology);
  const normalized = normalizeTopology(topology);

  const reporter = config.progressReporter ?? new SilentProgressReporter();
  const runtimeType = config.runtimeType ?? "parachain";

  // Keystore directories must be world-writable because the Docker container
  // runs as uid 1337 (storage-hub user) while the host creates these as the runner user.
  // On Linux CI, native bind mounts enforce host permissions strictly.
  // Mirrors: chmod -R 777 docker/dev-keystores in .github/workflows/network.yml
  const keystoreTempDir = tmp.dirSync({
    prefix: "storagehub-test-keystores-",
    unsafeCleanup: true,
    mode: WORLD_WRITABLE_MODE
  });
  fs.chmodSync(keystoreTempDir.name, WORLD_WRITABLE_MODE);

  for (let i = 0; i < normalized.bsps.length; i++) {
    ensureWorldWritableDirectory(path.join(keystoreTempDir.name, `bsp-${i}`));
  }
  for (let i = 0; i < normalized.msps.length; i++) {
    ensureWorldWritableDirectory(path.join(keystoreTempDir.name, `msp-${i}`));
  }
  for (let i = 0; i < normalized.fishermen.length; i++) {
    ensureWorldWritableDirectory(path.join(keystoreTempDir.name, `fisherman-${i}`));
  }
  for (let i = 0; i < normalized.users.length; i++) {
    ensureWorldWritableDirectory(path.join(keystoreTempDir.name, `user-${i}`));
  }

  const { services, identities } = generateComposeServices(normalized, {
    runtimeType,
    image: config.image,
    keystorePath: keystoreTempDir.name
  });

  const composeFile = writeComposeFile(services);
  const cwd = path.resolve(process.cwd(), "..", "docker");

  const nodeUrls = new Map<string, string>();
  for (const [index, nodeInfo] of identities.bsps.entries()) {
    nodeUrls.set(`bsp-${index}`, `ws://127.0.0.1:${nodeInfo.ports.rpc}`);
  }
  for (const [index, nodeInfo] of identities.msps.entries()) {
    nodeUrls.set(`msp-${index}`, `ws://127.0.0.1:${nodeInfo.ports.rpc}`);
  }
  for (const [index, nodeInfo] of identities.fishermen.entries()) {
    nodeUrls.set(`fisherman-${index}`, `ws://127.0.0.1:${nodeInfo.ports.rpc}`);
  }
  for (const [index, nodeInfo] of identities.users.entries()) {
    nodeUrls.set(`user-${index}`, `ws://127.0.0.1:${nodeInfo.ports.rpc}`);
  }

  const connectionPool = new LazyConnectionPool(nodeUrls, 50, runtimeType);

  const context = new DynamicNetworkContext(
    connectionPool,
    identities,
    composeFile,
    runtimeType,
    keystoreTempDir
  );

  let bsp0Api: EnrichedBspApi | undefined;

  try {
    let bootnodeInfo: BootnodeInfo | undefined;
    if (identities.bsps.length > 0) {
      await startProvidersPhase(
        "bsp",
        [identities.bsps[0]],
        composeFile,
        cwd,
        reporter,
        runtimeType
      );

      bootnodeInfo = await getBootnodeInfo(identities.bsps[0]);
    }

    await startIndexerPostgresPhase(identities, composeFile, cwd, reporter, bootnodeInfo);

    if (identities.bsps.length > 1) {
      bsp0Api = await BspNetTestApi.create(
        `ws://127.0.0.1:${identities.bsps[0].ports.rpc}`,
        runtimeType
      );

      await startProvidersPhase(
        "bsp",
        identities.bsps.slice(1),
        composeFile,
        cwd,
        reporter,
        runtimeType,
        bootnodeInfo,
        bsp0Api
      );
    }

    if (!bsp0Api && identities.bsps.length > 0) {
      bsp0Api = await BspNetTestApi.create(
        `ws://127.0.0.1:${identities.bsps[0].ports.rpc}`,
        runtimeType
      );
    }

    await startMspContainersPhase(
      identities.msps,
      composeFile,
      cwd,
      reporter,
      runtimeType,
      bootnodeInfo,
      bsp0Api
    );
    await startUsersPhase(
      identities.users,
      composeFile,
      cwd,
      reporter,
      runtimeType,
      bootnodeInfo,
      bsp0Api
    );

    // Dynamic topologies require at least one user node; user-0 is the setup producer.
    const user0Info = identities.users[0];
    let user0Api = await BspNetTestApi.create(`ws://127.0.0.1:${user0Info.ports.rpc}`, runtimeType);
    await syncNodeTip(bsp0Api!, user0Api);
    await waitForNodeIdle("user", user0Info.index);

    // bsp0Api is no longer needed once user-0 is synced and healthy.
    if (bsp0Api) {
      await bsp0Api.disconnect();
      bsp0Api = undefined;
    }

    ({ api: user0Api } = await runWithAuraRecovery(
      user0Api,
      runtimeType,
      user0Info.ports.rpc,
      "preFundAccounts",
      (api) => context.preFundAccounts(api)
    ));
    ({ api: user0Api } = await runWithAuraRecovery(
      user0Api,
      runtimeType,
      user0Info.ports.rpc,
      "setupRuntimeParams",
      (api) => context.setupRuntimeParams(api)
    ));

    user0Api = await registerProvidersWithSetupProducer(
      user0Api,
      identities.bsps,
      runtimeType,
      user0Info.ports.rpc
    );
    user0Api = await registerProvidersWithSetupProducer(
      user0Api,
      identities.msps,
      runtimeType,
      user0Info.ports.rpc
    );

    ({ api: user0Api } = await runWithAuraRecovery(
      user0Api,
      runtimeType,
      user0Info.ports.rpc,
      "finalSetupSeal",
      (api) => api.block.seal()
    ));

    // Trusted MSP peer IDs are resolved by BSPs on startup.
    // MSP registration is complete now, so restart BSPs to refresh trusted MSP peer mapping.
    await restartBspsAfterMspRegistration(
      identities.bsps,
      composeFile,
      cwd,
      reporter,
      runtimeType,
      user0Api
    );

    await user0Api.disconnect();

    await startProvidersPhase(
      "fisherman",
      identities.fishermen,
      composeFile,
      cwd,
      reporter,
      runtimeType,
      bootnodeInfo
    );

    return context;
  } catch (error) {
    if (bsp0Api) {
      try {
        await bsp0Api.disconnect();
      } catch {
        // Cleanup errors are non-critical
      }
    }

    try {
      await context.cleanup();
    } catch {
      // Cleanup errors are non-critical
    }

    throw error;
  }
}

function writeComposeFile(services: Record<string, unknown>): string {
  const composeContents = {
    name: "storage-hub",
    services,
    networks: {
      "storage-hub_default": {
        driver: "bridge"
      }
    }
  };

  const updatedCompose = yaml.stringify(composeContents, {
    collectionStyle: "flow",
    defaultStringType: "QUOTE_DOUBLE",
    doubleQuotedAsJSON: true,
    flowCollectionPadding: true
  });

  const tmpFile = tmp.fileSync({ postfix: ".yml" });
  fs.writeFileSync(tmpFile.name, updatedCompose);

  return tmpFile.name;
}

async function getBootnodeInfo(nodeInfo: NodeIdentityInfo): Promise<BootnodeInfo> {
  const containerName = `storage-hub-sh-${nodeInfo.nodeType}-${nodeInfo.index}`;
  const ip = await getContainerIp(containerName);
  const peerId = await getContainerPeerId(`http://127.0.0.1:${nodeInfo.ports.rpc}`, false);
  return { ip, peerId };
}

async function waitForRpcReady(rpcPort: number, maxRetries = 30): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const peerId = await sendCustomRpc(`http://127.0.0.1:${rpcPort}`, "system_localPeerId");
      if (peerId) return;
    } catch {}
    await sleep(1000);
  }
  throw new Error(`RPC port ${rpcPort} failed to become ready after ${maxRetries} attempts`);
}

async function registerProvider(api: EnrichedBspApi, nodeInfo: NodeIdentityInfo): Promise<void> {
  const containerName = `storage-hub-sh-${nodeInfo.nodeType}-${nodeInfo.index}`;
  const containerIp = await getContainerIp(containerName);
  const peerId = await getContainerPeerId(`http://127.0.0.1:${nodeInfo.ports.rpc}`, false);
  const multiaddress = `/ip4/${containerIp}/tcp/30350/p2p/${peerId}`;

  if (nodeInfo.nodeType === "bsp") {
    const bspId = nodeInfo.providerId;
    if (!bspId) {
      throw new Error(`Missing deterministic BSP providerId for bsp-${nodeInfo.index}`);
    }
    const capacity = nodeInfo.config.capacity ?? CAPACITY_512;
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.providers.forceBspSignUp(
            nodeInfo.identity.keyring.address,
            bspId,
            capacity,
            [multiaddress],
            nodeInfo.identity.keyring.address,
            null
          )
        )
      ]
    });
  } else if (nodeInfo.nodeType === "msp") {
    const mspId = nodeInfo.providerId;
    if (!mspId) {
      throw new Error(`Missing deterministic MSP providerId for msp-${nodeInfo.index}`);
    }
    const capacity = nodeInfo.config.capacity ?? CAPACITY_512;
    await api.block.seal({
      calls: [
        api.tx.sudo.sudo(
          api.tx.providers.forceMspSignUp(
            nodeInfo.identity.keyring.address,
            mspId,
            capacity,
            [multiaddress],
            100 * 1024 * 1024,
            "Terms of Service...",
            9999999,
            nodeInfo.identity.keyring.address
          )
        )
      ]
    });
  }
}

async function startIndexerPostgresPhase(
  identities: NodeIdentities,
  composeFile: string,
  cwd: string,
  reporter: ProgressReporter,
  bootnodeInfo?: BootnodeInfo
): Promise<void> {
  const nodesWithInfra: { type: "msp" | "fisherman"; index: number }[] = [];

  for (const [index] of identities.msps.entries()) {
    nodesWithInfra.push({ type: "msp", index });
  }
  for (const [index] of identities.fishermen.entries()) {
    nodesWithInfra.push({ type: "fisherman", index });
  }

  if (nodesWithInfra.length === 0) return;

  const postgresTimer = new PhaseTimer();
  reporter.onPhaseStart("POSTGRES", nodesWithInfra.length);

  const postgresServices = nodesWithInfra.map(({ type, index }) => `sh-${type}-${index}-postgres`);
  await compose.upMany(postgresServices, {
    cwd,
    config: composeFile,
    log: false
  });

  await Promise.all(
    nodesWithInfra.map(async ({ type, index }, i) => {
      await waitForLog({
        containerName: `storage-hub-sh-${type}-${index}-postgres`,
        searchString: "database system is ready to accept connections"
      });
      reporter.onNodeReady("postgres", i, nodesWithInfra.length);
    })
  );

  reporter.onPhaseComplete("POSTGRES", postgresTimer.elapsed());

  const indexerTimer = new PhaseTimer();
  reporter.onPhaseStart("INDEXER", nodesWithInfra.length);

  const env: Record<string, string> = {
    ...(process.env as Record<string, string>)
  };
  if (bootnodeInfo) {
    env.BSP_IP = bootnodeInfo.ip;
    env.BSP_PEER_ID = bootnodeInfo.peerId;
  }

  const indexerServices = nodesWithInfra.map(({ type, index }) => `sh-${type}-${index}-indexer`);
  await compose.upMany(indexerServices, {
    cwd,
    config: composeFile,
    log: false,
    env
  });

  await Promise.all(
    nodesWithInfra.map(async ({ type, index }, i) => {
      await waitForLog({
        containerName: `storage-hub-sh-${type}-${index}-indexer`,
        searchString: "💤 Idle",
        timeout: 30000
      });
      reporter.onNodeReady("indexer", i, nodesWithInfra.length);
    })
  );

  reporter.onPhaseComplete("INDEXER", indexerTimer.elapsed());
}

async function startProvidersPhase(
  type: "bsp" | "fisherman",
  nodes: NodeIdentityInfo[],
  composeFile: string,
  cwd: string,
  reporter: ProgressReporter,
  runtimeType: "parachain" | "solochain",
  bootnodeInfo?: BootnodeInfo,
  syncApi?: EnrichedBspApi
): Promise<void> {
  if (nodes.length === 0) return;

  const timer = new PhaseTimer();
  reporter.onPhaseStart(type.toUpperCase(), nodes.length);

  const env: Record<string, string> = {
    ...(process.env as Record<string, string>)
  };
  if (bootnodeInfo) {
    env.BSP_IP = bootnodeInfo.ip;
    env.BSP_PEER_ID = bootnodeInfo.peerId;
  }

  for (const nodeInfo of nodes) {
    try {
      const serviceName = `sh-${type}-${nodeInfo.index}`;
      await compose.upOne(serviceName, {
        cwd,
        config: composeFile,
        log: false,
        env
      });

      await waitForRpcReady(nodeInfo.ports.rpc);
      await waitForNodeIdle(type, nodeInfo.index);

      const nodeApi = await BspNetTestApi.create(
        `ws://127.0.0.1:${nodeInfo.ports.rpc}`,
        runtimeType
      );

      if (syncApi) {
        await syncNodeTip(syncApi, nodeApi);
      }

      await injectKeys(nodeApi, nodeInfo.identity);

      await nodeApi.disconnect();
      reporter.onNodeReady(type, nodeInfo.index, nodes.length);
    } catch (error) {
      reporter.onError(type, nodeInfo.index, error as Error);
      throw error;
    }
  }

  reporter.onPhaseComplete(type.toUpperCase(), timer.elapsed());
}

async function startMspContainersPhase(
  msps: NodeIdentityInfo[],
  composeFile: string,
  cwd: string,
  reporter: ProgressReporter,
  runtimeType: "parachain" | "solochain",
  bootnodeInfo?: BootnodeInfo,
  syncApi?: EnrichedBspApi
): Promise<void> {
  if (msps.length === 0) return;

  const timer = new PhaseTimer();
  reporter.onPhaseStart("MSP", msps.length);

  const env: Record<string, string> = {
    ...(process.env as Record<string, string>)
  };
  if (bootnodeInfo) {
    env.BSP_IP = bootnodeInfo.ip;
    env.BSP_PEER_ID = bootnodeInfo.peerId;
  }

  const mspServices = msps.map((_, index) => `sh-msp-${index}`);
  await compose.upMany(mspServices, {
    cwd,
    config: composeFile,
    log: false,
    env
  });

  for (const [index, nodeInfo] of msps.entries()) {
    try {
      await waitForRpcReady(nodeInfo.ports.rpc);
      await waitForNodeIdle("msp", nodeInfo.index);

      const mspApi = await BspNetTestApi.create(
        `ws://127.0.0.1:${nodeInfo.ports.rpc}`,
        runtimeType
      );

      await injectKeys(mspApi, nodeInfo.identity);

      if (syncApi) {
        await syncNodeTip(syncApi, mspApi);
      }

      await mspApi.disconnect();

      reporter.onNodeReady("msp", index, msps.length);
    } catch (error) {
      reporter.onError("msp", index, error as Error);
      throw error;
    }
  }

  reporter.onPhaseComplete("MSP", timer.elapsed());
}

async function restartBspsAfterMspRegistration(
  bsps: NodeIdentityInfo[],
  composeFile: string,
  cwd: string,
  reporter: ProgressReporter,
  runtimeType: "parachain" | "solochain",
  syncApi: EnrichedBspApi
): Promise<void> {
  if (bsps.length === 0) return;

  const timer = new PhaseTimer();
  reporter.onPhaseStart("BSP-RESTART", bsps.length);

  for (const [index, nodeInfo] of bsps.entries()) {
    try {
      await compose.restartOne(`sh-bsp-${nodeInfo.index}`, {
        cwd,
        config: composeFile,
        log: false
      });

      await waitForRpcReady(nodeInfo.ports.rpc);
      await waitForNodeIdle("bsp", nodeInfo.index);

      const bspApi = await BspNetTestApi.create(
        `ws://127.0.0.1:${nodeInfo.ports.rpc}`,
        runtimeType
      );

      await syncNodeTip(syncApi, bspApi);
      await fetchProviderId(bspApi, nodeInfo.identity);

      await bspApi.disconnect();
      reporter.onNodeReady("bsp", index, bsps.length);
    } catch (error) {
      reporter.onError("bsp", index, error as Error);
      throw error;
    }
  }

  reporter.onPhaseComplete("BSP-RESTART", timer.elapsed());
}

async function startUsersPhase(
  users: NodeIdentityInfo[],
  composeFile: string,
  cwd: string,
  reporter: ProgressReporter,
  runtimeType: "parachain" | "solochain",
  bootnode?: BootnodeInfo,
  syncApi?: EnrichedBspApi
): Promise<void> {
  if (users.length === 0) return;

  const timer = new PhaseTimer();
  reporter.onPhaseStart("USER", users.length);

  const env: Record<string, string> = {
    ...(process.env as Record<string, string>)
  };
  if (bootnode) {
    env.BSP_IP = bootnode.ip;
    env.BSP_PEER_ID = bootnode.peerId;
  }

  const userServices = users.map((_, index) => `sh-user-${index}`);
  await compose.upMany(userServices, {
    cwd,
    config: composeFile,
    log: false,
    env
  });

  for (const [index, nodeInfo] of users.entries()) {
    try {
      await waitForRpcReady(nodeInfo.ports.rpc);
      await waitForNodeIdle("user", nodeInfo.index);

      const api = await BspNetTestApi.create(`ws://127.0.0.1:${nodeInfo.ports.rpc}`, runtimeType);

      await injectKeys(api, nodeInfo.identity);

      if (syncApi) {
        await syncNodeTip(syncApi, api);
      }

      await api.disconnect();

      reporter.onNodeReady("user", index, users.length);
    } catch (error) {
      reporter.onError("user", index, error as Error);
      throw error;
    }
  }

  reporter.onPhaseComplete("USER", timer.elapsed());
}

/**
 * Docker Compose service generation for dynamic networks.
 *
 * Generates Docker Compose service definitions for nodes, databases, and supporting
 * services. See topology.ts for the infrastructure architecture.
 */

import path from "node:path";
import type { GeneratedIdentity } from "./keyGenerator";
import type { NodeConfig, NormalizedTopology } from "./topology";
import type { Ports } from "./portAllocator";
import { PortAllocator } from "./portAllocator";
import { generateNodeIdentity } from "./keyGenerator";

/**
 * Docker Compose service definition.
 */
export interface DockerService {
  image: string;
  container_name?: string;
  platform?: string;
  command?: string[] | string;
  ports?: string[];
  volumes?: string[];
  networks?: string[] | Record<string, any>;
  environment?: Record<string, string>;
  depends_on?: string[];
}

/**
 * Complete metadata for a node including identity and port allocations.
 */
export interface NodeIdentityInfo {
  identity: GeneratedIdentity;
  ports: Ports;
  config: NodeConfig;
  nodeType: "bsp" | "msp" | "fisherman" | "user";
  index: number;
}

/**
 * Collection of all node identities in the network.
 */
export interface NodeIdentities {
  bsps: NodeIdentityInfo[];
  msps: NodeIdentityInfo[];
  fishermen: NodeIdentityInfo[];
  users: NodeIdentityInfo[];
}

/**
 * Bootnode information for connecting nodes to the network.
 */
export interface BootnodeInfo {
  ip: string;
  peerId: string;
}

/**
 * Configuration for dynamic network service generation.
 */
export interface ServiceGeneratorConfig {
  /** Runtime type (parachain or solochain) */
  runtimeType?: "parachain" | "solochain";
  /** Docker image to use */
  image?: string;
  /**
   * Base path for keystore volumes.
   * Can be relative (resolved from cwd) or absolute (used directly).
   * For ephemeral tests, pass an absolute temp directory path.
   */
  keystorePath?: string;
  /** Network name for docker-compose */
  networkName?: string;
}

/**
 * Generates a storage-hub node service definition.
 *
 * @param nodeType - Type of provider node
 * @param identity - Generated identity for the node
 * @param ports - Port allocation for the node
 * @param config - Node-specific configuration
 * @param index - Node index within its type
 * @param baseConfig - Shared network configuration
 * @returns Docker service definition
 */
export function generateNodeService(
  nodeType: "bsp" | "msp" | "fisherman",
  identity: GeneratedIdentity,
  ports: Ports,
  config: NodeConfig,
  index: number,
  baseConfig: ServiceGeneratorConfig
): DockerService {
  const serviceName = `sh-${nodeType}-${index}`;
  const keystoreBasePath = baseConfig.keystorePath ?? "../docker/dev-keystores";
  // Resolve keystore path: absolute paths used directly, relative paths resolved from cwd
  // Each node gets its own keystore directory to prevent key conflicts
  // e.g., /tmp/xxx/bsp-0, /tmp/xxx/bsp-1, etc.
  const keystoreDir = path.isAbsolute(keystoreBasePath)
    ? path.join(keystoreBasePath, `${nodeType}-${index}`)
    : path.join(process.cwd(), keystoreBasePath, `${nodeType}-${index}`);
  const image = baseConfig.image ?? "storage-hub:local";
  const networkName = baseConfig.networkName ?? "storage-hub_default";

  // Build command args based on node type
  // Fishermen are special - they don't use --provider/--provider-type, they use --fisherman
  const args: string[] = [];

  // Determine P2P port based on node type (matching fullnet-base-template.yml)
  // BSP/MSP use 30350, fisherman uses 30666
  const p2pPort = nodeType === "fisherman" ? 30666 : 30350;

  // Common args for all node types
  args.push("--dev");
  args.push(`--name=${serviceName}`);
  args.push("--no-hardware-benchmarks");
  args.push("--unsafe-rpc-external");
  args.push("--rpc-methods=unsafe");
  args.push(`--port=${p2pPort}`);
  args.push("--rpc-cors=all");
  args.push(`--node-key=${identity.nodeKey}`);
  args.push("--keystore-path=/keystore");
  args.push("--sealing=manual");
  args.push("--base-path=/data");

  // Add provider-specific args for BSP/MSP (not fisherman)
  if (nodeType === "bsp" || nodeType === "msp") {
    args.push("--provider");
    args.push(`--provider-type=${nodeType}`);
    args.push("--max-storage-capacity=4294967295");
    args.push("--jump-capacity=1073741824");
  }

  // Add fisherman-specific flags (fishermen always need a database)
  if (nodeType === "fisherman") {
    args.push("--fisherman");
    const dbUrl = `postgresql://postgres:postgres@storage-hub-sh-${nodeType}-${index}-postgres:5432/${nodeType}_${index}`;
    args.push(`--fisherman-database-url=${dbUrl}`);
    args.push("--fisherman-batch-interval-seconds=5");
  }

  // Add chain spec if solochain
  if (baseConfig.runtimeType === "solochain") {
    args.push("--chain=solochain-evm-dev");
  }

  // Add storage backend
  if (config.rocksdb) {
    args.push("--storage-layer=rocks-db");
    args.push(`--storage-path=/tmp/${serviceName}`);
  }

  // Add bootnode for all nodes except BSP-0 (uses env var placeholder resolved at container startup)
  // BSP-0 is the bootnode, all other nodes connect to it
  if (nodeType !== "bsp" || index > 0) {
    args.push(
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Docker compose env var substitution
      "--bootnodes=/ip4/${BSP_IP:-default_bsp_ip}/tcp/30350/p2p/${BSP_PEER_ID:-default_bsp_peer_id}"
    );
  }

  // Add MSP-specific flags
  if (nodeType === "msp") {
    args.push("--msp-charging-period=12");
    args.push("--msp-distribute-files");
    // MSPs need database access for move bucket operations.
    // Connect to the dedicated Postgres instance that the indexer writes to.
    const dbUrl = `postgresql://postgres:postgres@storage-hub-sh-${nodeType}-${index}-postgres:5432/${nodeType}_${index}`;
    args.push(`--msp-database-url=${dbUrl}`);
  }

  // Add custom capacity if specified (overrides default)
  if (config.capacity) {
    // Remove the default --max-storage-capacity and add the custom one
    const capacityIndex = args.findIndex((arg) => arg.startsWith("--max-storage-capacity="));
    if (capacityIndex !== -1) {
      args[capacityIndex] = `--max-storage-capacity=${config.capacity}`;
    }
  }

  // Add any additional custom args
  if (config.additionalArgs) {
    args.push(...config.additionalArgs);
  }

  // MSP and Fisherman nodes depend on their dedicated indexer being ready
  // (indexer itself depends on postgres, so we only need to depend on indexer)
  // BSP nodes have no dependencies
  const depends_on: string[] = [];
  if (nodeType === "msp" || nodeType === "fisherman") {
    depends_on.push(`sh-${nodeType}-${index}-indexer`);
  }

  return {
    image,
    container_name: `storage-hub-${serviceName}`,
    platform: "linux/amd64",
    command: args,
    ports: [`${ports.rpc}:9944`, `${ports.p2p}:${p2pPort}`],
    volumes: [`${keystoreDir}:/keystore:rw`],
    networks: [networkName],
    depends_on
  };
}

/**
 * Generates a Postgres database service for an MSP or Fisherman node.
 *
 * In production-like architecture, each MSP and Fisherman gets a dedicated
 * Postgres instance that the indexer writes to and the node reads from.
 *
 * @param nodeType - Type of node this database serves ("msp" or "fisherman")
 * @param index - Node index
 * @param ports - Port allocation (uses postgres port)
 * @param baseConfig - Shared network configuration
 * @returns Docker service definition
 */
export function generatePostgresService(
  nodeType: string,
  index: number,
  ports: Ports,
  baseConfig: ServiceGeneratorConfig
): DockerService {
  const serviceName = `sh-${nodeType}-${index}-postgres`;
  const dbName = `${nodeType}_${index}`;
  const networkName = baseConfig.networkName ?? "storage-hub_default";

  return {
    image: "postgres:16-alpine",
    container_name: `storage-hub-${serviceName}`,
    environment: {
      POSTGRES_USER: "postgres",
      POSTGRES_PASSWORD: "postgres",
      POSTGRES_DB: dbName
    },
    ports: [`${ports.postgres}:5432`],
    networks: [networkName]
  };
}

/**
 * Generates a standalone indexer service for an MSP or Fisherman.
 *
 * In production-like architecture, indexers are separate containers that:
 * - Connect to the network via bootnode
 * - Subscribe to blockchain events
 * - Write indexed data to their dedicated Postgres instance
 * - Do NOT have keystores (they don't sign transactions)
 * - Do NOT have provider flags (they're not storage providers)
 *
 * Based on fullnet-base-template.yml lines 145-167.
 *
 * @param nodeType - Type of node this indexer serves ("msp" or "fisherman")
 * @param index - Node index
 * @param nodeKey - Node key for P2P identity
 * @param indexerPorts - Port allocation for the indexer
 * @param baseConfig - Shared network configuration
 * @returns Docker service definition
 */
export function generateIndexerService(
  nodeType: "msp" | "fisherman",
  index: number,
  nodeKey: string,
  indexerPorts: Ports,
  baseConfig: ServiceGeneratorConfig
): DockerService {
  const serviceName = `sh-${nodeType}-${index}-indexer`;
  const dbUrl = `postgresql://postgres:postgres@storage-hub-sh-${nodeType}-${index}-postgres:5432/${nodeType}_${index}`;
  const image = baseConfig.image ?? "storage-hub:local";
  const networkName = baseConfig.networkName ?? "storage-hub_default";

  // Indexers use port 30777 for P2P (matching fullnet-base-template.yml)
  const p2pPort = 30777;

  const args = [
    "--dev",
    `--name=${serviceName}`,
    "--no-hardware-benchmarks",
    "--unsafe-rpc-external",
    "--rpc-methods=unsafe",
    `--port=${p2pPort}`,
    "--rpc-cors=all",
    `--node-key=${nodeKey}`,
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Docker compose env var substitution
    "--bootnodes=/ip4/${BSP_IP:-default_bsp_ip}/tcp/30350/p2p/${BSP_PEER_ID:-default_bsp_peer_id}",
    "--sealing=manual",
    "--base-path=/data",
    "--indexer",
    `--indexer-database-url=${dbUrl}`
    // NOTE: No --keystore-path (indexers don't sign transactions)
    // NOTE: No --provider flags (indexers are not providers)
  ];

  // Add chain spec if solochain
  if (baseConfig.runtimeType === "solochain") {
    args.push("--chain=solochain-evm-dev");
  }

  return {
    image,
    container_name: `storage-hub-${serviceName}`,
    platform: "linux/amd64",
    command: args,
    ports: [`${indexerPorts.rpc}:9944`, `${indexerPorts.p2p}:${p2pPort}`],
    networks: [networkName],
    depends_on: [`sh-${nodeType}-${index}-postgres`]
  };
}

/**
 * Generates a user node service definition.
 *
 * User nodes are provider nodes (--provider-type=user) used for transaction submission.
 * They connect to the network via a bootnode (typically the first BSP) using
 * environment variable placeholders that are resolved at container startup.
 *
 * In production-like architecture, user nodes never have indexers or databases.
 *
 * @param identity - Generated identity for the node
 * @param ports - Port allocation for the node
 * @param config - Node-specific configuration
 * @param index - Node index
 * @param baseConfig - Shared network configuration
 * @returns Docker service definition
 */
export function generateUserService(
  identity: GeneratedIdentity,
  ports: Ports,
  config: NodeConfig,
  index: number,
  baseConfig: ServiceGeneratorConfig
): DockerService {
  const serviceName = `sh-user-${index}`;
  const keystoreBasePath = baseConfig.keystorePath ?? "../docker/dev-keystores";
  // Each user node gets its own keystore directory to prevent key conflicts
  const keystoreDir = path.isAbsolute(keystoreBasePath)
    ? path.join(keystoreBasePath, `user-${index}`)
    : path.join(process.cwd(), keystoreBasePath, `user-${index}`);
  const image = baseConfig.image ?? "storage-hub:local";
  const networkName = baseConfig.networkName ?? "storage-hub_default";

  // User nodes use P2P port 30444 (matching fullnet-base-template.yml)
  const p2pPort = 30444;

  const args = [
    "--dev",
    "--provider",
    "--provider-type=user",
    `--name=${serviceName}`,
    "--no-hardware-benchmarks",
    "--unsafe-rpc-external",
    "--rpc-methods=unsafe",
    "--rpc-cors=all",
    "--sealing=manual",
    `--port=${p2pPort}`,
    `--node-key=${identity.nodeKey}`,
    "--keystore-path=/keystore",
    "--base-path=/data"
  ];

  // Add chain spec if solochain
  if (baseConfig.runtimeType === "solochain") {
    args.push("--chain=solochain-evm-dev");
  }

  // Add storage backend
  if (config.rocksdb) {
    args.push("--storage-layer=rocks-db");
    args.push(`--storage-path=/tmp/${serviceName}`);
  }

  // Add bootnode using environment variable placeholder (resolved at container startup)
  args.push(
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Docker compose env var substitution
    "--bootnodes=/ip4/${BSP_IP:-default_bsp_ip}/tcp/30350/p2p/${BSP_PEER_ID:-default_bsp_peer_id}"
  );

  // Add any additional custom args
  if (config.additionalArgs) {
    args.push(...config.additionalArgs);
  }

  // User nodes never have indexers or database dependencies
  // in production-like architecture

  // Add resource volume mount for test files (matching fullnet-base-template.yml)
  // Path is relative to docker directory where compose runs
  const resourceDir = path.resolve(process.cwd(), "..", "docker", "resource");

  return {
    image,
    container_name: `storage-hub-${serviceName}`,
    platform: "linux/amd64",
    command: args,
    ports: [`${ports.rpc}:9944`, `${ports.p2p}:${p2pPort}`],
    volumes: [`${keystoreDir}:/keystore:rw`, `${resourceDir}:/res:ro`],
    networks: [networkName]
  };
}

/**
 * Generates all Docker Compose services for a network topology.
 *
 * In production-like architecture:
 * - BSPs: Only the BSP container (no Postgres, no Indexer)
 * - MSPs: Postgres + Indexer + MSP (3 containers per MSP)
 * - Fishermen: Postgres + Indexer + Fisherman (3 containers per Fisherman)
 * - Users: Only the User container (no Postgres, no Indexer)
 *
 * Note: User nodes are generated without bootnode info initially.
 * Bootnode connection is configured at container startup time via environment variables.
 *
 * @param topology - Normalized network topology
 * @param baseConfig - Shared network configuration
 * @returns Service definitions and node identity metadata
 */
export function generateComposeServices(
  topology: NormalizedTopology,
  baseConfig: ServiceGeneratorConfig
): { services: Record<string, DockerService>; identities: NodeIdentities } {
  const services: Record<string, DockerService> = {};
  const identities: NodeIdentities = {
    bsps: [],
    msps: [],
    fishermen: [],
    users: []
  };
  const portAllocator = new PortAllocator();

  // Generate BSP services (no Postgres, no Indexer)
  for (const [index, config] of topology.bsps.entries()) {
    const identity = generateNodeIdentity("bsp", index);
    const ports = portAllocator.allocate("bsp", index);

    services[`sh-bsp-${index}`] = generateNodeService(
      "bsp",
      identity,
      ports,
      config,
      index,
      baseConfig
    );

    identities.bsps.push({
      identity,
      ports,
      config,
      nodeType: "bsp",
      index
    });
  }

  // Generate MSP services (Postgres + Indexer + MSP triplet)
  for (const [index, config] of topology.msps.entries()) {
    const identity = generateNodeIdentity("msp", index);
    const ports = portAllocator.allocate("msp", index);
    // Allocate separate ports for the indexer
    const indexerPorts = portAllocator.allocate("msp-indexer", index);
    // Generate a separate node key for the indexer
    const indexerIdentity = generateNodeIdentity("msp", index + 1000); // Offset to avoid collision

    // 1. Postgres service
    services[`sh-msp-${index}-postgres`] = generatePostgresService("msp", index, ports, baseConfig);

    // 2. Indexer service (depends on Postgres)
    services[`sh-msp-${index}-indexer`] = generateIndexerService(
      "msp",
      index,
      indexerIdentity.nodeKey,
      indexerPorts,
      baseConfig
    );

    // 3. MSP service (depends on Indexer)
    services[`sh-msp-${index}`] = generateNodeService(
      "msp",
      identity,
      ports,
      config,
      index,
      baseConfig
    );

    identities.msps.push({
      identity,
      ports,
      config,
      nodeType: "msp",
      index
    });
  }

  // Generate Fisherman services (Postgres + Indexer + Fisherman triplet)
  for (const [index, config] of topology.fishermen.entries()) {
    const identity = generateNodeIdentity("fisherman", index);
    const ports = portAllocator.allocate("fisherman", index);
    // Allocate separate ports for the indexer
    const indexerPorts = portAllocator.allocate("fisherman-indexer", index);
    // Generate a separate node key for the indexer
    const indexerIdentity = generateNodeIdentity("fisherman", index + 1000); // Offset to avoid collision

    // 1. Postgres service
    services[`sh-fisherman-${index}-postgres`] = generatePostgresService(
      "fisherman",
      index,
      ports,
      baseConfig
    );

    // 2. Indexer service (depends on Postgres)
    services[`sh-fisherman-${index}-indexer`] = generateIndexerService(
      "fisherman",
      index,
      indexerIdentity.nodeKey,
      indexerPorts,
      baseConfig
    );

    // 3. Fisherman service (depends on Indexer)
    services[`sh-fisherman-${index}`] = generateNodeService(
      "fisherman",
      identity,
      ports,
      config,
      index,
      baseConfig
    );

    identities.fishermen.push({
      identity,
      ports,
      config,
      nodeType: "fisherman",
      index
    });
  }

  // Generate User services (no Postgres, no Indexer)
  for (const [index, config] of topology.users.entries()) {
    const identity = generateNodeIdentity("user", index);
    const ports = portAllocator.allocate("user", index);

    services[`sh-user-${index}`] = generateUserService(identity, ports, config, index, baseConfig);

    identities.users.push({
      identity,
      ports,
      config,
      nodeType: "user",
      index
    });
  }

  return { services, identities };
}

/**
 * Type-safe network topology definitions for dynamic test networks.
 *
 * Architecture:
 * - BSPs and Users never have indexers or databases
 * - MSPs and Fishermen always have a dedicated Indexer + Postgres pair
 * - Indexers are standalone containers that write to Postgres
 * - MSPs/Fishermen connect to Postgres via database URL flags
 */

/**
 * Configuration for a single node in the network.
 *
 * Note: Indexer configuration is implicit based on node type:
 * - MSP nodes always get a dedicated Indexer + Postgres
 * - Fisherman nodes always get a dedicated Indexer + Postgres
 * - BSP and User nodes never have indexers
 */
export interface NodeConfig {
  /** Storage capacity in bytes (default: 512 MB for all provider types) */
  capacity?: bigint;
  /** Enable RocksDB storage backend (default: false - uses in-memory) */
  rocksdb?: boolean;
  /** Additional CLI arguments to pass to the node */
  additionalArgs?: string[];
}

/**
 * Complete network topology specification.
 *
 * Supports both simple count-based and detailed config-based node definitions:
 * - number: Creates N nodes with default config
 * - NodeConfig[]: Creates nodes with individual configurations
 *
 * Indexer architecture (implicit, not configurable):
 * - Each MSP gets a dedicated Postgres + Indexer pair
 * - Each Fisherman gets a dedicated Postgres + Indexer pair
 * - BSPs and Users never have indexers
 */
export interface NetworkTopology {
  /** Number of BSP nodes or array of BSP configurations */
  bsps: number | NodeConfig[];
  /** Number of MSP nodes or array of MSP configurations */
  msps: number | NodeConfig[];
  /** Number of fisherman nodes or array of fisherman configurations */
  fishermen: number | NodeConfig[];
  /** Number of user nodes or array of user configurations (default: 1) */
  users?: number | NodeConfig[];
  /** Number of collator nodes (default: 1) */
  collators?: number;
}

/**
 * Normalized topology with all counts converted to config arrays.
 * Internal representation used after normalization.
 */
export interface NormalizedTopology {
  bsps: NodeConfig[];
  msps: NodeConfig[];
  fishermen: NodeConfig[];
  users: NodeConfig[];
  collators: number;
}

/**
 * Normalizes a network topology by converting number counts to config arrays.
 *
 * @param topology - The topology to normalize
 * @returns Normalized topology with all nodes as config arrays
 *
 * @example
 * ```ts
 * const topology = { bsps: 3, msps: 1, fishermen: 0 };
 * const normalized = normalizeTopology(topology);
 * // normalized.bsps = [{}, {}, {}]
 * // normalized.msps = [{}]
 * // normalized.fishermen = []
 * ```
 */
export function normalizeTopology(topology: NetworkTopology): NormalizedTopology {
  // Normalize users: default to 1 user node if not specified
  const normalizeUsers = (): NodeConfig[] => {
    if (topology.users === undefined) return [{}]; // Default: 1 user
    if (typeof topology.users === "number") return Array(topology.users).fill({});
    return topology.users;
  };

  return {
    bsps: typeof topology.bsps === "number" ? Array(topology.bsps).fill({}) : topology.bsps,
    msps: typeof topology.msps === "number" ? Array(topology.msps).fill({}) : topology.msps,
    fishermen:
      typeof topology.fishermen === "number"
        ? Array(topology.fishermen).fill({})
        : topology.fishermen,
    users: normalizeUsers(),
    collators: topology.collators ?? 1
  };
}

/**
 * Validates that a topology configuration is valid.
 *
 * Requirements:
 * - At least 1 BSP (required for block production in dev mode)
 * - At least 1 MSP (required for storage request handling)
 * - At least 1 User (required for submitting transactions)
 *
 * Note: Indexer configuration is implicit and not validated here.
 * MSPs and Fishermen always get dedicated Indexer + Postgres pairs.
 *
 * @param topology - The topology to validate
 * @throws Error if topology is invalid
 */
export function validateTopology(topology: NetworkTopology): void {
  const normalized = normalizeTopology(topology);

  // BSP-0 is required for block production (Aura assigns to first dev node)
  if (normalized.bsps.length === 0) {
    throw new Error("Topology must have at least 1 BSP (required for block production)");
  }

  // MSP is required for storage request handling
  if (normalized.msps.length === 0) {
    throw new Error("Topology must have at least 1 MSP (required for storage requests)");
  }

  // User node is required for submitting transactions
  if (normalized.users.length === 0) {
    throw new Error("Topology must have at least 1 User (required for transactions)");
  }
}

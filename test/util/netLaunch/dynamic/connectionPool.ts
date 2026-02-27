/**
 * Lazy connection pool for API instances.
 *
 * Manages WebSocket connections to nodes with lazy initialization and LRU eviction.
 * Prevents resource exhaustion when working with large networks by
 * maintaining a bounded pool of active connections.
 */

import type { EnrichedBspApi } from "../../bspNet/test-api";
import { BspNetTestApi } from "../../bspNet/test-api";

/**
 * Lazy connection pool with LRU eviction.
 *
 * Features:
 * - Lazy initialization: Connections created only when accessed
 * - LRU eviction: Oldest unused connections dropped when pool is full
 * - Automatic cleanup: Gracefully disconnects all connections
 *
 * @example
 * ```ts
 * const pool = new LazyConnectionPool(
 *   new Map([
 *     ["bsp-0", "ws://localhost:9666"],
 *     ["bsp-1", "ws://localhost:9667"]
 *   ])
 * );
 *
 * const api = await pool.getOrCreate("bsp-0");
 * // Use api...
 *
 * await pool.cleanup(); // Disconnect all
 * ```
 */
export class LazyConnectionPool {
  private connections = new Map<string, EnrichedBspApi>();
  private accessOrder: string[] = [];
  private maxConnections: number;
  private nodeUrls: Map<string, string>;
  private runtimeType: "parachain" | "solochain";

  /**
   * Creates a new connection pool.
   *
   * @param nodeUrls - Map of node IDs to WebSocket URLs
   * @param maxConnections - Maximum concurrent connections (default: 50)
   * @param runtimeType - Runtime type for API initialization (default: "parachain")
   */
  constructor(
    nodeUrls: Map<string, string>,
    maxConnections = 50,
    runtimeType: "parachain" | "solochain" = "parachain"
  ) {
    this.nodeUrls = nodeUrls;
    this.maxConnections = maxConnections;
    this.runtimeType = runtimeType;
  }

  /**
   * Gets an existing connection or creates a new one.
   *
   * Implements LRU eviction: If pool is at capacity, the least recently
   * accessed connection is disconnected and removed.
   *
   * @param nodeId - Unique identifier for the node
   * @returns Connected API instance
   * @throws Error if nodeId not found in configured URLs
   */
  async getOrCreate(nodeId: string): Promise<EnrichedBspApi> {
    // Check cache
    const existingConnection = this.connections.get(nodeId);
    if (existingConnection) {
      // Update access order (move to end = most recent)
      this.accessOrder = this.accessOrder.filter((id) => id !== nodeId);
      this.accessOrder.push(nodeId);
      return existingConnection;
    }

    // LRU eviction if at capacity
    if (this.connections.size >= this.maxConnections) {
      const oldest = this.accessOrder.shift();
      if (oldest) {
        const oldConnection = this.connections.get(oldest);
        if (oldConnection) {
          await oldConnection.disconnect();
        }
        this.connections.delete(oldest);
      }
    }

    // Create new connection
    const url = this.nodeUrls.get(nodeId);
    if (!url) {
      throw new Error(
        `Unknown node ID: ${nodeId}. Available nodes: ${Array.from(this.nodeUrls.keys()).join(
          ", "
        )}`
      );
    }

    const api = await BspNetTestApi.create(url as `ws://${string}`, this.runtimeType);
    this.connections.set(nodeId, api);
    this.accessOrder.push(nodeId);

    return api;
  }

  /**
   * Disconnects all active connections and clears the pool.
   *
   * Should be called in test cleanup (e.g., after() hooks).
   */
  async cleanup(): Promise<void> {
    const disconnectPromises: Promise<void>[] = [];

    for (const api of this.connections.values()) {
      disconnectPromises.push(api.disconnect());
    }

    await Promise.all(disconnectPromises);

    this.connections.clear();
    this.accessOrder = [];
  }

  /**
   * Gets the number of active connections.
   */
  get activeConnections(): number {
    return this.connections.size;
  }

  /**
   * Gets all node IDs currently connected.
   */
  get connectedNodes(): string[] {
    return Array.from(this.connections.keys());
  }
}

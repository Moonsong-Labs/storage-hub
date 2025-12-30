/**
 * Dynamic port allocation for network nodes.
 *
 * Manages port allocation for RPC, P2P, and Postgres services, ensuring no
 * conflicts occur when launching large networks. Uses sequential allocation
 * via a global counter.
 */

/**
 * Port configuration for a single node or indexer.
 */
export interface Ports {
  /** RPC endpoint port (WebSocket) */
  rpc: number;
  /** P2P networking port */
  p2p: number;
  /** Postgres database port (used by MSP/Fisherman for their dedicated Postgres instance) */
  postgres: number;
}

/**
 * Configuration for port allocation strategy.
 */
export interface PortAllocatorConfig {
  /** Base port for RPC endpoints (default: 9666) */
  rpcBase?: number;
  /** Base port for P2P networking (default: 30350) */
  p2pBase?: number;
  /** Base port for Postgres databases (default: 5432) */
  postgresBase?: number;
}

/**
 * Allocates ports for dynamically created nodes.
 *
 * Uses sequential port allocation with a global counter to ensure no port
 * collisions occur when launching large networks with multiple node types.
 *
 * Port ranges:
 * - RPC: rpcBase + globalOffset (default: 9666+)
 * - P2P: p2pBase + globalOffset (default: 30350+)
 * - Postgres: postgresBase + globalOffset (default: 5432+)
 *
 * @example
 * ```ts
 * const allocator = new PortAllocator();
 * const ports1 = allocator.allocate("bsp", 0);
 * // { rpc: 9666, p2p: 30350, postgres: 5432 }
 * const ports2 = allocator.allocate("msp", 0);
 * // { rpc: 9667, p2p: 30351, postgres: 5433 } - uses global counter
 * ```
 */
export class PortAllocator {
  private rpcBase: number;
  private p2pBase: number;
  private postgresBase: number;
  private globalCounter = 0;

  constructor(config: PortAllocatorConfig = {}) {
    this.rpcBase = config.rpcBase ?? 9666;
    this.p2pBase = config.p2pBase ?? 30350;
    this.postgresBase = config.postgresBase ?? 5432;
  }

  /**
   * Allocates ports for a single node using a global counter.
   *
   * @param _nodeType - The type of node (for logging/debugging)
   * @param _index - The index of this node within its type (for reference only)
   * @returns Port configuration for the node
   */
  allocate(_nodeType: string, _index: number): Ports {
    const offset = this.globalCounter++;

    const rpcPort = this.rpcBase + offset;
    const p2pPort = this.p2pBase + offset;
    const postgresPort = this.postgresBase + offset;

    return { rpc: rpcPort, p2p: p2pPort, postgres: postgresPort };
  }

  /**
   * Allocates sequential ports for multiple nodes of the same type.
   *
   * @param nodeType - The type of node
   * @param count - Number of nodes to allocate ports for
   * @returns Array of port configurations
   */
  allocateBatch(nodeType: string, count: number): Ports[] {
    const ports: Ports[] = [];
    for (let i = 0; i < count; i++) {
      ports.push(this.allocate(nodeType, i));
    }
    return ports;
  }

  /**
   * Resets all port allocations.
   * Useful for test isolation.
   */
  reset(): void {
    this.globalCounter = 0;
  }

  /**
   * Gets the current allocation count.
   * Useful for debugging and capacity planning.
   */
  get allocationCount(): number {
    return this.globalCounter;
  }
}

/**
 * Type definitions for StorageHub EVM client
 */

import type { Address, Chain, WalletClient } from "viem";

/**
 * Configuration options for StorageHubClient
 */
export type StorageHubClientOptions = {
  /**
   * RPC endpoint URL for the StorageHub chain
   */
  rpcUrl: string;
  /**
   * Viem chain configuration
   */
  chain: Chain;
  /**
   * Wallet client for transaction signing
   */
  walletClient: WalletClient;
  /**
   * Filesystem precompile contract address
   */
  filesystemContractAddress: Address;
};

/**
 * Optional EVM write overrides for SDK calls.
 *
 * Use these when you need to customize the transaction envelope or
 * sidestep under-estimation issues on Frontier/weight based pallets.
 *
 * - If `gas` is not provided, the SDK will estimate gas for the function
 *   and apply `gasMultiplier` (default 5) for headroom.
 * - You can provide legacy `gasPrice`, or EIP-1559 fees via
 *   `maxFeePerGas` and `maxPriorityFeePerGas`.
 */
export type EvmWriteOptions = {
  /**
   * Explicit gas limit. If omitted, the SDK will estimate and multiply.
   */
  gas?: bigint;
  /**
   * Multiplier applied over the SDK gas estimate when `gas` is not supplied.
   * Defaults to 5.
   */
  gasMultiplier?: number;
  /**
   * Legacy gas price (wei). If set, EIP-1559 fields are ignored by most clients.
   */
  gasPrice?: bigint;
  /**
   * EIP-1559: max fee per gas (wei). Use with `maxPriorityFeePerGas`.
   */
  maxFeePerGas?: bigint;
  /**
   * EIP-1559: max priority fee per gas (wei).
   */
  maxPriorityFeePerGas?: bigint;
};

/**
 * Replication levels for storage requests.
 * Each level provides different redundancy and availability guarantees.
 */
export enum ReplicationLevel {
  /** Basic replication (default) */
  Basic = 0,
  /** Standard replication */
  Standard = 1,
  /** High security replication */
  HighSecurity = 2,
  /** Super high security replication */
  SuperHighSecurity = 3,
  /** Ultra high security replication */
  UltraHighSecurity = 4,
  /** Custom replication (requires specifying exact replica count) */
  Custom = 5
}

/**
 * File operations supported by the StorageHub protocol.
 */
export enum FileOperation {
  /** Delete operation for a file */
  Delete = 0
}

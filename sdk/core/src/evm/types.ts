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
 *
 * Use this type to explicitly control transaction fees when submitting
 * EIP-1559 transactions, especially under network congestion or when
 * automatic fee estimation is unreliable.
 *
 * Notes:
 * - These fields are mutually exclusive with legacy `gasPrice`.
 * - The effective gas price paid is:
 *   `min(maxFeePerGas, baseFeePerGas + maxPriorityFeePerGas)`.
 */
export type Eip1559FeeOptions = {
  /**
   * Maximum total fee per gas unit (wei).
   *
   * Acts as an upper bound that protects against sudden base fee spikes
   * between blocks.
   */
  maxFeePerGas: bigint;

  /**
   * Priority fee (tip) per gas unit (wei) paid to the block producer.
   *
   * Higher values increase the likelihood of faster inclusion under
   * congestion.
   */
  maxPriorityFeePerGas: bigint;
};

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
   * Legacy gas price (wei).
   *
   * @deprecated StorageHub SDK is moving to EIP-1559 only. This field is ignored by the SDK.
   * Use `maxFeePerGas` and `maxPriorityFeePerGas` instead.
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

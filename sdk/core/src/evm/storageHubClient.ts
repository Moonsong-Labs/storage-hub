/**
 * StorageHubClient - Unified EVM client for StorageHub blockchain
 *
 * Provides ergonomic read/write methods for StorageHub precompiles using viem.
 * Handles gas estimation automatically with Frontier-optimized defaults.
 *
 * All arguments are strongly typed. String data (names, paths) are passed as strings and encoded internally.
 * Binary data (signatures) are passed as Uint8Array. Hex values are 0x-prefixed strings (32-byte IDs).
 */

import { filesystemAbi } from '../abi/filesystem';
import type { FileInfo } from '../types';
import type { EvmWriteOptions, StorageHubClientOptions } from './types';
import { FileOperation, ReplicationLevel } from './types';
import {
  type Address,
  createPublicClient,
  getContract,
  type GetContractReturnType,
  hexToBytes,
  http,
  keccak256,
  parseGwei,
  type PublicClient,
  stringToBytes,
  stringToHex,
  type WalletClient,
} from 'viem';

// Re-export filesystemAbi for external use
export { filesystemAbi };

// Internal type definitions for FileSystem contract
type EvmClient = PublicClient | WalletClient;
type FileSystemContract<TClient extends EvmClient> = GetContractReturnType<
  typeof filesystemAbi,
  TClient
>;

/**
 * Internal constant precompile address for FileSystem on StorageHub runtimes.
 * If a chain uses a different address, this constant should be updated accordingly.
 */
const FILE_SYSTEM_PRECOMPILE_ADDRESS = "0x0000000000000000000000000000000000000064" as Address;

export class StorageHubClient {
  private readonly publicClient: PublicClient; // Internal for gas estimation
  private readonly walletClient: WalletClient; // User-provided
  private readonly filesystemContractAddress: Address; // Contract address for filesystem precompile

  // TODO: Make these constants retrievable from the precompile instead of hardcoded values
  private static readonly MAX_BUCKET_NAME_BYTES = 100;
  private static readonly MAX_LOCATION_BYTES = 512;
  private static readonly MAX_PEER_ID_BYTES = 100;

  // TODO: Gas estimation defaults
  private static readonly DEFAULT_GAS_MULTIPLIER = 5;
  private static readonly DEFAULT_GAS_PRICE = parseGwei("1");

  /**
   * Get write contract instance bound to the wallet client.
   *
   * @returns Contract instance for write operations (transactions)
   */
  private getWriteContract(): FileSystemContract<WalletClient> {
    return getContract({
      address: this.filesystemContractAddress,
      abi: filesystemAbi,
      client: this.walletClient
    });
  }

  /**
   * Get read contract instance bound to the public client.
   *
   * @returns Contract instance for read operations (view calls)
   */
  private getReadContract(): FileSystemContract<PublicClient> {
    return getContract({
      address: this.filesystemContractAddress,
      abi: filesystemAbi,
      client: this.publicClient
    });
  }

  /**
   * Reusable gas estimation for any contract method.
   *
   * Uses internal PublicClient for reliable estimation on Frontier chains.
   * Applies safety multiplier to handle weight→gas conversion issues.
   *
   * @param functionName - Contract method name
   * @param args - Method arguments
   * @param options - Gas overrides (explicit gas, multiplier, etc.)
   * @returns Estimated gas limit with safety multiplier applied
   */
  private async estimateGas(
    functionName: string,
    args: readonly unknown[],
    options?: EvmWriteOptions
  ): Promise<bigint> {
    // User provided explicit gas limit
    if (options?.gas) {
      return options.gas;
    }

    const accountAddr = this.walletClient.account?.address;
    const gasEstimation: bigint = await this.publicClient.estimateContractGas({
      address: this.filesystemContractAddress,
      abi: filesystemAbi,
      functionName,
      args,
      account: accountAddr
    });

    const multiplier = options?.gasMultiplier ?? StorageHubClient.DEFAULT_GAS_MULTIPLIER;
    return gasEstimation * BigInt(Math.max(1, Math.floor(multiplier)));
  }

  /**
   * Build transaction options with gas and fee settings.
   * Handles both legacy and EIP-1559 fee structures.
   */
  private buildTxOptions(gasLimit: bigint, options?: EvmWriteOptions): Record<string, unknown> {
    const useEip1559 =
      options?.maxFeePerGas !== undefined || options?.maxPriorityFeePerGas !== undefined;
    const txOpts: Record<string, unknown> = { gas: gasLimit };

    if (useEip1559) {
      // User wants EIP-1559 fees
      if (options?.maxFeePerGas) txOpts.maxFeePerGas = options.maxFeePerGas;
      if (options?.maxPriorityFeePerGas) txOpts.maxPriorityFeePerGas = options.maxPriorityFeePerGas;
    } else {
      // Default to legacy gas pricing
      txOpts.gasPrice = options?.gasPrice ?? StorageHubClient.DEFAULT_GAS_PRICE;
    }

    return txOpts;
  }

  /**
   * Validate string length in UTF-8 bytes and convert to hex.
   * @param str - Input string to validate and encode
   * @param maxBytes - Maximum allowed byte length
   * @param label - Label for error messages
   * @returns 0x-prefixed hex string
   */
  private validateStringLength(str: string, maxBytes: number, label: string): `0x${string}` {
    const bytes = stringToBytes(str);
    if (bytes.length > maxBytes) {
      throw new Error(`${label} exceeds maximum length of ${maxBytes} bytes (got ${bytes.length})`);
    }
    return stringToHex(str);
  }

  /**
   * Serialize FileOperationIntention and sign it
   */
  private async signIntention(
    fileKey: `0x${string}`,
    operation: FileOperation,
  ): Promise<{
    signedIntention: readonly [`0x${string}`, number];
    signature: `0x${string}`;
  }> {
    const fileKeyBytes = hexToBytes(fileKey);
    if (fileKeyBytes.length !== 32) {
      throw new Error(`Invalid file key: expected 32 bytes, got ${fileKeyBytes.length} bytes`);
    }

    const serialized = new Uint8Array([...fileKeyBytes, operation]);
    // TODO: we need to replace this with signMessage or EIP-712 (sign structure data)
    // we cannot sign this raw message/bytes with Metamask or any other EIP1193 wallet
    const hash = keccak256(serialized);
    const signature = await this.walletClient.account!.sign!({ hash });

    return {
      signedIntention: [fileKey, operation],
      signature,
    };
  }

  /**
   * Create a StorageHub client with automatic gas estimation.
   *
   * @param opts.rpcUrl - RPC endpoint URL for the StorageHub chain
   * @param opts.chain - Viem chain configuration
   * @param opts.walletClient - Wallet client for transaction signing
   * @param opts.filesystemContractAddress - Optional filesystem precompile address
   */
  constructor(opts: StorageHubClientOptions) {
    // Create internal PublicClient for gas estimation
    this.publicClient = createPublicClient({
      chain: opts.chain,
      transport: http(opts.rpcUrl)
    });
    this.walletClient = opts.walletClient;

    // Store the filesystem contract address with default fallback
    this.filesystemContractAddress =
      opts.filesystemContractAddress ?? FILE_SYSTEM_PRECOMPILE_ADDRESS;
  }

  // -------- Reads --------

  /**
   * Derive a bucket ID deterministically from owner + name.
   * @param owner - EVM address of the bucket owner
   * @param name - bucket name as string (max 100 UTF-8 bytes)
   * @returns bucketId as 0x-prefixed 32-byte hex
   */
  deriveBucketId(owner: Address, name: string) {
    const nameHex = this.validateStringLength(
      name,
      StorageHubClient.MAX_BUCKET_NAME_BYTES,
      "Bucket name"
    );
    const contract = this.getReadContract();
    return contract.read.deriveBucketId?.([owner, nameHex]);
  }

  /**
   * Get how many file deletion requests a user currently has pending.
   * @param user - user EVM address
   * @returns count as number
   */
  getPendingFileDeletionRequestsCount(user: Address) {
    const contract = this.getReadContract();
    return contract.read.getPendingFileDeletionRequestsCount?.([user]);
  }

  // -------- Writes --------

  /**
   * Create a new bucket.
   * @param mspId - 32-byte MSP ID (0x-prefixed hex)
   * @param name - bucket name as string (max 100 UTF-8 bytes)
   * @param isPrivate - true for private bucket
   * @param valuePropId - 32-byte value proposition ID (0x-prefixed hex)
   * @param options - optional gas and fee overrides
   */
  async createBucket(
    mspId: `0x${string}`,
    name: string,
    isPrivate: boolean,
    valuePropId: `0x${string}`,
    options?: EvmWriteOptions
  ) {
    const nameHex = this.validateStringLength(
      name,
      StorageHubClient.MAX_BUCKET_NAME_BYTES,
      "Bucket name"
    );
    const args = [mspId, nameHex, isPrivate, valuePropId] as const;
    const gasLimit = await this.estimateGas("createBucket", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.createBucket?.(args, txOpts);
  }

  /**
   * Request moving a bucket to a new MSP/value proposition.
   * @param bucketId - 32-byte bucket ID
   * @param newMspId - 32-byte new MSP ID
   * @param newValuePropId - 32-byte new value proposition ID
   * @param options - optional gas and fee overrides
   */
  async requestMoveBucket(
    bucketId: `0x${string}`,
    newMspId: `0x${string}`,
    newValuePropId: `0x${string}`,
    options?: EvmWriteOptions
  ) {
    const args = [bucketId, newMspId, newValuePropId] as const;
    const gasLimit = await this.estimateGas("requestMoveBucket", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.requestMoveBucket?.(args, txOpts);
  }

  /**
   * Update bucket privacy flag.
   * @param bucketId - 32-byte bucket ID
   * @param isPrivate - true for private
   * @param options - optional gas and fee overrides
   */
  async updateBucketPrivacy(
    bucketId: `0x${string}`,
    isPrivate: boolean,
    options?: EvmWriteOptions
  ) {
    const args = [bucketId, isPrivate] as const;
    const gasLimit = await this.estimateGas("updateBucketPrivacy", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.updateBucketPrivacy?.(args, txOpts);
  }

  /**
   * Create and associate a collection with a bucket.
   * @param bucketId - 32-byte bucket ID
   * @param options - optional gas and fee overrides
   */
  async createAndAssociateCollectionWithBucket(bucketId: `0x${string}`, options?: EvmWriteOptions) {
    const args = [bucketId] as const;
    const gasLimit = await this.estimateGas(
      "createAndAssociateCollectionWithBucket",
      args,
      options
    );
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.createAndAssociateCollectionWithBucket?.(args, txOpts);
  }

  /**
   * Delete an empty bucket.
   * @param bucketId - 32-byte bucket ID
   * @param options - optional gas and fee overrides
   */
  async deleteBucket(bucketId: `0x${string}`, options?: EvmWriteOptions) {
    const args = [bucketId] as const;
    const gasLimit = await this.estimateGas("deleteBucket", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.deleteBucket?.(args, txOpts);
  }

  /**
   * Issue a storage request for a file.
   * @param bucketId - 32-byte bucket ID
   * @param location - file path as string (max 512 UTF-8 bytes)
   * @param fingerprint - 32-byte file fingerprint
   * @param size - file size as bigint (storage units)
   * @param mspId - 32-byte MSP ID
   * @param peerIds - array of peer ID strings (max 5 entries, each max 100 UTF-8 bytes)
   * @param replicationLevel - replication level
   * @param replicas - number of replicas (only required for ReplicationLevel.Custom)
   * @param options - optional gas and fee overrides
   */
  async issueStorageRequest(
    bucketId: `0x${string}`,
    location: string,
    fingerprint: `0x${string}`,
    size: bigint,
    mspId: `0x${string}`,
    peerIds: string[],
    replicationLevel: ReplicationLevel,
    replicas: number,
    options?: EvmWriteOptions
  ) {
    const locationHex = this.validateStringLength(
      location,
      StorageHubClient.MAX_LOCATION_BYTES,
      "File location"
    );
    const peerIdsHex = peerIds.map((peerId, i) =>
      this.validateStringLength(peerId, StorageHubClient.MAX_PEER_ID_BYTES, `Peer ID ${i + 1}`)
    );
    const args = [
      bucketId,
      locationHex,
      fingerprint,
      size,
      mspId,
      peerIdsHex,
      replicationLevel,
      replicas
    ] as const;
    const gasLimit = await this.estimateGas("issueStorageRequest", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.issueStorageRequest?.(args, txOpts);
  }

  /**
   * Revoke a pending storage request by file key.
   * @param fileKey - 32-byte file key
   * @param options - optional gas and fee overrides
   */
  async revokeStorageRequest(fileKey: `0x${string}`, options?: EvmWriteOptions) {
    const args = [fileKey] as const;
    const gasLimit = await this.estimateGas("revokeStorageRequest", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.revokeStorageRequest?.(args, txOpts);
  }

  /**
   * Request deletion of a file from the network.
   * @param fileInfo File information containing all required data
   * @param operation File operation to perform (defaults to Delete)
   * @param options Optional transaction options
   * @returns Transaction hash
   */
  async requestDeleteFile(
    fileInfo: FileInfo,
    operation: FileOperation = FileOperation.Delete,
    options?: EvmWriteOptions,
  ): Promise<`0x${string}`> {
    // Create signed intention and execute transaction
    const { signedIntention, signature } = await this.signIntention(fileInfo.fileKey, operation);
    const locationHex = this.validateStringLength(
      fileInfo.location,
      StorageHubClient.MAX_LOCATION_BYTES,
      'File location',
    );
    const args = [
      signedIntention,
      signature,
      fileInfo.bucketId,
      locationHex,
      fileInfo.size,
      fileInfo.fingerprint,
    ] as const;

    const gasLimit = await this.estimateGas("requestDeleteFile", args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.requestDeleteFile?.(args, txOpts);
  }
}

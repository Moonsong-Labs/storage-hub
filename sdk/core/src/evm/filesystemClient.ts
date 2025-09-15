/**
 * FileSystemClient wrapper
 *
 * Provides ergonomic read/write methods for the FileSystem precompile using viem.
 * Internally relies on getFileSystemContract and the precompile address.
 *
 * All arguments are strongly typed. Binary data should be passed as Uint8Array (e.g., TextEncoder for strings).
 * Hex values should be 0x-prefixed strings (32-byte IDs like bucketId, mspId, etc.).
 */

import { toHex, parseGwei, type Address, type PublicClient, type WalletClient } from 'viem';
import { getFileSystemContract, type FileSystemContract, filesystemAbi, FILE_SYSTEM_PRECOMPILE_ADDRESS } from './filesystem';
import type { EvmWriteOptions } from './types';

export type FileSystemClientOptions = {
  client: PublicClient | WalletClient;
  /**
   * Optional separate public client for gas estimation.
   * If not provided, will attempt to use the main client for estimation.
   * Recommended for Frontier-based chains where wallet client estimation may be unreliable.
   */
  publicClient?: PublicClient;
};

export class FileSystemClient {
  private readonly contract: FileSystemContract<PublicClient> | FileSystemContract<WalletClient>;
  private readonly publicClient: PublicClient | undefined;
  private readonly client: PublicClient | WalletClient;
  private static readonly MAX_BUCKET_NAME_BYTES = 100;
  private static readonly DEFAULT_GAS_MULTIPLIER = 5;
  private static readonly DEFAULT_GAS_PRICE = parseGwei('1');
  private getRead(): NonNullable<typeof this.contract.read> {
    if (this.contract.read) return this.contract.read;
    throw new Error('FileSystemClient: read client not available');
  }
  private getReadMethod<K extends keyof NonNullable<typeof this.contract.read>>(name: K) {
    const r = this.getRead();
    const m = r[name];
    if (!m) throw new Error(`FileSystemClient: read method ${String(name)} unavailable`);
    return m as NonNullable<typeof r[K]>;
  }
  private getWalletClient(): WalletClient | undefined {
    // Check if the client is a WalletClient (has account property)
    return 'account' in this.client ? (this.client as WalletClient) : undefined;
  }

  /**
   * @deprecated This method will be removed in Phase 4 when all write methods are updated
   * Temporary method to support legacy write methods that haven't been updated yet
   */
  private getWriteMethod<K extends keyof NonNullable<typeof this.contract.write>>(name: K) {
    if (!('write' in this.contract) || !this.contract.write) {
      throw new Error('FileSystemClient: WalletClient required for write operations');
    }
    const m = this.contract.write[name];
    if (!m) throw new Error(`FileSystemClient: write method ${String(name)} unavailable`);
    return m as NonNullable<NonNullable<typeof this.contract.write>[K]>;
  }

  /**
   * Reusable gas estimation for any contract method.
   * Handles automatic estimation with safety multiplier or uses explicit gas.
   */
  private async estimateGas(
    functionName: string,
    args: readonly unknown[],
    options?: EvmWriteOptions
  ): Promise<bigint> {
    if (options?.gas) {
      // User provided explicit gas limit
      return options.gas;
    }

    // Automatic gas estimation with safety multiplier
    const wc = this.getWalletClient();
    if (!wc) throw new Error('FileSystemClient: WalletClient required for gas estimation');

    const accountAddr: any = (wc as any).account?.address;
    const estimationClient = this.publicClient || (this.contract as any).client;

    const gasEst: bigint = await estimationClient.estimateContractGas({
      address: FILE_SYSTEM_PRECOMPILE_ADDRESS,
      abi: filesystemAbi,
      functionName,
      args: args as any,
      account: accountAddr,
    });

    const mult = options?.gasMultiplier ?? FileSystemClient.DEFAULT_GAS_MULTIPLIER;
    return gasEst * BigInt(Math.max(1, Math.floor(mult)));
  }

  /**
   * Build transaction options with gas and fee settings.
   * Handles both legacy and EIP-1559 fee structures.
   */
  private buildTxOptions(gasLimit: bigint, options?: EvmWriteOptions): Record<string, unknown> {
    const useEip1559 = options?.maxFeePerGas !== undefined || options?.maxPriorityFeePerGas !== undefined;
    const txOpts: Record<string, unknown> = { gas: gasLimit };

    if (useEip1559) {
      // User wants EIP-1559 fees
      if (options?.maxFeePerGas) txOpts.maxFeePerGas = options.maxFeePerGas;
      if (options?.maxPriorityFeePerGas) txOpts.maxPriorityFeePerGas = options.maxPriorityFeePerGas;
    } else {
      // Default to legacy gas pricing (better for Frontier chains)
      txOpts.gasPrice = options?.gasPrice ?? FileSystemClient.DEFAULT_GAS_PRICE;
    }

    return txOpts;
  }
  private static assertMaxBytes(bytes: Uint8Array, max: number, label: string) {
    if (bytes.length > max) {
      throw new Error(`${label} exceeds maximum length of ${max} bytes`);
    }
  }

  /**
   * Create a FileSystem client bound to the precompile address.
   * @param opts.client - viem PublicClient (reads) or WalletClient (writes)
   * @param opts.publicClient - optional separate PublicClient for gas estimation (recommended for Frontier chains)
   */
  constructor(opts: FileSystemClientOptions) {
    // For now we always use the precompile address; address option reserved for future flexibility.
    this.contract = getFileSystemContract(opts.client);
    this.publicClient = opts.publicClient;
    this.client = opts.client;
  }

  // -------- Reads --------

  /**
   * Derive a bucket ID deterministically from owner + name.
   * @param owner - EVM address of the bucket owner
   * @param name - bucket name as bytes (max 100 bytes). Use TextEncoder for UTF-8 strings
   * @returns bucketId as 0x-prefixed 32-byte hex
   */
  deriveBucketId(owner: Address, name: Uint8Array) {
    FileSystemClient.assertMaxBytes(name, FileSystemClient.MAX_BUCKET_NAME_BYTES, 'Bucket name');
    return this.getReadMethod('deriveBucketId')([owner, toHex(name)]);
  }

  /**
   * Get how many file deletion requests a user currently has pending.
   * @param user - user EVM address
   * @returns count as number
   */
  getPendingFileDeletionRequestsCount(user: Address) {
    return this.getReadMethod('getPendingFileDeletionRequestsCount')([user]);
  }

  // -------- Writes --------

  /**
   * Create a new bucket.
   * 
   * Gas estimation and fees are handled automatically with sensible defaults.
   * The SDK will estimate gas and apply a 5x safety multiplier, using 1 gwei gas price.
   * 
   * @param mspId - 32-byte MSP ID (0x-prefixed hex)
   * @param name - bucket name bytes (<= 100 bytes)
   * @param isPrivate - true for private bucket
   * @param valuePropId - 32-byte value proposition ID (0x-prefixed hex)
   * @param options - optional gas and fee overrides
   * 
   * @example
   * // Simple usage (automatic gas estimation)
   * const txHash = await fs.createBucket(mspId, bucketName, false, valuePropId);
   * 
   * @example
   * // With custom gas options
   * const txHash = await fs.createBucket(mspId, bucketName, false, valuePropId, {
   *   gasMultiplier: 8,
   *   gasPrice: parseGwei('2')
   * });
   */
  async createBucket(
    mspId: `0x${string}`,
    name: Uint8Array,
    isPrivate: boolean,
    valuePropId: `0x${string}`,
    options?: EvmWriteOptions,
  ) {
    FileSystemClient.assertMaxBytes(name, FileSystemClient.MAX_BUCKET_NAME_BYTES, 'Bucket name');
    const nameHex = toHex(name);
    const args = [mspId, nameHex, isPrivate, valuePropId] as const;

    const wc = this.getWalletClient();
    if (!wc) throw new Error('FileSystemClient: WalletClient required for createBucket');

    // Use reusable gas estimation and transaction option builders
    const gasLimit = await this.estimateGas('createBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Direct contract access pattern (proven to work)
    const directContract = getFileSystemContract(wc);
    return (directContract as any).write.createBucket(args, txOpts);
  }

  /**
   * Request moving a bucket to a new MSP/value proposition.
   * 
   * Gas estimation and fees are handled automatically with sensible defaults.
   * 
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

    const wc = this.getWalletClient();
    if (!wc) throw new Error('FileSystemClient: WalletClient required for requestMoveBucket');

    // Reuse the same gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('requestMoveBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const directContract = getFileSystemContract(wc);
    return (directContract as any).write.requestMoveBucket(args, txOpts);
  }

  /**
   * Update bucket privacy flag.
   * @param bucketId - 32-byte bucket ID
   * @param isPrivate - true for private
   */
  updateBucketPrivacy(bucketId: `0x${string}`, isPrivate: boolean) {
    return this.getWriteMethod('updateBucketPrivacy')([bucketId, isPrivate]);
  }

  /**
   * Create and associate a collection with a bucket.
   * @param bucketId - 32-byte bucket ID
   */
  createAndAssociateCollectionWithBucket(bucketId: `0x${string}`) {
    return this.getWriteMethod('createAndAssociateCollectionWithBucket')([bucketId]);
  }

  /**
   * Delete an empty bucket.
   * @param bucketId - 32-byte bucket ID
   */
  deleteBucket(bucketId: `0x${string}`) {
    return this.getWriteMethod('deleteBucket')([bucketId]);
  }

  /**
   * Issue a storage request for a file.
   * @param bucketId - 32-byte bucket ID
   * @param location - file path bytes (<= 512 bytes)
   * @param fingerprint - 32-byte file fingerprint
   * @param size - file size as bigint (storage units)
   * @param mspId - 32-byte MSP ID
   * @param peerIds - array of peer id bytes (<= 5 entries, each <= 100 bytes)
   * @param replicationTarget - 0 Basic, 1 Standard, 2 HighSecurity, 3 SuperHighSecurity, 4 UltraHighSecurity, 5 Custom
   * @param customReplicationTarget - required if replicationTarget = 5 (Custom)
   */
  issueStorageRequest(
    bucketId: `0x${string}`,
    location: Uint8Array,
    fingerprint: `0x${string}`,
    size: bigint,
    mspId: `0x${string}`,
    peerIds: Uint8Array[],
    replicationTarget: number,
    customReplicationTarget: number
  ) {
    return this.getWriteMethod('issueStorageRequest')([
      bucketId,
      toHex(location),
      fingerprint,
      size,
      mspId,
      peerIds.map((p) => toHex(p)),
      replicationTarget,
      customReplicationTarget,
    ]);
  }

  /**
   * Revoke a pending storage request by file key.
   * @param fileKey - 32-byte file key
   */
  revokeStorageRequest(fileKey: `0x${string}`) {
    return this.getWriteMethod('revokeStorageRequest')([fileKey]);
  }

  /**
   * Request deletion of a file using a signed intention.
   * @param signedIntention - tuple [fileKey: 0x32, operation: number] where operation must be 0 (Delete)
   * @param signature - 65-byte secp256k1 signature over the SCALE-encoded intention
   * @param bucketId - 32-byte bucket ID
   * @param location - file path bytes (<= 512 bytes)
   * @param size - file size as bigint (storage units)
   * @param fingerprint - 32-byte file fingerprint
   */
  requestDeleteFile(
    signedIntention: readonly [`0x${string}`, number],
    signature: Uint8Array,
    bucketId: `0x${string}`,
    location: Uint8Array,
    size: bigint,
    fingerprint: `0x${string}`
  ) {
    return this.getWriteMethod('requestDeleteFile')([
      signedIntention,
      toHex(signature),
      bucketId,
      toHex(location),
      size,
      fingerprint,
    ]);
  }
}



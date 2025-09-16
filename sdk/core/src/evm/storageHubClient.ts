/**
 * StorageHubClient - Unified EVM client for StorageHub blockchain
 * 
 * Provides ergonomic read/write methods for StorageHub precompiles using viem.
 * Handles gas estimation automatically with Frontier-optimized defaults.
 * 
 * All arguments are strongly typed. String data (names, paths) are passed as strings and encoded internally.
 * Binary data (signatures) are passed as Uint8Array. Hex values are 0x-prefixed strings (32-byte IDs).
 */

import { FILE_SYSTEM_PRECOMPILE_ADDRESS, filesystemAbi, type FileSystemContract, getFileSystemContract } from './filesystem';
import type { EvmWriteOptions, StorageHubClientOptions } from './types';
import { type Address, createPublicClient, http, parseGwei, type PublicClient, stringToBytes, stringToHex, toHex, type WalletClient, type GetContractReturnType } from 'viem';


export class StorageHubClient {
  private readonly publicClient: PublicClient;   // Internal for gas estimation
  private readonly walletClient: WalletClient;   // User-provided
  private readonly contract: FileSystemContract<PublicClient>;  // For reads
  private static readonly MAX_BUCKET_NAME_BYTES = 100;
  private static readonly MAX_LOCATION_BYTES = 512;
  private static readonly MAX_PEER_ID_BYTES = 100;
  private static readonly DEFAULT_GAS_MULTIPLIER = 5;
  private static readonly DEFAULT_GAS_PRICE = parseGwei('1');

  /**
   * Get write contract instance bound to the wallet client.
   * 
   * @returns Contract instance for write operations (transactions)
   */
  private getWriteContract() {
    return getFileSystemContract(this.walletClient);
  }

  /**
   * Get read contract instance bound to the public client.
   * 
   * @returns Contract instance for read operations (view calls)
   */
  private getReadContract() {
    return getFileSystemContract(this.publicClient);
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
      address: FILE_SYSTEM_PRECOMPILE_ADDRESS,
      abi: filesystemAbi,
      functionName,
      args,
      account: accountAddr,
    });

    const multiplier = options?.gasMultiplier ?? StorageHubClient.DEFAULT_GAS_MULTIPLIER;
    return gasEstimation * BigInt(Math.max(1, Math.floor(multiplier)));
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
   * Create a StorageHub client with automatic gas estimation.
   * 
   * @param opts.rpcUrl - RPC endpoint URL for the StorageHub chain
   * @param opts.chain - Viem chain configuration
   * @param opts.walletClient - Wallet client for transaction signing
   */
  constructor(opts: StorageHubClientOptions) {
    // Create internal PublicClient for gas estimation
    this.publicClient = createPublicClient({
      chain: opts.chain,
      transport: http(opts.rpcUrl)
    });
    this.walletClient = opts.walletClient;

    // Use PublicClient for the contract (reads only)
    this.contract = getFileSystemContract(this.publicClient);
  }

  // -------- Reads --------

  /**
   * Derive a bucket ID deterministically from owner + name.
   * @param owner - EVM address of the bucket owner
   * @param name - bucket name as string (max 100 UTF-8 bytes)
   * @returns bucketId as 0x-prefixed 32-byte hex
   */
  deriveBucketId(owner: Address, name: string) {
    const nameHex = this.validateStringLength(name, StorageHubClient.MAX_BUCKET_NAME_BYTES, 'Bucket name');
    const contract = this.getReadContract();
    return contract.read.deriveBucketId!([owner, nameHex]);
  }

  /**
   * Get how many file deletion requests a user currently has pending.
   * @param user - user EVM address
   * @returns count as number
   */
  getPendingFileDeletionRequestsCount(user: Address) {
    const contract = this.getReadContract();
    return contract.read.getPendingFileDeletionRequestsCount!([user]);
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
    options?: EvmWriteOptions,
  ) {
    const nameHex = this.validateStringLength(name, StorageHubClient.MAX_BUCKET_NAME_BYTES, 'Bucket name');
    const args = [mspId, nameHex, isPrivate, valuePropId] as const;
    const gasLimit = await this.estimateGas('createBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.createBucket!(args, txOpts);
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
    const gasLimit = await this.estimateGas('requestMoveBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.requestMoveBucket!(args, txOpts);
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
    const gasLimit = await this.estimateGas('updateBucketPrivacy', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.updateBucketPrivacy!(args, txOpts);
  }

  /**
   * Create and associate a collection with a bucket.
   * @param bucketId - 32-byte bucket ID
   * @param options - optional gas and fee overrides
   */
  async createAndAssociateCollectionWithBucket(
    bucketId: `0x${string}`,
    options?: EvmWriteOptions
  ) {
    const args = [bucketId] as const;
    const gasLimit = await this.estimateGas('createAndAssociateCollectionWithBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.createAndAssociateCollectionWithBucket!(args, txOpts);
  }

  /**
   * Delete an empty bucket.
   * @param bucketId - 32-byte bucket ID
   * @param options - optional gas and fee overrides
   */
  async deleteBucket(
    bucketId: `0x${string}`,
    options?: EvmWriteOptions
  ) {
    const args = [bucketId] as const;
    const gasLimit = await this.estimateGas('deleteBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.deleteBucket!(args, txOpts);
  }

  /**
   * Issue a storage request for a file.
   * @param bucketId - 32-byte bucket ID
   * @param location - file path as string (max 512 UTF-8 bytes)
   * @param fingerprint - 32-byte file fingerprint
   * @param size - file size as bigint (storage units)
   * @param mspId - 32-byte MSP ID
   * @param peerIds - array of peer ID strings (max 5 entries, each max 100 UTF-8 bytes)
   * @param replicationTarget - 0 Basic, 1 Standard, 2 HighSecurity, 3 SuperHighSecurity, 4 UltraHighSecurity, 5 Custom
   * @param customReplicationTarget - required if replicationTarget = 5 (Custom)
   * @param options - optional gas and fee overrides
   */
  async issueStorageRequest(
    bucketId: `0x${string}`,
    location: string,
    fingerprint: `0x${string}`,
    size: bigint,
    mspId: `0x${string}`,
    peerIds: string[],
    replicationTarget: number,
    customReplicationTarget: number,
    options?: EvmWriteOptions
  ) {
    const locationHex = this.validateStringLength(location, StorageHubClient.MAX_LOCATION_BYTES, 'File location');
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
      replicationTarget,
      customReplicationTarget,
    ] as const;
    const gasLimit = await this.estimateGas('issueStorageRequest', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.issueStorageRequest!(args, txOpts);
  }

  /**
   * Revoke a pending storage request by file key.
   * @param fileKey - 32-byte file key
   * @param options - optional gas and fee overrides
   */
  async revokeStorageRequest(
    fileKey: `0x${string}`,
    options?: EvmWriteOptions
  ) {
    const args = [fileKey] as const;
    const gasLimit = await this.estimateGas('revokeStorageRequest', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.revokeStorageRequest!(args, txOpts);
  }

  /**
   * Request deletion of a file using a signed intention.
   * @param signedIntention - tuple [fileKey: 0x32, operation: number] where operation must be 0 (Delete)
   * @param signature - 65-byte secp256k1 signature over the SCALE-encoded intention
   * @param bucketId - 32-byte bucket ID
   * @param location - file path as string (max 512 UTF-8 bytes)
   * @param size - file size as bigint (storage units)
   * @param fingerprint - 32-byte file fingerprint
   * @param options - optional gas and fee overrides
   */
  async requestDeleteFile(
    signedIntention: readonly [`0x${string}`, number],
    signature: Uint8Array,
    bucketId: `0x${string}`,
    location: string,
    size: bigint,
    fingerprint: `0x${string}`,
    options?: EvmWriteOptions
  ) {
    const signatureHex = toHex(signature);
    const locationHex = this.validateStringLength(location, StorageHubClient.MAX_LOCATION_BYTES, 'File location');
    const args = [
      signedIntention,
      signatureHex,
      bucketId,
      locationHex,
      size,
      fingerprint,
    ] as const;

    const gasLimit = await this.estimateGas('requestDeleteFile', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    const contract = this.getWriteContract();
    return await contract.write.requestDeleteFile!(args, txOpts);
  }
}

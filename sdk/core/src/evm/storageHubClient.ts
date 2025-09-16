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
   * Get any contract method - reads and writes handled automatically.
   * Universal wrapper that auto-detects and returns the correct method.
   * 
   * @param methodName - Name of the contract method
   * @returns Validated contract method ready to call
   */
  private getContract(methodName: string) {
    const contract = getFileSystemContract(this.walletClient);

    // Try read methods first (cheaper), then write methods
    // Safe to use ! operators: we validate method exists below
    const readMethod = contract.read?.[methodName as keyof typeof contract.read];
    const writeMethod = contract.write?.[methodName as keyof typeof contract.write];

    const method = readMethod || writeMethod;
    if (!method) {
      throw new Error(`StorageHubClient: method ${methodName} not available`);
    }

    return method;
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
   * 
   * @example
   * // Automatic estimation with 5x multiplier
   * const gas = await this.estimateGas('createBucket', args);
   * 
   * @example  
   * // Custom multiplier for complex operations
   * const gas = await this.estimateGas('createBucket', args, { gasMultiplier: 8 });
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

    // Automatic gas estimation using internal PublicClient
    const accountAddr = this.walletClient.account?.address;

    const gasEst: bigint = await this.publicClient.estimateContractGas({
      address: FILE_SYSTEM_PRECOMPILE_ADDRESS,
      abi: filesystemAbi,
      functionName,
      args,
      account: accountAddr,
    });

    const mult = options?.gasMultiplier ?? StorageHubClient.DEFAULT_GAS_MULTIPLIER;
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

  private static assertMaxBytes(bytes: Uint8Array, max: number, label: string) {
    if (bytes.length > max) {
      throw new Error(`${label} exceeds maximum length of ${max} bytes`);
    }
  }

  /**
   * Create a StorageHub client with automatic gas estimation.
   * 
   * @param opts.rpcUrl - RPC endpoint URL for the StorageHub chain
   * @param opts.chain - Viem chain configuration
   * @param opts.walletClient - Wallet client for transaction signing
   * 
   * @example
   * const hub = new StorageHubClient({
   *   rpcUrl: 'http://localhost:9944',
   *   chain: storageHubChain,
   *   walletClient: myWalletClient
   * });
   */
  constructor(opts: StorageHubClientOptions) {
    // Create internal PublicClient for reliable gas estimation
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
    const deriveBucketId = this.getContract('deriveBucketId');
    return deriveBucketId([owner, nameHex]);
  }

  /**
   * Get how many file deletion requests a user currently has pending.
   * @param user - user EVM address
   * @returns count as number
   */
  getPendingFileDeletionRequestsCount(user: Address) {
    const getPendingFileDeletionRequestsCount = this.getContract('getPendingFileDeletionRequestsCount');
    return getPendingFileDeletionRequestsCount([user]);
  }

  // -------- Writes --------

  /**
   * Create a new bucket.
   * 
   * Gas estimation and fees are handled automatically with sensible defaults.
   * The SDK will estimate gas and apply a 5x safety multiplier, using 1 gwei gas price.
   * 
   * @param mspId - 32-byte MSP ID (0x-prefixed hex)
   * @param name - bucket name as string (max 100 UTF-8 bytes)
   * @param isPrivate - true for private bucket
   * @param valuePropId - 32-byte value proposition ID (0x-prefixed hex)
   * @param options - optional gas and fee overrides
   * 
   * @example
   * // Simple usage (automatic gas estimation)
   * const txHash = await hub.createBucket(mspId, "my-bucket", false, valuePropId);
   * 
   * @example
   * // With custom gas options
   * const txHash = await hub.createBucket(mspId, "my-bucket", false, valuePropId, {
   *   gasMultiplier: 8,
   *   gasPrice: parseGwei('2')
   * });
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

    // Use reusable gas estimation and transaction option builders
    const gasLimit = await this.estimateGas('createBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const createBucket = this.getContract('createBucket');
    return createBucket(args, txOpts);
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

    // Reuse the same gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('requestMoveBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const requestMoveBucket = this.getContract('requestMoveBucket');
    return requestMoveBucket(args, txOpts);
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

    // Use reusable gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('updateBucketPrivacy', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const updateBucketPrivacy = this.getContract('updateBucketPrivacy');
    return updateBucketPrivacy(args, txOpts);
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

    // Use reusable gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('createAndAssociateCollectionWithBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const createAndAssociateCollectionWithBucket = this.getContract('createAndAssociateCollectionWithBucket');
    return createAndAssociateCollectionWithBucket(args, txOpts);
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

    // Use reusable gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('deleteBucket', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const deleteBucket = this.getContract('deleteBucket');
    return deleteBucket(args, txOpts);
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
    // Input validation and hex encoding
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

    // Use reusable gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('issueStorageRequest', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const issueStorageRequest = this.getContract('issueStorageRequest');
    return issueStorageRequest(args, txOpts);
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

    // Use reusable gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('revokeStorageRequest', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const revokeStorageRequest = this.getContract('revokeStorageRequest');
    return revokeStorageRequest(args, txOpts);
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
    // Input validation and hex encoding
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

    // Use reusable gas estimation and transaction building logic
    const gasLimit = await this.estimateGas('requestDeleteFile', args, options);
    const txOpts = this.buildTxOptions(gasLimit, options);

    // Use unified contract wrapper - get the exact method ready to call
    const requestDeleteFile = this.getContract('requestDeleteFile');
    return requestDeleteFile(args, txOpts);
  }
}

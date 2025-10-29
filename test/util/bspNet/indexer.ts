import type { SqlClient } from "../index";
import { hexToBuffer } from "../indexerHelpers";

/**
 * Options for waitForIndexing
 */
export interface WaitForIndexingOptions {
  /** The indexer API (the node doing the indexing/finalization) */
  indexerApi: any; // EnrichedBspApi type (avoiding circular dependency)
  /** Optional. The producer API to get block number and finalization status from. Defaults to indexerApi if not provided (for embedded indexer scenarios). */
  producerApi?: any;
  /** Whether to seal a new block on the producer (default: true) */
  sealBlock?: boolean;
  /** Whether to finalize blocks on the indexer node (default: true) */
  finalizeOnIndexer?: boolean;
}

/**
 * Waits for the indexer to process blocks.
 *
 * This function is the implementation for `api.indexer.waitForIndexing()` and should
 * typically be called via that method rather than directly.
 *
 * # Behavior
 *
 * 1. Optionally seals a new block on the producer node
 * 2. Gets the current block number from the producer node
 * 3. Waits for indexer to process the block via docker logs
 * 4. If `finalizeOnIndexer` is `true`:
 *    - Gets the finalized block hash from the producer chain
 *    - Waits for indexer to import that block
 *    - Explicitly finalizes the block on the indexer node
 *
 * This is necessary because non-producer nodes (like standalone indexers)
 * must explicitly finalize imported blocks to trigger the indexer's
 * finality notification stream.
 */
export const waitForIndexing = async (options: WaitForIndexingOptions): Promise<void> => {
  const { indexerApi, producerApi, sealBlock = true, finalizeOnIndexer = true } = options;

  // Default to indexerApi when producerApi is not provided
  const blockProducerApi = producerApi ?? indexerApi;

  if (sealBlock) {
    await blockProducerApi.block.seal();
  }

  const currentBlock = (await blockProducerApi.query.system.number()).toNumber();

  // Finalize blocks on indexer node BEFORE waiting for indexing log
  // This is necessary because the indexer only processes finalized blocks
  if (finalizeOnIndexer) {
    const finalisedBlockHash = await blockProducerApi.rpc.chain.getFinalizedHead();
    await indexerApi.wait.blockImported(finalisedBlockHash.toString());
    await indexerApi.block.finaliseBlock(finalisedBlockHash.toString());
  }

  // Wait for indexer to process this block (after finalization)
  // Use the indexer's container name from its API connection
  await indexerApi.docker.waitForLog({
    searchString: `Indexing block #${currentBlock}:`,
    containerName: indexerApi.shConsts.NODE_INFOS.indexer.containerName,
    timeout: 30000
  });
};

/**
 * Options for verifyFileIndexed
 */
export interface VerifyFileIndexedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The name of the bucket containing the file */
  bucketName: string;
  /** The file key to verify */
  fileKey: string;
}

/**
 * Verifies that a file has been indexed in the database.
 *
 * @returns The indexed file record from the database.
 * @throws Error if the file is not found or the file key doesn't match.
 */
export const verifyFileIndexed = async (options: VerifyFileIndexedOptions) => {
  const { sql, bucketName, fileKey } = options;

  const files = await sql`
    SELECT * FROM file
    WHERE bucket_id = (
      SELECT id FROM bucket WHERE name = ${bucketName}
    )
  `;

  if (files.length === 0) {
    throw new Error(`No file found for bucket: ${bucketName}`);
  }

  const dbFileKey = `0x${files[0].file_key.toString("hex")}`;
  if (dbFileKey !== fileKey) {
    throw new Error(`File key mismatch. Expected: ${fileKey}, Got: ${dbFileKey}`);
  }

  return files[0];
};

/**
 * Options for verifyProviderAssociation
 */
export interface VerifyProviderAssociationOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check association for */
  fileKey: string;
  /** The provider ID to verify association with */
  providerId: string;
  /** The type of provider ("msp" or "bsp") */
  providerType: "msp" | "bsp";
}

/**
 * Verifies that a provider association exists in the database.
 *
 * @returns The provider association record from the database.
 * @throws Error if no association is found.
 */
export const verifyProviderAssociation = async (options: VerifyProviderAssociationOptions) => {
  const { sql, fileKey, providerId, providerType } = options;

  const tableName = providerType === "msp" ? "msp_file" : "bsp_file";
  const columnName = providerType === "msp" ? "msp_id" : "bsp_id";

  const associations = await sql`
    SELECT * FROM ${sql(tableName)}
    WHERE file_key = ${hexToBuffer(fileKey)}
    AND ${sql(columnName)} = ${hexToBuffer(providerId)}
  `;

  if (associations.length === 0) {
    throw new Error(
      `No ${providerType.toUpperCase()} association found for file ${fileKey} and provider ${providerId}`
    );
  }

  return associations[0];
};

/**
 * Options for waitForFishermanBatchDeletions
 */
export interface WaitForFishermanBatchDeletionsOptions {
  /** The enriched BSP API */
  api: any; // EnrichedBspApi type (avoiding circular dependency)
  /** Either "User" or "Incomplete" to determine which deletion cycle to wait for */
  deletionType: "User" | "Incomplete";
  /** Optional. The number of expected extrinsics to verify in the transaction pool */
  expectExt?: number;
  /** Optional. Whether to seal a block after verifying extrinsics. Defaults to false. */
  sealBlock?: boolean;
}

/**
 * Waits for fisherman to process batch deletions by sealing blocks until
 * the fisherman submits extrinsics for the specified deletion type.
 *
 * This handles the alternating User/Incomplete deletion cycle timing issue
 * where fisherman might be on the wrong cycle when deletions are created.
 *
 * The function uses a polling loop that:
 * 1. Seals a block
 * 2. Checks for the fisherman log message (with short timeout)
 * 3. If not found, waits and repeats
 * 4. Once found, optionally verifies extrinsics in tx pool
 *
 * If `expectExt` is provided, this function will verify that the expected
 * number of extrinsics are present in the transaction pool before returning.
 *
 * If `sealBlock` is true, a block will be sealed after verifying extrinsics.
 * Defaults to false to allow manual block sealing in tests.
 */
export const waitForFishermanBatchDeletions = async (
  options: WaitForFishermanBatchDeletionsOptions
): Promise<void> => {
  const { api, deletionType, expectExt, sealBlock = false } = options;

  const searchString =
    deletionType === "User"
      ? "🎣 Successfully submitted delete_files extrinsic for"
      : "🎣 Successfully submitted delete_files_for_incomplete_storage_request extrinsic for";

  // Poll for fisherman log message, sealing blocks between checks
  // Wait 5 second interval (fisherman configuration "--fisherman-batch-interval-seconds=5")
  // to leave time for the fisherman to switch processing deletion types
  const maxAttempts = 5; // 5 attempts * 5 seconds = 25 seconds total timeout
  let found = false;

  for (let attempt = 0; attempt < maxAttempts && !found; attempt++) {
    // Seal a block to trigger fisherman processing
    await api.block.seal();

    // Check if fisherman has submitted the extrinsics (with short timeout to avoid blocking)
    try {
      await api.docker.waitForLog({
        searchString,
        containerName: "storage-hub-sh-fisherman-1",
        timeout: 5000 // 5 second timeout matches the fisherman batch interval
      });
      found = true;
    } catch (error) {
      // Log not found yet, continue to next iteration
      if (attempt === maxAttempts - 1) {
        throw new Error(
          `Timeout waiting for fisherman to process ${deletionType} deletions after ${maxAttempts * 5} seconds`
        );
      }
    }
  }

  // If expectExt is provided, verify extrinsics are in the tx pool
  if (expectExt !== undefined && expectExt > 0) {
    const extrinsicMethod =
      deletionType === "User" ? "deleteFiles" : "deleteFilesForIncompleteStorageRequest";

    await api.assert.extrinsicPresent({
      method: extrinsicMethod,
      module: "fileSystem",
      checkTxPool: true,
      assertLength: expectExt,
      timeout: 500 // This is a small timeout since the fisherman should have already submitted the extrinsics by this point
    });
  }

  // Optionally seal a block after verification
  if (sealBlock) {
    await api.block.seal();
  }
};

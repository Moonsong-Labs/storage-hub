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
 * Options for verifyDeletionSignaturesStored
 */
export interface VerifyDeletionSignaturesStoredOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** Array of file keys to verify have deletion signatures */
  fileKeys: string[];
}

/**
 * Verifies that deletion signatures are stored in the database for all specified file keys.
 *
 * This function waits for the first file to have a deletion signature stored, then verifies
 * that all files have non-empty SCALE-encoded deletion signatures in the database.
 *
 * @throws Error if any file doesn't have a deletion signature stored or if the signature is empty.
 */
export const verifyDeletionSignaturesStored = async (
  options: VerifyDeletionSignaturesStoredOptions
): Promise<void> => {
  const { sql, fileKeys } = options;

  // Wait for first file to have signature stored using waitFor utility
  const { waitFor } = await import("../index");
  await waitFor({
    lambda: async () => {
      const files = await sql`
        SELECT deletion_signature FROM file
        WHERE file_key = ${hexToBuffer(fileKeys[0])}
        AND deletion_signature IS NOT NULL
      `;
      return files.length > 0;
    }
  });

  // Verify all files have SCALE-encoded signatures
  for (const fileKey of fileKeys) {
    const filesWithSignature = await sql`
      SELECT deletion_signature FROM file
      WHERE file_key = ${hexToBuffer(fileKey)}
      AND deletion_signature IS NOT NULL
    `;

    if (filesWithSignature.length !== 1) {
      throw new Error(`File should have deletion signature stored: ${fileKey}`);
    }

    if (filesWithSignature[0].deletion_signature.length === 0) {
      throw new Error(`SCALE-encoded signature should not be empty for file: ${fileKey}`);
    }
  }
};

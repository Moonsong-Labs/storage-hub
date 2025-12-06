import type { EnrichedBspApi, SqlClient } from "../index";
import { waitFor } from "../index";
import assert from "node:assert";
import { hexToBuffer } from "./helpers";

/**
 * Options for waitForIndexing
 */
export interface WaitForIndexingOptions {
  /** The indexer API (the node doing the indexing/finalization) */
  indexerApi: EnrichedBspApi;
  /** The SQL client instance */
  sql: SqlClient;
  /** Optional. The producer API to get block number and finalization status from. Defaults to indexerApi if not provided (for embedded indexer scenarios). */
  producerApi?: EnrichedBspApi;
  /** Whether to seal a new block on the producer (default: true) */
  sealBlock?: boolean;
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
 * 4. Gets the finalized block hash from the producer chain
 * 5. Waits for indexer to import that block
 * 6. Explicitly finalizes the block on the indexer node
 */
export const waitForIndexing = async (options: WaitForIndexingOptions): Promise<void> => {
  const { indexerApi, sql, producerApi, sealBlock = false } = options;

  // Default to indexerApi when producerApi is not provided
  const blockProducerApi = producerApi ?? indexerApi;

  if (sealBlock) {
    // Ensure we seal a finalized block for the indexer to process
    await blockProducerApi.block.seal({ finaliseBlock: true });
  }

  // Get the finalized block hash and extract the block number from the header
  // Note: We use the finalized block number (not best block) because the indexer
  // only processes finalized blocks. Using system.number() would return the best block
  // which may not be finalized yet, causing the log wait to timeout.
  const finalisedBlockHash = await blockProducerApi.rpc.chain.getFinalizedHead();
  const finalisedHeader = await blockProducerApi.rpc.chain.getHeader(finalisedBlockHash);
  const finalisedBlockNumber = finalisedHeader.number.toNumber();

  // Finalize block on indexer node
  // This is necessary because the indexer only processes finalized blocks
  await indexerApi.wait.blockImported(finalisedBlockHash.toString());
  await indexerApi.block.finaliseBlock(finalisedBlockHash.toString());

  // Wait for indexer to process this block
  // Use the indexer's container name from its API connection
  await indexerApi.docker.waitForLog({
    searchString: `Indexing block #${finalisedBlockNumber}:`,
    containerName: indexerApi.shConsts.NODE_INFOS.indexer.containerName,
    timeout: 30000
  });

  // Wait for indexer to successfully index the block
  await waitFor({
    lambda: async () => {
      const lastIndexedBlock = await indexerApi.indexer.getLastIndexedBlock({ sql });
      return lastIndexedBlock === finalisedBlockNumber;
    }
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

/**
 * Options for waitForBlockIndexed
 */
export interface WaitForBlockIndexedOptions {
  /** The indexer API */
  api: any; // EnrichedBspApi type
  /** Optional block number to wait for. Defaults to current block. */
  blockNumber?: number;
}

/**
 * Waits for a specific block to be indexed by checking docker logs.
 *
 * @returns A Promise that resolves when the block has been indexed.
 */
export const waitForBlockIndexed = async (options: WaitForBlockIndexedOptions): Promise<void> => {
  const { api, blockNumber } = options;
  const targetBlock = blockNumber ?? (await api.query.system.number()).toNumber();

  await api.docker.waitForLog({
    searchString: `Indexing block #${targetBlock}:`,
    containerName: "storage-hub-sh-user-1",
    timeout: 10000
  });
};

/**
 * Options for waitForFileIndexed
 */
export interface WaitForFileIndexedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to wait for */
  fileKey: string;
}

/**
 * Waits for a file to be indexed in the database.
 *
 * @returns A Promise that resolves when the file is indexed.
 */
export const waitForFileIndexed = async (options: WaitForFileIndexedOptions): Promise<void> => {
  const { sql, fileKey } = options;
  await waitFor({
    lambda: async () => {
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;
      return files.length > 0;
    }
  });
};

/**
 * Options for waitForBucketIndexed
 */
export interface WaitForBucketIndexedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The bucket name to wait for */
  bucketName: string;
}

/**
 * Waits for a bucket to be indexed in the database by name.
 *
 * @returns A Promise that resolves when the bucket is indexed.
 */
export const waitForBucketIndexed = async (options: WaitForBucketIndexedOptions): Promise<void> => {
  const { sql, bucketName } = options;
  await waitFor({
    lambda: async () => {
      const buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      return buckets.length > 0;
    }
  });
};

/**
 * Options for waitForBucketByIdIndexed
 */
export interface WaitForBucketByIdIndexedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The onchain bucket ID to wait for */
  bucketId: string;
  /** Optional MSP ID filter */
  mspId?: string;
}

/**
 * Waits for a bucket to be indexed in the database by onchain bucket ID.
 *
 * @returns A Promise that resolves when the bucket is indexed.
 */
export const waitForBucketByIdIndexed = async (
  options: WaitForBucketByIdIndexedOptions
): Promise<void> => {
  const { sql, bucketId, mspId } = options;
  await waitFor({
    lambda: async () => {
      const query = mspId
        ? sql`
            SELECT * FROM bucket WHERE onchain_bucket_id = ${hexToBuffer(bucketId)} AND
            msp_id = ${mspId}
          `
        : sql`
            SELECT * FROM bucket WHERE onchain_bucket_id = ${hexToBuffer(bucketId)}
          `;
      const buckets = await query;
      return buckets.length > 0;
    },
    iterations: 30,
    delay: 1000
  });
};

/**
 * Options for waitForBucketDeleted
 */
export interface WaitForBucketDeletedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The onchain bucket ID to wait for deletion */
  bucketId: string;
}

/**
 * Waits for a bucket to be marked as deleted in the database.
 *
 * @returns A Promise that resolves when the bucket is marked as deleted.
 */
export const waitForBucketDeleted = async (options: WaitForBucketDeletedOptions): Promise<void> => {
  const { sql, bucketId } = options;
  await waitFor({
    lambda: async () => {
      const buckets = await sql`
        SELECT * FROM bucket WHERE onchain_bucket_id = ${hexToBuffer(bucketId)} AND deleted_at IS NOT NULL
      `;
      return buckets.length > 0;
    }
  });
};

/**
 * Options for waitForMspFileAssociation
 */
export interface WaitForMspFileAssociationOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check association for */
  fileKey: string;
  /** Optional MSP ID filter */
  mspId?: string;
}

/**
 * Waits for an MSP file association to be created in the database.
 *
 * @returns A Promise that resolves when the association exists.
 */
export const waitForMspFileAssociation = async (
  options: WaitForMspFileAssociationOptions
): Promise<void> => {
  const { sql, fileKey, mspId } = options;
  await waitFor({
    lambda: async () => {
      const query = mspId
        ? sql`
            SELECT mf.* FROM msp_file mf
            INNER JOIN file f ON mf.file_id = f.id
            INNER JOIN msp m ON mf.msp_id = m.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
            AND m.onchain_msp_id = ${mspId}
          `
        : sql`
            SELECT mf.* FROM msp_file mf
            INNER JOIN file f ON mf.file_id = f.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
          `;
      const files = await query;
      return files.length > 0;
    }
  });
};

/**
 * Options for waitForBspFileAssociation
 */
export interface WaitForBspFileAssociationOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check association for */
  fileKey: string;
  /** Optional BSP ID filter */
  bspId?: string;
}

/**
 * Waits for a BSP file association to be created in the database.
 *
 * @returns A Promise that resolves when the association exists.
 */
export const waitForBspFileAssociation = async (
  options: WaitForBspFileAssociationOptions
): Promise<void> => {
  const { sql, fileKey, bspId } = options;
  await waitFor({
    lambda: async () => {
      const query = bspId
        ? sql`
            SELECT bf.* FROM bsp_file bf
            INNER JOIN file f ON bf.file_id = f.id
            INNER JOIN bsp b ON bf.bsp_id = b.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
            AND b.onchain_bsp_id = ${bspId}
          `
        : sql`
            SELECT bf.* FROM bsp_file bf
            INNER JOIN file f ON bf.file_id = f.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
          `;
      const files = await query;
      return files.length > 0;
    }
  });
};

/**
 * Options for waitForFileDeleted
 */
export interface WaitForFileDeletedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to wait for deletion */
  fileKey: string;
}

/**
 * Waits for a file to be deleted from the database.
 *
 * @returns A Promise that resolves when the file is deleted.
 */
export const waitForFileDeleted = async (options: WaitForFileDeletedOptions): Promise<void> => {
  const { sql, fileKey } = options;
  await waitFor({
    lambda: async () => {
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;
      return files.length === 0;
    },
    iterations: 30,
    delay: 1000
  });
};

/**
 * Options for waitForBspFileAssociationRemoved
 */
export interface WaitForBspFileAssociationRemovedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check association for */
  fileKey: string;
  /** Optional BSP ID filter */
  bspId?: string;
}

/**
 * Waits for a BSP file association to be removed from the database.
 *
 * @returns A Promise that resolves when the association is removed.
 */
export const waitForBspFileAssociationRemoved = async (
  options: WaitForBspFileAssociationRemovedOptions
): Promise<void> => {
  const { sql, fileKey, bspId } = options;
  await waitFor({
    lambda: async () => {
      const query = bspId
        ? sql`
            SELECT bf.* FROM bsp_file bf
            INNER JOIN file f ON bf.file_id = f.id
            INNER JOIN bsp b ON bf.bsp_id = b.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
            AND b.onchain_bsp_id = ${bspId}
          `
        : sql`
            SELECT bf.* FROM bsp_file bf
            INNER JOIN file f ON bf.file_id = f.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
          `;
      const files = await query;
      return files.length === 0;
    }
  });
};

/**
 * Options for waitForMspFileAssociationRemoved
 */
export interface WaitForMspFileAssociationRemovedOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check association for */
  fileKey: string;
  /** Optional MSP ID filter */
  mspId?: string;
}

/**
 * Waits for an MSP file association to be removed from the database.
 *
 * @returns A Promise that resolves when the association is removed.
 */
export const waitForMspFileAssociationRemoved = async (
  options: WaitForMspFileAssociationRemovedOptions
): Promise<void> => {
  const { sql, fileKey, mspId } = options;
  await waitFor({
    lambda: async () => {
      const query = mspId
        ? sql`
            SELECT mf.* FROM msp_file mf
            INNER JOIN file f ON mf.file_id = f.id
            INNER JOIN msp m ON mf.msp_id = m.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
            AND m.onchain_msp_id = ${mspId}
          `
        : sql`
            SELECT mf.* FROM msp_file mf
            INNER JOIN file f ON mf.file_id = f.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
          `;
      const files = await query;
      return files.length === 0;
    }
  });
};

/**
 * Options for verifyNoBspFileAssociation
 */
export interface VerifyNoBspFileAssociationOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check */
  fileKey: string;
}

/**
 * Verifies that no BSP file associations exist for a given file.
 *
 * @throws Error if associations are found.
 */
export const verifyNoBspFileAssociation = async (
  options: VerifyNoBspFileAssociationOptions
): Promise<void> => {
  const { sql, fileKey } = options;
  const associations = await sql`
    SELECT bf.* FROM bsp_file bf
    INNER JOIN file f ON bf.file_id = f.id
    WHERE f.file_key = ${hexToBuffer(fileKey)}
  `;
  assert.strictEqual(
    associations.length,
    0,
    `Expected no BSP file associations for file ${fileKey}, but found ${associations.length}`
  );
};

/**
 * Options for verifyNoMspFileAssociation
 */
export interface VerifyNoMspFileAssociationOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The file key to check */
  fileKey: string;
}

/**
 * Verifies that no MSP file associations exist for a given file.
 *
 * @throws Error if associations are found.
 */
export const verifyNoMspFileAssociation = async (
  options: VerifyNoMspFileAssociationOptions
): Promise<void> => {
  const { sql, fileKey } = options;
  const associations = await sql`
    SELECT mf.* FROM msp_file mf
    INNER JOIN file f ON mf.file_id = f.id
    WHERE f.file_key = ${hexToBuffer(fileKey)}
  `;
  assert.strictEqual(
    associations.length,
    0,
    `Expected no MSP file associations for file ${fileKey}, but found ${associations.length}`
  );
};

/**
 * Options for verifyNoOrphanedBspAssociations
 */
export interface VerifyNoOrphanedBspAssociationsOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The BSP ID to check */
  bspId: string;
}

/**
 * Verifies that no orphaned BSP associations exist (associations without corresponding files).
 *
 * @throws Error if orphaned associations are found.
 */
export const verifyNoOrphanedBspAssociations = async (
  options: VerifyNoOrphanedBspAssociationsOptions
): Promise<void> => {
  const { sql, bspId } = options;
  const orphanedAssociations = await sql`
    SELECT bf.* FROM bsp_file bf
    INNER JOIN bsp b ON bf.bsp_id = b.id
    LEFT JOIN file f ON bf.file_id = f.id
    WHERE b.onchain_bsp_id = ${bspId} AND f.id IS NULL
  `;
  assert.strictEqual(
    orphanedAssociations.length,
    0,
    `Expected no orphaned BSP associations for BSP ${bspId}, but found ${orphanedAssociations.length}`
  );
};

/**
 * Options for verifyNoOrphanedMspAssociations
 */
export interface VerifyNoOrphanedMspAssociationsOptions {
  /** The SQL client instance */
  sql: SqlClient;
  /** The MSP ID to check */
  mspId: string;
}

/**
 * Verifies that no orphaned MSP associations exist (associations without corresponding files).
 *
 * @throws Error if orphaned associations are found.
 */
export const verifyNoOrphanedMspAssociations = async (
  options: VerifyNoOrphanedMspAssociationsOptions
): Promise<void> => {
  const { sql, mspId } = options;
  const orphanedAssociations = await sql`
    SELECT mf.* FROM msp_file mf
    INNER JOIN msp m ON mf.msp_id = m.id
    LEFT JOIN file f ON mf.file_id = f.id
    WHERE m.onchain_msp_id = ${mspId} AND f.id IS NULL
  `;
  assert.strictEqual(
    orphanedAssociations.length,
    0,
    `Expected no orphaned MSP associations for MSP ${mspId}, but found ${orphanedAssociations.length}`
  );
};

/**
 * Options for getLastIndexedBlock
 */
export interface GetLastIndexedBlockOptions {
  /** The SQL client instance */
  sql: SqlClient;
}

/**
 * Get the last indexed block number from the service_state table.
 *
 * @returns The last indexed finalized block number.
 */
export const getLastIndexedBlock = async (options: GetLastIndexedBlockOptions): Promise<number> => {
  const { sql } = options;
  const result = await sql`SELECT last_indexed_finalized_block FROM service_state WHERE id = 1`;
  return Number(result[0].last_indexed_finalized_block);
};

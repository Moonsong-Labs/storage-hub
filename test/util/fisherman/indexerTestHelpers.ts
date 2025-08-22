import type { EnrichedBspApi, SqlClient } from "../index";
import { waitFor } from "../index";
import { hexToBuffer } from "../indexerHelpers";
import type { ApiPromise } from "@polkadot/api";

export const waitForIndexing = async (api: EnrichedBspApi, sealBlock = true): Promise<void> => {
  if (sealBlock) {
    await api.block.seal();
  }

  const currentBlock = (await api.query.system.number()).toNumber();

  // Wait for indexer to process this block - try BSP container first for fishing mode
  await api.docker.waitForLog({
    searchString: `Indexing block #${currentBlock}:`,
    containerName: "storage-hub-sh-user-1",
    timeout: 30000
  });
};

export const waitForFileInStorage = async (api: ApiPromise, fileKey: string) => {
  await waitFor({
    lambda: async () => (await api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
  });
};

export const verifyFileIndexed = async (sql: SqlClient, bucketName: string, fileKey: string) => {
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

export const verifyProviderAssociation = async (
  sql: SqlClient,
  fileKey: string,
  providerId: string,
  providerType: "msp" | "bsp"
) => {
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

export const waitForFileInForest = async (api: ApiPromise, bucketId: string, fileKey: string) => {
  await waitFor({
    lambda: async () => {
      const isFileInForest = await api.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
      return isFileInForest.isTrue;
    }
  });
};

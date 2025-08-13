import type { SqlClient, EnrichedBspApi } from "./index";
import { waitFor } from "./index";

export const hexToBuffer = (hex: string): Buffer => {
  const cleanHex = hex.startsWith("0x") ? hex.slice(2) : hex;
  return Buffer.from(cleanHex, "hex");
};

export const waitForBlockIndexed = async (
  api: EnrichedBspApi,
  blockNumber?: number
): Promise<void> => {
  const targetBlock = blockNumber ?? (await api.query.system.number()).toNumber();

  await api.docker.waitForLog({
    searchString: `Indexing block #${targetBlock}:`,
    containerName: "storage-hub-sh-user-1",
    timeout: 10000
  });
};

export const waitForFileIndexed = async (sql: SqlClient, fileKey: string) => {
  await waitFor({
    lambda: async () => {
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;
      return files.length > 0;
    }
  });
};

export const waitForBucketIndexed = async (sql: SqlClient, bucketName: string) => {
  await waitFor({
    lambda: async () => {
      const buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      return buckets.length > 0;
    }
  });
};

export const waitForBucketByIdIndexed = async (
  sql: SqlClient,
  bucketId: string,
  mspId?: string
) => {
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

export const waitForBucketDeleted = async (sql: SqlClient, bucketId: string) => {
  await waitFor({
    lambda: async () => {
      const buckets = await sql`
        SELECT * FROM bucket WHERE onchain_bucket_id = ${hexToBuffer(bucketId)} AND deleted_at IS NOT NULL
      `;
      return buckets.length > 0;
    }
  });
};

export const waitForMspFileAssociation = async (
  sql: SqlClient,
  fileKey: string,
  mspId?: string
) => {
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

export const waitForBspFileAssociation = async (
  sql: SqlClient,
  fileKey: string,
  bspId?: string
) => {
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

export const waitForFileDeleted = async (sql: SqlClient, fileKey: string) => {
  await waitFor({
    lambda: async () => {
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;
      return files.length === 0;
    },
    iterations: 20,
    delay: 500
  });
};

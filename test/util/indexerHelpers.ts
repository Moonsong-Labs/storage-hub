import type { SqlClient, EnrichedBspApi } from "./index";
import { waitFor } from "./index";
import assert from "node:assert";

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
    iterations: 30,
    delay: 1000
  });
};

export const waitForBspFileAssociationRemoved = async (
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
      return files.length === 0;
    }
  });
};

export const waitForMspFileAssociationRemoved = async (
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
      return files.length === 0;
    }
  });
};

export const verifyNoBspFileAssociation = async (sql: SqlClient, fileKey: string) => {
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

export const verifyNoMspFileAssociation = async (sql: SqlClient, fileKey: string) => {
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

export const verifyNoOrphanedBspAssociations = async (sql: SqlClient, bspId: string) => {
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

export const verifyNoOrphanedMspAssociations = async (sql: SqlClient, mspId: string) => {
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

export const calculateNextChallengeTick = async (
  api: EnrichedBspApi,
  providerId: string
): Promise<number> => {
  try {
    // Use the proofs dealer API to query the next challenge tick for the provider
    const result = await api.call.proofsDealerApi.getNextTickToSubmitProofFor(providerId);

    if (result.isErr) {
      throw new Error(`API returned error: ${result.asErr.toString()}`);
    }

    return result.asOk.toNumber();
  } catch (error) {
    throw new Error(
      `Failed to calculate next challenge tick for provider ${providerId}: ${
        error instanceof Error ? error.message : String(error)
      }`
    );
  }
};

/**
 * Triggers a complete provider charging cycle by advancing to the next challenge tick,
 * waiting for proof submission, and processing the charge transaction.
 *
 * @param api - The enriched API instance
 * @param providerId - The provider ID to trigger charging for
 * @param shouldSealBlocks - Whether to automatically seal blocks (default: true).
 *                          WARNING: When false, cannot verify transaction outcomes.
 *                          chargingCompleted and userBecameInsolvent will return false.
 * @param userAddress - Optional user address for balance tracking
 * @returns Object containing charging event details including if user became insolvent
 *          Note: When shouldSealBlocks=false, chargingCompleted and userBecameInsolvent
 *          cannot be verified and will return false
 */
export const triggerProviderChargingCycle = async (
  api: EnrichedBspApi,
  providerId: string,
  userAddress?: string
): Promise<{
  proofAcceptedEvents: any[];
  lastChargeableInfoUpdatedEvents: any[];
  chargingCompleted: boolean;
  userBecameInsolvent: boolean;
  balanceBeforeCharge?: string;
  balanceAfterCharge?: string;
}> => {
  // Log balance before charging if user address provided
  let balanceBeforeCharge: string | undefined;
  if (userAddress) {
    const beforeBalance = (await api.query.system.account(userAddress)).data.free;
    balanceBeforeCharge = beforeBalance.toString();
  }

  // Calculate next challenge tick to trigger proof submission and charging
  const nextChallengeTick = await calculateNextChallengeTick(api, providerId);

  // Advance to next challenge tick
  const currentBlock = await api.rpc.chain.getBlock();
  const currentBlockNumber = currentBlock.block.header.number.toNumber();
  if (nextChallengeTick > currentBlockNumber) {
    const blocksToAdvance = nextChallengeTick - currentBlockNumber;
    for (let i = 0; i < blocksToAdvance; i++) {
      await api.block.seal();
    }
  }

  // Wait for BSP to submit proof
  await api.assert.extrinsicPresent({
    method: "submitProof",
    module: "proofsDealer",
    checkTxPool: true
  });

  // Seal block to process proof submission
  await api.block.seal();

  // Assert for the event of the proof successfully submitted and verified
  const proofAcceptedEvents = await api.assert.eventMany("proofsDealer", "ProofAccepted");

  // Seal another block to update last chargeable info
  await api.block.seal();

  // Assert for the event of the last chargeable info being updated
  const lastChargeableInfoUpdatedEvents = await api.assert.eventMany(
    "paymentStreams",
    "LastChargeableInfoUpdated"
  );

  // Wait for charging transaction to be submitted
  await api.wait.waitForTxInPool({
    module: "paymentStreams",
    method: "chargeMultipleUsersPaymentStreams",
    expectedEvent: "PaymentStreamCharged",
    timeout: 45000,
    shouldSeal: false
  });

  // Seal block to process charging
  const blockResult = await api.block.seal();

  // Check if charging completed successfully and if user became insolvent
  const chargingCompleted =
    blockResult.events?.find((event) => event.event.method === "PaymentStreamCharged") !==
    undefined;

  const userBecameInsolvent =
    blockResult.events?.find((event) => event.event.method === "UserWithoutFunds") !== undefined;

  // Log balance after charging if user address provided
  let balanceAfterCharge: string | undefined;
  if (userAddress) {
    const afterBalance = (await api.query.system.account(userAddress)).data.free;
    balanceAfterCharge = afterBalance.toString();
  }

  return {
    proofAcceptedEvents,
    lastChargeableInfoUpdatedEvents,
    chargingCompleted,
    userBecameInsolvent,
    balanceBeforeCharge,
    balanceAfterCharge
  };
};

/**
 * Keeps charging a user until they become insolvent (UserWithoutFunds event is emitted).
 * This function will repeatedly call triggerProviderChargingCycle until the user runs out of funds.
 *
 * @param api - The enriched API instance
 * @param providerId - The provider ID to charge the user
 * @param maxAttempts - Maximum number of charging attempts to prevent infinite loops (default: 10)
 * @param userAddress - Optional user address for balance logging and debugging
 * @returns Object containing details about all charging cycles and final result
 */
export const chargeUserUntilInsolvent = async (
  api: EnrichedBspApi,
  providerId: string,
  maxAttempts = 10,
  userAddress?: string
): Promise<{
  totalCharges: number;
  userBecameInsolvent: boolean;
  finalResult: Awaited<ReturnType<typeof triggerProviderChargingCycle>>;
}> => {
  let attempts = 0;
  let finalResult: Awaited<ReturnType<typeof triggerProviderChargingCycle>>;

  do {
    attempts++;

    finalResult = await triggerProviderChargingCycle(api, providerId, userAddress);

    if (finalResult.userBecameInsolvent) {
      break;
    }

    if (attempts >= maxAttempts) {
      break;
    }
  } while (!finalResult.userBecameInsolvent && attempts < maxAttempts);

  return {
    totalCharges: attempts,
    userBecameInsolvent: finalResult.userBecameInsolvent,
    finalResult
  };
};

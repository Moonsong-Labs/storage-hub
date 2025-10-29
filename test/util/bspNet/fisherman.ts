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
    } catch (_) {
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

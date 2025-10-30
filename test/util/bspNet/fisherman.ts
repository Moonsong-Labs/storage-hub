/**
 * Options for verifyBspDeletionResults
 */
export interface VerifyBspDeletionResultsOptions {
  /** The enriched BSP API for assertions and event fetching */
  userApi: any;
  /** The BSP API instance for forest root verification */
  bspApi: any;
  /** Events array from the sealed block */
  events: any[];
  /** Expected number of BSP deletion events. Defaults to 1. */
  expectedCount?: number;
}

/**
 * Options for verifyBucketDeletionResults
 */
export interface VerifyBucketDeletionResultsOptions {
  /** The enriched BSP API for assertions and event fetching */
  userApi: any;
  /** The MSP API instance for bucket forest root verification */
  mspApi: any;
  /** Events array from the sealed block */
  events: any[];
  /** Expected number of bucket deletion events */
  expectedCount: number;
}

/**
 * Options for waitForFishermanBatchDeletions
 */
export interface WaitForFishermanBatchDeletionsOptions {
  /** The enriched BSP API */
  api: any; // EnrichedBspApi type (avoiding circular dependency)
  /** Either "User" or "Incomplete" to determine which deletion cycle to wait for */
  deletionType: "User" | "Incomplete";
  /** Optional. Total expected extrinsics (BSP + bucket) to verify in the transaction pool */
  expectExt?: number;
  /** Optional. Whether to seal a block after verifying extrinsics. Defaults to false. */
  sealBlock?: boolean;
}

/**
 * Result returned when sealBlock is true
 */
export interface FishermanBatchDeletionsResult {
  /** Block hash of the sealed block */
  blockHash: string;
  /** Events from the sealed block */
  events: any[];
}

/**
 * Verifies BSP deletion results from a batch deletion operation.
 *
 * This function verifies:
 * 1. The expected number of BSP deletion events are present
 * 2. The BSP forest root has changed (oldRoot !== newRoot)
 * 3. The current BSP forest root matches the newRoot from the deletion event
 *
 * @param options - Verification options
 * @returns A promise that resolves when verification is complete
 * @throws Error if any verification step fails
 */
export const verifyBspDeletionResults = async (
  options: VerifyBspDeletionResultsOptions
): Promise<void> => {
  const { userApi, bspApi, events, expectedCount = 1 } = options;

  // Import assert dynamically to avoid top-level import issues
  const { strictEqual, notEqual } = await import("node:assert");

  // Verify BSP deletion event count
  const bspDeletionEvents = await userApi.assert.eventMany(
    "fileSystem",
    "BspFileDeletionsCompleted",
    events
  );
  strictEqual(
    bspDeletionEvents.length,
    expectedCount,
    `Should have exactly ${expectedCount} BSP deletion event(s)`
  );

  // Verify BSP root changed
  const bspDeletionEvent = userApi.assert.fetchEvent(
    userApi.events.fileSystem.BspFileDeletionsCompleted,
    events
  );

  // Use a simple polling mechanism for verification
  const waitFor = async (lambda: () => Promise<boolean>, timeoutMs = 10000) => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      if (await lambda()) return;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error("Timeout waiting for condition");
  };

  await waitFor(async () => {
    notEqual(
      bspDeletionEvent.data.oldRoot.toString(),
      bspDeletionEvent.data.newRoot.toString(),
      "BSP forest root should have changed after file deletion"
    );
    const currentBspRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
    strictEqual(
      currentBspRoot.toString(),
      bspDeletionEvent.data.newRoot.toString(),
      "Current BSP forest root should match the new root from deletion event"
    );
    return true;
  });
};

/**
 * Verifies bucket deletion results from a batch deletion operation.
 *
 * This function verifies:
 * 1. The expected number of bucket deletion events are present
 * 2. For each bucket, the forest root has changed (oldRoot !== newRoot)
 * 3. For each bucket, the current forest root matches the newRoot from the deletion event
 *
 * @param options - Verification options
 * @returns A promise that resolves when verification is complete
 * @throws Error if any verification step fails
 */
export const verifyBucketDeletionResults = async (
  options: VerifyBucketDeletionResultsOptions
): Promise<void> => {
  const { userApi, mspApi, events, expectedCount } = options;

  // Import assert dynamically to avoid top-level import issues
  const { strictEqual, notEqual } = await import("node:assert");

  // Verify bucket deletion event count
  const bucketDeletionEvents = await userApi.assert.eventMany(
    "fileSystem",
    "BucketFileDeletionsCompleted",
    events
  );
  strictEqual(
    bucketDeletionEvents.length,
    expectedCount,
    `Should have exactly ${expectedCount} bucket deletion event(s)`
  );

  // Use a simple polling mechanism for verification
  const waitFor = async (lambda: () => Promise<boolean>, timeoutMs = 10000) => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      if (await lambda()) return;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error("Timeout waiting for condition");
  };

  // Verify MSP roots changed for all buckets
  for (const bucketDeletionRecord of bucketDeletionEvents) {
    const bucketDeletionEvent = bucketDeletionRecord.event;
    if (userApi.events.fileSystem.BucketFileDeletionsCompleted.is(bucketDeletionEvent)) {
      await waitFor(async () => {
        notEqual(
          bucketDeletionEvent.data.oldRoot.toString(),
          bucketDeletionEvent.data.newRoot.toString(),
          "MSP forest root should have changed after file deletion"
        );
        const currentBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(
          bucketDeletionEvent.data.bucketId.toString()
        );
        strictEqual(
          currentBucketRoot.toString(),
          bucketDeletionEvent.data.newRoot.toString(),
          "Current bucket forest root should match the new root from deletion event"
        );
        return true;
      });
    }
  }
};

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
 * If `sealBlock` is true, a block will be sealed after verifying extrinsics
 * and the result (with events) will be returned.
 * Defaults to false to allow manual block sealing in tests.
 *
 * @returns When sealBlock is true, returns the sealed block result with events.
 *          When sealBlock is false, returns undefined.
 */
export const waitForFishermanBatchDeletions = async (
  options: WaitForFishermanBatchDeletionsOptions
): Promise<FishermanBatchDeletionsResult | undefined> => {
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

  // Ensure deletion type extrinsic is present in transaction pool
  await api.assert.extrinsicPresent({
    method: deletionType === "User" ? "deleteFiles" : "deleteFilesForIncompleteStorageRequest",
    module: "fileSystem",
    checkTxPool: true,
    assertLength: expectExt,
    timeout: 500 // This is a small timeout since the fisherman should have already submitted the extrinsics by this point
  });

  // Optionally seal a block after verification and return the result
  if (sealBlock) {
    const result = await api.block.seal();
    return {
      blockHash: result.blockHash,
      events: result.events || []
    };
  }
};

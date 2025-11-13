import { strictEqual, notEqual } from "node:assert";
import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { EventRecord, SignedBlock } from "@polkadot/types/interfaces";
import { waitFor } from "./waits";
import type { EnrichedBspApi } from ".";

/**
 * Result returned by verification functions when returnResults is true
 */
export interface DeletionVerificationResult {
  /** Number of successful deletions */
  successful: number;
  /** Number of deletions that failed with ForestProofVerificationFailed error */
  failedWithForestProofError: number;
}

/**
 * Options for verifyBspDeletionResults
 */
export interface VerifyBspDeletionResultsOptions {
  /** The api for assertions and event fetching */
  userApi: EnrichedBspApi;
  /** The BSP API instance for forest root verification */
  bspApi: EnrichedBspApi;
  /** Events array from the sealed block */
  events: EventRecord[];
  /** Expected number of BSP deletion events. Defaults to 1. */
  expectedCount?: number;
  /** Optional. When true, returns result object instead of throwing. Defaults to false. */
  returnResults?: boolean;
  /** Block data containing extrinsics (required when returnResults is true to identify which extrinsic failed) */
  blockData?: SignedBlock;
}

/**
 * Options for verifyBucketDeletionResults
 */
export interface VerifyBucketDeletionResultsOptions {
  /** The api for assertions and event fetching */
  userApi: EnrichedBspApi;
  /** The MSP API instance for bucket forest root verification */
  mspApi: EnrichedBspApi;
  /** Events array from the sealed block */
  events: EventRecord[];
  /** Expected number of bucket deletion events */
  expectedCount: number;
  /** Optional. When true, returns result object instead of throwing. Defaults to false. */
  returnResults?: boolean;
  /** Block data containing extrinsics (required when returnResults is true to identify which extrinsic failed) */
  blockData?: SignedBlock;
  /** Optional. Array of bucket IDs to skip forest root verification for (e.g., when MSP stopped storing the bucket) */
  skipBucketIds?: string[];
}

/**
 * Options for waitForFishermanBatchDeletions
 */
export interface WaitForFishermanBatchDeletionsOptions {
  /** The block producer api (normally userApi) */
  blockProducerApi: EnrichedBspApi;
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
  events: EventRecord[];
  /** Block data containing extrinsics (needed to identify which extrinsic failed) */
  blockData: SignedBlock;
}

/**
 * Checks if the deletion extrinsic specifically failed with ForestProofVerificationFailed.
 *
 * Since an extrinsic either succeeds (emits completion event) OR fails (emits ExtrinsicFailed),
 * we handle both cases:
 *
 * 1. If completion event exists: The deletion succeeded, so return false.
 *
 * 2. If no completion event exists: Check each ExtrinsicFailed event, identify which extrinsic
 *    it belongs to (using phase), check if that extrinsic is a deletion extrinsic by examining
 *    the method/section (fileSystem.deleteFiles or fileSystem.deleteFilesForIncompleteStorageRequest),
 *    and only then check if the error is ForestProofVerificationFailed.
 *
 * @param api - The API instance to decode errors
 * @param events - Array of events from a sealed block
 * @param blockData - Block data containing extrinsics array (required for precise error checking)
 * @param completionEventMatcher - Function to check if an event is a deletion completion event
 * @returns true if deletion extrinsic failed with ForestProofVerificationFailed, false otherwise
 * @throws Error if blockData is not provided
 */
const hasDeletionExtrinsicForestProofError = (
  api: ApiPromise,
  events: EventRecord[],
  blockData: SignedBlock,
  completionEventMatcher: (event: any) => boolean
): boolean => {
  // First check if completion event exists anywhere
  const hasCompletionEvent = events.some(({ event }) => completionEventMatcher(event));

  if (hasCompletionEvent) {
    // Deletion extrinsic succeeded, so no ForestProofVerificationFailed from it
    return false;
  }

  // Check each ExtrinsicFailed event
  for (const { event, phase } of events) {
    if (api.events.system.ExtrinsicFailed.is(event)) {
      // Get the extrinsic index from the phase
      if (!phase.isApplyExtrinsic) continue;
      const extIndex = phase.asApplyExtrinsic.toNumber();

      // Get the actual extrinsic to check its method/section
      const extrinsic = blockData.block.extrinsics[extIndex];
      if (!extrinsic) continue;

      const { method, section } = extrinsic.method;

      // Check if this is a deletion extrinsic
      const isDeletionExtrinsic =
        section === "fileSystem" &&
        (method === "deleteFiles" || method === "deleteFilesForIncompleteStorageRequest");

      if (!isDeletionExtrinsic) continue;

      // This is a deletion extrinsic that failed - check if it's ForestProofVerificationFailed
      const errorEventData = event.data;
      if (errorEventData.dispatchError.isModule) {
        try {
          const decoded = api.registry.findMetaError(errorEventData.dispatchError.asModule);
          if (
            decoded.section === "proofsDealer" &&
            decoded.method === "ForestProofVerificationFailed"
          ) {
            return true;
          }
        } catch (_) {
          // Error decoding failed, skip
        }
      }
    }
  }

  return false;
};

/**
 * Options for retryable batch deletions with verification
 */
export interface RetryableBatchDeletionsOptions {
  /** The block producer api (normally userApi) */
  blockProducerApi: EnrichedBspApi;
  /** Either "User" or "Incomplete" to determine which deletion cycle to wait for */
  deletionType: "User" | "Incomplete";
  /** Optional. Total expected extrinsics (BSP + bucket) to verify in the transaction pool */
  expectExt?: number;
  /** The api for assertions and event fetching */
  userApi: EnrichedBspApi;
  /** Optional BSP API instance for forest root verification. If provided, BSP deletions will be verified. */
  bspApi?: EnrichedBspApi;
  /** Expected number of BSP deletion events. Defaults to 1 if bspApi is provided. */
  expectedBspCount?: number;
  /** Optional MSP API instance for bucket forest root verification. If provided, bucket deletions will be verified. */
  mspApi?: EnrichedBspApi;
  /** Expected number of bucket deletion events. Required if mspApi is provided. */
  expectedBucketCount?: number;
  /** Maximum number of retry attempts. Defaults to 3. */
  maxRetries?: number;
  /** Optional. Array of bucket IDs to skip forest root verification for (e.g., when MSP stopped storing the bucket) */
  skipBucketIds?: string[];
}

/**
 * Verifies BSP deletion results from a batch deletion operation.
 *
 * This function verifies:
 * 1. The expected number of BSP deletion events are present (when returnResults is false)
 * 2. The BSP forest root has changed (oldRoot !== newRoot) for all completion events
 * 3. The current BSP forest root matches the newRoot from the deletion event
 *
 * When returnResults is true, this function:
 * - Groups events by extrinsic to precisely track success/failure
 * - Checks for ForestProofVerificationFailed errors per extrinsic
 * - Throws immediately for non-ForestProofVerificationFailed errors
 * - Always verifies forest roots for all completion events
 * - Returns counts of successful and failed deletions
 *
 * @param options - Verification options
 * @returns When returnResults is true, returns DeletionVerificationResult. Otherwise void.
 * @throws Error if any verification step fails (when returnResults is false) or for non-retryable errors
 */
export const verifyBspDeletionResults = async (
  options: VerifyBspDeletionResultsOptions
): Promise<DeletionVerificationResult | undefined> => {
  const { userApi, bspApi, events, expectedCount = 1, returnResults = false, blockData } = options;

  if (returnResults) {
    // Validate blockData is provided when returnResults is true
    if (!blockData) {
      throw new Error(
        "blockData is required when returnResults is true. " +
          "Ensure blockData is passed to identify which extrinsic failed with ForestProofVerificationFailed."
      );
    }

    // Check if the deletion extrinsic specifically failed with ForestProofVerificationFailed
    const hasForestProofError = hasDeletionExtrinsicForestProofError(
      userApi,
      events,
      blockData,
      (event) => userApi.events.fileSystem.BspFileDeletionsCompleted.is(event)
    );

    // Try to find BSP deletion completion event
    const bspDeletionEvents = events.filter(({ event }) =>
      userApi.events.fileSystem.BspFileDeletionsCompleted.is(event)
    );

    if (bspDeletionEvents.length > 0) {
      // Completion event exists - verify forest root changes
      const bspDeletionEvent = bspDeletionEvents[0].event;
      assert(userApi.events.fileSystem.BspFileDeletionsCompleted.is(bspDeletionEvent));
      await waitFor({
        lambda: async () => {
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
        },
        delay: 10_000
      });

      // If there's a completion event with ForestProofVerificationFailed, count as failure
      // Otherwise count as success
      return hasForestProofError
        ? { successful: 0, failedWithForestProofError: 1 }
        : { successful: 1, failedWithForestProofError: 0 };
    }
    // No completion event found
    if (hasForestProofError) {
      // Expected: extrinsic failed with ForestProofVerificationFailed
      return { successful: 0, failedWithForestProofError: 1 };
    }
    // Unexpected: no completion event and no ForestProofVerificationFailed error
    throw new Error(
      "No BSP deletion completion event found and no ForestProofVerificationFailed error detected"
    );
  }

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

  await waitFor({
    lambda: async () => {
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
    },
    delay: 10_000
  });
};

/**
 * Verifies bucket deletion results from a batch deletion operation.
 *
 * This function verifies:
 * 1. The expected number of bucket deletion events are present (when returnResults is false)
 * 2. For each bucket, the forest root has changed (oldRoot !== newRoot)
 * 3. For each bucket, the current forest root matches the newRoot from the deletion event
 *
 * When returnResults is true, this function:
 * - Groups events by extrinsic to precisely track success/failure
 * - Checks for ForestProofVerificationFailed errors per extrinsic
 * - Throws immediately for non-ForestProofVerificationFailed errors
 * - Always verifies forest roots for all completion events
 * - Returns counts of successful and failed deletions
 *
 * @param options - Verification options
 * @returns When returnResults is true, returns DeletionVerificationResult. Otherwise void.
 * @throws Error if any verification step fails (when returnResults is false) or for non-retryable errors
 */
export const verifyBucketDeletionResults = async (
  options: VerifyBucketDeletionResultsOptions
): Promise<DeletionVerificationResult | undefined> => {
  const {
    userApi,
    mspApi,
    events,
    expectedCount,
    returnResults = false,
    blockData,
    skipBucketIds = []
  } = options;

  if (returnResults) {
    // Validate blockData is provided when returnResults is true
    if (!blockData) {
      throw new Error(
        "blockData is required when returnResults is true. " +
          "Ensure blockData is passed to identify which extrinsic failed with ForestProofVerificationFailed."
      );
    }

    // Check if the deletion extrinsic specifically failed with ForestProofVerificationFailed
    const hasForestProofError = hasDeletionExtrinsicForestProofError(
      userApi,
      events,
      blockData,
      (event) => userApi.events.fileSystem.BucketFileDeletionsCompleted.is(event)
    );

    // Try to find bucket deletion completion events
    const bucketDeletionEvents = events.filter(({ event }) =>
      userApi.events.fileSystem.BucketFileDeletionsCompleted.is(event)
    );

    // Validate expected count of bucket deletion events
    strictEqual(
      bucketDeletionEvents.length,
      expectedCount,
      `Should have exactly ${expectedCount} bucket deletion event(s)`
    );

    // If expected count is 0 and we have 0 events, that's correct - return success
    if (expectedCount === 0 && bucketDeletionEvents.length === 0) {
      return { successful: 1, failedWithForestProofError: 0 };
    }

    if (bucketDeletionEvents.length > 0) {
      // Completion events exist - verify forest root changes for all buckets (except skipped ones)
      for (const bucketDeletionRecord of bucketDeletionEvents) {
        const bucketDeletionEvent = bucketDeletionRecord.event;
        assert(userApi.events.fileSystem.BucketFileDeletionsCompleted.is(bucketDeletionEvent));
        const bucketId = bucketDeletionEvent.data.bucketId.toString();

        // Skip forest root verification for buckets in the skip list
        if (skipBucketIds.includes(bucketId)) {
          continue;
        }

        await waitFor({
          lambda: async () => {
            notEqual(
              bucketDeletionEvent.data.oldRoot.toString(),
              bucketDeletionEvent.data.newRoot.toString(),
              `MSP forest root should have changed for bucket ${bucketId}`
            );
            const currentBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);
            strictEqual(
              currentBucketRoot.toString(),
              bucketDeletionEvent.data.newRoot.toString(),
              `Current bucket forest root should match new root for bucket ${bucketId}`
            );
            return true;
          }
        });
      }

      // If there's a completion event with ForestProofVerificationFailed, count as failure
      // Otherwise count as success
      return hasForestProofError
        ? { successful: 0, failedWithForestProofError: 1 }
        : { successful: 1, failedWithForestProofError: 0 };
    }
    // No completion event found (but expectedCount > 0)
    if (hasForestProofError) {
      // Expected: extrinsic failed with ForestProofVerificationFailed
      return { successful: 0, failedWithForestProofError: 1 };
    }
    // Unexpected: no completion event and no ForestProofVerificationFailed error
    throw new Error(
      "No bucket deletion completion event found and no ForestProofVerificationFailed error detected"
    );
  }

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

  // Verify MSP roots changed for all buckets (except skipped ones)
  for (const bucketDeletionRecord of bucketDeletionEvents) {
    const bucketDeletionEvent = bucketDeletionRecord.event;
    if (userApi.events.fileSystem.BucketFileDeletionsCompleted.is(bucketDeletionEvent)) {
      const bucketId = bucketDeletionEvent.data.bucketId.toString();

      // Skip forest root verification for buckets in the skip list
      if (skipBucketIds.includes(bucketId)) {
        continue;
      }

      await waitFor({
        lambda: async () => {
          notEqual(
            bucketDeletionEvent.data.oldRoot.toString(),
            bucketDeletionEvent.data.newRoot.toString(),
            "MSP forest root should have changed after file deletion"
          );
          const currentBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);
          strictEqual(
            currentBucketRoot.toString(),
            bucketDeletionEvent.data.newRoot.toString(),
            "Current bucket forest root should match the new root from deletion event"
          );
          return true;
        }
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
 * The function first checks if the extrinsics are already in the transaction pool.
 * If they are, the polling loop is skipped. Otherwise, it uses a polling loop that:
 * 1. Seals a block
 * 2. Checks for the fisherman log message (with short timeout)
 * 3. If not found, waits and repeats
 * 4. Once found, optionally verifies extrinsics in tx pool
 *
 * After the loop (or if skipped), the function verifies that the expected
 * number of extrinsics are present in the transaction pool.
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
  const { blockProducerApi: api, deletionType, expectExt, sealBlock = false } = options;

  const methodName =
    deletionType === "User" ? "deleteFiles" : "deleteFilesForIncompleteStorageRequest";

  // Check if extrinsics are already in the transaction pool before starting the loop
  let extrinsicsAlreadyPresent = false;
  try {
    await api.assert.extrinsicPresent({
      method: methodName,
      module: "fileSystem",
      checkTxPool: true,
      assertLength: expectExt,
      exactLength: !!expectExt,
      timeout: 5000
    });
    extrinsicsAlreadyPresent = true;
  } catch (_) {
    // Extrinsics not in pool yet, will proceed with the loop
  }

  // If extrinsics are already present, skip the polling loop
  if (!extrinsicsAlreadyPresent) {
    // Poll for fisherman extrinsic, sealing blocks between checks
    // Wait 5 second interval (fisherman configuration "--fisherman-batch-interval-seconds=5")
    // to leave time for the fisherman to switch processing deletion types
    const maxAttempts = 5; // 5 attempts * 5 seconds = 25 seconds total timeout
    let found = false;

    for (let attempt = 0; attempt < maxAttempts && !found; attempt++) {
      // Seal a block to trigger fisherman interval processing
      await api.block.seal();

      // Check if fisherman has submitted the extrinsics (with short timeout to avoid blocking)
      try {
        // Ensure deletion type extrinsic is present in transaction pool
        await api.assert.extrinsicPresent({
          method: methodName,
          module: "fileSystem",
          checkTxPool: true,
          assertLength: expectExt,
          exactLength: !!expectExt,
          timeout: 10000 // Small timeout since fisherman should have submitted by now
        });
        found = true;
      } catch (_) {
        // Extrinsic not found yet, continue to next iteration
        if (attempt === maxAttempts - 1) {
          throw new Error(
            `Timeout waiting for fisherman to process ${deletionType} deletions after ${maxAttempts * 5} seconds`
          );
        }
      }
    }
  }

  // Optionally seal a block after verification and return the result
  if (sealBlock) {
    const result = await api.block.seal();
    // Always fetch blockData since we need it for verification (seal() only sets it when extrinsics are explicitly passed)
    const blockData = await api.rpc.chain.getBlock(result.blockReceipt.blockHash);
    return {
      blockHash: result.blockReceipt.blockHash.toString(),
      events: result.events || [],
      blockData
    };
  }
};

/**
 * Waits for fisherman batch deletions and verifies BSP deletion results with retry logic.
 *
 * This function will retry both `waitForBatchDeletions` and verification functions
 * if `ForestProofVerificationFailed` errors are detected in the events, up to a maximum
 * number of attempts.
 *
 * The verification functions handle precise event-to-extrinsic correlation and track
 * successful vs failed deletions, simplifying the retry logic significantly.
 *
 * @param options - Options for the retryable batch deletions
 * @returns A promise that resolves when verification succeeds without ForestProofVerificationFailed errors
 * @throws Error if max retries are exceeded or if other verification failures occur
 */
export const retryableWaitAndVerifyBatchDeletions = async (
  options: RetryableBatchDeletionsOptions
): Promise<FishermanBatchDeletionsResult> => {
  const {
    blockProducerApi,
    deletionType,
    expectExt,
    userApi,
    bspApi,
    expectedBspCount = 1,
    mspApi,
    expectedBucketCount,
    maxRetries = 3,
    skipBucketIds
  } = options;

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    // Wait for batch deletions and seal block
    const deletionResult = await waitForFishermanBatchDeletions({
      blockProducerApi,
      deletionType,
      expectExt,
      sealBlock: true
    });

    if (!deletionResult) {
      throw new Error("waitForBatchDeletions returned undefined when sealBlock is true");
    }

    if (!deletionResult.events || deletionResult.events.length === 0) {
      throw new Error("Deletion result should have events");
    }

    // Verify BSP deletions if needed
    const bspResult: DeletionVerificationResult = bspApi
      ? ((await verifyBspDeletionResults({
          userApi,
          bspApi,
          events: deletionResult.events,
          expectedCount: expectedBspCount,
          returnResults: true,
          blockData: deletionResult.blockData
        })) as DeletionVerificationResult)
      : { successful: 0, failedWithForestProofError: 0 };

    // Verify bucket deletions if needed
    const bucketResult: DeletionVerificationResult =
      mspApi && expectedBucketCount !== undefined
        ? ((await verifyBucketDeletionResults({
            userApi,
            mspApi,
            events: deletionResult.events,
            expectedCount: expectedBucketCount,
            returnResults: true,
            blockData: deletionResult.blockData,
            skipBucketIds
          })) as DeletionVerificationResult)
        : { successful: 0, failedWithForestProofError: 0 };

    // Check if all succeeded
    const allSucceeded =
      bspResult.failedWithForestProofError === 0 && bucketResult.failedWithForestProofError === 0;

    if (allSucceeded) {
      // All deletions succeeded, return the result
      return deletionResult;
    }

    // Some deletions failed with ForestProofVerificationFailed
    if (attempt < maxRetries - 1) {
      // Retry if not last attempt
      console.log(
        `Attempt ${attempt + 1}/${maxRetries}: ` +
          `${bspResult.failedWithForestProofError} BSP, ${bucketResult.failedWithForestProofError} bucket ` +
          "failed with ForestProofVerificationFailed. Retrying..."
      );
      await new Promise((resolve) => setTimeout(resolve, 2000));
    } else {
      // Max retries exceeded
      throw new Error(
        `Failed after ${maxRetries} attempts. ` +
          `Final attempt: ${bspResult.successful} BSP, ${bucketResult.successful} bucket succeeded. ` +
          `${bspResult.failedWithForestProofError} BSP, ${bucketResult.failedWithForestProofError} bucket ` +
          "failed with ForestProofVerificationFailed."
      );
    }
  }

  // This should never be reached, but TypeScript needs it
  throw new Error(`Failed after ${maxRetries} attempts`);
};

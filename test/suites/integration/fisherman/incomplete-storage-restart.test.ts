import assert, { notEqual, strictEqual } from "node:assert";
import type { H256 } from "@polkadot/types/interfaces";
import {
  describeMspNet,
  type EnrichedBspApi,
  shUser,
  bspKey,
  mspKey,
  waitFor,
  assertEventMany,
  ShConsts
} from "../../../util";
import { waitForFishermanBatchDeletions } from "../../../util/fisherman/indexerTestHelpers";

/**
 * FISHERMAN INCOMPLETE STORAGE REQUESTS WITH RESTART SCENARIOS
 *
 * Purpose: Tests the fisherman's ability to handle incomplete storage requests across
 *          container restarts, including initial sync behavior with pagination and limits.
 */
await describeMspNet(
  "Fisherman Incomplete Storage Restart Tests",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    fishermanIncompleteSyncMax: 100,
    fishermanIncompleteSyncPageSize: 20
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api, createFishermanApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let valuePropId: H256 | null = null;

    /**
     * Generates an array of destination file paths with sequential numbering.
     * Used to create predictable file names for batch storage request creation.
     *
     * @param prefix - The base path/name prefix for the files
     * @param count - Number of destination paths to generate
     * @param startIndex - Starting number for sequential naming (default: 0)
     * @returns Array of destination file paths like ["prefix-0.txt", "prefix-1.txt", ...]
     *
     * @example
     * buildDestinations("test/data", 3) => ["test/data-0.txt", "test/data-1.txt", "test/data-2.txt"]
     * buildDestinations("test/data", 2, 5) => ["test/data-5.txt", "test/data-6.txt"]
     */
    const buildDestinations = (prefix: string, count: number, startIndex = 0) =>
      Array.from({ length: count }, (_, index) => `${prefix}-${startIndex + index}.txt`);

    /**
     * Creates a new storage bucket on the blockchain and returns its ID.
     * Waits for the bucket creation transaction to be confirmed via NewBucket event.
     *
     * @param bucketName - Unique name for the bucket
     * @returns Object containing bucketId (H256) and bucketIdHex (string representation)
     * @throws Assertion error if NewBucket event is not emitted or doesn't match expected type
     */
    const ensureBucket = async (bucketName: string) => {
      // Now create bucket with the value proposition ID
      const newBucketEvent = await userApi.file.newBucket(
        bucketName,
        shUser,
        valuePropId?.toHex(),
        ShConsts.DUMMY_MSP_ID
      );

      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      assert(newBucketEventData, "NewBucket event did not match expected type");

      return {
        bucketId: newBucketEventData.bucketId as H256,
        bucketIdHex: newBucketEventData.bucketId.toString()
      };
    };

    /**
     * Advances the blockchain past the expiry blocks of given storage requests.
     * Storage requests have an expiration block after which they become "incomplete"
     * and eligible for fisherman processing. This function finds the latest expiry
     * among all provided file keys and skips blocks to pass that point.
     *
     * @param fileKeys - Array of file key identifiers to check expiry for
     * @param offset - Additional blocks to advance past the expiry (default: 1)
     *
     * @example
     * // Move blockchain past expiry of these storage requests
     * await advancePastExpiry(["0xabc...", "0xdef..."], 5);
     * // Now these requests are expired + 5 blocks
     */
    const advancePastExpiry = async (fileKeys: string[]) => {
      const expiries = await Promise.all(
        fileKeys.map(async (fileKey) => {
          const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
          assert(storageRequest.isSome, "Expected storage request to exist");
          return storageRequest.unwrap().expiresAt.toNumber();
        })
      );

      const targetBlock = Math.max(...expiries);
      const result = await userApi.block.skipTo(targetBlock);
      assertEventMany(userApi, "fileSystem", "IncompleteStorageRequest", result.events);
    };

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");

      // Wait for user node to be ready
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available for fisherman test");

      // Fund shUser account with extra balance for creating many buckets
      // Each bucket creation requires a deposit, and Test 3 creates 150 buckets
      const extraFunding = 100000n * 10n ** 12n; // 100,000 units
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(shUser.address, extraFunding))
        ],
        signer: userApi.accounts.sudo
      });

      // Create a new value proposition if none exists
      const addValuePropResult = await userApi.block.seal({
        calls: [
          userApi.tx.providers.addValueProp(
            // Price per giga unit of data per block (make it large enough)
            1n,
            // Commitment (empty Uint8Array for default)
            new Uint8Array(),
            // Bucket data limit - use a reasonable size that aligns with typical capacities
            // Each test creates up to 150 files × ~346KB = ~52MB
            1024n * 1024n * 1024n // 1GB should be sufficient for all test files
          )
        ],
        signer: mspKey
      });

      // Extract the value prop ID from the event
      const events = assertEventMany(
        userApi,
        "providers",
        "ValuePropAdded",
        addValuePropResult.events
      );
      assert(events.length > 0, "ValuePropAdded event not found");

      // Get the event data using the proper type checking pattern
      const valuePropAddedEventData =
        userApi.events.providers.ValuePropAdded.is(events[0].event) && events[0].event.data;

      assert(valuePropAddedEventData, "ValuePropAdded event data doesn't match expected type");
      valuePropId = valuePropAddedEventData.valuePropId;

      await userApi.block.seal({ finaliseBlock: true });
    });

    it("Basic restart with pending incomplete requests", async () => {
      const bucketName = "test-restart-basic";
      const source = "res/whatsup.jpg";

      const { bucketId } = await ensureBucket(bucketName);

      await userApi.docker.pauseContainer("storage-hub-sh-fisherman-1");

      // Pause MSP container so that we trigger the incomplete storage request
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");
      // Create more incomplete requests while fisherman is down
      const destinations = buildDestinations("test/restart-basic-new", 1, 5);
      const initialRequests: string[] = [];
      const bspAccount = userApi.createType("Address", bspKey.address);
      const requestCount = destinations.length;

      // Step 1: Create all storage requests first
      for (const destination of destinations) {
        const { fileKey } = await userApi.file.newStorageRequest(
          source,
          destination,
          bucketId,
          undefined,
          undefined,
          1 // Create storage requests that will expire to never reaching the replication target
        );
        initialRequests.push(fileKey);
      }

      await userApi.wait.bspVolunteer();

      await waitFor({
        lambda: async () => {
          for (const fileKey of initialRequests) {
            const isStored = (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey))
              .isFileFound;
            if (!isStored) {
              return false;
            }
          }
          return true;
        },
        iterations: 30,
        delay: 1000
      });

      // Step 5: Wait for all BSP stored confirmations
      await userApi.wait.bspStored({
        bspAccount,
        expectedExts: requestCount
      });

      await advancePastExpiry(initialRequests);
      const pendingNewRequest = await userApi.query.fileSystem.incompleteStorageRequests(
        initialRequests[0]
      );
      assert(
        pendingNewRequest.isSome,
        "Expected pending incomplete storage request while fisherman is down"
      );

      // Create enough blocks to trigger sync mode detection (more than sync_mode_min_blocks_behind)
      // Default sync_mode_min_blocks_behind is 5, so we need at least 6 blocks
      for (let i = 0; i < 6; i++) {
        await userApi.block.seal({ finaliseBlock: true });
      }

      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-fisherman-1" });

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-fisherman-1",
        tail: 10
      });

      // Waiting for the fisherman node to be in sync with the chain.
      await userApi.block.seal({ finaliseBlock: true });
      // Wait for fisherman to detect it's out of sync and start syncing
      await userApi.docker.waitForLog({
        searchString: "🎣 Handling coming out of sync mode",
        containerName: "storage-hub-sh-fisherman-1",
        timeout: 30000
      });

      // Wait for sync to complete
      await userApi.docker.waitForLog({
        searchString: "🎣 Completed initial incomplete storage requests sync",
        containerName: "storage-hub-sh-fisherman-1",
        timeout: 30000
      });

      // Wait for fisherman to process incomplete storage deletions
      await waitForFishermanBatchDeletions(userApi, "Incomplete");

      // Verify delete extrinsic is submitted for the BSP
      await userApi.assert.extrinsicPresent({
        method: "deleteFilesForIncompleteStorageRequest",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 1
      });

      // Seal block to process the extrinsic
      const deletionResult = await userApi.block.seal();

      const {
        data: { oldRoot, newRoot }
      } = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionsCompleted,
        deletionResult.events
      );

      // Verify BSP root changed
      await waitFor({
        lambda: async () => {
          notEqual(
            oldRoot.toString(),
            newRoot.toString(),
            "BSP forest root should have changed after file deletion"
          );
          const currentBspRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
          strictEqual(
            currentBspRoot.toString(),
            newRoot.toString(),
            "Current BSP forest root should match the new root from deletion event"
          );
          return true;
        }
      });

      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
    });
  }
);

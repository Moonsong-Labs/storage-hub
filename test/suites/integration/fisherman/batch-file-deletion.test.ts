import assert, { strictEqual, notEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent,
  ShConsts
} from "../../../util";
import {
  hexToBuffer,
  waitForFileIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation
} from "../../../util/indexerHelpers";
import { waitForIndexing } from "../../../util/fisherman/indexerTestHelpers";

/**
 * FISHERMAN BATCH FILE DELETION - COMPREHENSIVE BATCH PROCESSING TESTS
 *
 * Purpose: Validates the fisherman's batch processing capabilities for file deletions,
 *          ensuring multiple files are grouped by target (BSP/Bucket) and processed
 *          in parallel with efficient batch extrinsics.
 *
 * Test Structure:
 * 1. User Deletion Batching (deleteFiles extrinsic)
 *    - 3 buckets × 2 files each = 6 files total
 *    - Expected: 1 BSP extrinsic (6 files) + 3 Bucket extrinsics (2 files each)
 *    - Validates: Database signatures, BSP batching, bucket batching, parallel processing
 *
 * 2. Incomplete Storage Deletion Batching (deleteFilesForIncompleteStorageRequest extrinsic)
 *    - 3 buckets × 2 files each = 6 files total (revoked storage requests)
 *    - Expected: 1 BSP extrinsic (6 files) + 3 Bucket extrinsics (2 files each)
 *    - Validates: Incomplete storage flow, fisherman catchup, BSP/bucket batching
 *
 * Key Features Tested:
 * - Batch interval timing (5 seconds configured in docker)
 * - Multiple files batched into single extrinsic per target
 * - Parallel processing across multiple targets (BSPs and Buckets)
 * - Alternating between User and Incomplete deletion types
 * - Forest root change verification for BSP and all buckets
 *
 * Architecture:
 * - Time-based batch intervals (5 seconds for tests, 60 default)
 * - Global lock prevents overlapping batches
 * - Single task processes all targets using parallel futures
 * - One extrinsic per target containing multiple file deletions
 */
await describeMspNet(
  "Fisherman Batch File Deletion",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createFishermanApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available for fisherman test");
      fishermanApi = await createFishermanApi();

      await userApi.block.seal({ finaliseBlock: true });
      await waitForIndexing(userApi);
    });

    it("batches user-requested file deletions across multiple buckets with parallel BSP and bucket processing", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const fileKeys: string[] = [];
      const bucketIds: string[] = [];
      const locations: string[] = [];
      const fingerprints: string[] = [];
      const fileSizes: number[] = [];

      // Create 3 buckets, each with 2 files (6 files total)
      for (let bucketIndex = 0; bucketIndex < 3; bucketIndex++) {
        const bucketName = `test-batch-bucket-${bucketIndex}`;

        // Create bucket
        const newBucketEvent = await userApi.createBucket(bucketName, valuePropId);
        const newBucketEventData =
          userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

        if (!newBucketEventData) {
          throw new Error("NewBucket event data not found");
        }

        const bucketId = newBucketEventData.bucketId;

        // Create 2 files in this bucket
        for (let fileIndex = 0; fileIndex < 2; fileIndex++) {
          const result = await userApi.file.newStorageRequest(
            "res/smile.jpg",
            `test/batch-b${bucketIndex}-f${fileIndex}.txt`,
            bucketId,
            shUser,
            ShConsts.DUMMY_MSP_ID,
            1
          );

          fileKeys.push(result.fileKey);
          bucketIds.push(bucketId.toString());
          locations.push(result.location);
          fingerprints.push(result.fingerprint);
          fileSizes.push(result.fileSize);

          // Wait for MSP to store the file
          await waitFor({
            lambda: async () =>
              (await msp1Api.rpc.storagehubclient.isFileInFileStorage(result.fileKey)).isFileFound
          });

          await userApi.wait.mspResponseInTxPool();

          // Wait for BSP to volunteer and store
          await userApi.wait.bspVolunteer();
          await waitFor({
            lambda: async () =>
              (await bspApi.rpc.storagehubclient.isFileInFileStorage(result.fileKey)).isFileFound
          });

          const bspAddress = userApi.createType("Address", bspKey.address);
          await userApi.wait.bspStored({
            expectedExts: 1,
            sealBlock: true,
            bspAccount: bspAddress
          });

          await waitForIndexing(userApi);
          await waitForFileIndexed(sql, result.fileKey);
          await waitForMspFileAssociation(sql, result.fileKey);
          await waitForBspFileAssociation(sql, result.fileKey);
        }
      }

      // Request deletion for all 6 files
      for (let i = 0; i < fileKeys.length; i++) {
        const fileOperationIntention = {
          fileKey: fileKeys[i],
          operation: { Delete: null }
        };

        const intentionCodec = userApi.createType(
          "PalletFileSystemFileOperationIntention",
          fileOperationIntention
        );
        const intentionPayload = intentionCodec.toU8a();
        const rawSignature = shUser.sign(intentionPayload);
        const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

        const deletionRequestResult = await userApi.block.seal({
          calls: [
            userApi.tx.fileSystem.requestDeleteFile(
              fileOperationIntention,
              userSignature,
              bucketIds[i],
              locations[i],
              fileSizes[i],
              fingerprints[i]
            )
          ],
          signer: shUser
        });

        assertEventPresent(
          userApi,
          "fileSystem",
          "FileDeletionRequested",
          deletionRequestResult.events
        );
      }

      await waitForIndexing(userApi, false);

      // Verify deletion signatures are stored in database
      await verifyDeletionSignaturesStored(sql, fileKeys);

      // Verify extrinsics are submitted (1 BSP + 3 Buckets = 4 total)
      await userApi.assert.extrinsicPresent({
        method: "deleteFiles",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 4, // 1 BSP extrinsic (6 files) + 3 Bucket extrinsics (2 files each)
        timeout: 30000
      });

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify BSP deletion event
      const bspDeletionEvents = (deletionResult.events || []).filter((record) =>
        userApi.events.fileSystem.BspFileDeletionsCompleted.is(record.event)
      );

      assert.equal(
        bspDeletionEvents.length,
        1,
        "Should have exactly 1 BSP deletion event (batches all 6 files)"
      );

      // Verify bucket deletion events
      const bucketDeletionEvents = (deletionResult.events || []).filter((record) =>
        userApi.events.fileSystem.BucketFileDeletionsCompleted.is(record.event)
      );

      assert.equal(
        bucketDeletionEvents.length,
        3,
        "Should have exactly 3 bucket deletion events (one per bucket with 2 files each)"
      );

      // Verify BSP root changed
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionsCompleted,
        deletionResult.events
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
        }
      });

      // Verify MSP roots changed for all 3 buckets
      for (const bucketDeletionRecord of bucketDeletionEvents) {
        const bucketDeletionEvent = bucketDeletionRecord.event;
        if (userApi.events.fileSystem.BucketFileDeletionsCompleted.is(bucketDeletionEvent)) {
          await waitFor({
            lambda: async () => {
              notEqual(
                bucketDeletionEvent.data.oldRoot.toString(),
                bucketDeletionEvent.data.newRoot.toString(),
                "MSP forest root should have changed after file deletion"
              );
              const currentBucketRoot = await msp1Api.rpc.storagehubclient.getForestRoot(
                bucketDeletionEvent.data.bucketId.toString()
              );
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
    });

    it("batches incomplete storage request deletions across multiple buckets with parallel BSP and bucket processing", async () => {
      const fileKeys: string[] = [];
      const bucketIds: string[] = [];

      // Pause MSP to ensure only BSP confirms
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");

      try {
        // Create 3 buckets, each with 2 files (6 files total) that will become incomplete
        for (let bucketIndex = 0; bucketIndex < 3; bucketIndex++) {
          const bucketName = `test-incomplete-bucket-${bucketIndex}`;

          // Create bucket
          const newBucketEvent = await userApi.createBucket(bucketName, null);
          const newBucketEventData =
            userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

          if (!newBucketEventData) {
            throw new Error("NewBucket event data not found");
          }

          const bucketId = newBucketEventData.bucketId;

          // Create 2 files in this bucket
          for (let fileIndex = 0; fileIndex < 2; fileIndex++) {
            const result = await userApi.file.newStorageRequest(
              "res/whatsup.jpg",
              `test/incomplete-b${bucketIndex}-f${fileIndex}.txt`,
              bucketId,
              shUser,
              ShConsts.DUMMY_MSP_ID,
              2
            );

            fileKeys.push(result.fileKey);
            bucketIds.push(bucketId.toString());

            // Wait for BSP to volunteer and store
            await userApi.wait.bspVolunteer();
            await waitFor({
              lambda: async () =>
                (await bspApi.rpc.storagehubclient.isFileInFileStorage(result.fileKey)).isFileFound
            });

            const bspAddress = userApi.createType("Address", bspKey.address);
            await userApi.wait.bspStored({
              expectedExts: 1,
              sealBlock: true,
              bspAccount: bspAddress
            });

            await waitForIndexing(userApi);
          }
        }

        // Revoke all 6 storage requests to create incomplete deletions
        for (const fileKey of fileKeys) {
          const revokeResult = await userApi.block.seal({
            calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
            signer: shUser
          });

          assertEventPresent(userApi, "fileSystem", "StorageRequestRevoked", revokeResult.events);
          assertEventPresent(
            userApi,
            "fileSystem",
            "IncompleteStorageRequest",
            revokeResult.events
          );
        }

        // Verify incomplete storage request state
        const incompleteStorageRequests =
          await userApi.query.fileSystem.incompleteStorageRequests.entries();
        assert(incompleteStorageRequests.length > 0, "Should have incomplete storage requests");

        await waitForIndexing(userApi, false);

        // Wait for fisherman to catch up with chain
        await userApi.wait.nodeCatchUpToChainTip(fishermanApi);

        // Verify incomplete deletion extrinsics are submitted (1 BSP + 3 Buckets = 4 total)
        // Note: May need to wait for alternation cycle (User vs Incomplete)
        await userApi.assert.extrinsicPresent({
          method: "deleteFilesForIncompleteStorageRequest",
          module: "fileSystem",
          checkTxPool: true,
          assertLength: 4, // 1 BSP extrinsic (6 files) + 3 Bucket extrinsics (2 files each)
          timeout: 60000 // Longer timeout to account for alternation between User/Incomplete types
        });

        // Seal block to process the extrinsics
        const deletionResult = await userApi.block.seal();

        // Verify BSP deletion event
        const bspDeletionEvents = (deletionResult.events || []).filter((record) =>
          userApi.events.fileSystem.BspFileDeletionsCompleted.is(record.event)
        );

        assert.equal(
          bspDeletionEvents.length,
          1,
          "Should have exactly 1 BSP deletion event (batches all 6 files)"
        );

        // Verify bucket deletion events
        const bucketDeletionEvents = (deletionResult.events || []).filter((record) =>
          userApi.events.fileSystem.BucketFileDeletionsCompleted.is(record.event)
        );

        assert.equal(
          bucketDeletionEvents.length,
          3,
          "Should have exactly 3 bucket deletion events (one per bucket with 2 files each)"
        );

        // Verify BSP root changed
        const bspDeletionEvent = userApi.assert.fetchEvent(
          userApi.events.fileSystem.BspFileDeletionsCompleted,
          deletionResult.events
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
          }
        });

        // Verify MSP roots changed for all 3 buckets
        for (const bucketDeletionRecord of bucketDeletionEvents) {
          const bucketDeletionEvent = bucketDeletionRecord.event;
          if (userApi.events.fileSystem.BucketFileDeletionsCompleted.is(bucketDeletionEvent)) {
            await waitFor({
              lambda: async () => {
                notEqual(
                  bucketDeletionEvent.data.oldRoot.toString(),
                  bucketDeletionEvent.data.newRoot.toString(),
                  "MSP forest root should have changed after file deletion"
                );
                const currentBucketRoot = await msp1Api.rpc.storagehubclient.getForestRoot(
                  bucketDeletionEvent.data.bucketId.toString()
                );
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
      } finally {
        // Always resume MSP container
        await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
        await userApi.docker.waitForLog({
          searchString: "💤 Idle",
          containerName: "storage-hub-sh-msp-1"
        });
      }
    });
  }
);

/**
 * Helper function to verify deletion signatures are stored in database
 */
async function verifyDeletionSignaturesStored(sql: SqlClient, fileKeys: string[]): Promise<void> {
  // Wait for first file to have signature stored
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
    assert.equal(filesWithSignature.length, 1, "File should have deletion signature stored");
    assert(
      filesWithSignature[0].deletion_signature.length > 0,
      "SCALE-encoded signature should not be empty"
    );
  }
}

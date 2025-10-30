import assert from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";
import {
  waitForFileIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation
} from "../../../util/indexerHelpers";

/**
 * FISHERMAN BATCH FILE DELETION INTEGRATION TESTS
 *
 * Validates fisherman batch processing for file deletions, ensuring files are grouped by
 * target (BSP/Bucket) and submitted in batched extrinsics.
 *
 * Test 1: User-Requested Deletions
 * - Setup: 3 buckets × 2 files = 6 files, users submit `requestDeleteFile` extrinsics
 * - Fisherman submits: 1 `deleteFiles` for BSP (6 files) + 3 `deleteFiles` for buckets (2 each)
 * - Events: `FileDeletionRequested`, `BspFileDeletionsCompleted`, `BucketFileDeletionsCompleted`
 * - Verifies: Database signatures, forest root updates, batch grouping
 *
 * Test 2: Incomplete Storage Deletions
 * - Setup: 3 buckets × 2 files = 6 files, users revoke via `revokeStorageRequest`
 * - Fisherman submits: 1 `deleteFilesForIncompleteStorageRequest` for BSP (6 files) + 3 for buckets
 * - Events: `StorageRequestRevoked`, `IncompleteStorageRequest`, `BspFileDeletionsCompleted`, `BucketFileDeletionsCompleted`
 * - Verifies: Incomplete storage cleanup, forest root updates
 *
 * Batch interval: 5 seconds (test config), 60 seconds (default)
 */
await describeMspNet(
  "Fisherman Batch File Deletion",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createFishermanApi,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
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
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(
        createFishermanApi,
        "Fisherman API not available. Ensure `fisherman` is set to `true` in the network configuration."
      );
      fishermanApi = await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      await userApi.block.seal({ finaliseBlock: true });
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sealBlock: false });
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

      // Create 3 buckets and prepare storage request transactions (6 files total)
      const storageRequestTxs = [];
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);

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

        // Prepare 2 file storage requests for this bucket
        for (let fileIndex = 0; fileIndex < 2; fileIndex++) {
          const {
            file_key,
            file_metadata: { location, fingerprint, file_size }
          } = await userApi.rpc.storagehubclient.loadFileInStorage(
            "res/smile.jpg",
            `test/batch-b${bucketIndex}-f${fileIndex}.txt`,
            ownerHex,
            bucketId.toString()
          );

          fileKeys.push(file_key.toString());
          bucketIds.push(bucketId.toString());
          locations.push(location.toHex());
          fingerprints.push(fingerprint.toHex());
          fileSizes.push(file_size.toNumber());

          storageRequestTxs.push(
            userApi.tx.fileSystem.issueStorageRequest(
              bucketId,
              location,
              fingerprint,
              file_size,
              userApi.shConsts.DUMMY_MSP_ID,
              [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
              { Custom: 1 }
            )
          );
        }
      }

      // Pause MSP to control storage request flow
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);
      // TODO: Figure out why MSP 2 responds to storage requests for MSP 1
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp2.containerName);

      // Seal all storage requests in a single block
      await userApi.block.seal({ calls: storageRequestTxs, signer: shUser });

      // Wait for all BSP volunteers to appear in tx pool
      await userApi.wait.bspVolunteer(fileKeys.length);
      await userApi.block.seal();

      // Wait for all BSP stored confirmations
      // BSP batches extrinsics, so we need to iteratively seal blocks and count events
      let totalConfirmations = 0;
      const maxAttempts = 3;
      for (
        let attempt = 0;
        attempt < maxAttempts && totalConfirmations < fileKeys.length;
        attempt++
      ) {
        // Wait for at least one bspConfirmStoring extrinsic in tx pool (don't check exact count)
        await userApi.wait.bspStored({
          sealBlock: false,
          timeoutMs: 5000
        });

        // Seal the block and count BspConfirmedStoring events
        const { events } = await userApi.block.seal();
        const confirmEvents = await userApi.assert.eventMany(
          "fileSystem",
          "BspConfirmedStoring",
          events
        );

        // Count total file keys in all BspConfirmedStoring events
        for (const eventRecord of confirmEvents) {
          if (userApi.events.fileSystem.BspConfirmedStoring.is(eventRecord.event)) {
            totalConfirmations += eventRecord.event.data.confirmedFileKeys.length;
          }
        }
      }

      assert.strictEqual(
        totalConfirmations,
        fileKeys.length,
        `Expected ${fileKeys.length} BSP confirmations, but got ${totalConfirmations}`
      );

      // Unpause MSP to process responses
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Wait for it to catch up to the tip of the chain
      await userApi.wait.nodeCatchUpToChainTip(msp1Api);

      // Wait for all MSP acceptance
      // MSP batches extrinsics, so we need to iteratively seal blocks and count events
      let totalAcceptance = 0;
      for (let attempt = 0; attempt < maxAttempts && totalAcceptance < fileKeys.length; attempt++) {
        await userApi.wait.mspResponseInTxPool();

        // Seal the block and count BspConfirmedStoring events
        const { events } = await userApi.block.seal();

        const acceptEvents = await userApi.assert.eventMany(
          "fileSystem",
          "MspAcceptedStorageRequest",
          events
        );

        // Count total MspAcceptedStorageRequest events
        totalAcceptance += acceptEvents.length;
      }

      assert.strictEqual(
        totalAcceptance,
        fileKeys.length,
        `Expected ${fileKeys.length} MSP acceptance, but got ${totalAcceptance}`
      );

      // Wait for BSP to store all files locally
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      // Wait for MSP to store all files
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Wait for all files to be indexed
      for (const fileKey of fileKeys) {
        await waitForFileIndexed(sql, fileKey);
        await waitForMspFileAssociation(sql, fileKey);
        await waitForBspFileAssociation(sql, fileKey);
      }

      // Build all deletion request calls
      const deletionCalls = [];
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

        deletionCalls.push(
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketIds[i],
            locations[i],
            fileSizes[i],
            fingerprints[i]
          )
        );
      }

      // Seal a single block with all deletion requests
      const deletionRequestResult = await userApi.block.seal({
        calls: deletionCalls,
        signer: shUser
      });

      // Verify all FileDeletionRequested events are present (one per file)
      const deletionRequestedEvents = (deletionRequestResult.events || []).filter((record) =>
        userApi.events.fileSystem.FileDeletionRequested.is(record.event)
      );

      assert.equal(
        deletionRequestedEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} FileDeletionRequested events`
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sealBlock: false });

      // Verify deletion signatures are stored in database for the User deletion type
      await indexerApi.indexer.verifyDeletionSignaturesStored({ sql, fileKeys });

      // Wait for fisherman to process user deletions and verify extrinsics are in tx pool
      const deletionResult = await userApi.fisherman.waitForBatchDeletions({
        deletionType: "User",
        expectExt: 4,
        sealBlock: true // Seal and return events for verification
      });

      assert(deletionResult, "Deletion result should be defined when sealBlock is true");

      // Verify BSP deletions
      await userApi.fisherman.verifyBspDeletionResults({
        userApi,
        bspApi,
        events: deletionResult.events,
        expectedCount: 1
      });

      // Verify bucket deletions
      await userApi.fisherman.verifyBucketDeletionResults({
        userApi,
        mspApi: msp1Api,
        events: deletionResult.events,
        expectedCount: 3
      });
    });

    it("batches incomplete storage request deletions across multiple buckets with parallel BSP and bucket processing", async () => {
      const fileKeys: string[] = [];
      const bucketIds: string[] = [];

      // Get value proposition before pausing MSP
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Create 3 buckets and prepare storage request transactions (6 files total) that will become incomplete
      const storageRequestTxs = [];
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);

      for (let bucketIndex = 0; bucketIndex < 3; bucketIndex++) {
        const bucketName = `test-incomplete-bucket-${bucketIndex}`;

        // Create bucket
        const newBucketEvent = await userApi.createBucket(bucketName, valuePropId);
        const newBucketEventData =
          userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

        if (!newBucketEventData) {
          throw new Error("NewBucket event data not found");
        }

        const bucketId = newBucketEventData.bucketId;

        // Prepare 2 file storage requests for this bucket
        for (let fileIndex = 0; fileIndex < 2; fileIndex++) {
          const {
            file_key,
            file_metadata: { location, fingerprint, file_size }
          } = await userApi.rpc.storagehubclient.loadFileInStorage(
            "res/whatsup.jpg",
            `test/incomplete-b${bucketIndex}-f${fileIndex}.txt`,
            ownerHex,
            bucketId.toString()
          );

          fileKeys.push(file_key.toString());
          bucketIds.push(bucketId.toString());

          storageRequestTxs.push(
            userApi.tx.fileSystem.issueStorageRequest(
              bucketId,
              location,
              fingerprint,
              file_size,
              userApi.shConsts.DUMMY_MSP_ID,
              [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
              { Custom: 2 } // Keep storage request alive so user can revoke it (assuming there is only a single BSP in the network)
            )
          );
        }
      }

      // Pause MSP to control storage request flow
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Seal all storage requests in a single block
      await userApi.block.seal({ calls: storageRequestTxs, signer: shUser });

      // Wait for all BSP volunteers to appear in tx pool
      await userApi.wait.bspVolunteer(fileKeys.length);
      await userApi.block.seal();

      // Wait for all BSP stored confirmations
      // BSP batches extrinsics, so we need to iteratively seal blocks and count events
      let totalConfirmations = 0;
      const maxAttempts = 3;
      for (
        let attempt = 0;
        attempt < maxAttempts && totalConfirmations < fileKeys.length;
        attempt++
      ) {
        // Wait for at least one bspConfirmStoring extrinsic in tx pool (don't check exact count)
        await userApi.wait.bspStored({
          sealBlock: false,
          timeoutMs: 5000
        });

        // Seal the block and count BspConfirmedStoring events
        const { events } = await userApi.block.seal();
        const confirmEvents = await userApi.assert.eventMany(
          "fileSystem",
          "BspConfirmedStoring",
          events
        );

        // Count total file keys in all BspConfirmedStoring events
        for (const eventRecord of confirmEvents) {
          if (userApi.events.fileSystem.BspConfirmedStoring.is(eventRecord.event)) {
            totalConfirmations += eventRecord.event.data.confirmedFileKeys.length;
          }
        }
      }

      assert.strictEqual(
        totalConfirmations,
        fileKeys.length,
        `Expected ${fileKeys.length} BSP confirmations, but got ${totalConfirmations}`
      );

      // Unpause MSP to process responses
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Wait for it to catch up to the tip of the chain
      await userApi.wait.nodeCatchUpToChainTip(msp1Api);

      // Wait for all MSP acceptance
      // MSP batches extrinsics, so we need to iteratively seal blocks and count events
      let totalAcceptance = 0;
      for (let attempt = 0; attempt < maxAttempts && totalAcceptance < fileKeys.length; attempt++) {
        await userApi.wait.mspResponseInTxPool();

        // Seal the block and count BspConfirmedStoring events
        const { events } = await userApi.block.seal();

        const acceptEvents = await userApi.assert.eventMany(
          "fileSystem",
          "MspAcceptedStorageRequest",
          events
        );

        // Count total MspAcceptedStorageRequest events
        totalAcceptance += acceptEvents.length;
      }

      assert.strictEqual(
        totalAcceptance,
        fileKeys.length,
        `Expected ${fileKeys.length} MSP acceptance, but got ${totalAcceptance}`
      );

      // Wait for BSP to store all files locally
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      // Wait for MSP to store all files
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Build all revocation calls
      const revocationCalls = fileKeys.map((fileKey) =>
        userApi.tx.fileSystem.revokeStorageRequest(fileKey)
      );

      // Seal a single block with all revocation requests
      const revokeResult = await userApi.block.seal({
        calls: revocationCalls,
        signer: shUser
      });

      // Verify all StorageRequestRevoked events are present (one per file)
      const revokedEvents = (revokeResult.events || []).filter((record) =>
        userApi.events.fileSystem.StorageRequestRevoked.is(record.event)
      );

      assert.equal(
        revokedEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} StorageRequestRevoked events`
      );

      // Verify all IncompleteStorageRequest events are present (one per file)
      const incompleteEvents = (revokeResult.events || []).filter((record) =>
        userApi.events.fileSystem.IncompleteStorageRequest.is(record.event)
      );

      assert.equal(
        incompleteEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} IncompleteStorageRequest events`
      );

      // Verify incomplete storage request state
      const incompleteStorageRequests =
        await userApi.query.fileSystem.incompleteStorageRequests.entries();
      assert(incompleteStorageRequests.length > 0, "Should have incomplete storage requests");

      // Seal and finalize block
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Wait for fisherman to catch up with chain
      await userApi.wait.nodeCatchUpToChainTip(fishermanApi);

      // Wait for fisherman to process incomplete storage deletions and verify extrinsics are in tx pool
      const deletionResult = await userApi.fisherman.waitForBatchDeletions({
        deletionType: "Incomplete",
        expectExt: 4,
        sealBlock: true // Seal and return events for verification
      });

      assert(deletionResult, "Deletion result should be defined when sealBlock is true");

      // Verify BSP deletions
      await userApi.fisherman.verifyBspDeletionResults({
        userApi,
        bspApi,
        events: deletionResult.events,
        expectedCount: 1
      });

      // Verify bucket deletions
      await userApi.fisherman.verifyBucketDeletionResults({
        userApi,
        mspApi: msp1Api,
        events: deletionResult.events,
        expectedCount: 3
      });

      // Always resume MSP container
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp2.containerName
      });
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.msp2.containerName
      });
    });
  }
);

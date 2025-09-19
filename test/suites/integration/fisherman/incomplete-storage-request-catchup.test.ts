import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent,
  assertEventMany,
  mspKey,
  sleep
} from "../../../util";
import { createBucketAndSendNewStorageRequest } from "../../../util/bspNet/fileHelpers";

/**
 * FISHERMAN INCOMPLETE STORAGE REQUESTS WITH CATCHUP
 *
 * Purpose: Tests the fisherman's ability to process incomplete storage request events
 *          (Expired, Revoked) from UNFINALIZED blocks during blockchain catchup scenarios.
 *
 * What makes this test unique:
 * - Creates incomplete storage request scenarios (expired, revoked) in unfinalized blocks.
 * - Tests fisherman indexer's catchup mechanism for these specific events.
 * - Verifies that the fisherman correctly identifies which providers (MSP, BSP, or both)
 *   need to perform a deletion and submits the appropriate extrinsics.
 */
await describeMspNet(
  "Fisherman Incomplete Storage Requests with Catchup",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      console.log("processing createUserApi");
      userApi = await createUserApi();

      console.log("processing createBspApi");
      bspApi = await createBspApi();

      console.log("processing createMspApi");
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");

      console.log("waiting for idle");
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      console.log("creating block");
      await userApi.rpc.engine.createBlock(true, true);
    });

    it("processes expired request (BSP only) in unfinalized block", async () => {
      const bucketName = "test-expired-bsp-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/expired-bsp.txt";

      // Pause MSP container to ensure only BSP accepts
      console.log("pausing msp container");
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");

      try {
        console.log("creating bucket and sending storage request");
        // TODO: Use userApi.file.createBucketAndSendNewStorageRequest - does not support passing finalized block
        const { fileKey } = await createBucketAndSendNewStorageRequest(
          userApi,
          source,
          destination,
          bucketName,
          null,
          null,
          null,
          1,
          false
        );

        // Wait for BSP to volunteer and store
        console.log("waiting for bsp to volunteer");
        await userApi.wait.bspVolunteer(undefined, false);
        console.log("waiting for is in file storage");
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });

        console.log("waiting for bsp to store");
        const bspAddress = userApi.createType("Address", bspKey.address);
        await userApi.wait.bspStored({
          expectedExts: 1,
          bspAccount: bspAddress,
          finalizeBlock: false
        });

        // Skip ahead to trigger expiration
        console.log("skipping ahead");
        const currentBlock = await userApi.rpc.chain.getBlock();
        console.log("current block number:", currentBlock.block.header.number.toNumber());
        const currentBlockNumber = currentBlock.block.header.number.toNumber();
        console.log("current block number:", currentBlockNumber);
        console.log("querying storage request ttl parameter");
        const storageRequestTtl = (
          await userApi.query.parameters.parameters({
            RuntimeConfig: {
              StorageRequestTtl: null
            }
          })
        )
          .unwrap()
          .asRuntimeConfig.asStorageRequestTtl.toNumber();
        console.log("storage request ttl:", storageRequestTtl);

        console.log("skipping ahead to trigger expiration");
        await userApi.block.skipTo(currentBlockNumber + storageRequestTtl, { finalised: false });

        // Verify only one delete extrinsic is submitted (for the BSP)
        console.log("waiting for delete file extrinsic in tx pool");
        await waitFor({
          lambda: async () => {
            const deleteFileMatch = await userApi.assert.extrinsicPresent({
              method: "deleteFileForIncompleteStorageRequest",
              module: "fileSystem",
              checkTxPool: true,
              assertLength: 1
            });
            return deleteFileMatch.length >= 1;
          },
          iterations: 300,
          delay: 100
        });
        console.log("delete file extrinsic found in tx pool");

        // Seal block to process the extrinsic
        console.log("sealing block to process delete extrinsic");
        const deletionResult = await userApi.block.seal();
        console.log("block sealed with deletion result");

        // Verify FileDeletedFromIncompleteStorageRequest event
        console.log("verifying FileDeletedFromIncompleteStorageRequest event");
        assertEventPresent(
          userApi,
          "fileSystem",
          "FileDeletedFromIncompleteStorageRequest",
          deletionResult.events
        );
        console.log("FileDeletedFromIncompleteStorageRequest event verified");
      } finally {
        // Always resume MSP container even if test fails
        console.log("resuming msp container");
        await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
        console.log("waiting for msp idle log");
        await userApi.docker.waitForLog({
          searchString: "💤 Idle",
          containerName: "storage-hub-sh-msp-1"
        });
        console.log("sleeping for 3 seconds");
        await sleep(3000);
        console.log("test cleanup complete");
      }
    });

    it("processes revoked request (MSP and BSP) in unfinalized block", async () => {
      console.log("starting revoked request test");
      const bucketName = "test-revoked-catchup";
      const source = "res/smile.jpg";
      const destination = "test/revoked-catchup.txt";

      console.log("creating bucket and sending storage request");
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        2, // Keep the storage request opened to be able to revoke
        false
      );
      console.log("storage request created with fileKey:", fileKey);

      console.log("waiting for msp response in tx pool");
      await userApi.wait.mspResponseInTxPool(1);
      console.log("msp response found in tx pool");

      // Wait for BSP to volunteer and store
      console.log("waiting for bsp volunteer");
      await userApi.wait.bspVolunteer(undefined, false);
      console.log("bsp volunteered");

      console.log("waiting for file in bsp file storage");
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });
      console.log("file found in bsp file storage");

      console.log("creating bsp address");
      const bspAddress = userApi.createType("Address", bspKey.address);
      console.log("waiting for bsp stored event");
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress,
        finalizeBlock: false
      });
      console.log("bsp stored event received");

      // Revoke the storage request in an unfinalized block
      console.log("revoking storage request in unfinalized block");
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser,
        finaliseBlock: false
      });
      console.log("storage request revoked");

      console.log("verifying StorageRequestRevoked event");
      assertEventPresent(
        userApi,
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );
      console.log("StorageRequestRevoked event verified");

      // Verify two delete extrinsics are submitted (for MSP and BSP)
      console.log("waiting for two delete file extrinsics in tx pool");
      await waitFor({
        lambda: async () => {
          const deleteFileMatch = await userApi.assert.extrinsicPresent({
            method: "deleteFileForIncompleteStorageRequest",
            module: "fileSystem",
            checkTxPool: true,
            assertLength: 2
          });
          return deleteFileMatch.length >= 2;
        },
        iterations: 300,
        delay: 100
      });
      console.log("two delete file extrinsics found in tx pool");

      // Seal block to process the extrinsics
      console.log("sealing block to process delete extrinsics");
      const deletionResult = await userApi.block.seal();
      console.log("block sealed with deletion results");

      // Verify FileDeletedFromIncompleteStorageRequest events
      console.log("verifying multiple FileDeletedFromIncompleteStorageRequest events");
      assertEventMany(
        userApi,
        "fileSystem",
        "FileDeletedFromIncompleteStorageRequest",
        deletionResult.events
      );
      console.log("multiple FileDeletedFromIncompleteStorageRequest events verified");
      console.log("revoked request test complete");
    });

    it("processes MSP stop storing bucket with incomplete request in unfinalized block", async () => {
      console.log("starting MSP stop storing bucket test");
      const bucketName = "test-msp-stop-incomplete-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/msp-stop-incomplete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      console.log("using msp id:", mspId);

      // Get value proposition for MSP
      console.log("querying value propositions for msp");
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;
      console.log("value prop id:", valuePropId);

      console.log("creating bucket and sending storage request");
      const { fileKey, bucketId } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        valuePropId,
        mspId,
        2, // Keep the storage request opened to be able to revoke
        false
      );
      console.log("storage request created with fileKey:", fileKey, "bucketId:", bucketId);

      // Wait for MSP to accept storage request
      console.log("waiting for msp response in tx pool");
      await userApi.wait.mspResponseInTxPool(1);
      console.log("msp response found in tx pool");

      // Wait for BSP to volunteer and store
      console.log("waiting for bsp volunteer");
      await userApi.wait.bspVolunteer(undefined, false);
      console.log("bsp volunteered");

      console.log("waiting for file in bsp file storage");
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });
      console.log("file found in bsp file storage");

      console.log("creating bsp address");
      const bspAddress = userApi.createType("Address", bspKey.address);
      console.log("waiting for bsp stored event");
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress,
        finalizeBlock: false
      });
      console.log("bsp stored event received");

      // MSP stops storing the bucket before revoke storage request so the incomplete storage request will have
      // no MSP storing the bucket at the time of file deletion
      console.log("msp stopping storage of bucket");
      const stopStoringResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.mspStopStoringBucket(bucketId)],
        signer: mspKey,
        finaliseBlock: false
      });
      console.log("msp stopped storing bucket");

      console.log("verifying MspStoppedStoringBucket event");
      assertEventPresent(
        userApi,
        "fileSystem",
        "MspStoppedStoringBucket",
        stopStoringResult.events
      );
      console.log("MspStoppedStoringBucket event verified");

      // Revoke the storage request to create incomplete state
      console.log("revoking storage request to create incomplete state");
      const revokeResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser,
        finaliseBlock: false
      });
      console.log("storage request revoked");

      console.log("verifying StorageRequestRevoked event");
      assertEventPresent(userApi, "fileSystem", "StorageRequestRevoked", revokeResult.events);
      console.log("StorageRequestRevoked event verified");

      console.log("verifying IncompleteStorageRequest event");
      assertEventPresent(userApi, "fileSystem", "IncompleteStorageRequest", revokeResult.events);
      console.log("IncompleteStorageRequest event verified");

      // Verify two delete extrinsics are submitted:
      // 1. For the bucket (no MSP present)
      // 2. For the BSP
      console.log("waiting for two delete file extrinsics in tx pool");
      await waitFor({
        lambda: async () => {
          const deleteFileMatch = await userApi.assert.extrinsicPresent({
            method: "deleteFileForIncompleteStorageRequest",
            module: "fileSystem",
            checkTxPool: true,
            assertLength: 2
          });
          return deleteFileMatch.length >= 2;
        },
        iterations: 300,
        delay: 100
      });
      console.log("two delete file extrinsics found in tx pool");

      // Seal block to process the extrinsics
      console.log("sealing block to process delete extrinsics");
      const deletionResult = await userApi.block.seal();
      console.log("block sealed with deletion results");

      // Verify FileDeletedFromIncompleteStorageRequest events
      console.log("verifying multiple FileDeletedFromIncompleteStorageRequest events");
      assertEventMany(
        userApi,
        "fileSystem",
        "FileDeletedFromIncompleteStorageRequest",
        deletionResult.events
      );
      console.log("multiple FileDeletedFromIncompleteStorageRequest events verified");

      // Verify BucketFileDeletionCompleted event with no MSP ID
      console.log("fetching BucketFileDeletionCompleted event");
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BucketFileDeletionCompleted,
        deletionResult.events
      );
      console.log("BucketFileDeletionCompleted event fetched");

      // Verify that msp_id is None in the deletion event
      console.log("verifying msp_id is None in deletion event");
      assert(mspDeletionEvent.data.mspId.isNone, "MSP ID should be None since bucket has no MSP");
      console.log("msp_id verified as None");

      // Verify bucket root changed
      console.log("verifying bucket root changed");
      assert(
        mspDeletionEvent.data.oldRoot.toString() !== mspDeletionEvent.data.newRoot.toString(),
        "Bucket root should have changed after file deletion"
      );
      console.log("bucket root change verified");
      console.log("MSP stop storing bucket test complete");
    });
  }
);

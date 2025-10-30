import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  waitFor,
  ShConsts
} from "../../../util";
import {
  hexToBuffer,
  waitForFileIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation
} from "../../../util/indexerHelpers";

/**
 * FISHERMAN FILE DELETION FLOW - BASIC HAPPY PATH
 *
 * Purpose: Tests the standard, straightforward file deletion workflow using finalized blocks.
 *          This is the foundation test for fisherman file deletion functionality.
 *
 * What makes this test unique:
 * - Uses finalized blocks throughout (standard blockchain behavior)
 * - Tests basic file storage and deletion workflow step-by-step
 * - Creates storage requests with single replication target
 * - Focuses on core functionality without edge cases or complex scenarios
 *
 * Test Scenario:
 * 1. Creates storage request with single replication target (BSP and MSP)
 * 2. BSP volunteers and confirms storage (using whatsup.jpg for automatic volunteering)
 * 3. MSP accepts storage request and confirms storage
 * 4. User sends file deletion request
 * 5. Verifies fisherman indexes all events correctly and processes deletions
 * 6. Verifies both BSP and MSP forest root changes after deletion
 *
 * Note: The user node is running the indexer, so any finalize blocks we seal on the user node, directly affects the data that is being
 * indexed in the database.
 */
await describeMspNet(
  "Fisherman File Deletion Flow",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api, createSqlClient }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let sql: SqlClient;
    let fileKey: string;
    let bucketId: string;
    let location: string;
    let fingerprint: string;
    let fileSize: number;

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

      await userApi.block.seal({ finaliseBlock: true });
      await userApi.indexer.waitForIndexing({});
    });

    it("creates storage request, waits for MSP and BSP to accept and confirm, verifies indexer database", async () => {
      const bucketName = "test-fisherman-deletion";
      const source = "res/whatsup.jpg";
      const destination = "test/fisherman-delete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        1,
        true
      );

      fileKey = fileMetadata.fileKey;
      bucketId = fileMetadata.bucketId;
      location = fileMetadata.location;
      fingerprint = fileMetadata.fingerprint;
      fileSize = fileMetadata.fileSize;

      // Wait for MSP to store the file
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer();
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      await userApi.indexer.waitForIndexing({});
      await waitForFileIndexed(sql, fileKey);
      await waitForMspFileAssociation(sql, fileKey);
      await waitForBspFileAssociation(sql, fileKey);
    });

    it("user sends file deletion request and fisherman submits delete_files extrinsics", async () => {
      // Create file deletion request
      const fileOperationIntention = {
        fileKey: fileKey,
        operation: { Delete: null }
      };

      // Create the user signature for the file deletion intention
      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

      // Submit the file deletion request
      const deletionRequestResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketId,
            location,
            fileSize,
            fingerprint
          )
        ],
        signer: shUser
      });

      await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      await userApi.indexer.waitForIndexing({ producerApi: userApi, sealBlock: false });

      // Verify that the deletion signature was stored in the database (SCALE-encoded)
      await waitFor({
        lambda: async () => {
          const files = await sql`
            SELECT deletion_signature FROM file
            WHERE file_key = ${hexToBuffer(fileKey)}
            AND deletion_signature IS NOT NULL
          `;
          return files.length > 0;
        }
      });

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

      // Wait for fisherman to process user deletions and verify extrinsics are in tx pool
      const deletionResult = await userApi.fisherman.waitForBatchDeletions({
        deletionType: "User",
        expectExt: 2,
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
        expectedCount: 1
      });
    });
  }
);

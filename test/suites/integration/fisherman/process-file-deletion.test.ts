import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  waitFor,
  assertEventPresent
} from "../../../util";
import { createBucketAndSendNewStorageRequest } from "../../../util/bspNet/fileHelpers";
import {
  waitForFileIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation
} from "../../../util/indexerHelpers";
import { waitForIndexing } from "../../../util/fisherman/indexerTestHelpers";
import {
  waitForDeleteFileExtrinsic,
  waitForFishermanProcessing
} from "../../../util/fisherman/fishermanHelpers";

/**
 * FISHERMAN PROCESS FILE DELETION - COMPREHENSIVE EVENT PROCESSING
 *
 * Purpose: Tests the fisherman's comprehensive event processing capabilities for various
 *          file deletion scenarios and edge cases.
 *
 * What makes this test unique:
 * - Tests MULTIPLE types of deletion-related events:
 *   * FileDeletionRequested - direct user deletion requests
 *   * StorageRequestExpired - cleanup of expired storage requests
 *   * StorageRequestRevoked - cleanup of user-revoked requests
 *   * StorageRequestRejected - cleanup of provider-rejected requests
 * - Tests multiple provider scenarios (both BSP and MSP for same file)
 * - Includes extensive log verification for fisherman processing
 * - Uses container pausing/resuming to simulate network conditions
 * - Tests fisherman's preparation of delete_file extrinsics
 *
 * Test Scenarios:
 * 1. FileDeletionRequested: Normal user-initiated deletion with multiple providers
 * 2. StorageRequestExpired: Paused providers causing expiration, fisherman cleanup
 * 3. StorageRequestRevoked: User revokes request before acceptance, fisherman cleanup
 * 4. Multiple providers: File stored by both BSP and MSP, deletion affects both
 * 5. StorageRequestRejected: Provider rejection scenarios (placeholder for future)
 */
describeMspNet(
  "Fisherman Process File Deletion",
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

      await userApi.rpc.engine.createBlock(true, true);

      await waitForIndexing(userApi);
    });

    // Helper function to verify fisherman preparation logs
    async function verifyFishermanPreparationLogs(
      api: EnrichedBspApi,
      fileKey: string,
      expectedPatterns: string[]
    ): Promise<void> {
      const hexFileKey = fileKey.startsWith("0x") ? fileKey.slice(2) : fileKey;

      // Wait for the main processing log
      const processingFound = await waitForFishermanProcessing(
        api,
        `Processing file deletion request for signed intention file key: 0x${hexFileKey}`
      );
      assert(processingFound, "Should find fisherman processing log");

      // Wait for extrinsic submission log
      const submittingExtrinsic = await waitForFishermanProcessing(
        api,
        "Submitting delete_file extrinsic"
      );
      assert(submittingExtrinsic, "Should find extrinsic submission log");

      // Check for additional expected patterns
      for (const pattern of expectedPatterns) {
        const found = await waitForFishermanProcessing(api, pattern, 5000);
        if (!found) {
          console.warn(`Expected fisherman log pattern not found: ${pattern}`);
        }
      }
    }

    it("processes FileDeletionRequested event and prepares delete_file extrinsic", async () => {
      const bucketName = "test-fisherman-deletion";
      const source = "res/smile.jpg";
      const destination = "test/fisherman-delete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await createBucketAndSendNewStorageRequest(
          userApi,
          source,
          destination,
          bucketName,
          valuePropId,
          mspId,
          null,
          1
        );

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

      await waitForIndexing(userApi);
      await waitForFileIndexed(sql, fileKey);
      await waitForMspFileAssociation(sql, fileKey);
      await waitForBspFileAssociation(sql, fileKey);

      // Create file deletion request
      const fileOperationIntention = {
        fileKey: fileKey,
        operation: { Delete: null }
      };

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

      assertEventPresent(
        userApi,
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      await waitForIndexing(userApi);

      // Verify fisherman processes the FileDeletionRequested event
      await verifyFishermanPreparationLogs(userApi, fileKey, [
        "File deletion parameters prepared:",
        `File key: 0x${fileKey.startsWith("0x") ? fileKey.slice(2) : fileKey}`,
        "Provider ID:"
      ]);

      // Verify delete_file extrinsics are submitted
      const deleteFileFound = await waitForDeleteFileExtrinsic(userApi, 2);
      assert(
        deleteFileFound,
        "Should find 2 delete_file extrinsics in transaction pool (BSP and MSP)"
      );

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify both deletion completion events
      assertEventPresent(userApi, "fileSystem", "MspFileDeletionCompleted", deletionResult.events);
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", deletionResult.events);
    });

    it("processes StorageRequestRejected event when MSP doesn't accept in time", async () => {
      const bucketName = "test-fisherman-expired";
      const source = "res/whatsup.jpg";
      const destination = "test/expired.txt";

      // Pause MSP and BSP containers to prevent them from accepting the storage request
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");
      await userApi.docker.pauseContainer("storage-hub-sh-bsp-1");

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

      // Skip ahead to trigger expiration
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const storageRequestTtl = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            StorageRequestTtl: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asStorageRequestTtl.toNumber();
      await userApi.block.skipTo(currentBlockNumber + storageRequestTtl);

      await waitForIndexing(userApi);

      // Wait for StorageRequestRejected event to be processed by fisherman
      const rejectedProcessingFound = await waitForFishermanProcessing(
        userApi,
        `Found StorageRequestRejected event for file key: 0x${fileKey.startsWith("0x") ? fileKey.slice(2) : fileKey}`
      );
      assert(rejectedProcessingFound, "Should find fisherman processing rejected storage request");

      const incompleteProcessingFound = await waitForFishermanProcessing(
        userApi,
        `Processing incomplete storage request for file key: 0x${fileKey.startsWith("0x") ? fileKey.slice(2) : fileKey}`
      );
      assert(incompleteProcessingFound, "Should find fisherman processing incomplete storage");

      // Resume containers for cleanup
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-bsp-1" });

      // TODO: Verify extrinsic submission once implementation is complete
    });

    it("processes StorageRequestRevoked event and prepares deletion", async () => {
      const bucketName = "test-fisherman-revoked";
      const source = "res/smile.jpg";
      const destination = "test/revoked.txt";

      // Pause MSP and BSP to prevent acceptance before revocation
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");
      await userApi.docker.pauseContainer("storage-hub-sh-bsp-1");

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

      // Revoke the storage request
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );

      await waitForIndexing(userApi);

      // Wait for fisherman to process the revocation
      const revokedProcessingFound = await waitForFishermanProcessing(
        userApi,
        `Found StorageRequestRevoked event for file key: 0x${fileKey.startsWith("0x") ? fileKey.slice(2) : fileKey}`
      );
      assert(revokedProcessingFound, "Should find fisherman processing revoked storage request");

      const incompleteProcessingFound = await waitForFishermanProcessing(
        userApi,
        `Processing incomplete storage request for file key: 0x${fileKey.startsWith("0x") ? fileKey.slice(2) : fileKey}`
      );
      assert(incompleteProcessingFound, "Should find fisherman processing incomplete storage");

      // Resume containers
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-bsp-1" });

      // TODO: Verify extrinsic submission once implementation is complete
    });

    it("processes multiple providers for same file deletion", async () => {
      const bucketName = "test-fisherman-multiple";
      const source = "res/whatsup.jpg";
      const destination = "test/multiple.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await createBucketAndSendNewStorageRequest(
          userApi,
          source,
          destination,
          bucketName,
          valuePropId,
          mspId,
          null,
          1
        );

      // Wait for both MSP and BSP to store the file
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();
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

      await waitForIndexing(userApi);
      await waitForFileIndexed(sql, fileKey);
      await waitForMspFileAssociation(sql, fileKey);
      await waitForBspFileAssociation(sql, fileKey);

      // Create and submit file deletion request
      const fileOperationIntention = {
        fileKey: fileKey,
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
            bucketId,
            location,
            fileSize,
            fingerprint
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

      await waitForIndexing(userApi, false);

      // Verify TWO delete_file extrinsics are submitted (one for BSP and one for MSP)
      const deleteFileFound = await waitForDeleteFileExtrinsic(userApi, 2);
      assert(
        deleteFileFound,
        "Should find 2 delete_file extrinsics in transaction pool (BSP and MSP)"
      );

      // Seal block to process the extrinsics
      const deletionResult = await userApi.block.seal();

      // Verify both deletion completion events
      assertEventPresent(userApi, "fileSystem", "MspFileDeletionCompleted", deletionResult.events);
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", deletionResult.events);
    });

    it("handles StorageRequestRejected event processing", async () => {
      const bucketName = "test-fisherman-rejected";
      const source = "res/smile.jpg";
      const destination = "test/rejected.txt";

      // This test simulates a rejection scenario
      // In practice, rejection might happen due to various validation failures
      // For now, we'll create a request and manually trigger a rejection-like scenario

      // Pause containers to prevent normal processing
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");
      await userApi.docker.pauseContainer("storage-hub-sh-bsp-1");

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

      // Skip some blocks and then resume to potentially trigger rejection-like behavior
      await userApi.block.seal();
      await userApi.block.seal();

      // Resume containers
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-bsp-1" });

      await waitForIndexing(userApi);

      // Note: StorageRequestRejected events are harder to trigger in integration tests
      // This test serves as a placeholder for when such scenarios can be reliably created
      // The fisherman should handle rejection events similarly to expiration/revocation

      console.log(`Created test scenario for rejection handling with file key: ${fileKey}`);
      // TODO: Enhance this test when reliable rejection scenarios can be created
    });
  }
);

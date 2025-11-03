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

/**
 * FISHERMAN FILE DELETION FLOW WITH CATCHUP
 *
 * Purpose: Tests the fisherman's ability to build forest proofs that include files from
 *          unfinalized blocks when processing deletion requests.
 *
 * What makes this test unique:
 * - Pauses fisherman to accumulate events
 * - Creates finalized deletion request for an existing file
 * - Adds NEW files in unfinalized blocks (different from the file being deleted)
 * - Tests fisherman's ability to build proofs with unfinalized forest state
 *
 * Test Scenario:
 * 1. Creates and finalizes a file with BSP and MSP storage (this file will be deleted)
 * 2. Pauses fisherman service
 * 3. Finalizes a file deletion request for the file from step 1
 * 4. Creates 3 NEW files in unfinalized blocks (adds to BSP/MSP forests)
 * 5. Resumes fisherman - it should build proofs including the new unfinalized files
 */
await describeMspNet(
  "Fisherman File Deletion Flow with Catchup",
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
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    // File to be deleted (created in finalized state)
    let fileToDelete: {
      fileKey: string;
      bucketId: string;
      location: string;
      fingerprint: string;
      fileSize: number;
    };

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Wait for indexer to process the finalized block (producerApi will seal a finalized block by default)
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
    });

    it("Step 1: Create and finalize file that will be deleted later", async () => {
      const bucketName = "test-deletion-catchup-bucket";
      const source = "res/whatsup.jpg";
      const destination = "test/file-to-delete-catchup.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      fileToDelete = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        1,
        true // Finalize this file
      );

      // Wait for MSP to store the file
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileToDelete.fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer();
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileToDelete.fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      // Wait for indexer to process the finalized file
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
      await indexerApi.indexer.waitForFileIndexed({ sql, fileKey: fileToDelete.fileKey });
      await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey: fileToDelete.fileKey });
      await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey: fileToDelete.fileKey });
    });

    it("Step 2-5: Pause fisherman, finalize deletion, add unfinalized files, resume fisherman", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Step 2: Pause fisherman service
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.fisherman.containerName);

      // Step 3: Finalize file deletion request for the file created in step 1
      // Ensure file is in MSP's forest storage before deletion
      await waitFor({
        lambda: async () => {
          const isFileInForest = await msp1Api.rpc.storagehubclient.isFileInForest(
            fileToDelete.bucketId.toString(),
            fileToDelete.fileKey.toString()
          );
          return isFileInForest.isTrue;
        }
      });

      // Create file operation intention for deletion
      const fileOperationIntention = {
        fileKey: fileToDelete.fileKey,
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

      // Submit file deletion request in a FINALIZED block (fisherman is paused)
      const deletionRequestResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            fileToDelete.bucketId,
            fileToDelete.location,
            fileToDelete.fileSize,
            fileToDelete.fingerprint
          )
        ],
        signer: shUser,
        finaliseBlock: true
      });

      // Verify FileDeletionRequested event
      await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      // Wait for indexer to process the finalized deletion request
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Step 4: Create 3 NEW files in UNFINALIZED blocks (these will be added to forests)
      await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/catchup-new-0.txt",
            bucketIdOrName: "test-catchup-new-file-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/catchup-new-1.txt",
            bucketIdOrName: "test-catchup-new-file-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/catchup-new-2.txt",
            bucketIdOrName: "test-catchup-new-file-2",
            replicationTarget: 1
          }
        ],
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi: msp1Api,
        finaliseBlock: false
      });

      // Verify there's a gap between finalized and current head
      const finalizedHead = await userApi.rpc.chain.getFinalizedHead();
      const currentHead = await userApi.rpc.chain.getHeader();
      assert(
        currentHead.number.toNumber() >
          (await userApi.rpc.chain.getHeader(finalizedHead)).number.toNumber(),
        "Current head should be ahead of finalized head (unfinalized files added)"
      );

      // Step 5: Resume fisherman - it should process deletion with updated forest state
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName
      });

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName,
        tail: 10
      });

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2, // 1 BSP + 1 Bucket
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 1,
        maxRetries: 3
      });
    });
  }
);

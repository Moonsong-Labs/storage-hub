import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  sleep,
  waitFor,
  ShConsts
} from "../../../util";
import type { H256 } from "@polkadot/types/interfaces";

describeMspNet(
  "Fisherman File Deletion Flow with Catchup",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    fishermanIndexerMode: "fishing"
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api, createFishermanApi, createSqlClient }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    let sql: SqlClient;
    let bucketId: H256;
    let fileKey: H256;
    let fileLocation: string;
    let fileFingerprint: H256;
    let fileSize: number;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for fisherman node to be ready
      await userApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "💤 Idle",
        timeout: 30000
      });

      // Create fisherman and MSP APIs
      assert(createFishermanApi, "Fisherman API should be available when fisherman is enabled");
      fishermanApi = await createFishermanApi() as EnrichedBspApi;
      assert(fishermanApi, "Fisherman API should be created successfully");

      assert(createMsp1Api, "MSP1 API should be available");
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      // Initialize blockchain state
      await userApi.rpc.engine.createBlock(true, true);
      await sleep(1000);
      await userApi.block.seal();
      await userApi.block.seal();

      // Wait for fisherman indexer to start in fishing mode
      await fishermanApi.docker.waitForLog({
        containerName: ShConsts.NODE_INFOS.fisherman.containerName,
        searchString: "IndexerService starting up in Fishing mode!",
        timeout: 10000
      });
    });

    it("creates finalized block with storage request and BSP & MSP confirming", async () => {
      const bucketName = "test-deletion-catchup-bucket";
      const source = "res/whatsup.jpg";
      const destination = "test/file-to-delete-catchup.txt";

      // Create bucket
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      assert(newBucketEventData, "NewBucket event data not found");
      bucketId = newBucketEventData.bucketId;

      // Load file for BSP storage (using whatsup.jpg for automatic volunteering)
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        bucketId
      );

      // Store file metadata for later use
      fileLocation = location;
      fileFingerprint = fingerprint;
      fileSize = file_size;

      // Issue storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Basic: null }
          )
        ],
        signer: shUser
      });

      // Get the file key
      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");
      const eventData = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      fileKey = eventData.fileKey;

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer();

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      // Wait for BSP to confirm storage
      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      // Wait for MSP to receive the file
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      // Wait for MSP to accept the storage request
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Get the MspAcceptedStorageRequest event
      const { event: mspAcceptedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const mspAcceptedEventDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(mspAcceptedEvent) &&
        mspAcceptedEvent.data;

      assert(
        mspAcceptedEventDataBlob,
        "MspAcceptedStorageRequest event data does not match expected type"
      );

      const acceptedFileKey = mspAcceptedEventDataBlob.fileKey.toString();
      assert.equal(acceptedFileKey, fileKey.toString());

      // Wait for indexing
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify file is indexed with both BSP and MSP associations
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${fileKey.toString()}
      `;
      assert(files.length > 0, "File should be indexed");

      const bspFiles = await sql`
        SELECT * FROM bsp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${fileKey.toString()}
        )
      `;
      assert(bspFiles.length > 0, "BSP file association should be indexed");

      const mspFiles = await sql`
        SELECT * FROM msp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${fileKey.toString()}
        )
      `;
      assert(mspFiles.length > 0, "MSP file association should be indexed");
    });

    it("creates 3 unfinalized blocks", async () => {
      // Create 3 unfinalized blocks with some activity
      for (let i = 0; i < 3; i++) {
        // Create some blockchain activity (e.g., transfers)
        await userApi.block.seal({
          calls: [
            userApi.tx.balances.transferAllowDeath(
              userApi.shConsts.NODE_INFOS.bsp1.AddressId,
              1000000n
            )
          ],
          signer: shUser,
          finaliseBlock: false
        });

        // Add a small delay between blocks
        await sleep(500);
      }

      // Verify blocks were created but not finalized
      const finalizedHead = await userApi.rpc.chain.getFinalizedHead();
      const currentHead = await userApi.rpc.chain.getHeader();

      console.log(`Current head: ${currentHead.number.toString()}`);
      console.log(`Finalized head number: ${(await userApi.rpc.chain.getHeader(finalizedHead)).number.toString()}`);

      // There should be a gap between finalized and current head
      assert(
        currentHead.number.toNumber() > (await userApi.rpc.chain.getHeader(finalizedHead)).number.toNumber(),
        "Current head should be ahead of finalized head"
      );
    });

    it("sends file deletion request in unfinalized block and verifies indexing", async () => {
      // Ensure file is in MSP's forest storage before deletion attempt
      await waitFor({
        lambda: async () => {
          const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
            bucketId.toString(),
            fileKey.toString()
          );
          return isFileInForest.isTrue;
        }
      });

      // Create file operation intention for deletion
      const fileOperationIntention = {
        fileKey: fileKey,
        operation: { Delete: null }
      };

      // Create signature for the intention - encode the object
      const intentionType = userApi.createType("PalletFileSystemFileOperationIntention", fileOperationIntention);
      const encodedIntention = intentionType.toHex();
      const rawSignature = shUser.sign(encodedIntention);

      // Create the signature object with Sr25519 variant
      const signature = {
        Sr25519: rawSignature
      };

      // Submit file deletion request in an unfinalized block
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            signature,
            bucketId,
            fileLocation,
            fileSize,
            fileFingerprint
          )
        ],
        signer: shUser,
        finaliseBlock: false
      });

      // Verify FileDeletionRequested event
      const { event: deletionEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "FileDeletionRequested"
      );

      const deletionEventData =
        userApi.events.fileSystem.FileDeletionRequested.is(deletionEvent) && deletionEvent.data;

      assert(deletionEventData, "FileDeletionRequested event data not found");
      const eventFileKey = deletionEventData.signedDeleteIntention.fileKey;
      assert.equal(eventFileKey.toString(), fileKey.toString());

      // Wait for indexing to process the deletion request from unfinalized block
      await userApi.block.seal({ finaliseBlock: false });
      await userApi.block.seal({ finaliseBlock: false });

      // Verify current chain state
      const finalizedHead2 = await userApi.rpc.chain.getFinalizedHead();
      const currentHead2 = await userApi.rpc.chain.getHeader();

      console.log(`After deletion - Current head: ${currentHead2.number.toString()}`);
      console.log(`After deletion - Finalized head number: ${(await userApi.rpc.chain.getHeader(finalizedHead2)).number.toString()}`);

      // Verify fisherman can index events from unfinalized blocks
      // The indexer should still process events even though they're in unfinalized blocks
      console.log("Fisherman indexer is processing events from unfinalized blocks in catchup mode");

      // TODO: Once the fisherman extrinsic for file deletion is merged to main,
      // add test verification for:
      // 1. Fisherman node sends extrinsic to delete file on-chain
      // 2. Verify the appropriate event is emitted
      // 3. Verify database state is updated accordingly
      // 4. Verify fisherman properly handles unfinalized blocks during catchup
      console.log("TODO: Verify fisherman sends file deletion extrinsic from unfinalized blocks (not yet merged to main)");
    });
  }
);
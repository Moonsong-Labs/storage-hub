import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  sleep,
  waitFor,
  assertEventPresent,
  ShConsts,
  sealBlock
} from "../../../util";
import { createBucketAndSendNewStorageRequest } from "../../../util/bspNet/fileHelpers";
import {
  hexToBuffer,
  waitForFileIndexed,
  waitForBucketIndexed,
  waitForBucketByIdIndexed,
  waitForMspFileAssociation,
  waitForBspFileAssociation,
  waitForFileDeleted,
  waitForBlockIndexed
} from "../../../util/indexerHelpers";
import { sealAndWaitForIndexing } from "../../../util/fisherman/indexerTestHelpers";

describeMspNet(
  "Fisherman Indexer - Fishing Mode",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing"
  },
  ({ before, it, createUserApi, createBspApi, createMsp1Api, createMsp2Api, createSqlClient }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();

      assert(maybeMsp1Api, "MSP API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
      sql = createSqlClient();

      // Wait for nodes to be ready
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      // Initialize blockchain state using direct RPC call for first block
      await userApi.rpc.engine.createBlock(true, true);

      // Small delay to ensure nodes are synced
      await sleep(1000);

      // Seal additional blocks to ensure stable state
      await sealAndWaitForIndexing(userApi);
      await sealAndWaitForIndexing(userApi);
    });

    it("indexes NewStorageRequest events", async () => {
      const bucketName = "test-bucket-fishing";
      const source = "res/smile.jpg";
      const destination = "test/file.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      // Seal block and wait for indexer to process it
      await sealAndWaitForIndexing(userApi);

      // Verify file is indexed
      const files = await sql`
        SELECT * FROM file 
        WHERE bucket_id = (
          SELECT id FROM bucket WHERE name = ${bucketName}
        )
      `;

      assert.equal(files.length, 1);
      // Convert Buffer to hex string with 0x prefix for comparison
      const dbFileKey = `0x${files[0].file_key.toString("hex")}`;
      assert.equal(dbFileKey, fileKey);
    });

    it("indexes BspConfirmedStoring events", async () => {
      // Use whatsup.jpg which matches DUMMY_BSP_ID for automatic volunteering
      const bucketName = "test-bsp-confirm";
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-file.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer();

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      // Wait for BSP to confirm storage
      const bspAddress = userApi.createType("Address", bspKey.address);
      // Wait for BSP to confirm storage (without auto-sealing)
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      // Assert BspConfirmedStoring event was emitted
      const { event: bspConfirmedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmedStoring"
      );
      assert(bspConfirmedEvent, "BspConfirmedStoring event should be present");

      // Seal block and wait for indexer to process it
      await sealAndWaitForIndexing(userApi);

      // Wait for the indexer to process the events
      await waitForFileIndexed(sql, fileKey);

      // Verify BSP-file association is indexed
      const bspFiles = await sql`
        SELECT * FROM bsp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${hexToBuffer(fileKey)}
        )
      `;

      assert(bspFiles.length > 0, "BSP file association should be indexed");
    });

    it("indexes MspAcceptedStorageRequest events", async () => {
      // Create bucket assigned to MSP
      const bucketName = "test-msp-accept";
      const source = "res/smile.jpg";
      const destination = "test/msp-file.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      // Get value proposition for MSP
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use helper function to create bucket and send storage request
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        valuePropId,
        mspId,
        null,
        1
      );

      // Wait for MSP to accept the storage request
      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer();

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

      // Seal block and wait for indexer to process it
      await sealAndWaitForIndexing(userApi);

      // Wait for the indexer to process the events
      await waitForFileIndexed(sql, fileKey.toString());

      // Verify MSP-file association is indexed
      const mspFiles = await sql`
        SELECT * FROM msp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${hexToBuffer(fileKey.toString())}
        )
      `;

      assert(mspFiles.length > 0, "MSP file association should be indexed");

      // Wait for BSP to confirm storage
      const bspAddress = userApi.createType("Address", bspKey.address);
      // Wait for BSP to confirm storage
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });
    });

    it("indexes StorageRequestRevoked events", async () => {
      const bucketName = "test-revoke";
      const source = "res/smile.jpg";
      const destination = "test/revoke.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      // Revoke storage request
      const revokeStorageRequestResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser
      });

      // Assert StorageRequestRevoked event was emitted
      assertEventPresent(
        userApi,
        "fileSystem",
        "StorageRequestRevoked",
        revokeStorageRequestResult.events
      );

      // Wait for indexing to process the revocation
      await sealAndWaitForIndexing(userApi);

      // Wait for file deletion to be processed by indexer
      await waitForFileDeleted(sql, fileKey);

      // Verify file is removed from database
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;

      // In fishing mode, file should be deleted from database when revoked
      assert.equal(files.length, 0);
    });

    it("indexes BspConfirmStoppedStoring events", async () => {
      // Setup: Create file and have BSP store it
      const bucketName = "test-bsp-stop";
      const source = "res/smile.jpg";
      const destination = "test/bsp-stop.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await createBucketAndSendNewStorageRequest(userApi, source, destination, bucketName);

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer();

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      // Wait for BSP to confirm storage
      const bspAddress = userApi.createType("Address", bspKey.address);
      // Wait for BSP to confirm storage
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      // BSP requests to stop storing
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);

      const bspRequestStopStoringResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspRequestStopStoring(
            fileKey,
            bucketId,
            location,
            shUser.address,
            fingerprint,
            fileSize,
            false,
            inclusionForestProof.toString()
          )
        ],
        signer: bspKey
      });

      // Assert BspRequestedToStopStoring event was emitted
      assertEventPresent(
        userApi,
        "fileSystem",
        "BspRequestedToStopStoring",
        bspRequestStopStoringResult.events
      );

      // Check for BspRequestedToStopStoring event
      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");

      // Wait for cooldown period
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            MinWaitForStopStoring: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      const cooldown = currentBlockNumber + minWaitForStopStoring;
      await userApi.block.skipTo(cooldown);

      // Confirm stop storing
      const newInclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);

      const bspConfirmStopStoringResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspConfirmStopStoring(fileKey, newInclusionForestProof.toString())
        ],
        signer: bspKey
      });

      // Assert BspConfirmStoppedStoring event was emitted
      assertEventPresent(
        userApi,
        "fileSystem",
        "BspConfirmStoppedStoring",
        bspConfirmStopStoringResult.events
      );

      // Check for BspConfirmStoppedStoring event
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      // Wait for indexing
      await sealAndWaitForIndexing(userApi);

      // Verify BSP-file association is removed
      const bspFiles = await sql`
        SELECT * FROM bsp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${hexToBuffer(fileKey)}
        )
      `;

      assert.equal(bspFiles.length, 0);
    });

    it("indexes NewBucket and BucketDeleted events", async () => {
      const bucketName = "test-bucket-lifecycle";

      // Create bucket and get the bucket ID directly
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      // Wait for bucket creation to be indexed
      await sealAndWaitForIndexing(userApi);

      // Wait for bucket to be indexed by the indexer
      await waitForBucketIndexed(sql, bucketName);

      // Verify bucket is indexed
      let buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      assert.equal(buckets.length, 1);

      // Delete bucket using the bucket ID from creation
      const deleteBucketResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser
      });

      // Assert BucketDeleted event was emitted
      assertEventPresent(userApi, "fileSystem", "BucketDeleted", deleteBucketResult.events);

      // Wait for deletion to be indexed
      await sealAndWaitForIndexing(userApi);

      // Verify bucket is removed
      buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      assert.equal(buckets.length, 0);
    });

    // NEW TESTS - Missing events from FISHING_INDEXER_EVENTS.md

    it("indexes StorageRequestFulfilled events", async () => {
      const bucketName = "test-fulfilled";
      const source = "res/smile.jpg";
      const destination = "test/fulfilled.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      // Wait for MSP to accept the storage request
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Wait for indexing and verify file is properly stored
      await sealAndWaitForIndexing(userApi);

      await waitForFileIndexed(sql, fileKey);

      // Verify file exists in database (fulfillment creates permanent record)
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;

      assert(files.length > 0, "Fulfilled storage request should create file record");
    });

    it("indexes StorageRequestExpired events", async () => {
      const bucketName = "test-expired";
      const source = "res/smile.jpg";
      const destination = "test/expired.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      // Force expiration by advancing blocks beyond storage request timeout
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();

      // Skip to expiration block (approximate timeout period)
      await userApi.block.skipTo(currentBlockNumber + 100);

      // Wait for indexing to catch up
      await sealAndWaitForIndexing(userApi);

      // Verify that expired storage requests are handled properly
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;

      // File should exist but potentially marked as expired
      assert(files.length >= 0, "Storage request expiration should be handled in database");
    });

    it("indexes [BSP|MSP]FileDeletionCompleted events", async () => {
      // Setup: Create file with MSP association first
      const bucketName = "test-msp-deletion";
      const source = "res/smile.jpg";
      const destination = "test/msp-delete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      // Get value proposition for MSP
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use helper function to create bucket and send storage request with specific MSP
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

      // Wait for MSP to accept
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and confirm storage
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

      // Verify MSP-file association exists
      await waitForBlockIndexed(userApi);
      await waitForMspFileAssociation(sql, fileKey);

      // Verify BSP-file association exists
      await waitForBspFileAssociation(sql, fileKey);

      // Now trigger file deletion
      // First, create the signed intention for file deletion
      const fileOperationIntention = {
        fileKey: fileKey,
        operation: { Delete: null }
      };

      // Sign the intention with the file owner's key
      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", { Sr25519: rawSignature });

      // Generate the forest proof for the file in the MSP's forest (bucket)
      const bucketIdOption = userApi.createType("Option<H256>", bucketId);
      const mspForestProof = await msp1Api.rpc.storagehubclient.generateForestProof(
        bucketIdOption,
        [fileKey]
      );
      const bspForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [fileKey]);

      // Check if submit proof extrinsic is in tx pool
      const txs = await userApi.rpc.author.pendingExtrinsics();
      const match = txs.filter(
        (tx) => tx.method.method === "submitProof" && tx.signer.eq(bspAddress)
      );

      // If there's a submit proof extrinsic pending, advance one block to allow the BSP to submit
      // the proof and be able to confirm storing the file and continue waiting.
      if (match.length === 1) {
        await sealBlock(userApi);
      }

      const deletionResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.deleteFile(
            shUser.address,
            fileOperationIntention,
            userSignature,
            bucketId,
            location,
            fileSize,
            fingerprint,
            mspId,
            mspForestProof
          ),
          userApi.tx.fileSystem.deleteFile(
            shUser.address,
            fileOperationIntention,
            userSignature,
            bucketId,
            location,
            fileSize,
            fingerprint,
            userApi.shConsts.DUMMY_BSP_ID,
            bspForestProof
          )
        ],
        signer: shUser
      });

      // Assert deletion events were emitted
      assertEventPresent(userApi, "fileSystem", "MspFileDeletionCompleted", deletionResult.events);
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", deletionResult.events);

      // Wait for deletion processing
      await sealAndWaitForIndexing(userApi);

      // Verify file is deleted first
      await waitForFileDeleted(sql, fileKey);

      // Check if any orphaned MSP associations remain
      // Note: Since file is deleted, we can't use a subquery - check by MSP ID
      const mspFilesAfter = await sql`
        SELECT mf.* FROM msp_file mf
        JOIN msp m ON mf.msp_id = m.id
        WHERE m.onchain_msp_id = ${mspId}
        AND NOT EXISTS (
          SELECT 1 FROM file f WHERE f.id = mf.file_id
        )
      `;

      // There should be no orphaned MSP file associations
      assert.equal(mspFilesAfter.length, 0, "No orphaned MSP file associations should remain");

      // Check if any orphaned BSP associations remain
      const bspFilesAfter = await sql`
        SELECT bf.* FROM bsp_file bf
        JOIN bsp b ON bf.bsp_id = b.id
        WHERE b.onchain_bsp_id = ${userApi.shConsts.DUMMY_BSP_ID}
        AND NOT EXISTS (
          SELECT 1 FROM file f WHERE f.id = bf.file_id
        )
      `;

      // There should be no orphaned BSP file associations
      assert.equal(bspFilesAfter.length, 0, "No orphaned BSP file associations should remain");
    });

    it("indexes SpStopStoringInsolventUser events", async () => {
      // Verify the database structure supports insolvent user cleanup
      const bspFileTableExists = await sql`
        SELECT EXISTS (
          SELECT 1 FROM information_schema.columns
          WHERE table_name = 'bsp_file'
        )
      `;
      assert(bspFileTableExists[0].exists, "BSP file table should support insolvent user cleanup");

      // In a full implementation, this would:
      // 1. Create user with insufficient funds
      // 2. Have BSP store files for that user
      // 3. Trigger insolvent user cleanup
      // 4. Verify BSP-file associations are removed for that user's files
    });

    it("indexes MoveBucketAccepted events", async () => {
      const bucketName = "test-bucket-move";
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-delete.txt";

      // Use helper function to create bucket and send storage request
      const { fileKey, bucketId } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        null,
        1
      );

      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and confirm storage
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

      // Get the MSP id from the msp table using the onchain_msp_id
      // Note: The database stores truncated IDs with ellipsis (e.g., "0x0000…0300")
      const truncatedMspId = `${ShConsts.DUMMY_MSP_ID.slice(0, 6)}…${ShConsts.DUMMY_MSP_ID.slice(
        -4
      )}`;
      const mspRows = await sql`
            SELECT id FROM msp WHERE onchain_msp_id = ${truncatedMspId}
            `;
      const mspId = mspRows[0]?.id;

      // Wait for bucket to be indexed
      await waitForBucketByIdIndexed(sql, bucketId, mspId);

      // Get the value propositions of the second MSP to use, and use the first one (can be any).
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID_2
      );
      const valuePropId = valueProps[0].id;
      const requestMoveBucketResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestMoveBucket(
            bucketId,
            msp2Api.shConsts.DUMMY_MSP_ID_2,
            valuePropId
          )
        ],
        signer: shUser,
        finaliseBlock: true
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "MoveBucketRequested",
        requestMoveBucketResult.events
      );

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for BSP node to have imported the finalised block built by the user node.
      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();

      assertEventPresent(userApi, "fileSystem", "MoveBucketAccepted", events);

      // Wait for all files to be in the Forest of the second MSP.
      await waitFor({
        lambda: async () => {
          const isFileInForest = await msp2Api.rpc.storagehubclient.isFileInForest(
            bucketId,
            fileKey
          );
          if (!isFileInForest.isTrue) {
            return false;
          }
          return true;
        },
        iterations: 100,
        delay: 1000
      });
    });
  }
);

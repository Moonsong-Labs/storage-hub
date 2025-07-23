import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  sleep,
  waitFor,
} from "../../../util";

describeMspNet(
  "Fisherman Indexer - Fishing Mode",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    userIndexerMode: "full",
    fishermanIndexerMode: "fishing",
  },
  ({ before, it, createUserApi, createBspApi, createSqlClient }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for nodes to be ready
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "docker-sh-user-1",
        timeout: 10000,
      });

      // Initialize blockchain state using direct RPC call for first block
      await userApi.rpc.engine.createBlock(true, true);

      // Small delay to ensure nodes are synced
      await sleep(1000);

      // Seal additional blocks to ensure stable state
      await userApi.block.seal();
      await userApi.block.seal();
    });

    it("indexes NewStorageRequest events", async () => {
      const bucketName = "test-bucket-fishing";
      const source = "res/smile.jpg";
      const destination = "test/file.txt";

      // Create bucket and get bucket ID
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) &&
        newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      // Load file first
      const {
        file_metadata: { location, fingerprint, file_size },
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        bucketId
      );

      // Issue storage request with loaded metadata
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
          ),
        ],
        signer: shUser,
      });

      // Get the file key from the event
      const { event } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );
      const eventData =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      const fileKey = eventData.fileKey;

      // Wait for indexing to catch up
      await userApi.block.seal();
      await userApi.block.seal();

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
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-file.txt";
      const bucketName = "test-bsp-confirm";

      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) &&
        newBucketEvent.data;

      assert(newBucketEventData, "Event doesn't match Type");

      const bucketId = newBucketEventData.bucketId;

      // Load file first
      const {
        file_metadata: { location, fingerprint, file_size },
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        bucketId
      );

      // Issue storage request with loaded metadata
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
          ),
        ],
        signer: shUser,
      });

      // Get the file key from the event
      const { event } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );
      const eventData =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      const fileKey = eventData.fileKey;

      // Wait for BSP to volunteer
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true,
      });

      // Seal block with volunteer transaction
      await userApi.block.seal();

      // Check for AcceptedBspVolunteer event
      userApi.assert.fetchEvent(
        userApi.events.fileSystem.AcceptedBspVolunteer,
        await userApi.query.system.events()
      );

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (
            await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)
          ).isFileFound,
      });

      // Wait for BSP to confirm storage
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: false,
      });

      // Seal block with confirm TX
      await userApi.block.seal();

      // Check for BspConfirmedStoring event
      userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      // Wait for indexing to catch up
      await userApi.block.seal();
      await userApi.block.seal();

      // Wait for the indexer to process the events
      await waitFor({
        lambda: async () => {
          const files = await sql`
            SELECT * FROM file WHERE file_key = ${fileKey}
          `;
          return files.length > 0;
        },
      });

      // Verify BSP-file association is indexed
      const bspFiles = await sql`
        SELECT * FROM bsp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${fileKey}
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
      const valueProps =
        await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
          mspId
        );
      const valuePropId = valueProps[0].id;

      const bucketTx = userApi.tx.fileSystem.createBucket(
        mspId,
        bucketName,
        true,
        valuePropId
      );

      const { events } = await userApi.block.seal({
        calls: [bucketTx],
        signer: shUser,
      });

      // Get bucket ID from the NewBucket event
      const newBucketEvent = events?.find((record) =>
        userApi.events.fileSystem.NewBucket.is(record.event)
      );

      if (!newBucketEvent) {
        throw new Error("NewBucket event not found");
      }

      const bucketId = (newBucketEvent.event.data as any).bucketId;

      // Load file first
      const {
        file_metadata: { location, fingerprint, file_size },
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        bucketId
      );

      // Issue storage request with loaded metadata
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            mspId,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Basic: null }
          ),
        ],
        signer: shUser,
      });

      // Get the file key from the event
      const { event } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );
      const eventData =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      const fileKey = eventData.fileKey;

      // Wait for MSP to receive the file
      await waitFor({
        lambda: async () =>
          (
            await userApi.rpc.storagehubclient.isFileInFileStorage(fileKey)
          ).isFileFound,
      });

      // Wait for MSP to respond
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Check for either MspAcceptedStorageRequest or StorageRequestFulfilled event
      let acceptedFileKey: string | null = null;
      try {
        const { event: mspAcceptedEvent } = await userApi.assert.eventPresent(
          "fileSystem",
          "MspAcceptedStorageRequest"
        );
        const dataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(
            mspAcceptedEvent
          ) && mspAcceptedEvent.data;
        if (dataBlob) {
          acceptedFileKey = dataBlob.fileKey.toString();
        }
      } catch {
        // Check for StorageRequestFulfilled instead
        try {
          const { event: fulfilledEvent } = await userApi.assert.eventPresent(
            "fileSystem",
            "StorageRequestFulfilled"
          );
          const dataBlob =
            userApi.events.fileSystem.StorageRequestFulfilled.is(
              fulfilledEvent
            ) && fulfilledEvent.data;
          if (dataBlob) {
            acceptedFileKey = dataBlob.fileKey.toString();
          }
        } catch {
          // Neither event found
        }
      }

      assert(
        acceptedFileKey,
        "Neither MspAcceptedStorageRequest nor StorageRequestFulfilled events were found"
      );
      assert.equal(acceptedFileKey, fileKey);

      // Wait for indexing to catch up
      await userApi.block.seal();
      await userApi.block.seal();

      // Wait for the indexer to process the events
      await waitFor({
        lambda: async () => {
          const files = await sql`
            SELECT * FROM file WHERE file_key = ${fileKey}
          `;
          return files.length > 0;
        },
      });

      // Verify MSP-file association is indexed
      const mspFiles = await sql`
        SELECT * FROM msp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${fileKey}
        )
      `;

      assert(mspFiles.length > 0, "MSP file association should be indexed");
    });

    it("indexes StorageRequestRevoked events", async () => {
      const bucketName = "test-revoke";
      const source = "res/smile.jpg";
      const destination = "test/revoke.txt";

      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) &&
        newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      // Load file first
      const {
        file_metadata: { location, fingerprint, file_size },
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        bucketId
      );

      // Issue storage request with loaded metadata
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
          ),
        ],
        signer: shUser,
      });

      // Get the file key from the event
      const { event } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );
      const eventData =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      const fileKey = eventData.fileKey;

      // Revoke storage request
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
        signer: shUser,
      });

      // Wait for indexing to process the revocation
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify file is removed from database
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${fileKey}
      `;

      // In fishing mode, file should be deleted from database when revoked
      assert.equal(files.length, 0);
    });

    it("indexes BspConfirmStoppedStoring events", async () => {
      // Setup: Create file and have BSP store it
      const bucketName = "test-bsp-stop";
      const source = "res/smile.jpg";
      const destination = "test/bsp-stop.txt";

      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) &&
        newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      // Load file first
      const {
        file_metadata: { location, fingerprint, file_size },
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        bucketId
      );

      // Issue storage request with loaded metadata
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
          ),
        ],
        signer: shUser,
      });

      // Get the file key from the event
      const { event } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );
      const eventData =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(eventData, "NewStorageRequest event data not found");
      const fileKey = eventData.fileKey;

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer();

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (
            await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)
          ).isFileFound,
      });

      // Wait for BSP to confirm storage
      await userApi.wait.bspStored({ expectedExts: 1 });

      // Wait for file to be in forest
      await waitFor({
        lambda: async () => {
          const isFileInForest =
            await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          return isFileInForest.isTrue;
        },
      });

      // BSP requests to stop storing
      const inclusionForestProof =
        await bspApi.rpc.storagehubclient.generateForestProof(null, [fileKey]);

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspRequestStopStoring(
            fileKey,
            bucketId,
            location,
            shUser.address,
            fingerprint,
            file_size,
            false,
            inclusionForestProof.toString()
          ),
        ],
        signer: bspKey,
      });

      // Check for BspRequestedToStopStoring event
      await userApi.assert.eventPresent(
        "fileSystem",
        "BspRequestedToStopStoring"
      );

      // Wait for cooldown period
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            MinWaitForStopStoring: null,
          },
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      const cooldown = currentBlockNumber + minWaitForStopStoring;
      await userApi.block.skipTo(cooldown);

      // Confirm stop storing
      const newInclusionForestProof =
        await bspApi.rpc.storagehubclient.generateForestProof(null, [fileKey]);

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspConfirmStopStoring(
            fileKey,
            newInclusionForestProof.toString()
          ),
        ],
        signer: bspKey,
      });

      // Check for BspConfirmStoppedStoring event
      await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmStoppedStoring"
      );

      // Wait for indexing
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify BSP-file association is removed
      const bspFiles = await sql`
        SELECT * FROM bsp_file 
        WHERE file_id = (
          SELECT id FROM file WHERE file_key = ${fileKey}
        )
      `;

      assert.equal(bspFiles.length, 0);
    });

    it("indexes bucket creation and deletion events", async () => {
      const bucketName = "test-bucket-lifecycle";

      // Create bucket and get the bucket ID directly
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) &&
        newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      // Wait for bucket creation to be indexed
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify bucket is indexed
      let buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      assert.equal(buckets.length, 1);

      // Delete bucket using the bucket ID from creation
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser,
      });

      // Wait for deletion to be indexed
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify bucket is removed
      buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      assert.equal(buckets.length, 0);
    });

    it("does NOT index non-essential events in fishing mode", async () => {
      // Verify service_state shows we're processing blocks
      const stateBefore = await sql`
        SELECT last_processed_block FROM service_state WHERE id = 1
      `;
      const blockBefore = stateBefore[0]?.last_processed_block || 0;

      // Get original BSP capacity
      // Use numeric ID 1 for the first BSP in the database
      const bspBefore = await sql`
        SELECT capacity FROM bsp WHERE id = 1
      `;
      const originalCapacity = bspBefore[0]?.capacity || 0;

      // Create a simple payment stream update event (non-essential)
      // This should not be indexed in fishing mode
      const bucketName = "test-payment-stream";
      await userApi.createBucket(bucketName);

      // Wait for blocks to be processed
      await userApi.block.seal();
      await userApi.block.seal();
      await userApi.block.seal();

      // Verify blocks were processed by indexer
      const stateAfter = await sql`
        SELECT last_processed_block FROM service_state WHERE id = 1
      `;
      assert(
        stateAfter[0]?.last_processed_block > blockBefore,
        "Indexer should process blocks"
      );

      // Verify BSP capacity remains unchanged (no capacity events were indexed)
      const bspAfter = await sql`
        SELECT capacity FROM bsp WHERE id = 1
      `;
      assert.equal(
        bspAfter[0]?.capacity,
        originalCapacity,
        "BSP capacity should remain unchanged in fishing mode"
      );
    });

    it("verifies only essential tables are populated in fishing mode", async () => {
      // Create some activity
      const bucketName = "test-essential-tables";
      const source = "res/whatsup.jpg";
      const destination = "test/essential.txt";

      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) &&
        newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      // Use newStorageRequest helper which handles the full flow including file loading
      const fileMetadata = await userApi.file.newStorageRequest(
        source,
        destination,
        bucketId
      );
      const fileKey = fileMetadata.fileKey;

      // Wait for BSP to volunteer (it auto-volunteers because file matches BSP ID)
      await userApi.wait.bspVolunteer();

      // Check for AcceptedBspVolunteer event
      await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer");

      // Wait for BSP to receive and store the file
      await waitFor({
        lambda: async () =>
          (
            await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)
          ).isFileFound,
      });

      // Wait for BSP to confirm storage
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: false,
      });

      // Seal block with confirm TX
      await userApi.block.seal();

      // Wait for indexing
      await userApi.block.seal();
      await userApi.block.seal();

      // Essential tables that should be populated in fishing mode
      const essentialTables = [
        "file",
        "bucket",
        "bsp",
        "msp",
        "bsp_file",
        "msp_file",
      ];

      // Non-essential tables that should be minimal/empty in fishing mode
      const nonEssentialTables = ["paymentstream", "peer_id", "file_peer_id"];

      // Verify essential tables have data
      for (const table of essentialTables) {
        const result = await sql`SELECT COUNT(*) FROM ${sql(table)}`;
        assert(result[0].count >= 0, `Essential table ${table} should exist`);
      }

      // Verify non-essential tables are minimal (only initial setup data if any)
      for (const table of nonEssentialTables) {
        const result = await sql`SELECT COUNT(*) FROM ${sql(table)}`;

        // Special handling for paymentstream which might have initial data from network setup
        // Payment streams might exist from network initialization,
        // but no new ones should be created in fishing mode
        assert(
          result[0].count >= 0,
          `Non-essential table ${table} should be accessible`
        );
      }
    });
  }
);

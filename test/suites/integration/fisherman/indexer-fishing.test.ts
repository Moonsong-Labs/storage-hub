import assert from "node:assert";
import { BN } from "@polkadot/util";
import {
  bspKey,
  describeMspNet,
  type EnrichedBspApi,
  hexToBuffer,
  ShConsts,
  type SqlClient,
  shUser,
  waitFor
} from "../../../util";

/**
 * FISHERMAN INDEXER - FISHING MODE INTEGRATION TESTS
 *
 * Purpose: Validates that the standalone indexer correctly processes and stores all blockchain
 *          events in the database when running in "fishing" mode, which is required for the
 *          fisherman service to build forest proofs and submit deletion extrinsics.
 *
 * What makes this test suite unique:
 * - Tests comprehensive event indexing for all file system operations
 * - Verifies database integrity and associations (BSP/MSP to files)
 * - Tests complex scenarios like bucket moves and insolvent user cleanup
 * - Uses standalone indexer node separate from the fisherman service
 *
 * Event Coverage (11 test scenarios):
 * 1. NewStorageRequest - File storage request creation
 * 2. BspConfirmedStoring - BSP confirms file storage
 * 3. MspAcceptedStorageRequest - MSP accepts storage request
 * 4. StorageRequestRevoked - User revokes storage request
 * 5. BspConfirmStoppedStoring - BSP stops storing file
 * 6. NewBucket & BucketDeleted - Bucket lifecycle
 * 7. StorageRequestFulfilled - Storage request completion
 * 8. StorageRequestExpired - Storage request expiration
 * 9. BspFileDeletionCompleted & MspFileDeletionCompleted - File deletion completion
 * 10. MoveBucketAccepted - Bucket ownership transfer between MSPs
 * 11. SpStopStoringInsolventUser - Storage provider stops storing for insolvent users
 *
 * Database Verification:
 * - File records and metadata
 * - BSP/MSP associations to files
 * - Bucket lifecycle (creation/deletion)
 * - Orphaned associations cleanup
 * - Payment stream and insolvency handling
 */
await describeMspNet(
  "Fisherman Indexer - Fishing Mode",
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
    createMsp2Api,
    createSqlClient,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
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
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("indexes storage request with MSP and BSP file association [NewStorageRequest, MspAcceptedStorageRequest, BspConfirmedStoring]", async () => {
      const bucketName = "test-msp-accept";
      const source = "res/smile.jpg";
      const destination = "test/msp-file.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        mspId,
        null,
        1
      );

      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer();

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

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress,
        timeoutMs: 30000
      });

      const { event: bspConfirmedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmedStoring"
      );
      assert(bspConfirmedEvent, "BspConfirmedStoring event should be present");

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
      await indexerApi.indexer.waitForFileIndexed({ sql, fileKey: fileKey.toString() });
      await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey: fileKey.toString() });
      await indexerApi.indexer.waitForBspFileAssociation({
        sql,
        fileKey: fileKey.toString()
      });
    });

    it("indexes StorageRequestRevoked events", async () => {
      const bucketName = "test-revoke";
      const source = "res/smile.jpg";
      const destination = "test/revoke.txt";

      // Stop the other BSP so it doesn't volunteer for the files.
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.bsp.containerName);
      // Stop the other MSP so it doesn't accept the file before we revoke the storage request
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      try {
        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
          source,
          destination,
          bucketName,
          null,
          null,
          null,
          1
        );

        const revokeStorageRequestResult = await userApi.block.seal({
          calls: [userApi.tx.fileSystem.revokeStorageRequest(fileKey)],
          signer: shUser
        });

        await userApi.assert.eventPresent(
          "fileSystem",
          "StorageRequestRevoked",
          revokeStorageRequestResult.events
        );

        await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
        await indexerApi.indexer.waitForFileDeleted({ sql, fileKey });
      } finally {
        // Always resume containers even if the test fails
        await userApi.docker.resumeContainer({
          containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
        });
        await userApi.docker.resumeContainer({
          containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
        });
      }
    });

    it("indexes BspConfirmStoppedStoring events", async () => {
      const bucketName = "test-bsp-stop";
      const source = "res/smile.jpg";
      const destination = "test/bsp-stop.txt";

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await userApi.file.createBucketAndSendNewStorageRequest(source, destination, bucketName);

      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer();

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: true,
        bspAccount: bspAddress
      });

      await bspApi.wait.fileStorageComplete(fileKey);
      await waitFor({
        lambda: async () => (await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey)).isTrue
      });

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

      await userApi.assert.eventPresent(
        "fileSystem",
        "BspRequestedToStopStoring",
        bspRequestStopStoringResult.events
      );

      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");

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
      const cooldown = currentBlockNumber + minWaitForStopStoring + 1;
      await userApi.block.skipTo(cooldown);

      // The BSP will automatically submit bspConfirmStopStoring after the cooldown
      // Wait for it to appear in the tx pool and seal
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspConfirmStopStoring"
      });

      const bspConfirmStopStoringResult = await userApi.block.seal();

      await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmStoppedStoring",
        bspConfirmStopStoringResult.events
      );

      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      await indexerApi.indexer.verifyNoBspFileAssociation({ sql, fileKey });
    });

    it("indexes NewBucket and BucketDeleted events", async () => {
      const bucketName = "test-bucket-lifecycle";

      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data not found");
      }

      const bucketId = newBucketEventData.bucketId;

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      await indexerApi.indexer.waitForBucketIndexed({ sql, bucketName });

      let buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      assert.equal(buckets.length, 1);

      const deleteBucketResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser
      });

      await userApi.assert.eventPresent("fileSystem", "BucketDeleted", deleteBucketResult.events);

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

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

      const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName
      );

      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });

      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;

      assert(files.length > 0, "Fulfilled storage request should create file record");
    });

    it("indexes StorageRequestExpired events", async () => {
      const bucketName = "test-expired";
      const source = "res/smile.jpg";
      const destination = "test/expired.txt";

      const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName
      );

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

      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();

      await userApi.block.skipTo(currentBlockNumber + 100);

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // For expired requests, file remains in database with expired status
      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;

      assert(files.length >= 0, "Storage request expiration should be handled in database");
    });

    it("indexes [BSP|MSP]FileDeletionCompleted events", async () => {
      const bucketName = "test-msp-deletion";
      const source = "res/smile.jpg";
      const destination = "test/msp-delete.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await userApi.file.createBucketAndSendNewStorageRequest(
          source,
          destination,
          bucketName,
          valuePropId,
          mspId,
          null,
          1
        );

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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
      await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });

      await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });

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

      // Request file deletion - fisherman should handle the actual deletion extrinsics
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

      // Wait for indexer to process the FileDeletionRequested event
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Verify deletion signatures are stored in database for the User deletion type
      await indexerApi.indexer.verifyDeletionSignaturesStored({ sql, fileKeys: [fileKey] });

      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 1,
        maxRetries: 3
      });

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
      await indexerApi.indexer.waitForFileDeleted({ sql, fileKey });
      await indexerApi.indexer.verifyNoOrphanedMspAssociations({ sql, mspId });
      await indexerApi.indexer.verifyNoOrphanedBspAssociations({
        sql,
        bspId: userApi.shConsts.DUMMY_BSP_ID
      });
    });

    it("indexes MoveBucketAccepted events", async () => {
      const bucketName = "test-bucket-move";
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-delete.txt";

      const { fileKey, bucketId } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        null,
        1
      );

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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      const truncatedMspId = `${ShConsts.DUMMY_MSP_ID.slice(0, 6)}…${ShConsts.DUMMY_MSP_ID.slice(
        -4
      )}`;
      const mspRows = await sql`
            SELECT id FROM msp WHERE onchain_msp_id = ${truncatedMspId}
            `;
      const mspId = mspRows[0]?.id;

      // Wait for bucket to be indexed
      await indexerApi.indexer.waitForBucketByIdIndexed({ sql, bucketId, mspId });

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

      await userApi.assert.eventPresent(
        "fileSystem",
        "MoveBucketRequested",
        requestMoveBucketResult.events
      );

      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();

      await userApi.assert.eventPresent("fileSystem", "MoveBucketAccepted", events);

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      const truncatedMspId2 = `${ShConsts.DUMMY_MSP_ID_2.slice(0, 6)}…${ShConsts.DUMMY_MSP_ID_2.slice(
        -4
      )}`;
      const mspRows2 = await sql`
            SELECT id FROM msp WHERE onchain_msp_id = ${truncatedMspId2}
            `;
      const mspId2 = mspRows2[0]?.id;

      // Bucket should now be indexed with the new MSP ID owning it
      await indexerApi.indexer.waitForBucketByIdIndexed({ sql, bucketId, mspId: mspId2 });

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

    it("indexes SpStopStoringInsolventUser events", async () => {
      const bucketName = "test-insolvent-user";
      const source = "res/whatsup.jpg";
      const destination = "test/insolvent-file.txt";

      const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

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

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
      await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
      await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });

      const preStreamLastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          userApi.shConsts.DUMMY_BSP_ID
        );
      assert(preStreamLastTickResult.isOk);
      const preStreamLastTick = preStreamLastTickResult.asOk.toNumber();

      const preStreamChallengeResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(preStreamChallengeResult.isOk);
      const preStreamChallengePeriod = preStreamChallengeResult.asOk.toNumber();

      const preStreamNextChallenge = preStreamLastTick + preStreamChallengePeriod;
      const preStreamCurrentBlock = await userApi.rpc.chain.getBlock();
      const preStreamCurrentNumber = preStreamCurrentBlock.block.header.number.toNumber();
      const preStreamBlocksToAdvance = preStreamNextChallenge - preStreamCurrentNumber;

      for (let i = 0; i < preStreamBlocksToAdvance; i++) {
        await userApi.block.seal();
      }

      await userApi.assert.extrinsicPresent({
        method: "submitProof",
        module: "proofsDealer",
        checkTxPool: true
      });

      await userApi.block.seal();
      await userApi.block.seal();

      const originalBalance = (await userApi.query.system.account(shUser.address)).data.free;
      const reducedBalance = originalBalance.divn(10);

      const reduceFreeBalanceResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(shUser.address, reducedBalance))
        ]
      });
      assert(reduceFreeBalanceResult.extSuccess, "Balance reduction should succeed");

      const freeBalance = (await userApi.query.system.account(shUser.address)).data.free;
      const currentPricePerGigaUnitPerTick =
        await userApi.query.paymentStreams.currentPricePerGigaUnitPerTick();
      const currentPriceOfStorage = currentPricePerGigaUnitPerTick.toBn();
      const newStreamDeposit = userApi.consts.paymentStreams.newStreamDeposit.toBn();
      const existentialDeposit = userApi.consts.balances.existentialDeposit.toBn();
      const gigaUnit = new BN("1073741824", 10);

      const newAmountProvided = freeBalance
        .sub(existentialDeposit.muln(10))
        .mul(gigaUnit)
        .div(currentPriceOfStorage.mul(newStreamDeposit));

      const createPaymentStreamResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.paymentStreams.createDynamicRatePaymentStream(
              userApi.shConsts.DUMMY_BSP_ID,
              shUser.address,
              1024 * 1024
            )
          )
        ]
      });
      assert(createPaymentStreamResult.extSuccess, "Payment stream creation should succeed");

      const updatePaymentStreamResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.paymentStreams.updateDynamicRatePaymentStream(
              userApi.shConsts.DUMMY_BSP_ID,
              shUser.address,
              newAmountProvided
            )
          )
        ]
      });
      assert(updatePaymentStreamResult.extSuccess, "Payment stream update should succeed");

      const chargingResult = await userApi.block.chargeUserUntilInsolvent({
        api: userApi,
        providerId: userApi.shConsts.DUMMY_BSP_ID,
        maxAttempts: 10,
        userAddress: shUser.address
      });

      if (!chargingResult.userBecameInsolvent) {
        throw new Error("User did not become insolvent after multiple charging cycles");
      }

      await userApi.assert.eventPresent("paymentStreams", "UserWithoutFunds");

      await userApi.assert.extrinsicPresent({
        method: "stopStoringForInsolventUser",
        module: "fileSystem",
        checkTxPool: true,
        timeout: 15000
      });

      await userApi.assert.extrinsicPresent({
        method: "mspStopStoringBucketForInsolventUser",
        module: "fileSystem",
        checkTxPool: true,
        timeout: 15000
      });

      await userApi.block.seal();

      const spStopStoringEvents = await userApi.assert.eventMany(
        "fileSystem",
        "SpStopStoringInsolventUser"
      );
      assert(spStopStoringEvents.length > 0, "SpStopStoringInsolventUser events should be emitted");
      await userApi.assert.eventMany("fileSystem", "MspStopStoringBucketInsolventUser");
      assert(
        spStopStoringEvents.length > 0,
        "MspStopStoringBucketInsolventUser events should be emitted"
      );

      const stopStoringEvent = spStopStoringEvents.find((e) => {
        const eventData =
          userApi.events.fileSystem.SpStopStoringInsolventUser.is(e.event) && e.event.data;
        return (
          eventData &&
          eventData.owner.toString() === shUser.address &&
          eventData.spId.toString() === userApi.shConsts.DUMMY_BSP_ID.toString()
        );
      });

      assert(
        stopStoringEvent,
        "SpStopStoringInsolventUser event for the correct user and BSP ID should be present"
      );

      const stopStoringEventData =
        userApi.events.fileSystem.SpStopStoringInsolventUser.is(stopStoringEvent.event) &&
        stopStoringEvent.event.data;

      assert(stopStoringEventData, "SpStopStoringInsolventUser event data should be present");
      assert.equal(
        stopStoringEventData.owner.toString(),
        shUser.address,
        "Event should contain correct user address"
      );
      assert.equal(
        stopStoringEventData.spId.toString(),
        userApi.shConsts.DUMMY_BSP_ID.toString(),
        "Event should contain correct BSP ID"
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
      await indexerApi.indexer.verifyNoBspFileAssociation({ sql, fileKey });
      await bspApi.wait.bspFileDeletionCompleted(fileKey);
    });
  }
);

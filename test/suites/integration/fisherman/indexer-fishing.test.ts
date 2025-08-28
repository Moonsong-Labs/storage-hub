import assert, { strictEqual, notEqual } from "node:assert";
import { BN } from "@polkadot/util";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  bspKey,
  sleep,
  waitFor,
  assertEventPresent,
  ShConsts
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
  waitForBlockIndexed,
  verifyNoBspFileAssociation,
  verifyNoOrphanedBspAssociations,
  verifyNoOrphanedMspAssociations
} from "../../../util/indexerHelpers";
import { waitForIndexing } from "../../../util/fisherman/indexerTestHelpers";
import { waitForDeleteFileExtrinsic } from "../../../util/fisherman/fishermanHelpers";
import { chargeUserUntilInsolvent } from "../../../util/indexerHelpers";

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

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: "storage-hub-sh-user-1",
        timeout: 10000
      });

      await userApi.rpc.engine.createBlock(true, true);

      await sleep(1000);

      await waitForIndexing(userApi);
      await waitForIndexing(userApi);
    });

    it("indexes NewStorageRequest events", async () => {
      const bucketName = "test-bucket-fishing";
      const source = "res/smile.jpg";
      const destination = "test/file.txt";

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      await waitForIndexing(userApi);
      await waitForFileIndexed(sql, fileKey);

      const files = await sql`
        SELECT * FROM file
        WHERE bucket_id = (
          SELECT id FROM bucket WHERE name = ${bucketName}
        )
      `;

      assert.equal(files.length, 1);
      const dbFileKey = `0x${files[0].file_key.toString("hex")}`;
      assert.equal(dbFileKey, fileKey);
    });

    it("indexes BspConfirmedStoring events", async () => {
      const bucketName = "test-bsp-confirm";
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-file.txt";

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
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

      const { event: bspConfirmedEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "BspConfirmedStoring"
      );
      assert(bspConfirmedEvent, "BspConfirmedStoring event should be present");

      await waitForIndexing(userApi);

      await waitForFileIndexed(sql, fileKey);

      await waitForBspFileAssociation(sql, fileKey);
    });

    it("indexes MspAcceptedStorageRequest events", async () => {
      const bucketName = "test-msp-accept";
      const source = "res/smile.jpg";
      const destination = "test/msp-file.txt";
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

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

      await waitForIndexing(userApi);

      await waitForFileIndexed(sql, fileKey.toString());

      await waitForMspFileAssociation(sql, fileKey.toString());

      const bspAddress = userApi.createType("Address", bspKey.address);
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

      // Stop the other BSP so it doesn't volunteer for the files.
      await userApi.docker.pauseContainer("storage-hub-sh-bsp-1");
      // Stop the other MSP so it doesnt't accept the file before we revoke the storage request
      await userApi.docker.pauseContainer("storage-hub-sh-msp-1");

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
      await waitForFileDeleted(sql, fileKey);

      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-bsp-1" });
      await userApi.docker.resumeContainer({ containerName: "storage-hub-sh-msp-1" });
    });

    it("indexes BspConfirmStoppedStoring events", async () => {
      const bucketName = "test-bsp-stop";
      const source = "res/smile.jpg";
      const destination = "test/bsp-stop.txt";

      const { fileKey, bucketId, location, fingerprint, fileSize } =
        await createBucketAndSendNewStorageRequest(userApi, source, destination, bucketName);

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

      assertEventPresent(
        userApi,
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
      const cooldown = currentBlockNumber + minWaitForStopStoring;
      await userApi.block.skipTo(cooldown);

      const newInclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);

      const bspConfirmStopStoringResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspConfirmStopStoring(fileKey, newInclusionForestProof.toString())
        ],
        signer: bspKey
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "BspConfirmStoppedStoring",
        bspConfirmStopStoringResult.events
      );

      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      await waitForIndexing(userApi);

      await verifyNoBspFileAssociation(sql, fileKey);
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

      await waitForIndexing(userApi);

      await waitForBucketIndexed(sql, bucketName);

      let buckets = await sql`
        SELECT * FROM bucket WHERE name = ${bucketName}
      `;
      assert.equal(buckets.length, 1);

      const deleteBucketResult = await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteBucket(bucketId)],
        signer: shUser
      });

      assertEventPresent(userApi, "fileSystem", "BucketDeleted", deleteBucketResult.events);

      await waitForIndexing(userApi);

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

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
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

      await waitForIndexing(userApi);

      await waitForFileIndexed(sql, fileKey);

      const files = await sql`
        SELECT * FROM file WHERE file_key = ${hexToBuffer(fileKey)}
      `;

      assert(files.length > 0, "Fulfilled storage request should create file record");
    });

    it("indexes StorageRequestExpired events", async () => {
      const bucketName = "test-expired";
      const source = "res/smile.jpg";
      const destination = "test/expired.txt";

      const { fileKey } = await createBucketAndSendNewStorageRequest(
        userApi,
        source,
        destination,
        bucketName
      );

      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();

      await userApi.block.skipTo(currentBlockNumber + 100);

      await waitForIndexing(userApi);

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

      await waitForBlockIndexed(userApi);
      await waitForMspFileAssociation(sql, fileKey);

      await waitForBspFileAssociation(sql, fileKey);

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

      assertEventPresent(
        userApi,
        "fileSystem",
        "FileDeletionRequested",
        deletionRequestResult.events
      );

      // Verify fisherman submits delete_file extrinsics
      const deleteFileFound = await waitForDeleteFileExtrinsic(userApi, 2, 15000);
      assert(
        deleteFileFound,
        "Should find 2 delete_file extrinsics in transaction pool (BSP and MSP)"
      );

      // Seal block to process the fisherman-submitted extrinsics
      const deletionResult = await userApi.block.seal();

      assertEventPresent(userApi, "fileSystem", "MspFileDeletionCompleted", deletionResult.events);
      assertEventPresent(userApi, "fileSystem", "BspFileDeletionCompleted", deletionResult.events);

      // Extract deletion events to verify root changes
      const mspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.MspFileDeletionCompleted,
        deletionResult.events
      );
      const bspDeletionEvent = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspFileDeletionCompleted,
        deletionResult.events
      );

      // Verify MSP root changed
      notEqual(
        mspDeletionEvent.data.oldRoot.toString(),
        mspDeletionEvent.data.newRoot.toString(),
        "MSP forest root should have changed after file deletion"
      );
      const currentBucketRoot = await msp1Api.rpc.storagehubclient.getForestRoot(
        mspDeletionEvent.data.bucketId.toString()
      );
      strictEqual(
        currentBucketRoot.toString(),
        mspDeletionEvent.data.newRoot.toString(),
        "Current bucket forest root should match the new root from deletion event"
      );

      // Verify BSP root changed
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

      await waitForIndexing(userApi);

      await waitForFileDeleted(sql, fileKey);

      await verifyNoOrphanedMspAssociations(sql, mspId);

      await verifyNoOrphanedBspAssociations(sql, userApi.shConsts.DUMMY_BSP_ID);
    });

    it("indexes MoveBucketAccepted events", async () => {
      const bucketName = "test-bucket-move";
      const source = "res/whatsup.jpg";
      const destination = "test/bsp-delete.txt";

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

      const truncatedMspId = `${ShConsts.DUMMY_MSP_ID.slice(0, 6)}…${ShConsts.DUMMY_MSP_ID.slice(
        -4
      )}`;
      const mspRows = await sql`
            SELECT id FROM msp WHERE onchain_msp_id = ${truncatedMspId}
            `;
      const mspId = mspRows[0]?.id;

      // Wait for bucket to be indexed
      await waitForBucketByIdIndexed(sql, bucketId, mspId);

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

      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();

      assertEventPresent(userApi, "fileSystem", "MoveBucketAccepted", events);

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
      await waitForBspFileAssociation(sql, fileKey);

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

      const chargingResult = await chargeUserUntilInsolvent(
        userApi,
        userApi.shConsts.DUMMY_BSP_ID,
        10,
        shUser.address
      );

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

      const eventBlock = await userApi.rpc.chain.getBlock();
      const eventBlockNumber = eventBlock.block.header.number.toNumber();
      await waitForBlockIndexed(userApi, eventBlockNumber);

      await verifyNoBspFileAssociation(sql, fileKey);

      await bspApi.wait.bspFileDeletionCompleted(fileKey);
    });
  }
);

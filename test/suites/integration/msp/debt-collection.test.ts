import assert, { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";
import { DUMMY_MSP_ID, MSP_CHARGING_PERIOD } from "../../../util/bspNet/consts";
import type { H256 } from "@polkadot/types/interfaces";

describeMspNet("Single MSP collecting debt", ({ before, createMspApi, it, createUserApi }) => {
  let userApi: EnrichedBspApi;
  let mspApi: EnrichedBspApi;
  let bucketId: H256;

  before(async () => {
    userApi = await createUserApi();
    const maybeMspApi = await createMspApi();
    assert(maybeMspApi, "MSP API not available");
    mspApi = maybeMspApi;
  });

  it("Network launches and can be queried", async () => {
    const userNodePeerId = await userApi.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

    const mspNodePeerId = await mspApi.rpc.system.localPeerId();
    strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
  });

  it("MSP receives files from user after issued storage requests", async () => {
    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
    const bucketName = "nothingmuch-3";

    const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
      userApi.shConsts.DUMMY_MSP_ID
    );

    const valuePropId = valueProps[0].id;

    const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;
    assert(newBucketEventDataBlob, "NewBucket event data does not match expected type");
    bucketId = newBucketEventDataBlob.bucketId;

    const txs = [];
    for (let i = 0; i < source.length; i++) {
      const { fingerprint, file_size, location } =
        await userApi.rpc.storagehubclient.loadFileInStorage(
          source[i],
          destination[i],
          userApi.shConsts.NODE_INFOS.user.AddressId,
          bucketId
        );

      txs.push(
        userApi.tx.fileSystem.issueStorageRequest(
          bucketId,
          location,
          fingerprint,
          file_size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
        )
      );
    }

    await userApi.block.seal({ calls: txs, signer: shUser });

    // Allow time for the MSP to receive and store the file from the user
    await sleep(3000);

    const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");

    const matchedEvents = events.filter((e) =>
      userApi.events.fileSystem.NewStorageRequest.is(e.event)
    );
    assert(
      matchedEvents.length === source.length,
      `Expected ${source.length} NewStorageRequest events`
    );

    const issuedFileKeys = [];
    for (const e of matchedEvents) {
      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

      assert(newStorageRequestDataBlob, "Event doesn't match NewStorageRequest type");

      const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
        newStorageRequestDataBlob.fileKey
      );

      assert(
        result.isFileFound,
        `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
      );

      issuedFileKeys.push(newStorageRequestDataBlob.fileKey);
    }

    // Seal block containing the MSP's transaction response to the storage request
    await userApi.wait.mspResponseInTxPool();
    await userApi.block.seal();

    let mspAcceptedStorageRequestDataBlob: any = undefined;

    const eventsRecorded = await userApi.query.system.events();
    const mspAcceptedStorageRequestEvent = eventsRecorded.find(
      (e) => e.event.section === "fileSystem" && e.event.method === "MspAcceptedStorageRequest"
    );

    if (mspAcceptedStorageRequestEvent) {
      mspAcceptedStorageRequestDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(
          mspAcceptedStorageRequestEvent.event
        ) && mspAcceptedStorageRequestEvent.event.data;
    }

    const acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();

    assert(acceptedFileKey, "MspAcceptedStorageRequest event were found");

    // There is only a single key being accepted since it is the first file key to be processed and there is nothing to batch.
    strictEqual(
      issuedFileKeys.some((key) => key.toString() === acceptedFileKey),
      true
    );

    // Allow time for the MSP to update the local forest root
    await sleep(3000);

    const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(
      newBucketEventDataBlob.bucketId.toString()
    );

    const { event: bucketRootChangedEvent } = await userApi.assert.eventPresent(
      "providers",
      "BucketRootChanged"
    );

    const bucketRootChangedDataBlob =
      userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent) &&
      bucketRootChangedEvent.data;

    assert(
      bucketRootChangedDataBlob,
      "Expected BucketRootChanged event but received event of different type"
    );

    strictEqual(bucketRootChangedDataBlob.newRoot.toString(), localBucketRoot.toString());

    const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
      newBucketEventDataBlob.bucketId.toString(),
      acceptedFileKey
    );

    assert(isFileInForest.isTrue, "File is not in forest");

    // Seal block containing the MSP's transaction response to the storage request
    await userApi.wait.mspResponseInTxPool();
    await userApi.block.seal();

    const fileKeys2: string[] = [];

    const mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
      "fileSystem",
      "MspAcceptedStorageRequest"
    );

    for (const e of mspAcceptedStorageRequestEvents) {
      const mspAcceptedStorageRequestDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) && e.event.data;
      if (mspAcceptedStorageRequestDataBlob) {
        fileKeys2.push(mspAcceptedStorageRequestDataBlob.fileKey.toString());
      }
    }

    assert(fileKeys2.length === 2, "Expected 2 file keys");

    // Allow time for the MSP to update the local forest root
    await sleep(3000);

    const localBucketRoot2 = await mspApi.rpc.storagehubclient.getForestRoot(
      newBucketEventDataBlob.bucketId.toString()
    );

    const { event: bucketRootChangedEvent2 } = await userApi.assert.eventPresent(
      "providers",
      "BucketRootChanged"
    );

    const bucketRootChangedDataBlob2 =
      userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent2) &&
      bucketRootChangedEvent2.data;

    assert(
      bucketRootChangedDataBlob2,
      "Expected BucketRootChanged event but received event of different type"
    );

    strictEqual(bucketRootChangedDataBlob2.newRoot.toString(), localBucketRoot2.toString());

    for (const fileKey of fileKeys2) {
      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        newBucketEventDataBlob.bucketId.toString(),
        fileKey
      );
      assert(isFileInForest.isTrue, "File is not in forest");
    }
  });

  it("MSP is charging user", async () => {
    let currentBlock = await userApi.rpc.chain.getHeader();
    let currentBlockNumber = currentBlock.number.toNumber();

    const blocksToAdvance = MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD) + 1;
    await userApi.block.skipTo(currentBlockNumber + blocksToAdvance - 1);

    // Wait for the MSP to try to charge the user and seal a block.
    await sleep(2000);
    await userApi.block.seal();

    // Verify that the MSP charged the users after the notified.
    await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");

    // Advance many MSP charging periods, to charge again, but this time with a known number of
    // blocks since last charged. That way, we can check for the exact amount charged.
    // Since the MSP is going to charge each period, the last charge should be for one period.
    currentBlock = await userApi.rpc.chain.getHeader();
    currentBlockNumber = currentBlock.number.toNumber();
    await userApi.block.skipTo(currentBlockNumber + 10 * MSP_CHARGING_PERIOD);

    // Calculate the expected rate of the payment stream and compare it to the actual rate.
    const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
      userApi.shConsts.DUMMY_MSP_ID
    );
    const bucketSize = (await userApi.query.providers.buckets(bucketId)).unwrap().size_.toNumber();
    const pricePerGigaUnitOfDataPerBlock =
      valueProps[0].value_prop.price_per_giga_unit_of_data_per_block.toNumber();
    const unitsInGigaUnit = 1024 * 1024 * 1024;
    const expectedRateOfPaymentStream =
      Math.ceil((pricePerGigaUnitOfDataPerBlock * bucketSize) / unitsInGigaUnit) +
      userApi.consts.providers.zeroSizeBucketFixedRate.toNumber();
    const paymentStream = (
      await userApi.query.paymentStreams.fixedRatePaymentStreams(
        DUMMY_MSP_ID,
        userApi.shConsts.NODE_INFOS.user.AddressId
      )
    ).unwrap();
    const paymentStreamRate = paymentStream.rate.toNumber();
    strictEqual(
      paymentStreamRate,
      expectedRateOfPaymentStream,
      "Payment stream rate not matching the expected value"
    );

    // The expected amount to be charged is the rate of the payment stream times the charging period.
    const expectedChargedAmount = paymentStreamRate * MSP_CHARGING_PERIOD;

    // Verify that it charged for the correct amount.
    const paymentStreamChargedEvent = await userApi.assert.eventPresent(
      "paymentStreams",
      "PaymentStreamCharged"
    );
    assert(userApi.events.paymentStreams.PaymentStreamCharged.is(paymentStreamChargedEvent.event));
    const paymentStreamChargedEventAmount = paymentStreamChargedEvent.event.data.amount.toNumber();

    strictEqual(
      paymentStreamChargedEventAmount,
      expectedChargedAmount,
      "Charged amount not matching the expected value"
    );
  });
});

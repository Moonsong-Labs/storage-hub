import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi } from "../../../util";
import { MSP_CHARGING_PERIOD } from "../../../util/bspNet/consts";

await describeMspNet(
  "MSP test: MSP stops storing buckets that belong to insolvent users",
  ({ before, createMsp1Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let firstBucketId: string;
    let firstFileKey: string;
    let secondBucketId: string;
    let secondFileKey: string;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      // Update the storage request TTL so storage request don't expire during this test.
      const storageRequestTtlRuntimeParameter = {
        RuntimeConfig: {
          StorageRequestTtl: [null, 500]
        }
      };
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(storageRequestTtlRuntimeParameter)
          )
        ]
      });
    });

    it("User creates two new buckets and issues storage requests, MSP accepts them.", async () => {
      // Get the value propositions from the MSP. It should have at least one
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      // Get the ID of the first one. This is going to be used to access the price per giga-unit of data per tick.
      const valuePropId = valueProps[0].id;

      // Create a new bucket with the MSP and issue a storage request to it.
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "msp-stop-storing-insolvent-user";
      const { fileKey, bucketId } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        userApi.shConsts.DUMMY_MSP_ID
      );

      // Wait until the MSP detects this new bucket and sends a response accepting it.
      await userApi.wait.mspResponseInTxPool(1);

      // Seal the block containing the MSP's response.
      await userApi.block.seal();

      // Check that there's only one `MspAcceptedStorageRequest` event
      const mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      assert(
        mspAcceptedStorageRequestEvents.length === 1,
        "Expected a single MspAcceptedStorageRequest event"
      );

      // Get its file key and ensure it matches the one sent.
      const mspAcceptedStorageRequestEvent = mspAcceptedStorageRequestEvents[0];
      if (mspAcceptedStorageRequestEvent) {
        const mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(
            mspAcceptedStorageRequestEvent.event
          ) && mspAcceptedStorageRequestEvent.event.data;
        assert(mspAcceptedStorageRequestDataBlob, "MspAcceptedStorageRequest event was found");
        const acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();
        strictEqual(acceptedFileKey.toString(), fileKey.toString());
      }

      // Ensure the `BucketRootChanged` event was emitted
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

      // Wait for the MSP to download the file from the user and store it in its storage.
      await mspApi.wait.fileStorageComplete(fileKey);

      // Get the local root of the bucket now that the file has been stored.
      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId.toString());

      // Ensure the new local root of the bucket matches the one in the event.
      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), localBucketRoot.toString());

      // Ensure that the file has been stored in the MSP's forest, in the bucket's trie
      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        bucketId.toString(),
        fileKey
      );
      assert(isFileInForest.isTrue, "File is not in forest");

      // Save the bucket ID and file key for future use.
      firstBucketId = bucketId;
      firstFileKey = fileKey;

      // Create a second bucket with the MSP and issue a storage request to it.
      const source2 = "res/adolphus.jpg";
      const destination2 = "test/adolphus.jpg";
      const bucketName2 = "msp-stop-storing-insolvent-user-2";
      const { fileKey: fileKey2, bucketId: bucketId2 } =
        await userApi.file.createBucketAndSendNewStorageRequest(
          source2,
          destination2,
          bucketName2,
          valuePropId,
          userApi.shConsts.DUMMY_MSP_ID
        );

      // Wait until the MSP detects this new bucket and sends a response accepting it.
      await userApi.wait.mspResponseInTxPool(1);

      // Seal the block containing the MSP's response.
      await userApi.block.seal();

      // Check that there's only one `MspAcceptedStorageRequest` event
      const mspAcceptedStorageRequestEvents2 = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      assert(
        mspAcceptedStorageRequestEvents2.length === 1,
        "Expected a single MspAcceptedStorageRequest event"
      );

      // Get its file key and ensure it matches the one sent.
      const mspAcceptedStorageRequestEvent2 = mspAcceptedStorageRequestEvents2[0];
      if (mspAcceptedStorageRequestEvent2) {
        const mspAcceptedStorageRequestDataBlob2 =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(
            mspAcceptedStorageRequestEvent2.event
          ) && mspAcceptedStorageRequestEvent2.event.data;
        assert(mspAcceptedStorageRequestDataBlob2, "MspAcceptedStorageRequest event was found");
        const acceptedFileKey2 = mspAcceptedStorageRequestDataBlob2.fileKey.toString();
        strictEqual(acceptedFileKey2.toString(), fileKey2.toString());
      }

      // Ensure the `BucketRootChanged` event was emitted
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

      // Wait for the MSP to download the file from the user and store it
      // in its storage.
      await mspApi.wait.fileStorageComplete(fileKey2);

      // Get the local root of the bucket now that the file has been stored.
      const localBucketRoot2 = await mspApi.rpc.storagehubclient.getForestRoot(
        bucketId2.toString()
      );

      // Ensure the new local root of the bucket matches the one in the event.
      strictEqual(bucketRootChangedDataBlob2.newRoot.toString(), localBucketRoot2.toString());

      // Ensure that the file has been stored in the MSP's forest, in the bucket's trie
      const isFileInForest2 = await mspApi.rpc.storagehubclient.isFileInForest(
        bucketId2.toString(),
        fileKey2
      );
      assert(isFileInForest2.isTrue, "File is not in forest");

      // Save the bucket ID and file key for future use.
      secondBucketId = bucketId2;
      secondFileKey = fileKey2;
    });

    it("Payment stream between the user and the MSP gets flagged as without funds.", async () => {
      // To make the user insolvent, reduce its free balance so it can pay for this MSP's charging period.
      // First, update the payment stream to have a high rate (at least a few times the Existential Deposit)
      // so it's possible to reduce the user's balance to a point where it can't pay for the next MSP charging period.
      const existentialDeposit = userApi.consts.balances.existentialDeposit;
      const updateFixedRatePaymentStreamResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.paymentStreams.updateFixedRatePaymentStream(
              userApi.shConsts.DUMMY_MSP_ID,
              userApi.shConsts.NODE_INFOS.user.AddressId,
              existentialDeposit.muln(10)
            )
          )
        ]
      });
      const { extSuccess } = updateFixedRatePaymentStreamResult;
      strictEqual(extSuccess, true, "Extrinsic should be successful");

      // Assert that event fixed-rate payment stream update was emitted
      userApi.assertEvent(
        "paymentStreams",
        "FixedRatePaymentStreamUpdated",
        updateFixedRatePaymentStreamResult.events
      );

      // Get the payment stream's updated rate.
      const paymentStream = (
        await userApi.query.paymentStreams.fixedRatePaymentStreams(
          userApi.shConsts.DUMMY_MSP_ID,
          userApi.shConsts.NODE_INFOS.user.AddressId
        )
      ).unwrap();
      const paymentStreamRate = paymentStream.rate;

      // Make it so the user can't pay for the next MSP charging period, only for one tick.
      const reducedFreeBalance = paymentStreamRate;
      const reduceFreeBalanceResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.balances.forceSetBalance(
              userApi.shConsts.NODE_INFOS.user.AddressId,
              reducedFreeBalance
            )
          )
        ]
      });
      const changeBalanceSuccess = reduceFreeBalanceResult.extSuccess;
      strictEqual(changeBalanceSuccess, true, "Extrinsic should be successful");
      strictEqual(
        (
          await userApi.query.system.account(userApi.shConsts.NODE_INFOS.user.AddressId)
        ).data.free.toString(),
        reducedFreeBalance.toString()
      );

      // Get the current block.
      const currentBlock = await userApi.rpc.chain.getHeader();
      const currentBlockNumber = currentBlock.number.toNumber();

      // Advance to the next time the MSP is going to try to charge the user.
      const blocksToAdvance = MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD);
      await userApi.block.skipTo(currentBlockNumber + blocksToAdvance);

      // Wait until the MSP tries to charge the user.
      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true
      });

      // Seal a block with the transaction.
      await userApi.block.seal();

      // Assert that event for the MSP charging the user was emitted.
      const usersChargedEvents = await userApi.assert.eventMany("paymentStreams", "UsersCharged");

      // Assert that at least one of the events is for the MSP's payment stream.
      const mspUsersChargedEvent = usersChargedEvents.filter((e) => {
        const event = e.event;
        assert(userApi.events.paymentStreams.UsersCharged.is(event));
        return event.data.providerId.eq(userApi.shConsts.DUMMY_MSP_ID);
      });
      assert(
        mspUsersChargedEvent.length === 1,
        "Expected a single PaymentStreamCharged event for the MSP"
      );

      // Assert that the event of the MSP charging the payment stream was not emitted since the
      // stream was flagged as without funds.
      assert.rejects(userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged"));

      // Assert that the payment stream between the user and the MSP has been flagged as without funds.
      const insolventPaymentStreamInfoAfterCharging =
        await userApi.query.paymentStreams.fixedRatePaymentStreams(
          userApi.shConsts.DUMMY_MSP_ID,
          userApi.shConsts.NODE_INFOS.user.AddressId
        );
      assert(insolventPaymentStreamInfoAfterCharging.unwrap().outOfFundsTick.isSome);
    });

    it("Flags user as insolvent after the grace period and deletes payment stream.", async () => {
      // The grace period of marking a user as insolvent is equal to the new stream deposit period of the payment streams.
      // The user should be marked as insolvent after the grace period has passed.

      // Get the grace period.
      const gracePeriod = userApi.consts.paymentStreams.newStreamDeposit.toNumber();

      // Get the block at which the grace period will end.
      const currentBlock = await userApi.rpc.chain.getHeader();
      const currentBlockNumber = currentBlock.number.toNumber();
      const gracePeriodEndBlock = currentBlockNumber + gracePeriod;

      // Wait until the grace period ends.
      await userApi.block.skipTo(gracePeriodEndBlock);

      // Advance to the next time the MSP is going to try to charge the user.
      const blocksToAdvance = MSP_CHARGING_PERIOD - (gracePeriodEndBlock % MSP_CHARGING_PERIOD);
      await userApi.block.skipTo(gracePeriodEndBlock + blocksToAdvance);

      // Wait until the MSP tries to charge the user.
      await userApi.assert.extrinsicPresent({
        module: "paymentStreams",
        method: "chargeMultipleUsersPaymentStreams",
        checkTxPool: true
      });

      // Seal a block with the transaction.
      await userApi.block.seal();

      // Assert that the user without funds event was emitted.
      await userApi.assert.eventPresent("paymentStreams", "UserWithoutFunds");

      // Check that the payment stream between the user and the MSP has been deleted.
      const deletedPaymentStreamInfo = await userApi.query.paymentStreams.fixedRatePaymentStreams(
        userApi.shConsts.DUMMY_MSP_ID,
        userApi.shConsts.NODE_INFOS.user.AddressId
      );
      assert(deletedPaymentStreamInfo.isNone);
    });

    it("MSP stops storing the buckets and files of the now insolvent user.", async () => {
      // After the user has been marked as insolvent, the MSP should stop storing the buckets of the user.
      // For that, it will spawn multiple tasks, each submitting one extrinsic to delete one bucket.
      // Wait then until both extrinsics are in the tx pool, then seal a block with them and finalise it.
      // After that, the MSP should have deleted the bucket roots and the files from its storage.

      // Check that the MSP is trying to delete both buckets of the user.
      await userApi.assert.extrinsicPresent({
        method: "mspStopStoringBucketForInsolventUser",
        module: "fileSystem",
        checkTxPool: true,
        timeout: 10000,
        assertLength: 2,
        exactLength: true
      });

      // Seal a block to allow the MSP to stop storing both buckets, but don't finalise it yet, store it to finalise later.
      const block = await userApi.block.seal({ finaliseBlock: false });

      // Assert that both events for the MSP deleting the buckets were emitted.
      const stopStoringEvents = await userApi.assert.eventMany(
        "fileSystem",
        "MspStopStoringBucketInsolventUser"
      );
      assert(
        stopStoringEvents.length === 2,
        "Expected two MspStopStoringBucketInsolventUser events"
      );

      // Check that the bucket roots still exist since the blocks where they were deleted have not been finalised.
      const firstBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(firstBucketId);
      const secondBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(secondBucketId);
      assert(firstBucketRoot.isSome, "First bucket root should still exist");
      assert(secondBucketRoot.isSome, "Second bucket root should still exist");

      // And the files still exist in both the forest and file storages.
      const firstFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        firstBucketId,
        firstFileKey
      );
      assert(firstFileInForest.isTrue, "First file should still be in forest");
      const firstFileInFileStorage =
        await mspApi.rpc.storagehubclient.isFileInFileStorage(firstFileKey);
      assert(firstFileInFileStorage.isFileFound, "First file should still be in file storage");

      const secondFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        secondBucketId,
        secondFileKey
      );
      assert(secondFileInForest.isTrue, "Second file should still be in forest");
      const secondFileInFileStorage =
        await mspApi.rpc.storagehubclient.isFileInFileStorage(secondFileKey);
      assert(secondFileInFileStorage.isFileFound, "Second file should still be in file storage");

      // Finalise the block and check that the bucket roots are deleted.
      await mspApi.rpc.engine.finalizeBlock(block.blockReceipt.blockHash);

      // Wait until the buckets are deleted from the forest storage of the MSP.
      await mspApi.wait.mspBucketDeletionCompleted(firstBucketId);
      await mspApi.wait.mspBucketDeletionCompleted(secondBucketId);

      // Wait until both file keys are not found in the file storage of the MSP.
      await mspApi.wait.fileDeletionFromFileStorage(firstFileKey);
      await mspApi.wait.fileDeletionFromFileStorage(secondFileKey);
    });
  }
);

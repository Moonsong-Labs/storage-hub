import assert, { strictEqual } from "node:assert";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";
import { DUMMY_MSP_ID, MSP_CHARGING_PERIOD } from "../../../util/bspNet/consts";

await describeMspNet(
  "Single MSP collecting debt",
  ({ before, createMsp1Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bucketId: string;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
    });

    it("MSP receives files from user after issued storage requests", async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
      const bucketName = "nothingmuch-3";

      // Get the value propositions from the MSP. It should have at least one
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      // Get the ID of the first one. This is going to be used to access the price per giga-unit of data per tick
      const valuePropId = valueProps[0].id;

      // Check that the payment stream between the user and the MSP was created before files are added
      const paymentStreamBeforeFiles = await userApi.query.paymentStreams.fixedRatePaymentStreams(
        userApi.shConsts.DUMMY_MSP_ID,
        userApi.shConsts.NODE_INFOS.user.AddressId
      );

      // Use batchStorageRequests helper to create bucket and submit storage requests
      const batchResult = await userApi.file.batchStorageRequests({
        files: source.map((src, i) => ({
          source: src,
          destination: destination[i],
          bucketIdOrName: bucketName,
          replicationTarget: 1
        })),
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        valuePropId,
        owner: shUser,
        bspApi: undefined, // No BSP needed for this test
        mspApi
      });

      // Extract bucket ID and file keys from the batch result
      bucketId = batchResult.bucketIds[0];
      const issuedFileKeys = batchResult.fileKeys;

      // Check that the payment stream was created and has a rate equal to the rate for a zero-sized bucket
      assert(
        paymentStreamBeforeFiles.isSome ||
          (
            await userApi.query.paymentStreams.fixedRatePaymentStreams(
              userApi.shConsts.DUMMY_MSP_ID,
              userApi.shConsts.NODE_INFOS.user.AddressId
            )
          ).isSome,
        "Payment stream not found"
      );

      // Get the current payment stream after files are added
      let paymentStream = (
        await userApi.query.paymentStreams.fixedRatePaymentStreams(
          userApi.shConsts.DUMMY_MSP_ID,
          userApi.shConsts.NODE_INFOS.user.AddressId
        )
      ).unwrap();
      const zeroSizeBucketFixedRate = userApi.consts.providers.zeroSizeBucketFixedRate.toNumber();

      // The batchStorageRequests has already handled the MSP responses and acceptance
      // All files should already be accepted at this point
      // Let's verify the forest root is updated

      // Allow time for the MSP to update the local forest root
      await waitFor({
        lambda: async () => (await mspApi.rpc.storagehubclient.getForestRoot(bucketId)).isSome
      });

      // Get the root of the bucket now that the files have been stored
      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);

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

      // Ensure the new root of the bucket matches the one in the event
      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), localBucketRoot.toString());

      // Ensure that all files have been stored in the MSP's forest, in the bucket's trie
      for (const fileKey of issuedFileKeys) {
        const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
        assert(isFileInForest.isTrue, "File is not in forest");
      }

      // Check that the rate of the payment stream between the user and the MSP has been updated
      // to reflect the new size of the bucket
      paymentStream = (
        await userApi.query.paymentStreams.fixedRatePaymentStreams(
          DUMMY_MSP_ID,
          userApi.shConsts.NODE_INFOS.user.AddressId
        )
      ).unwrap();

      // Get the total size of all files in the bucket
      const bucketSize = (await userApi.query.providers.buckets(bucketId))
        .unwrap()
        .size_.toNumber();

      const unitsInGigaUnit = 1024 * 1024 * 1024;
      const expectedPaymentStreamRate = Math.round(
        (valueProps[0].value_prop.price_per_giga_unit_of_data_per_block.toNumber() * bucketSize) /
          unitsInGigaUnit +
          zeroSizeBucketFixedRate
      );
      strictEqual(paymentStream.rate.toNumber(), expectedPaymentStreamRate);

      await it("MSP is charging user", async () => {
        // Get the current block
        let currentBlock = await userApi.rpc.chain.getHeader();
        let currentBlockNumber = currentBlock.number.toNumber();

        // We want to advance to the next time the MSP is going to try to charge the user.
        const blocksToAdvance = MSP_CHARGING_PERIOD - (currentBlockNumber % MSP_CHARGING_PERIOD);
        await userApi.block.skipTo(currentBlockNumber + blocksToAdvance);

        // Wait for the MSP to try to charge the user and seal a block.
        await userApi.assert.extrinsicPresent({
          module: "paymentStreams",
          method: "chargeMultipleUsersPaymentStreams",
          checkTxPool: true
        });

        await userApi.block.seal();

        // Verify that the MSP was able to charge the user after the notify period.
        // Get all the PaymentStreamCharged events
        const firstPaymentStreamChargedEvents = await userApi.assert.eventMany(
          "paymentStreams",
          "PaymentStreamCharged"
        );

        // Keep only the ones that belong to the MSP, by checking the Provider ID
        const firstPaymentStreamChargedEventsFiltered = firstPaymentStreamChargedEvents.filter(
          (e) => {
            const event = e.event;
            assert(userApi.events.paymentStreams.PaymentStreamCharged.is(event));
            return event.data.providerId.eq(DUMMY_MSP_ID);
          }
        );

        // There should be only one PaymentStreamCharged event for the MSP
        assert(
          firstPaymentStreamChargedEventsFiltered.length === 1,
          "Expected a single PaymentStreamCharged event"
        );

        // Get it and check that the user account matches
        const firstPaymentStreamChargedEvent = firstPaymentStreamChargedEvents[0];
        assert(
          userApi.events.paymentStreams.PaymentStreamCharged.is(
            firstPaymentStreamChargedEvent.event
          ),
          "Expected PaymentStreamCharged event"
        );
        assert(
          firstPaymentStreamChargedEvent.event.data.userAccount.eq(
            userApi.shConsts.NODE_INFOS.user.AddressId
          ),
          "User account does not match"
        );

        // Advance one MSP charging period to charge again, but this time with a known number of
        // blocks since last charged. That way, we can check for the exact amount charged.
        // Since the MSP is going to charge each period, the last charge should be for one period.
        currentBlock = await userApi.rpc.chain.getHeader();
        currentBlockNumber = currentBlock.number.toNumber();
        await userApi.block.skipTo(currentBlockNumber + MSP_CHARGING_PERIOD - 1);

        // Check that the MSP tries to charge the user again.
        await userApi.assert.extrinsicPresent({
          module: "paymentStreams",
          method: "chargeMultipleUsersPaymentStreams",
          checkTxPool: true
        });

        // Calculate the expected rate of the payment stream and compare it to the actual rate.
        const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
          userApi.shConsts.DUMMY_MSP_ID
        );
        const bucketSize = (await userApi.query.providers.buckets(bucketId))
          .unwrap()
          .size_.toNumber();
        const pricePerGigaUnitOfDataPerBlock =
          valueProps[0].value_prop.price_per_giga_unit_of_data_per_block.toNumber();
        const unitsInGigaUnit = 1024 * 1024 * 1024;
        const expectedRateOfPaymentStream =
          Math.round((pricePerGigaUnitOfDataPerBlock * bucketSize) / unitsInGigaUnit) +
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

        // Seal the block containing the MSP's payment stream charge
        await userApi.block.seal();

        // Getting the PaymentStreamCharged events. There could be multiple of these events in the last block,
        // so we get them all and then filter the one where the Provider ID matches the MSP ID.
        const paymentStreamChargedEvents = await userApi.assert.eventMany(
          "paymentStreams",
          "PaymentStreamCharged"
        );
        const paymentStreamChargedEventsFiltered = paymentStreamChargedEvents.filter((e) => {
          const event = e.event;
          assert(userApi.events.paymentStreams.PaymentStreamCharged.is(event));
          return event.data.providerId.eq(DUMMY_MSP_ID);
        });

        // There should be only one PaymentStreamCharged event for the MSP
        assert(
          paymentStreamChargedEventsFiltered.length === 1,
          "Expected a single PaymentStreamCharged event"
        );

        // Verify that it charged for the correct amount.
        const paymentStreamChargedEvent = paymentStreamChargedEventsFiltered[0];
        assert(
          userApi.events.paymentStreams.PaymentStreamCharged.is(paymentStreamChargedEvent.event)
        );
        const paymentStreamChargedEventAmount =
          paymentStreamChargedEvent.event.data.amount.toNumber();

        strictEqual(
          paymentStreamChargedEventAmount,
          expectedChargedAmount,
          "Charged amount not matching the expected value"
        );
      });
    });
  }
);

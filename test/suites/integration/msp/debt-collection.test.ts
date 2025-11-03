import assert, { strictEqual } from "node:assert";
import type { Option } from "@polkadot/types";
import type { H256 } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
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

      // Create a bucket for the user in this MSP
      const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;
      assert(newBucketEventDataBlob, "NewBucket event data does not match expected type");

      // Check that the payment stream between the user and the MSP was created and has a rate equal
      // to the rate for a zero-sized bucket
      const maybePaymentStream = await userApi.query.paymentStreams.fixedRatePaymentStreams(
        userApi.shConsts.DUMMY_MSP_ID,
        userApi.shConsts.NODE_INFOS.user.AddressId
      );
      assert(maybePaymentStream.isSome, "Payment stream not found");
      let paymentStream = maybePaymentStream.unwrap();
      const zeroSizeBucketFixedRate = userApi.consts.providers.zeroSizeBucketFixedRate.toNumber();
      strictEqual(
        paymentStream.rate.toNumber(),
        zeroSizeBucketFixedRate,
        "Payment stream rate does not match the expected value"
      );

      bucketId = newBucketEventDataBlob.bucketId.toHex();

      // Load each file in storage and issue the storage requests
      const txs = [];
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      for (let i = 0; i < source.length; i++) {
        const {
          file_metadata: { location, fingerprint, file_size }
        } = await userApi.rpc.storagehubclient.loadFileInStorage(
          source[i],
          destination[i],
          ownerHex,
          bucketId
        );

        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Basic: null
            }
          )
        );
      }
      await userApi.block.seal({ calls: txs, signer: shUser });

      // Check that the storage request submission events were emitted
      const newStorageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "NewStorageRequest"
      );
      const matchedStorageRequestEvents = newStorageRequestEvents.filter((e) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );
      assert(
        matchedStorageRequestEvents.length === source.length,
        `Expected ${source.length} NewStorageRequest events`
      );

      // For each issued storage request, check that the file is in the MSP's storage
      const issuedFileKeys = [];
      for (const e of matchedStorageRequestEvents) {
        const newStorageRequestDataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

        assert(newStorageRequestDataBlob, "Event doesn't match NewStorageRequest type");

        await mspApi.wait.fileStorageComplete(newStorageRequestDataBlob.fileKey);

        issuedFileKeys.push(newStorageRequestDataBlob.fileKey);
      }

      // Seal block containing the MSP's transaction response to the first received storage request
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      let mspAcceptedStorageRequestDataBlob: any;

      // Check that there's only one `MspAcceptedStorageRequest` event
      let mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      assert(
        mspAcceptedStorageRequestEvents.length === 1,
        "Expected a single MspAcceptedStorageRequest event"
      );

      // Get its file key
      const mspAcceptedStorageRequestEvent = mspAcceptedStorageRequestEvents[0];
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
      await waitFor({
        lambda: async () =>
          (
            await mspApi.rpc.storagehubclient.getForestRoot(
              newBucketEventDataBlob.bucketId.toString()
            )
          ).isSome
      });

      // Get the root of the bucket now that the file has been stored
      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(
        newBucketEventDataBlob.bucketId.toString()
      );

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

      // Ensure that the file has been stored in the MSP's forest, in the bucket's trie
      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
        newBucketEventDataBlob.bucketId.toString(),
        acceptedFileKey
      );

      assert(isFileInForest.isTrue, "File is not in forest");

      // Check that the rate of the payment stream between the user and the MSP has been updated
      // to reflect the new size of the bucket
      paymentStream = (
        await userApi.query.paymentStreams.fixedRatePaymentStreams(
          DUMMY_MSP_ID,
          userApi.shConsts.NODE_INFOS.user.AddressId
        )
      ).unwrap();
      const bucketOption: Option<H256> = userApi.createType("Option<H256>", bucketId);
      const firstFileSize = (
        await mspApi.rpc.storagehubclient.getFileMetadata(bucketOption, acceptedFileKey)
      )
        .unwrap()
        .file_size.toNumber();
      const unitsInGigaUnit = 1024 * 1024 * 1024;
      let expectedPaymentStreamRate =
        (valueProps[0].value_prop.price_per_giga_unit_of_data_per_block.toNumber() *
          firstFileSize) /
          unitsInGigaUnit +
        zeroSizeBucketFixedRate;
      strictEqual(paymentStream.rate.toNumber(), Math.round(expectedPaymentStreamRate));

      // Seal block containing the MSP's transaction response to the storage request
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      const fileKeys2: string[] = [];

      // Since we gave enough time to the MSP to receive both files before processing the response for the
      // first one, we should have two `MspAcceptedStorageRequest` events (because of the batching)
      mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
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
      await waitFor({
        lambda: async () =>
          (
            await mspApi.rpc.storagehubclient.getForestRoot(
              newBucketEventDataBlob.bucketId.toString()
            )
          )
            .unwrap()
            .toHex() !== localBucketRoot.unwrap().toHex()
      });

      // Get the root of the bucket now that the files have been stored
      const localBucketRoot2 = await mspApi.rpc.storagehubclient.getForestRoot(
        newBucketEventDataBlob.bucketId.toString()
      );

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

      // Ensure the new root of the bucket matches the one in the event
      strictEqual(bucketRootChangedDataBlob2.newRoot.toString(), localBucketRoot2.toString());

      // Ensure that the files have been stored in the MSP's forest, in the bucket's trie
      for (const fileKey of fileKeys2) {
        const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
          newBucketEventDataBlob.bucketId.toString(),
          fileKey
        );
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
      const secondFileSize = (
        await mspApi.rpc.storagehubclient.getFileMetadata(bucketOption, fileKeys2[0])
      )
        .unwrap()
        .file_size.toNumber();
      const thirdFileSize = (
        await mspApi.rpc.storagehubclient.getFileMetadata(bucketOption, fileKeys2[1])
      )
        .unwrap()
        .file_size.toNumber();
      const sumOfSizeOfFiles = firstFileSize + secondFileSize + thirdFileSize;
      expectedPaymentStreamRate = Math.round(
        (valueProps[0].value_prop.price_per_giga_unit_of_data_per_block.toNumber() *
          sumOfSizeOfFiles) /
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

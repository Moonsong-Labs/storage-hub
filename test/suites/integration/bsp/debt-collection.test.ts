import assert, { strictEqual } from "node:assert";
import { after } from "node:test";
import { describeBspNet, fetchEventData, ShConsts, type EnrichedBspApi } from "../../../util";

describeBspNet(
  "BSPNet: Collect users debt",
  { initialised: "multi", networkConfig: "standard" },
  ({ before, it, createUserApi, createBspApi, getLaunchResponse, createApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;
    let userAddress: string;

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(launchResponse, "BSPNet failed to initialise");
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse?.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse?.bspThreeRpcPort}`);
      userAddress = ShConsts.NODE_INFOS.user.AddressId;
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
    });

    it("BSP correctly charges payment stream", async () => {
      // Make sure the payment stream between the user and the DUMMY_BSP_ID actually exists
      const paymentStreamExistsResult =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(
          ShConsts.DUMMY_BSP_ID
        );
      // Check if the first element of the returned vector is the user
      assert(paymentStreamExistsResult[0].toString() === userAddress);
      assert(paymentStreamExistsResult.length === 1);

      // Seal one more block.
      await userApi.sealBlock();

      // Check if the user owes the provider.
      let usersWithDebtResult = await bspApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        0
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 1);
      assert(usersWithDebtResult.asOk[0].toString() === userAddress);

      // Seal one more block with the pending extrinsics.
      await userApi.sealBlock();

      // Calculate the next challenge tick for the BSPs. It should be the same for all BSPs,
      // since they all have the same file they were initialised with, and responded to it at
      // the same time.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        ShConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        ShConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick.
      let currentBlock = await userApi.rpc.chain.getBlock();
      let currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlockNumber;

      // Advance blocksToAdvance blocks.
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.sealBlock();
      }

      await userApi.assert.extrinsicPresent({
        method: "submitProof",
        module: "proofsDealer",
        checkTxPool: true,
        assertLength: 3
      });

      // Check that no Providers have submitted a valid proof yet.
      currentBlock = await userApi.rpc.chain.getBlock();
      currentBlockNumber = currentBlock.block.header.number.toNumber();
      let providersWithProofs =
        await userApi.query.proofsDealer.validProofSubmittersLastTicks(currentBlockNumber);
      assert(providersWithProofs.isEmpty, "No Providers should have submitted a valid proof yet");

      // Seal one more block with the pending extrinsics.
      await userApi.sealBlock();

      // Assert for the the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
      strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

      // Check that the Providers were added to the list of Providers that have submitted proofs
      currentBlock = await userApi.rpc.chain.getBlock();
      currentBlockNumber = currentBlock.block.header.number.toNumber();
      providersWithProofs =
        await userApi.query.proofsDealer.validProofSubmittersLastTicks(currentBlockNumber);
      assert(
        providersWithProofs.isSome,
        "There should be Providers that have submitted a valid proof"
      );
      assert(
        providersWithProofs.unwrap().size === 3,
        "There should be three Providers that have submitted a valid proof"
      );

      // Check that the last chargeable info of the dummy BSP has not been updated yet
      let lastChargeableInfo = await userApi.query.paymentStreams.lastChargeableInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(lastChargeableInfo.priceIndex.toNumber() === 0);

      // Seal one more block to update the last chargeable info of the Provider
      await userApi.sealBlock();

      // Assert for the the event of the last chargeable info of the Providers being updated
      const lastChargeableInfoUpdatedEvents = await userApi.assert.eventMany(
        "paymentStreams",
        "LastChargeableInfoUpdated"
      );
      strictEqual(
        lastChargeableInfoUpdatedEvents.length,
        3,
        "There should be three last chargeable info updated events"
      );

      // Check the last chargeable info of the dummy BSP
      lastChargeableInfo = await userApi.query.paymentStreams.lastChargeableInfo(
        ShConsts.DUMMY_BSP_ID
      );

      // Check the info of the payment stream between the user and the DUMMY_BSP_ID
      const paymentStreamInfo = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
        ShConsts.DUMMY_BSP_ID,
        userAddress
      );

      // Check that the last chargeable price index of the dummy BSP is greater than the last charged price index of the payment stream
      // so that the payment stream can be charged by the BSP
      assert(
        paymentStreamInfo.unwrap().priceIndexWhenLastCharged.lt(lastChargeableInfo.priceIndex)
      );

      // Check that the user now owes the provider.
      usersWithDebtResult = await userApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        1
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 1);
      assert(usersWithDebtResult.asOk[0].toString() === userAddress);

      // Check that the three Providers have tried to charge the user
      // since the user has a payment stream with each of them
      await userApi.assert.extrinsicPresent({
        method: "chargePaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        assertLength: 3
      });

      // Seal a block to allow BSPs to charge the payment stream
      await userApi.sealBlock();

      // Assert that event for the BSP charging its payment stream was emitted
      await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");
    });

    it("Correctly updates payment stream on-chain to make user insolvent", async () => {
      // Make sure the payment stream between the user and the DUMMY_BSP_ID actually exists
      const paymentStreamExistsResult =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(
          ShConsts.DUMMY_BSP_ID
        );
      // Check if the first element of the returned vector is the user
      assert(paymentStreamExistsResult[0].toString() === userAddress);
      assert(paymentStreamExistsResult.length === 1);

      // Check the payment stream info between the user and the DUMMY_BSP_ID
      const paymentStreamInfoBeforeDeletion =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.DUMMY_BSP_ID,
          userAddress
        );

      // Add extra files to the user's storage with the DUMMY_BSP_ID
      await userApi.file.newStorageRequest("res/cloud.jpg", "test/cloud.jpg", "bucket-1");
      await userApi.wait.bspVolunteer();
      await userApi.wait.bspStored();
      await userApi.file.newStorageRequest("res/adolphus.jpg", "test/adolphus.jpg", "bucket-3");
      await userApi.wait.bspVolunteer();
      await userApi.wait.bspStored();

      // Check the payment stream info after adding the new files
      const paymentStreamInfoAfterAddingFiles =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.DUMMY_BSP_ID,
          userAddress
        );

      // The amount provided of the payment stream should be higher after adding the new files
      assert(
        paymentStreamInfoAfterAddingFiles
          .unwrap()
          .amountProvided.gt(paymentStreamInfoBeforeDeletion.unwrap().amountProvided)
      );

      // Seal one more block.
      await userApi.sealBlock();

      // Check if the user owes the provider.
      const usersWithDebtResult = await bspApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        0
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 1);
      assert(usersWithDebtResult.asOk[0].toString() === userAddress);

      // Seal one more block with the pending extrinsics.
      await userApi.sealBlock();

      // Get the current price of storage from the runtime, the new stream deposit and the ED
      const currentPriceOfStorage = await userApi.query.paymentStreams.currentPricePerUnitPerTick();
      const newStreamDeposit = userApi.consts.paymentStreams.newStreamDeposit;
      const existentialDeposit = userApi.consts.balances.existentialDeposit;

      // Get the current free balance of the user
      const freeBalance = (await userApi.query.system.account(userAddress)).data.free;

      // To make the user insolvent, we need to update the payment stream with a very high amount
      // and advance new stream deposit blocks
      // To do this, the new amount provided should be equal to the free balance of the user divided by
      // the current price of storage multiplied by the new stream deposit
      const newAmountProvidedForInsolvency = freeBalance
        .div(currentPriceOfStorage.mul(newStreamDeposit))
        .sub(existentialDeposit);

      // Make the user insolvent by updating the payment stream with a very high amount
      const updateDynamicRatePaymentStreamResult = await userApi.sealBlock(
        userApi.tx.sudo.sudo(
          userApi.tx.paymentStreams.updateDynamicRatePaymentStream(
            ShConsts.DUMMY_BSP_ID,
            userAddress,
            newAmountProvidedForInsolvency
          )
        )
      );
      const { extSuccess } = updateDynamicRatePaymentStreamResult;
      strictEqual(extSuccess, true, "Extrinsic should be successful");

      // Assert that event dynamic-rate payment stream update was emitted
      userApi.assertEvent(
        "paymentStreams",
        "DynamicRatePaymentStreamUpdated",
        updateDynamicRatePaymentStreamResult.events
      );
      // Get the on-chain payment stream information
      const [userAccount, providerId, newAmountProvided] = fetchEventData(
        userApi.events.paymentStreams.DynamicRatePaymentStreamUpdated,
        await userApi.query.system.events()
      );
      // Assert that the information on-chain is correct
      strictEqual(userAccount.toString(), userAddress);
      strictEqual(providerId.toString(), ShConsts.DUMMY_BSP_ID.toString());
      strictEqual(newAmountProvided.toNumber(), newAmountProvidedForInsolvency.toNumber());
    });

    it("Correctly flags update payment stream as without funds after charging", async () => {
      // Get the last chargeable info of the dummy BSP before proof submission
      const lastChargeableInfo = await userApi.query.paymentStreams.lastChargeableInfo(
        ShConsts.DUMMY_BSP_ID
      );
      // Calculate the next challenge tick for the DUMMY_BSP_ID.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        ShConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        ShConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick.
      let currentBlock = await userApi.rpc.chain.getBlock();
      let currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlockNumber;

      // Advance blocksToAdvance blocks.
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.sealBlock();
      }

      await userApi.assert.extrinsicPresent({
        method: "submitProof",
        module: "proofsDealer",
        checkTxPool: true,
        assertLength: 3
      });

      // Check that no Providers have submitted a valid proof yet.
      currentBlock = await userApi.rpc.chain.getBlock();
      currentBlockNumber = currentBlock.block.header.number.toNumber();
      let providersWithProofs =
        await userApi.query.proofsDealer.validProofSubmittersLastTicks(currentBlockNumber);
      assert(providersWithProofs.isEmpty, "No Providers should have submitted a valid proof yet");

      // Seal one more block with the pending extrinsics.
      await userApi.sealBlock();

      // Assert for the the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
      strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

      // Check that the Providers were added to the list of Providers that have submitted proofs
      currentBlock = await userApi.rpc.chain.getBlock();
      currentBlockNumber = currentBlock.block.header.number.toNumber();
      providersWithProofs =
        await userApi.query.proofsDealer.validProofSubmittersLastTicks(currentBlockNumber);
      assert(
        providersWithProofs.isSome,
        "There should be Providers that have submitted a valid proof"
      );
      assert(
        providersWithProofs.unwrap().size === 3,
        "There should be three Providers that have submitted a valid proof"
      );

      // Check that the last chargeable info of the dummy BSP has not been updated yet
      const lastChargeableInfoAfterProofSubmission =
        await userApi.query.paymentStreams.lastChargeableInfo(ShConsts.DUMMY_BSP_ID);
      assert(
        lastChargeableInfo.priceIndex.toNumber() ===
          lastChargeableInfoAfterProofSubmission.priceIndex.toNumber()
      );

      // Seal one more block to update the last chargeable info of the Provider
      await userApi.sealBlock();

      // Assert for the the event of the last chargeable info of the Providers being updated
      const lastChargeableInfoUpdatedEvents = await userApi.assert.eventMany(
        "paymentStreams",
        "LastChargeableInfoUpdated"
      );
      strictEqual(
        lastChargeableInfoUpdatedEvents.length,
        3,
        "There should be three last chargeable info updated events"
      );

      // Get the last chargeable info of the dummy BSP after it's updated
      const lastChargeableInfoAfterUpdate = await userApi.query.paymentStreams.lastChargeableInfo(
        ShConsts.DUMMY_BSP_ID
      );

      // Check the info of the payment stream between the user and the DUMMY_BSP_ID
      const paymentStreamInfo = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
        ShConsts.DUMMY_BSP_ID,
        userAddress
      );

      // Check that the last chargeable price index of the dummy BSP is greater than the last charged price index of the payment stream
      // so that the payment stream can be charged by the BSP
      assert(
        paymentStreamInfo
          .unwrap()
          .priceIndexWhenLastCharged.lt(lastChargeableInfoAfterUpdate.priceIndex)
      );

      // Check that the user now owes the provider.
      const usersWithDebtResult =
        await userApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
          ShConsts.DUMMY_BSP_ID,
          1
        );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 1);
      assert(usersWithDebtResult.asOk[0].toString() === userAddress);

      // Check that the three Providers have tried to charge the user
      // since the user has a payment stream with each of them
      await userApi.assert.extrinsicPresent({
        method: "chargePaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        assertLength: 3
      });

      // Seal a block to allow BSPs to charge the payment stream
      await userApi.sealBlock();

      // Assert that event for the BSP charging its payment stream was emitted
      await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");

      // Assert that the payment stream between the user and the DUMMY_BSP_ID has been flagged as without
      // funds, but the other two ones haven't
      const insolventPaymentStreamInfoAfterCharging =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.DUMMY_BSP_ID,
          userAddress
        );
      assert(insolventPaymentStreamInfoAfterCharging.unwrap().outOfFundsTick.isSome);
      const solventTwoPaymentStreamInfoAfterCharging =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.BSP_TWO_ID,
          userAddress
        );
      assert(solventTwoPaymentStreamInfoAfterCharging.unwrap().outOfFundsTick.isNone);
      const solventThreePaymentStreamInfoAfterCharging =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.BSP_THREE_ID,
          userAddress
        );
      assert(solventThreePaymentStreamInfoAfterCharging.unwrap().outOfFundsTick.isNone);
    });

    it("Correctly flags user as without funds after grace period, emits event and deletes payment stream", async () => {
      // Get the last chargeable info of the dummy BSP before proof submission
      const lastChargeableInfo = await userApi.query.paymentStreams.lastChargeableInfo(
        ShConsts.DUMMY_BSP_ID
      );
      // Calculate the next challenge tick for the DUMMY_BSP_ID.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        ShConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        ShConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick.
      let currentBlock = await userApi.rpc.chain.getBlock();
      let currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlockNumber;

      // Advance blocksToAdvance blocks
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.sealBlock();
      }

      await userApi.assert.extrinsicPresent({
        method: "submitProof",
        module: "proofsDealer",
        checkTxPool: true,
        assertLength: 3
      });

      // Check that no Providers have submitted a valid proof yet.
      currentBlock = await userApi.rpc.chain.getBlock();
      currentBlockNumber = currentBlock.block.header.number.toNumber();
      let providersWithProofs =
        await userApi.query.proofsDealer.validProofSubmittersLastTicks(currentBlockNumber);
      assert(providersWithProofs.isEmpty, "No Providers should have submitted a valid proof yet");

      // Seal one more block with the pending extrinsics.
      await userApi.sealBlock();

      // Assert for the the event of the proof successfully submitted and verified.
      const proofAcceptedEvents = await userApi.assert.eventMany("proofsDealer", "ProofAccepted");
      strictEqual(proofAcceptedEvents.length, 3, "There should be three proofs accepted events");

      // Check that the Providers were added to the list of Providers that have submitted proofs
      currentBlock = await userApi.rpc.chain.getBlock();
      currentBlockNumber = currentBlock.block.header.number.toNumber();
      providersWithProofs =
        await userApi.query.proofsDealer.validProofSubmittersLastTicks(currentBlockNumber);
      assert(
        providersWithProofs.isSome,
        "There should be Providers that have submitted a valid proof"
      );
      assert(
        providersWithProofs.unwrap().size === 3,
        "There should be three Providers that have submitted a valid proof"
      );

      // Check that the last chargeable info of the dummy BSP has not been updated yet
      const lastChargeableInfoAfterProofSubmission =
        await userApi.query.paymentStreams.lastChargeableInfo(ShConsts.DUMMY_BSP_ID);
      assert(
        lastChargeableInfo.priceIndex.toNumber() ===
          lastChargeableInfoAfterProofSubmission.priceIndex.toNumber()
      );

      // Seal one more block to update the last chargeable info of the Provider
      await userApi.sealBlock();

      // Assert for the the event of the last chargeable info of the Providers being updated
      const lastChargeableInfoUpdatedEvents = await userApi.assert.eventMany(
        "paymentStreams",
        "LastChargeableInfoUpdated"
      );
      strictEqual(
        lastChargeableInfoUpdatedEvents.length,
        3,
        "There should be three last chargeable info updated events"
      );

      // Check that the three Providers have tried to charge the user
      // since the user has a payment stream with each of them
      await userApi.assert.extrinsicPresent({
        method: "chargePaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        assertLength: 3
      });

      // Seal a block to allow BSPs to charge the payment stream
      const blockResult = await userApi.sealBlock();

      // Assert that event for the BSP charging its payment stream was emitted
      await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");

      // Check if the "UserWithoutFunds" event was emitted. If it wasn't, advance until
      // the next challenge period and check again
      if (!blockResult.events?.find((event) => event.event.method === "UserWithoutFunds")) {
        // Calculate the next challenge tick for the DUMMY_BSP_ID.
        // We first get the last tick for which the BSP submitted a proof.
        const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          ShConsts.DUMMY_BSP_ID
        );
        assert(lastTickResult.isOk);
        const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
        // Then we get the challenge period for the BSP.
        const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
          ShConsts.DUMMY_BSP_ID
        );
        assert(challengePeriodResult.isOk);
        const challengePeriod = challengePeriodResult.asOk.toNumber();
        // Then we calculate the next challenge tick.
        const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

        // Calculate how many blocks to advance until next challenge tick.
        currentBlock = await userApi.rpc.chain.getBlock();
        currentBlockNumber = currentBlock.block.header.number.toNumber();
        const blocksToAdvance = nextChallengeTick - currentBlockNumber;
        // Advance blocksToAdvance blocks
        for (let i = 0; i < blocksToAdvance; i++) {
          await userApi.sealBlock();
        }

        await userApi.assert.extrinsicPresent({
          method: "submitProof",
          module: "proofsDealer",
          checkTxPool: true,
          assertLength: 3
        });

        // Seal one more block with the pending extrinsics.
        await userApi.sealBlock();

        // Seal another block so the last chargeable info of the providers is updated
        await userApi.sealBlock();

        // Check that the three Providers have tried to charge the user
        // since the user has a payment stream with each of them
        await userApi.assert.extrinsicPresent({
          method: "chargePaymentStreams",
          module: "paymentStreams",
          checkTxPool: true,
          assertLength: 3
        });

        // Seal a block to allow BSPs to charge the payment stream
        await userApi.sealBlock();
      }

      // Assert that the user without funds event was emitted
      await userApi.assert.eventPresent("paymentStreams", "UserWithoutFunds");

      // Check that the payment stream between the user and the DUMMY_BSP_ID has been deleted
      const deletedPaymentStreamInfo = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
        ShConsts.DUMMY_BSP_ID,
        userAddress
      );
      assert(deletedPaymentStreamInfo.isNone);
    });

    it("BSP correctly deletes all files from an insolvent user", async () => {
      // We execute this loop three times since that's the amount of files the user has stored with the BSPs
      for (let i = 0; i < 3; i++) {
        // Check that the three Providers are trying to delete the files of the user
        await userApi.assert.extrinsicPresent({
          method: "stopStoringForInsolventUser",
          module: "fileSystem",
          checkTxPool: true,
          assertLength: 3
        });

        // Seal a block to allow BSPs to delete the files of the user
        await userApi.sealBlock();

        // Assert that event for the BSP deleting the files of the user was emitted
        const spStopStoringForInsolventUserEvents = await userApi.assert.eventMany(
          "fileSystem",
          "SpStopStoringInsolventUser"
        );
        strictEqual(
          spStopStoringForInsolventUserEvents.length,
          3,
          "There should be three stop storing for insolvent user events"
        );

        // For each event, fetch its info and check if the BSP correctly deleted the files of the user
        for (const event of spStopStoringForInsolventUserEvents) {
          const stopStoringInsolventUserBlob =
            userApi.events.fileSystem.SpStopStoringInsolventUser.is(event.event) &&
            event.event.data;
          assert(stopStoringInsolventUserBlob, "Event doesn't match Type");
          if (stopStoringInsolventUserBlob.spId.toString() === ShConsts.DUMMY_BSP_ID) {
            assert(
              (
                await bspApi.rpc.storagehubclient.isFileInForest(
                  null,
                  stopStoringInsolventUserBlob.fileKey
                )
              ).isFalse
            );
          } else if (stopStoringInsolventUserBlob.spId.toString() === ShConsts.BSP_TWO_ID) {
            assert(
              (
                await bspTwoApi.rpc.storagehubclient.isFileInForest(
                  null,
                  stopStoringInsolventUserBlob.fileKey
                )
              ).isFalse
            );
          } else if (stopStoringInsolventUserBlob.spId.toString() === ShConsts.BSP_THREE_ID) {
            assert(
              (
                await bspThreeApi.rpc.storagehubclient.isFileInForest(
                  null,
                  stopStoringInsolventUserBlob.fileKey
                )
              ).isFalse
            );
          }
        }

        // Seal a block to allow BSPs to delete the files of the user
        await userApi.sealBlock();
      }

      // After deleting all the files, the user should have no payment streams with any provider
      const paymentStreamInfoAfterDeletion =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.DUMMY_BSP_ID,
          userAddress
        );
      assert(paymentStreamInfoAfterDeletion.isNone);
      const paymentStreamInfoAfterDeletionTwo =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.BSP_TWO_ID,
          userAddress
        );
      assert(paymentStreamInfoAfterDeletionTwo.isNone);
      const paymentStreamInfoAfterDeletionThree =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.BSP_THREE_ID,
          userAddress
        );
      assert(paymentStreamInfoAfterDeletionThree.isNone);
    });
  }
);

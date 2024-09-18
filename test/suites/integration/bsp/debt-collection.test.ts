import "@storagehub/api-augment";
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

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(launchResponse, "BSPNet failed to initialise");
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse?.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse?.bspThreeRpcPort}`);
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
    });

    it("BSP correctly charges payment stream", async () => {
      // Force create a dynamic payment stream from Alice to the Provider
      const alice = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
      const createDynamicRatePaymentStreamResult = await userApi.sealBlock(
        userApi.tx.sudo.sudo(
          userApi.tx.paymentStreams.createDynamicRatePaymentStream(
            ShConsts.DUMMY_BSP_ID,
            alice,
            100
          )
        )
      );

      // Assert that event dynamic-rate payment stream creation was emitted
      userApi.assertEvent(
        "paymentStreams",
        "DynamicRatePaymentStreamCreated",
        createDynamicRatePaymentStreamResult.events
      );

      // Get the on-chain payment stream information
      const [userAccount, providerId, amountProvided] = fetchEventData(
        userApi.events.paymentStreams.DynamicRatePaymentStreamCreated,
        await userApi.query.system.events()
      );

      // Assert that the information on-chain is correct
      strictEqual(userAccount.toString(), alice);
      strictEqual(providerId.toString(), ShConsts.DUMMY_BSP_ID.toString());
      strictEqual(amountProvided.toNumber(), 100);

      // Make sure the payment stream between Alice and the DUMMY_BSP_ID actually exists
      const paymentStreamExistsResult =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(
          ShConsts.DUMMY_BSP_ID
        );
      // Check if the first element of the returned vector is alice
      assert(paymentStreamExistsResult[0].toString() === alice);
      assert(paymentStreamExistsResult.length === 1);

      // Seal one more block.
      await userApi.sealBlock();

      // Check if Alice owes the provider.
      let usersWithDebtResult = await bspApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        0
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 1);
      assert(usersWithDebtResult.asOk[0].toString() === alice);

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

      // Check the info of the payment stream between Alice and the DUMMY_BSP_ID
      const paymentStreamInfo = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
        ShConsts.DUMMY_BSP_ID,
        alice
      );

      // Check that the last chargeable price index of the dummy BSP is greater than the last charged price index of the payment stream
      // so that the payment stream can be charged by the BSP
      assert(
        paymentStreamInfo.unwrap().priceIndexWhenLastCharged.lt(lastChargeableInfo.priceIndex)
      );

      // Check that Alice now owes the provider.
      usersWithDebtResult = await userApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        1
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 1);
      assert(usersWithDebtResult.asOk[0].toString() === alice);

      await userApi.assert.extrinsicPresent({
        method: "chargePaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        assertLength: 1
      });

      // Seal a block to allow the BSP to charge the payment stream
      await userApi.sealBlock();

      // Assert that event for the BSP charging its payment stream was emitted
      await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");
    });
  }
);

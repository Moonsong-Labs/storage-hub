import assert, { strictEqual } from "node:assert";
import { after } from "node:test";
import { BN } from "@polkadot/util";
import { bob, describeBspNet, type EnrichedBspApi, fetchEvent, ShConsts } from "../../../util";

await describeBspNet(
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
      assert(
        launchResponse && "bspTwoRpcPort" in launchResponse && "bspThreeRpcPort" in launchResponse,
        "BSPNet failed to initialise with required ports"
      );
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse.bspThreeRpcPort}`);
      userAddress = ShConsts.NODE_INFOS.user.AddressId;
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
    });

    it("BSP correctly charges multiple payment streams", async () => {
      // Create a new payment stream between Bob and the DUMMY_BSP_ID
      const createBobPaymentStreamResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.paymentStreams.createDynamicRatePaymentStream(
              ShConsts.DUMMY_BSP_ID,
              bob.address,
              1024 * 1024 // 1 MB
            )
          )
        ]
      });
      const { extSuccess } = createBobPaymentStreamResult;
      strictEqual(extSuccess, true, "Extrinsic should be successful");

      // Make sure the payment streams between the users and the DUMMY_BSP_ID actually exists
      const paymentStreamExistsResult =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(
          ShConsts.DUMMY_BSP_ID
        );
      // Check that the returned vector mapped to strings has the user and Bob
      assert(paymentStreamExistsResult.map((x) => x.toString()).includes(userAddress));
      assert(paymentStreamExistsResult.map((x) => x.toString()).includes(bob.address));
      assert(paymentStreamExistsResult.length === 2);

      // Seal one more block.
      await userApi.block.seal();

      // Check if both the user and Bob owes the provider.
      let usersWithDebtResult = await bspApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        0
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 2);
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(userAddress));
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(bob.address));

      // Seal one more block with the pending extrinsics.
      await userApi.block.seal();

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
      if (nextChallengeTick > currentBlockNumber) {
        // Advance to the next challenge tick if needed
        await userApi.block.skipTo(nextChallengeTick);
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
      await userApi.block.seal();

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
      await userApi.block.seal();

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

      // Check the info of the payment streams between the users and the DUMMY_BSP_ID
      const paymentStreamInfo = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
        ShConsts.DUMMY_BSP_ID,
        userAddress
      );
      const bobPaymentStreamInfo = await userApi.query.paymentStreams.dynamicRatePaymentStreams(
        ShConsts.DUMMY_BSP_ID,
        bob.address
      );

      // Check that the last chargeable price index of the dummy BSP is greater than the last charged price index of the payment streams
      // so that the payment streams can be charged by the BSP
      assert(
        paymentStreamInfo.unwrap().priceIndexWhenLastCharged.lt(lastChargeableInfo.priceIndex)
      );
      assert(
        bobPaymentStreamInfo.unwrap().priceIndexWhenLastCharged.lt(lastChargeableInfo.priceIndex)
      );

      // Check that the user now owes the provider.
      usersWithDebtResult = await userApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        1
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 2);
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(userAddress));
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(bob.address));

      // Check that the three Providers have tried to charge the user
      // since the user has a payment stream with each of them
      await userApi.assert.extrinsicPresent({
        method: "chargeMultipleUsersPaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        assertLength: 3,
        exactLength: false
      });

      // Seal a block to allow BSPs to charge the payment stream
      await userApi.block.seal();

      // Assert that event for the BSP charging its users with payment streams was emitted
      await userApi.assert.eventPresent("paymentStreams", "UsersCharged");

      // Assert that the event for the DUMMY_BSP has both users charged
      const usersChargedEvents = await userApi.assert.eventMany("paymentStreams", "UsersCharged");
      strictEqual(usersChargedEvents.length, 3, "There should be three users charged event");
      const dummyBspEvent = usersChargedEvents.find(
        (event) => event.event.data[1].toString() === ShConsts.DUMMY_BSP_ID.toString()
      );
      assert(dummyBspEvent, "There should be an event for the DUMMY_BSP_ID");
      const usersChargedBlob =
        userApi.events.paymentStreams.UsersCharged.is(dummyBspEvent.event) &&
        dummyBspEvent.event.data;
      assert(usersChargedBlob, "Event doesn't match Type");
      assert(dummyBspEvent.event.data[0].map((x) => x.toString()).includes(userAddress));
      assert(dummyBspEvent.event.data[0].map((x) => x.toString()).includes(bob.address));
    });

    it("Correctly updates payment stream on-chain to make user insolvent", async () => {
      // Reduce the free balance of the user to make it insolvent
      const initialFreeBalance = (await userApi.query.system.account(userAddress)).data.free;
      const reduceFreeBalanceResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.balances.forceSetBalance(userAddress, initialFreeBalance.divn(10))
          )
        ]
      });
      const changeBalanceSuccess = reduceFreeBalanceResult.extSuccess;
      strictEqual(changeBalanceSuccess, true, "Extrinsic should be successful");

      // Make sure the payment streams between the users and the DUMMY_BSP_ID actually exists
      const paymentStreamExistsResult =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(
          ShConsts.DUMMY_BSP_ID
        );
      // Check that the returned vector mapped to strings has the user and Bob
      assert(paymentStreamExistsResult.map((x) => x.toString()).includes(userAddress));
      assert(paymentStreamExistsResult.map((x) => x.toString()).includes(bob.address));
      assert(paymentStreamExistsResult.length === 2);

      // Check the payment stream info between the user and the DUMMY_BSP_ID
      const paymentStreamInfoBeforeDeletion =
        await userApi.query.paymentStreams.dynamicRatePaymentStreams(
          ShConsts.DUMMY_BSP_ID,
          userAddress
        );

      // Add extra files to the user's storage with the three BSPs, waiting for them to be confirmed
      const cloudFileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        "res/cloud.jpg",
        "test/cloud.jpg",
        "bucket-1",
        null,
        null,
        null,
        7 // Make the replication target considerably bigger than 3 so statistically all BSPs can volunteer in the initial block
      );

      // Wait for the BSPs to volunteer, store and confirm storing the file.
      await userApi.wait.bspVolunteer(3);
      await bspApi.wait.fileStorageComplete(cloudFileMetadata.fileKey);
      await bspTwoApi.wait.fileStorageComplete(cloudFileMetadata.fileKey);
      await bspThreeApi.wait.fileStorageComplete(cloudFileMetadata.fileKey);
      await userApi.wait.bspStored({ expectedExts: 3 });

      const adolphusFileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        "res/adolphus.jpg",
        "test/adolphus.jpg",
        "bucket-3",
        null,
        null,
        null,
        7 // Make the replication target considerably bigger than 3 so statistically all BSPs can volunteer in the initial block
      );
      await userApi.wait.bspVolunteer(3);
      await bspApi.wait.fileStorageComplete(adolphusFileMetadata.fileKey);
      await bspTwoApi.wait.fileStorageComplete(adolphusFileMetadata.fileKey);
      await bspThreeApi.wait.fileStorageComplete(adolphusFileMetadata.fileKey);
      await userApi.wait.bspStored({ expectedExts: 3 });

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
      await userApi.block.seal();

      // Check if the user owes the provider.
      const usersWithDebtResult = await bspApi.call.paymentStreamsApi.getUsersWithDebtOverThreshold(
        ShConsts.DUMMY_BSP_ID,
        0
      );
      assert(usersWithDebtResult.isOk);
      assert(usersWithDebtResult.asOk.length === 2);
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(userAddress));
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(bob.address));

      // Seal one more block with the pending extrinsics.
      await userApi.block.seal();

      // Get the current price of storage from the runtime, the new stream deposit and the ED
      const currentPriceOfStorage =
        await userApi.query.paymentStreams.currentPricePerGigaUnitPerTick();
      const newStreamDeposit = userApi.consts.paymentStreams.newStreamDeposit;
      const existentialDeposit = userApi.consts.balances.existentialDeposit;

      // Get the current free balance of the user
      const freeBalance = (await userApi.query.system.account(userAddress)).data.free;

      // To make the user insolvent, we need to update the payment stream with a very high amount
      // of amount provided so when the BSP tries to charge it the user cannot pay its debt.
      // We set the new provided amount to be as much as possible considering the deposit
      // that the user is going to have to pay. We leave 10 * ED in the account to allow
      // the user to pay its other debts.
      const gigaUnit = new BN("1073741824", 10);
      const newAmountProvidedForInsolvency = freeBalance
        .sub(existentialDeposit.muln(10))
        .mul(gigaUnit)
        .div(currentPriceOfStorage.mul(newStreamDeposit));

      // Make the user insolvent by updating the payment stream with a very high amount
      const updateDynamicRatePaymentStreamResult = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.paymentStreams.updateDynamicRatePaymentStream(
              ShConsts.DUMMY_BSP_ID,
              userAddress,
              newAmountProvidedForInsolvency
            )
          )
        ]
      });
      const { extSuccess } = updateDynamicRatePaymentStreamResult;
      strictEqual(extSuccess, true, "Extrinsic should be successful");

      // Assert that event dynamic-rate payment stream update was emitted
      userApi.assertEvent(
        "paymentStreams",
        "DynamicRatePaymentStreamUpdated",
        updateDynamicRatePaymentStreamResult.events
      );
      // Get the on-chain payment stream information
      const {
        data: { userAccount, providerId, newAmountProvided }
      } = fetchEvent(
        userApi.events.paymentStreams.DynamicRatePaymentStreamUpdated,
        await userApi.query.system.events()
      );
      // Assert that the information on-chain is correct
      strictEqual(userAccount.toString(), userAddress);
      strictEqual(providerId.toString(), ShConsts.DUMMY_BSP_ID.toString());
      strictEqual(newAmountProvided.toString(), newAmountProvidedForInsolvency.toString());
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
        await userApi.block.seal();
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
      await userApi.block.seal();

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
      await userApi.block.seal();

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
      assert(usersWithDebtResult.asOk.length === 2);
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(userAddress));
      assert(usersWithDebtResult.asOk.map((x) => x.toString()).includes(bob.address));

      // Check that the three Providers have tried to charge the user
      // since the user has a payment stream with each of them
      await userApi.assert.extrinsicPresent({
        method: "chargeMultipleUsersPaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        assertLength: 3
      });

      // Seal a block to allow BSPs to charge the payment stream
      await userApi.block.seal();

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
        await userApi.block.seal();
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
      await userApi.block.seal();

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
      await userApi.block.seal();

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
        method: "chargeMultipleUsersPaymentStreams",
        module: "paymentStreams",
        checkTxPool: true,
        exactLength: false,
        assertLength: 3
      });

      // Seal a block to allow BSPs to charge the payment stream
      const blockResult = await userApi.block.seal();

      // Assert that event for the BSP charging its payment stream was emitted
      await userApi.assert.eventPresent("paymentStreams", "PaymentStreamCharged");

      // Check if the "UserWithoutFunds" event was emitted. If it wasn't, advance until
      // the next challenge period and check again
      if (!blockResult.events?.find((event) => event.event.method === "UserWithoutFunds")) {
        console.log("UserWithoutFunds event not found. Advancing to next challenge period.");
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
          await userApi.block.seal();
        }

        await userApi.assert.extrinsicPresent({
          method: "submitProof",
          module: "proofsDealer",
          checkTxPool: true,
          assertLength: 3
        });

        // Seal one more block with the pending extrinsics.
        await userApi.block.seal();

        // Seal another block so the last chargeable info of the providers is updated
        await userApi.block.seal();

        // Check that the three Providers have tried to charge the user
        // since the user has a payment stream with each of them
        await userApi.assert.extrinsicPresent({
          method: "chargeMultipleUsersPaymentStreams",
          module: "paymentStreams",
          checkTxPool: true,
          exactLength: false,
          assertLength: 3
        });

        // Seal a block to allow BSPs to charge the payment stream
        await userApi.block.seal();
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
        console.log("Removing file from insolvent user, loop: ", i + 1);

        try {
          // Check that the three Providers are trying to delete the files of the user
          const result = await userApi.assert.extrinsicPresent({
            method: "stopStoringForInsolventUser",
            module: "fileSystem",
            checkTxPool: true,
            assertLength: 3,
            timeout: 10000
          });

          // We check for each BSP which file key it's deleting and print it
          const txPool = await userApi.rpc.author.pendingExtrinsics();
          const stopStoringForInsolventUserExts = result.map((match) => txPool[match.extIndex]);

          for (const ext of stopStoringForInsolventUserExts) {
            const sender = ext.signer.toString();
            const bspIdSender = (
              await userApi.query.providers.accountIdToBackupStorageProviderId(sender)
            ).toString();
            const fileKey = ext.args[0].toString();
            console.log("BSP ", bspIdSender, " is deleting file with key: ", fileKey);
          }
        } catch (error) {
          console.log("Extrinsics not present: ", error);
          // We check for each BSP if it has already deleted all files, which shouldn't happen
          // We do this by checking the capacity of each BSP to make sure it's not 0, by using the
          // runtime API to check its total capacity and comparing it to its available capacity
          const bspOneAvailableCapacity =
            await userApi.call.storageProvidersApi.queryAvailableStorageCapacity(
              ShConsts.DUMMY_BSP_ID
            );
          const bspOneTotalCapacity =
            await userApi.call.storageProvidersApi.queryStorageProviderCapacity(
              ShConsts.DUMMY_BSP_ID
            );
          console.log(
            "BSP One total - available capacity: ",
            bspOneTotalCapacity.toNumber() - bspOneAvailableCapacity.toNumber()
          );

          const bspTwoAvailableCapacity =
            await userApi.call.storageProvidersApi.queryAvailableStorageCapacity(
              ShConsts.BSP_TWO_ID
            );
          const bspTwoTotalCapacity =
            await userApi.call.storageProvidersApi.queryStorageProviderCapacity(
              ShConsts.BSP_TWO_ID
            );
          console.log(
            "BSP Two total - available capacity: ",
            bspTwoTotalCapacity.toNumber() - bspTwoAvailableCapacity.toNumber()
          );

          const bspThreeAvailableCapacity =
            await userApi.call.storageProvidersApi.queryAvailableStorageCapacity(
              ShConsts.BSP_THREE_ID
            );
          const bspThreeTotalCapacity =
            await userApi.call.storageProvidersApi.queryStorageProviderCapacity(
              ShConsts.BSP_THREE_ID
            );
          console.log(
            "BSP Three total - available capacity: ",
            bspThreeTotalCapacity.toNumber() - bspThreeAvailableCapacity.toNumber()
          );
        }

        // Seal a block with the `stopStoringForInsolventUser` extrinsics.
        await userApi.block.seal();

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
          // Wait for BSPs to process the successful `stopStoringForInsolventUser` extrinsics.
          // i.e. wait for them to update the local forest root.
          if (stopStoringInsolventUserBlob.spId.toString() === ShConsts.DUMMY_BSP_ID) {
            await bspApi.wait.bspFileDeletionCompleted(stopStoringInsolventUserBlob.fileKey);
          } else if (stopStoringInsolventUserBlob.spId.toString() === ShConsts.BSP_TWO_ID) {
            await bspTwoApi.wait.bspFileDeletionCompleted(stopStoringInsolventUserBlob.fileKey);
          } else if (stopStoringInsolventUserBlob.spId.toString() === ShConsts.BSP_THREE_ID) {
            await bspThreeApi.wait.bspFileDeletionCompleted(stopStoringInsolventUserBlob.fileKey);
          }
        }
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

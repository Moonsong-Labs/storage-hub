import assert, { strictEqual } from "node:assert";
import { bspKey, describeBspNet, type EnrichedBspApi } from "../../../util";

await describeBspNet(
  "BSPNet: BSP Challenge Cycle and Proof Submission with changed capacity",
  { initialised: true },
  ({ it, before, createBspApi, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("BSP is challenged and correctly submits proof", async () => {
      // Calculate the next challenge tick for the BSP.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick.
      const currentBlockNumber = (await userApi.query.system.number()).toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlockNumber;

      // Advance blocksToAdvance blocks.
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.block.seal();
      }

      // Wait for task to execute and seal one more block.
      // In this block, the BSP should have submitted a proof.
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      await userApi.block.seal();

      // Assert for the the event of the proof successfully submitted and verified.
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted");
    });

    it("BSP's stake increased while next challenge deadline not changed", async () => {
      // Get current next deadline tick for BSP
      const initialNextDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(initialNextDeadlineResult.isOk);
      const initialNextDeadline = initialNextDeadlineResult.asOk.toNumber();

      // Skip blocks until the BSP can change its capacity.
      await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

      // Get current capacity to calculate increase
      const currentBspMetadata = await userApi.query.providers.backupStorageProviders(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(currentBspMetadata.isSome);
      const currentCapacity = currentBspMetadata.unwrap().capacity.toBigInt();
      const newCapacity = currentCapacity + BigInt(1024 * 1024); // Increase by 1MB

      // Send transaction to increase capacity
      await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
      const { extSuccess } = await userApi.block.seal({
        calls: [userApi.tx.providers.changeCapacity(newCapacity)],
        signer: bspKey
      });
      strictEqual(extSuccess, true, "Change capacity transaction should succeed");

      // Assert the capacity change event was emitted
      await userApi.assert.eventPresent("providers", "CapacityChanged");

      // Verify capacity was actually increased
      const updatedBspMetadata = await userApi.query.providers.backupStorageProviders(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(updatedBspMetadata.isSome);
      strictEqual(
        updatedBspMetadata.unwrap().capacity.toBigInt(),
        newCapacity,
        "BSP capacity should be updated to new value"
      );

      // Verify next deadline remains unchanged
      const currentNextDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(currentNextDeadlineResult.isOk);
      const currentNextDeadline = currentNextDeadlineResult.asOk.toNumber();

      strictEqual(
        currentNextDeadline,
        initialNextDeadline,
        "Next deadline should not change after increasing capacity"
      );
    });

    it("Next challenge tick correctly calculated with new shorter period", async () => {
      // Get current challenge period (which should already reflect the increased capacity)
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();

      // Get the last tick for which the BSP submitted a proof
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();

      // Calculate next challenge tick
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick
      const currentBlock = (await userApi.query.system.number()).toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlock;

      // Advance blocks until next challenge tick
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.block.seal();
      }

      // Wait for BSP to submit proof and seal one more block
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      await userApi.block.seal();

      // Verify proof was submitted successfully
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted");

      // Now get the new last tick and verify next deadline calculation
      const newLastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          userApi.shConsts.DUMMY_BSP_ID
        );
      assert(newLastTickResult.isOk);
      const newLastTickBspSubmittedProof = newLastTickResult.asOk.toNumber();

      // Get the next deadline tick
      const nextDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(nextDeadlineResult.isOk);
      const nextDeadline = nextDeadlineResult.asOk.toNumber();

      // Next deadline should be last proof tick + challenge period + tolerance
      const challengeTicksTolerance = Number(userApi.consts.proofsDealer.challengeTicksTolerance);
      const expectedNextDeadline =
        newLastTickBspSubmittedProof + challengePeriod + challengeTicksTolerance;

      strictEqual(
        nextDeadline,
        expectedNextDeadline,
        "Next deadline should be calculated using current challenge period"
      );
    });

    it("Challenge period adjusts correctly when capacity is decreased", async () => {
      // Get current capacity
      const currentBspMetadata = await userApi.query.providers.backupStorageProviders(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(currentBspMetadata.isSome);
      const currentCapacity = currentBspMetadata.unwrap().capacity.toBigInt();

      // Calculate new lower capacity (decrease by 1MB)
      const decreaseAmount = BigInt(1024 * 1024); // 1MB
      const newCapacity = currentCapacity - decreaseAmount;

      // Skip blocks until the BSP can change its capacity.
      await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

      // Send transaction to decrease capacity
      await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
      const { extSuccess } = await userApi.block.seal({
        calls: [userApi.tx.providers.changeCapacity(newCapacity)],
        signer: bspKey
      });
      strictEqual(extSuccess, true, "Change capacity transaction should succeed");

      // Assert the capacity change event was emitted
      await userApi.assert.eventPresent("providers", "CapacityChanged");

      // Verify capacity was actually decreased
      const updatedBspMetadata = await userApi.query.providers.backupStorageProviders(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(updatedBspMetadata.isSome);
      strictEqual(
        updatedBspMetadata.unwrap().capacity.toBigInt(),
        newCapacity,
        "BSP capacity should be updated to new lower value"
      );

      // Get current challenge period (which should reflect the decreased capacity)
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();

      // Get the last tick for which the BSP submitted a proof
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();

      // Calculate next challenge tick
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick
      const currentBlock = (await userApi.query.system.number()).toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlock;

      // Advance blocks until next challenge tick
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.block.seal();
      }

      // Wait for BSP to submit proof and seal one more block
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      await userApi.block.seal();

      // Verify proof was submitted successfully
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted");

      // Now get the new last tick and verify next deadline calculation
      const newLastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          userApi.shConsts.DUMMY_BSP_ID
        );
      assert(newLastTickResult.isOk);
      const newLastTickBspSubmittedProof = newLastTickResult.asOk.toNumber();

      // Get the next deadline tick
      const nextDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(nextDeadlineResult.isOk);
      const nextDeadline = nextDeadlineResult.asOk.toNumber();

      // Next deadline should be last proof tick + challenge period + tolerance
      const challengeTicksTolerance = Number(userApi.consts.proofsDealer.challengeTicksTolerance);
      const expectedNextDeadline =
        newLastTickBspSubmittedProof + challengePeriod + challengeTicksTolerance;

      strictEqual(
        nextDeadline,
        expectedNextDeadline,
        "Next deadline should be calculated using current challenge period"
      );
    });

    it("Next challenge tick correctly calculated with new longer period", async () => {
      // Get current challenge period (which should already reflect the decreased capacity)
      const challengePeriodResult = await userApi.call.proofsDealerApi.getChallengePeriod(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();

      // Get the last tick for which the BSP submitted a proof
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();

      // Calculate next challenge tick
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick
      const currentBlock = (await userApi.query.system.number()).toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlock;

      // Advance blocks until next challenge tick
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.block.seal();
      }

      // Wait for BSP to submit proof and seal one more block
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      await userApi.block.seal();

      // Verify proof was submitted successfully
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted");

      // Now get the new last tick and verify next deadline calculation
      const newLastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
          userApi.shConsts.DUMMY_BSP_ID
        );
      assert(newLastTickResult.isOk);
      const newLastTickBspSubmittedProof = newLastTickResult.asOk.toNumber();

      // Get the next deadline tick
      const nextDeadlineResult = await userApi.call.proofsDealerApi.getNextDeadlineTick(
        userApi.shConsts.DUMMY_BSP_ID
      );
      assert(nextDeadlineResult.isOk);
      const nextDeadline = nextDeadlineResult.asOk.toNumber();

      // Next deadline should be last proof tick + challenge period + tolerance
      const challengeTicksTolerance = Number(userApi.consts.proofsDealer.challengeTicksTolerance);
      const expectedNextDeadline =
        newLastTickBspSubmittedProof + challengePeriod + challengeTicksTolerance;

      strictEqual(
        nextDeadline,
        expectedNextDeadline,
        "Next deadline should be calculated using current challenge period"
      );
    });
  }
);

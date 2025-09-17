import assert, { strictEqual } from "node:assert";
import { describeBspNet, type EnrichedBspApi } from "../../../util";

await describeBspNet(
  "BSPNet: BSP Challenge Cycle and Proof Submission",
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
      console.log(userApi.consts.system.version.specName.toString());
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

    it("BSP fails to submit proof and is marked as slashable", async () => {
      // Stop BSP.
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.bsp.containerName);

      // Calculate the next deadline tick for the BSP. That is `ChallengeTicksTolerance`
      // after the next challenge tick for this BSP.
      // We first get the last tick for which the BSP submitted a proof.
      // This time we use the user API as the BSP is paused.
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
      // We get the challenge ticks tolerance.
      const challengeTicksTolerance = Number(userApi.consts.proofsDealer.challengeTicksTolerance);
      // And finally we calculate the next deadline tick.
      const nextDeadlineTick =
        lastTickBspSubmittedProof + challengePeriod + challengeTicksTolerance;

      // Advance to BSP deadline.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = nextDeadlineTick - currentBlockNumber;
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.block.seal();
      }

      // Check for event of slashable BSP.
      await userApi.assert.eventPresent("proofsDealer", "SlashableProvider");
    });

    it(
      "BSP resumes and sends pending proofs",
      {
        skip: "Sending pending proofs is not yet implemented."
      },
      async () => {}
    );

    it(
      "BSP is challenged again and correctly submits proof",
      {
        skip: "Correctly resuming BSP is not yet implemented."
      },
      async () => {
        // Resume BSP.
        await userApi.docker.resumeContainer({
          containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
        });

        // Advance to the next tick the BSP should submit a proof for, that is after the current block.
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
        let nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;
        // Increment challenge periods until we get a number that is greater than the current tick.
        const currentTick = (await userApi.call.proofsDealerApi.getCurrentTick()).toNumber();
        while (currentTick >= nextChallengeTick) {
          // Go one challenge period forward.
          nextChallengeTick += challengePeriod;
        }
        // Finally, advance to the next challenge tick.
        const currentBlock = await userApi.rpc.chain.getBlock();
        const currentBlockNumber = currentBlock.block.header.number.toNumber();
        const blocksToAdvance = nextChallengeTick - currentBlockNumber;

        await userApi.block.skipTo(blocksToAdvance);

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
      }
    );
  }
);

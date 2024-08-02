import "@storagehub/api-augment";
import assert, { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  NODE_INFOS,
  createApiObject,
  type BspNetApi,
  DUMMY_BSP_ID,
  type BspNetConfig,
  runInitialisedBspsNet,
  closeSimpleBspNet,
  sleep,
  pauseBspContainer,
  resumeBspContainer,
  type SealedBlock
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false }
  // { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe.only("BSPNet: BSP Challenge Cycle and Proof Submission", () => {
    let userApi: BspNetApi;
    let bspApi: BspNetApi;

    before(async () => {
      await runInitialisedBspsNet(bspNetConfig);
      userApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
      bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    });

    after(async () => {
      await userApi.disconnect();
      await bspApi.disconnect();
      await closeSimpleBspNet();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
    });

    it("BSP is challenged and correctly submits proof", async () => {
      // Calculate the next challenge tick for the BSP.
      // We first get the last tick for which the BSP submitted a proof.
      const lastTickResult =
        await bspApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult =
        await bspApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
      assert(challengePeriodResult.isOk);
      const challengePeriod = challengePeriodResult.asOk.toNumber();
      // Then we calculate the next challenge tick.
      const nextChallengeTick = lastTickBspSubmittedProof + challengePeriod;

      // Calculate how many blocks to advance until next challenge tick.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const blocksToAdvance = nextChallengeTick - currentBlockNumber;

      // Advance blocksToAdvance blocks.
      for (let i = 0; i < blocksToAdvance; i++) {
        await userApi.sealBlock();
      }

      // Wait for task to execute and seal one more block.
      // In this block, the BSP should have submitted a proof.
      await sleep(500);
      const blockResult = await userApi.sealBlock();

      // Assert for the the event of the proof successfully submitted and verified.
      bspApi.assertEvent("proofsDealer", "ProofAccepted", blockResult.events);
    });

    it("BSP fails to submit proof and is marked as slashable", async () => {
      // Stop BSP.
      await pauseBspContainer(NODE_INFOS.bsp.containerName);

      // Calculate the next deadline tick for the BSP. That is `ChallengeTicksTolerance`
      // after the next challenge tick for this BSP.
      // We first get the last tick for which the BSP submitted a proof.
      // This time we use the user API as the BSP is paused.
      const lastTickResult =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(DUMMY_BSP_ID);
      assert(lastTickResult.isOk);
      const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
      // Then we get the challenge period for the BSP.
      const challengePeriodResult =
        await userApi.call.proofsDealerApi.getChallengePeriod(DUMMY_BSP_ID);
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
      let blockResult: SealedBlock | undefined;
      for (let i = 0; i < blocksToAdvance; i++) {
        blockResult = await userApi.sealBlock();
      }

      // Check for event of slashable BSP.
      userApi.assertEvent("proofsDealer", "SlashableProvider", blockResult?.events);

      // Resume BSP.
      await resumeBspContainer(NODE_INFOS.bsp.containerName);
    });

    it("BSP is challenged and correctly submits proof", async () => {});
  });
}

import { strictEqual } from "node:assert";
import { ShConsts, describeBspNet, type EnrichedBspApi } from "../../../util";

describeBspNet(
  "BSP proofs resubmitted on chain re-org ♻️",
  { initialised: true, networkConfig: "standard" },
  ({ before, createUserApi, createBspApi, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let tickBspSubmittedProofForBeforeReorg: number;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    // This is skipped because it currently fails with timeout for ext inclusion
    it("resubmits a dropped proof Ext", { skip: "Not Impl" }, async () => {
      await userApi.block.seal(); // To make sure we have a finalised head
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, { waitBetweenBlocks: true });

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      await userApi.node.dropTxn({ module: "proofsDealer", method: "submitProof" });

      await userApi.block.seal();
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
    });

    it("Proof re-submitted after longer chain reorg with no Forest changes in between", async () => {
      await userApi.block.seal(); // To make sure we have a finalised head
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID],
        finalised: true
      });

      // Get the last tick for which the BSP submitted a proof, before submitting the new proof.
      const lastTickResult = await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(
        ShConsts.DUMMY_BSP_ID
      );
      tickBspSubmittedProofForBeforeReorg = lastTickResult.asOk.toNumber();

      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // The proof is submitted in this block.
      const { events: eventsFork1 } = await userApi.block.seal({ finaliseBlock: false });

      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork1);

      // Reorg away from the last block by creating a longer fork.
      await userApi.block.reOrgWithLongerChain();

      // Wait for the BSP to catch up to proofs in the new fork.
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // If queried now, the last tick should be the same as before submitting the last proof.
      const lastTickResultAfterReorg =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.DUMMY_BSP_ID);
      const lastTickBspSubmittedProofAfterReorg = lastTickResultAfterReorg.asOk.toNumber();
      strictEqual(
        lastTickBspSubmittedProofAfterReorg,
        tickBspSubmittedProofForBeforeReorg,
        "Last tick should be the same as before submitting the last proof"
      );

      // The proof is submitted in this block.
      const { events: eventsFork2 } = await userApi.block.seal({ finaliseBlock: false });

      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork2);
    });

    it("Proof re-submitted after finality reorg with no Forest changes in between", async () => {
      // Reorg away from the last block by finalising another block from another fork.
      await userApi.block.reOrgWithFinality();

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for BSP node to have imported the finalised block built by the user node.
      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait for the BSP to catch up to proofs in the new fork.
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // If queried now, the last tick should be the same as before submitting the last proof.
      const lastTickResultAfterFinality =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.DUMMY_BSP_ID);
      const lastTickBspSubmittedProofAfterFinality = lastTickResultAfterFinality.asOk.toNumber();
      strictEqual(
        lastTickBspSubmittedProofAfterFinality,
        tickBspSubmittedProofForBeforeReorg,
        "Last tick should be the same as before submitting the last proof"
      );

      // The proof is submitted in this block.
      const { events: eventsFork3 } = await userApi.block.seal({ finaliseBlock: false });

      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork3);
    });
  }
);

async function getNextChallengeHeight(api: EnrichedBspApi, bsp_id?: string): Promise<number> {
  const bsp_id_to_use = bsp_id ?? api.shConsts.DUMMY_BSP_ID;

  const lastTickResult =
    await api.call.proofsDealerApi.getLastTickProviderSubmittedProof(bsp_id_to_use);
  const lastTickBspSubmittedProof = lastTickResult.asOk.toNumber();
  const challengePeriodResult = await api.call.proofsDealerApi.getChallengePeriod(bsp_id_to_use);
  const challengePeriod = challengePeriodResult.asOk.toNumber();

  return lastTickBspSubmittedProof + challengePeriod;
}

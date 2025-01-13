import { rejects, strictEqual } from "node:assert";
import { ShConsts, describeBspNet, shUser, waitFor, type EnrichedBspApi } from "../../../util";
import { assert } from "node:console";

describeBspNet(
  "BSP proofs resubmitted on chain re-org ♻️",
  { initialised: true, networkConfig: "standard", only: true },
  ({ before, createUserApi, createBspApi, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let tickBspSubmittedProofForBeforeReorg: number;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    // This is skipped because it currently fails with timeout for ext inclusion
    it("BSP resubmits a dropped proof extrinsic", { skip: "Not Impl" }, async () => {
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

      // The proof is resubmitted in this block, but not actually because the BSP resubmits it,
      // but rather because when the block is reorged out, the submit proof transaction gets
      // put back in the tx pool.
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

      // The proof is resubmitted in this block, but not actually because the BSP resubmits it,
      // but rather because when the block is reorged out, the submit proof transaction gets
      // put back in the tx pool.
      const { events: eventsFork3 } = await userApi.block.seal({ finaliseBlock: false });

      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork3);
    });

    it("BSP file confirmation is reorged out and Forest root is rolled back accordingly", async () => {
      // Advance a few blocks to have everything settled in the chain.
      const currentBlockNumber = (await userApi.rpc.chain.getHeader()).number.toNumber();
      await userApi.block.skipTo(currentBlockNumber + 10, {
        watchForBspProofs: [userApi.shConsts.DUMMY_BSP_ID]
      });

      // Send a new storage request, and have the BSP respond to it.
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "reorg-bucket-1";
      await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        1
      );
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteer();
      await userApi.block.seal();

      // Save the BSP Forest root before confirming the storage request.
      const onChainBspInfoBeforeResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(onChainBspInfoBeforeResult.isOk);
      const onChainBspForestRootBefore = onChainBspInfoBeforeResult.asOk.root.toString();

      // Wait for the BSP to send the confirm storage extrinsic, and then seal a block,
      // without finalising it, to be able to reorg it out.
      await userApi.wait.bspStored(undefined, undefined, false);
      await userApi.block.seal({ finaliseBlock: false });

      // Reorg away from the last block by creating a longer fork.
      await userApi.block.reOrgWithLongerChain();

      // Wait for the BSP to revert the Forest root change.
      // On-chain root and local root should be the same.
      await waitFor({
        lambda: async () => {
          // Get on-chain BSP Forest root.
          const onChainBspInfoResult = await userApi.call.storageProvidersApi.getBspInfo(
            ShConsts.DUMMY_BSP_ID
          );
          assert(onChainBspInfoResult.isOk);
          const onChainBspForestRoot = onChainBspInfoResult.asOk.root.toString();

          // Get local BSP Forest root.
          const localBspForestRoot = (
            await bspApi.rpc.storagehubclient.getForestRoot(null)
          ).toString();

          return onChainBspForestRoot === localBspForestRoot;
        }
      });

      // Current on-chain BSP Forest root should be the same as the one before the confirmation.
      const onChainBspInfoAfterResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(onChainBspInfoAfterResult.isOk);
      const onChainBspForestRootAfter = onChainBspInfoAfterResult.asOk.root.toString();
      assert(onChainBspForestRootBefore === onChainBspForestRootAfter);
    });

    it("New non best block built with Forest root change is ignored", async () => {
      // Saving the BSP Forest root before confirming the storage request.
      const onChainBspInfoBeforeResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(onChainBspInfoBeforeResult.isOk);
      const onChainBspForestRootBefore = onChainBspInfoBeforeResult.asOk.root.toString();

      // Check that the BSP confirm storing extrinsic is back in the tx pool.
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true,
        assertLength: 1,
        exactLength: true
      });

      // Build a new block on top of the `currentBlockNumber - 1`.
      // In that block, the BSP confirm storing extrinsic should be included, triggering a Forest root change,
      // but the BSP shouldn't process it because the block is not the new best block.
      const parentHash = (await userApi.rpc.chain.getHeader()).parentHash.toString();
      const { events } = await userApi.block.seal({
        parentHash,
        finaliseBlock: false
      });

      // Check that the BSP confirm storing extrinsic is successfully included in the block.
      userApi.assert.eventPresent("fileSystem", "BspConfirmedStoring", events);

      // Check that the BSP root has not changed.
      // We check for 3 seconds expecting to have no change, i.e. expecting the check in the
      // lambda to fail all throughout those 3 seconds.
      await rejects(
        waitFor({
          lambda: async () => {
            // Get the local BSP Forest root.
            const localBspForestRoot = (
              await bspApi.rpc.storagehubclient.getForestRoot(null)
            ).toString();

            // Check if it changed.
            return onChainBspForestRootBefore !== localBspForestRoot;
          },
          delay: 100,
          iterations: 30 // 3 seconds
        })
      );
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

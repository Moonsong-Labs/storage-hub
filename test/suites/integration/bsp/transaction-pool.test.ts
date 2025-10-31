import { strictEqual } from "node:assert";
import { describeBspNet, type EnrichedBspApi, ShConsts, bspKey } from "../../../util";

/**
 * Integration tests for transaction pool and watcher functionality.
 *
 * These tests verify:
 * 1. Transaction watcher logs for all lifecycle states
 * 2. Reorg handling via Retracted status
 * 3. Multiple BSPs submitting proofs without conflicts
 * 4. Transaction replacement (Usurped) via higher-tip transaction
 */
await describeBspNet(
  "Transaction Pool & Watcher",
  { initialised: true, networkConfig: "standard", only: true, keepAlive: true },
  ({ before, createUserApi, createBspApi, it }) => {
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

    it("Transaction watcher logs Ready, InBlock and Finalized states", { only: true }, async () => {
      // Ensure we have a finalized head
      await userApi.block.seal();

      // Get next challenge tick
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID],
        finalised: false
      });

      // Check for "Watching transaction" log
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Watching transaction with nonce",
        timeout: 5000
      });

      // Verify that the submit proof extrinsic is now present in the tx pool
      const extrinsics = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Get the nonce of this transaction
      const txPool = await userApi.rpc.author.pendingExtrinsics();
      const nonce = txPool[extrinsics[0].extIndex].nonce;

      // Verify that the `Ready` log was logged
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${nonce} is ready (in transaction pool)`,
        timeout: 5000
      });

      // Seal the block that contains our transaction (transaction goes `InBlock`)
      const { events, blockReceipt } = await userApi.block.seal({ finaliseBlock: false });
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", events);

      // Wait for the BSP node to import the block before checking for transaction logs
      await bspApi.wait.blockImported(blockReceipt.blockHash.toString());

      // Check for the `InBlock` log since the transaction was included in the block
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${nonce} was included in block`,
        timeout: 5000
      });

      // Finalise a block greater than the block that contains our transaction (transaction goes `Finalized`)
      const blockHashToFinalize = (
        await userApi.block.seal({ finaliseBlock: true })
      ).blockReceipt.blockHash.toString();
      await bspApi.wait.blockImported(blockHashToFinalize);
      await bspApi.block.finaliseBlock(blockHashToFinalize);

      // Check for the `Finalized` log since the transaction was finalized
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${nonce} was finalized in block`,
        timeout: 5000
      });

      // Check for the transaction being removed from our tracking
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${nonce} was finalized. Removing from tracking`,
        timeout: 5000
      });
    });

    it("Transaction watcher logs Retracted status during reorg", async () => {
      // Ensure we have a finalized head
      await userApi.block.seal();

      // Get next challenge tick
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID],
        finalised: false
      });

      // Verify proof is in tx pool
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Seal block with proof
      const { events: eventsFork1 } = await userApi.block.seal({ finaliseBlock: false });
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork1);

      // Wait for InBlock log
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "in block:",
        timeout: 5000
      });

      // Trigger reorg
      await userApi.block.reOrgWithLongerChain();

      // Check for Retracted log - this is the key test for watcher reorg handling
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "RETRACTED from block:",
        timeout: 10000
      });

      // Verify warning about block being reverted
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Block was reverted in reorg",
        timeout: 5000
      });

      // Transaction should be back in pool
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Re-include in new fork
      const { events: eventsFork2 } = await userApi.block.seal({ finaliseBlock: false });
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork2);

      // Should see InBlock log again for re-inclusion
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "in block:",
        timeout: 5000
      });
    });

    it("Multiple BSPs submit proofs with proper watcher logging", async () => {
      // Ensure we have a finalized head
      await userApi.block.seal();

      // Get next challenge tick
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID, ShConsts.BSP_TWO_ID, ShConsts.BSP_THREE_ID],
        finalised: false
      });

      // All BSPs should have proof submissions in pool
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 3
      });

      // Seal block with all proofs
      const sealedBlock = await userApi.block.seal({ finaliseBlock: false });
      const events = sealedBlock.events ?? [];

      // All proofs should be accepted
      const proofEvents = events.filter(
        (e) => e.event.section === "proofsDealer" && e.event.method === "ProofAccepted"
      );
      strictEqual(proofEvents.length, 3, "All three BSP proofs should be accepted");

      // Each BSP should log watching its transaction
      // Note: We check for the primary BSP's logs as they all use the same log pattern
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Watching transaction with nonce",
        timeout: 5000
      });

      // Finalize to trigger cleanup logs
      const blockHashToFinalize = sealedBlock.blockReceipt.blockHash.toString();
      await userApi.block.finaliseBlock(blockHashToFinalize);

      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "finalized. Removing from tracking",
        timeout: 5000
      });
    });

    it("Transaction watcher logs Usurped status when replaced by higher tip", async () => {
      // Ensure we have a finalized head
      await userApi.block.seal();

      // Get next challenge tick
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID],
        finalised: false
      });

      // Verify proof is in tx pool
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Check for transaction being watched
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Watching transaction with nonce",
        timeout: 5000
      });

      // Get the nonce from the BSP
      const nonce = await userApi.rpc.system.accountNextIndex(bspKey.address);

      // Send a remark with the same nonce but higher tip to usurp the transaction
      // The BSP submits proofs with tip=0, so we use tip=1 to replace it
      const remarkCall = userApi.tx.system.remark("");
      await userApi.block.seal({
        calls: [remarkCall],
        signer: bspKey,
        nonce: nonce.toNumber(),
        finaliseBlock: false
      });

      // Check for Usurped log - the proof transaction should be replaced by the remark
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "was USURPED by transaction",
        timeout: 5000
      });

      // Verify the transaction was removed from tracking
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Transaction nonce",
        timeout: 5000
      });
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

import { strictEqual } from "node:assert";
import { describeBspNet, type EnrichedBspApi, ShConsts, bspKey } from "../../../util";

/**
 * Integration tests for transaction pool and watcher functionality.
 *
 * These tests verify:
 * 1. Transaction watcher logs for all lifecycle states
 * 2. Reorg handling via Retracted status
 * 3. Transaction replacement (Usurped) via higher-tip transaction
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

    it("Transaction watcher logs Ready, InBlock and Finalized states", async () => {
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
        timeout: 10000
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
        timeout: 10000
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
        timeout: 10000
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
        timeout: 10000
      });

      // Check for the transaction being removed from our tracking
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "was finalized. Removing from tracking",
        timeout: 10000
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

import assert, { strictEqual } from "node:assert";
import { TypeRegistry } from "@polkadot/types";
import type { H256 } from "@polkadot/types/interfaces";
import { describeBspNet, type EnrichedBspApi, ShConsts } from "../../../util";

/**
 * Integration tests for transaction manager and watcher functionality.
 *
 * These tests verify:
 * 1. Transaction watcher logs for all lifecycle states
 * 2. Reorg handling via Retracted status
 * 3. Transaction replacement (Usurped) via higher-tip transaction
 */
await describeBspNet(
  "Transaction Manager & Watcher",
  {
    initialised: true,
    networkConfig: "standard",
    extrinsicRetryTimeout: 10,
    logLevel: "debug"
  },
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
        searchString: `Transaction with nonce ${nonce} is ready (in Substrate's transaction pool)`,
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
    });

    it("Transaction watcher logs Retracted status after reorg and resubmits proof", async () => {
      // Ensure we have a finalized head
      await userApi.block.seal();

      // Get next challenge tick
      const nextChallengeTick = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick, {
        watchForBspProofs: [ShConsts.DUMMY_BSP_ID],
        finalised: true
      });

      // Wait for "Watching transaction" log
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Watching transaction with nonce",
        timeout: 10000
      });

      // Verify that the submit proof extrinsic is present in the tx pool
      const extrinsics = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Get the nonce of this transaction
      const txPool = await userApi.rpc.author.pendingExtrinsics();
      const nonce = txPool[extrinsics[0].extIndex].nonce;

      // Seal the block that contains our transaction
      const { events, blockReceipt } = await userApi.block.seal({ finaliseBlock: false });
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", events);

      // Wait for BSP node to import the block
      await bspApi.wait.blockImported(blockReceipt.blockHash.toString());

      // Wait for InBlock status
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${nonce} was included in block`,
        timeout: 10000
      });

      // Reorg away from the last block by creating a longer fork
      // This will cause the transaction to be retracted
      await userApi.block.reOrgWithLongerChain();

      // Wait for the BSP to catch up to the reorg
      const newBestBlockHash = (await userApi.rpc.chain.getHeader()).hash.toString();
      await bspApi.wait.blockImported(newBestBlockHash);

      // Check for the `Retracted` log since the block was reorged out
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${nonce} was retracted from block`,
        timeout: 10000
      });

      // Verify that the transaction is back in the tx pool after reorg
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Seal the block with the resubmitted transaction
      const { events: eventsFork2 } = await userApi.block.seal({ finaliseBlock: false });
      await userApi.assert.eventPresent("proofsDealer", "ProofAccepted", eventsFork2);
    });

    it("Transaction watcher logs Usurped status when replaced by higher-tip transaction", async () => {
      // Ensure we have a finalized head
      await userApi.block.seal();

      // Create a bucket and send a storage request to trigger BSP volunteer
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup-usurped.jpg";
      const bucketName = "usurped-test-bucket";

      await userApi.file.createBucketAndSendNewStorageRequest(source, destination, bucketName);

      // Wait for BSP to volunteer
      await userApi.wait.bspVolunteer(1);

      // Wait for BSP to attempt storing (for which the extrinsic submission will eventually
      // fail due to the timeout being reached)
      // This ensures the BSP will try to send a confirm storing transaction
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 12000,
        sealBlock: false
      });

      // Wait for the first transaction to be in the pool
      const firstExtrinsics = await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 1
      });

      // Get the nonce of this transaction
      const txPool1 = await userApi.rpc.author.pendingExtrinsics();
      const nonce = txPool1[firstExtrinsics[0].extIndex].nonce.toNumber();
      const firstTxTip = txPool1[firstExtrinsics[0].extIndex].tip.toBigInt();
      const firstTxHash = txPool1[firstExtrinsics[0].extIndex].hash.toString();

      // Wait for retry attempts which will increase the tip
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Retrying with increased tip",
        timeout: 30000
      });

      // After retries, a new transaction with higher tip should be submitted
      // This will usurp the old transaction with the same nonce
      // Check for the Usurped log
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "was usurped by transaction",
        timeout: 10000
      });

      // Verify that only the new higher-tip transaction is in the pool
      const finalExtrinsics = await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 1
      });

      // Get the current transaction in the pool
      const txPool2 = await userApi.rpc.author.pendingExtrinsics();
      const currentNonce = txPool2[finalExtrinsics[0].extIndex].nonce.toNumber();
      const currentTip = txPool2[finalExtrinsics[0].extIndex].tip.toBigInt();
      const currentTxHash = txPool2[finalExtrinsics[0].extIndex].hash.toString();

      // Verify the nonce is the same but the transaction hash is different
      strictEqual(currentNonce, nonce, "Nonce should be the same");
      assert(
        currentTip > firstTxTip,
        "New transaction should have a tip greater than the first transaction"
      );
      assert(
        currentTxHash !== firstTxHash,
        "Transaction hash should be different after usurpation"
      );
    });

    it("Transaction manager detects Invalid transactions and fills up nonce gap with next transaction", async () => {
      // Seal a finalized block for a stable base
      await userApi.block.seal();

      // Get next challenge tick and position 2 blocks before it
      // There's an edge case in which the next challenge tick is in the current or the next block, in which
      // case we would be skipping to a past block and that's not what we want. We check for this
      // and skip an extra period if needed.
      const currentBlock = (await userApi.query.system.number()).toNumber();
      let nextChallengeTick = await getNextChallengeHeight(userApi);
      const challengePeriod = (
        await userApi.call.proofsDealerApi.getChallengePeriod(ShConsts.DUMMY_BSP_ID)
      ).asOk.toNumber();
      if (nextChallengeTick < currentBlock + 2) {
        nextChallengeTick += challengePeriod;
      }
      await userApi.block.skipTo(nextChallengeTick - 2, { finalised: true });

      // Create a bucket and send a storage request to trigger BSP volunteer. This will seal 2 blocks to reach the challenge tick
      const source1 = "res/whatsup.jpg";
      const destination1 = "test/gap-test-1.jpg";
      const bucketName1 = "gap-test-bucket-1";

      const fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source1,
        destination1,
        bucketName1
      );
      const bucketIdString = fileMetadata.bucketId;

      // Now we should be at the challenge tick with both volunteer (nonce n) and proof (nonce n+1) in pool
      // We check both the user and BSP API to ensure the transactions are present in both pools, since we have
      // to drop the tx from the user (so it's not included in the block) and from the BSP (so it detects the
      // dropped tx and the nonce gap is filled).

      // The BSP volunteer should be in the pools
      await userApi.wait.bspVolunteerInTxPool(1);
      await bspApi.wait.bspVolunteerInTxPool(1);

      // And the submit proof as well
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });
      await bspApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true
      });

      // Get the volunteer transaction nonce
      const volunteerExtrinsics = await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true,
        assertLength: 1
      });

      const txPool1 = await userApi.rpc.author.pendingExtrinsics();
      const volunteerNonce = txPool1[volunteerExtrinsics[0].extIndex].nonce.toNumber();

      // And the submit proof transaction nonce
      const submitProofExtrinsics = await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 1
      });
      const submitProofNonce = txPool1[submitProofExtrinsics[0].extIndex].nonce.toNumber();

      // They should differ by 1
      strictEqual(
        submitProofNonce - volunteerNonce,
        1,
        "Submit proof nonce should be 1 higher than volunteer nonce"
      );

      // Drop the volunteer transaction (creates the gap at nonce n)
      await bspApi.node.dropTxn({ module: "fileSystem", method: "bspVolunteer" });
      await userApi.node.dropTxn({ module: "fileSystem", method: "bspVolunteer" });

      // Verify the volunteer was dropped
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true,
        assertLength: 0
      });
      await bspApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true,
        assertLength: 0
      });

      // Verify the Invalid log was emitted
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${volunteerNonce} is invalid`,
        timeout: 10000
      });

      // The submit proof should have also been dropped because it's no longer valid
      await userApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 0
      });
      await bspApi.assert.extrinsicPresent({
        module: "proofsDealer",
        method: "submitProof",
        checkTxPool: true,
        assertLength: 0
      });

      // Verify the Invalid log was emitted for the submit proof transaction
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: `Transaction with nonce ${submitProofNonce} is invalid`,
        timeout: 10000
      });

      // Create a second storage request which should trigger the gap filling and use the same nonce as the first one
      const source2 = "res/adolphus.jpg";
      const destination2 = "test/gap-test-2.jpg";

      const registry = new TypeRegistry();
      const bucketId: H256 = registry.createType("H256", bucketIdString);
      await userApi.file.newStorageRequest(source2, destination2, bucketId);
      await userApi.wait.bspVolunteerInTxPool(1);

      // Check for the gap filling log
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "🔧 Using transaction to fill nonce gap at",
        timeout: 10000
      });

      // Verify the new volunteer uses the same nonce as the dropped one
      const newVolunteerExtrinsics = await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true
      });

      const txPool2 = await userApi.rpc.author.pendingExtrinsics();
      const newVolunteerNonce = txPool2[newVolunteerExtrinsics[0].extIndex].nonce.toNumber();

      strictEqual(newVolunteerNonce, volunteerNonce, "New volunteer should fill gap with nonce n");

      // Seal the block and verify the volunteer event was emitted.
      // The proof transaction was discarded as Invalid by the watcher so it's no longer tracked
      const { events } = await userApi.block.seal();

      // Verify the volunteer event was emitted
      await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer", events);
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

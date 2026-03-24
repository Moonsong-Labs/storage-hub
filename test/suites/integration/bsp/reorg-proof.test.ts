import assert, { rejects, strictEqual } from "node:assert";
import {
  bspKey,
  describeBspNet,
  type EnrichedBspApi,
  type FileMetadata,
  ShConsts,
  shUser,
  waitFor
} from "../../../util";

//! IMPORTANT!
//! In order to understand better this test, we suggest following this [diagram](https://github.com/Moonsong-Labs/storage-hub/blob/main/resources/reorgsTestFlow.png).

await describeBspNet(
  "BSP proofs resubmitted on chain re-org ♻️",
  { initialised: true, networkConfig: "standard" },
  ({ before, createUserApi, createBspApi, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let tickBspSubmittedProofForBeforeReorg: number;
    let firstFileMetadata: FileMetadata;
    let rootAfterFirstConfirm: string;
    let ignoredBlockHash: string;
    let volunteerBlockHash: string;
    let secondFileMetadata: FileMetadata;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Set tick range to maximum threshold to immediately accept volunteers", async () => {
      const tickRangeToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 1]
        }
      };
      const { extSuccess } = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter)
          )
        ]
      });

      strictEqual(extSuccess, true, "Extrinsic should be successful");
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

      // Check if the proof was already accepted (fatxpool may auto-re-include it
      // in fork blocks — polkadot-sdk#5479). If not yet, seal to include from pool.
      let lastTickAfterReorg = (
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.DUMMY_BSP_ID)
      ).asOk.toNumber();
      if (lastTickAfterReorg < nextChallengeTick) {
        await userApi.block.seal({ finaliseBlock: false });
        lastTickAfterReorg = (
          await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.DUMMY_BSP_ID)
        ).asOk.toNumber();
      }
      assert(
        lastTickAfterReorg >= nextChallengeTick,
        `Proof should have been accepted after reorg (lastTick=${lastTickAfterReorg}, expected >= ${nextChallengeTick})`
      );
      tickBspSubmittedProofForBeforeReorg = lastTickAfterReorg;

      // Flush pending BSP txs (e.g. chargeMultipleUsersPaymentStreams) submitted during the
      // reorg. These occupy nonce slots — without including them, the BSP's on-chain nonce
      // won't advance and subsequent proof submissions will fail.
      await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
      // Seal an unfinalised block so test 4's reOrgWithFinality() has a non-finalized head.
      await userApi.block.seal({ finaliseBlock: false });
    });

    it("Proof re-submitted after finality reorg with no Forest changes in between", async () => {
      // Reorg away from the last block by finalising another block from another fork.
      await userApi.block.reOrgWithFinality();

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for BSP node to have imported the finalised block built by the user node.
      await bspApi.wait.blockImported(finalisedBlockHash.toString());
      await bspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // fatxpool suppresses Retracted events (polkadot-sdk#5479) and may auto-re-include
      // the proof in the new finality block. Seal a block and verify on-chain state.
      await userApi.block.seal({ finaliseBlock: false });

      const lastTickResultAfterFinality =
        await userApi.call.proofsDealerApi.getLastTickProviderSubmittedProof(ShConsts.DUMMY_BSP_ID);
      const lastTickAfterFinality = lastTickResultAfterFinality.asOk.toNumber();
      assert(
        lastTickAfterFinality >= tickBspSubmittedProofForBeforeReorg,
        `Proof should have been re-included after finality reorg. ` +
          `Last proven tick: ${lastTickAfterFinality}, expected >= ${tickBspSubmittedProofForBeforeReorg}`
      );

    });

    it("BSP file confirmation is reorged out and Forest root is rolled back accordingly", async () => {
      // Advance to the next challenge tick to settle proofs before the reorg test.
      const nextChallengeTick2 = await getNextChallengeHeight(userApi);
      await userApi.block.skipTo(nextChallengeTick2, {
        watchForBspProofs: [userApi.shConsts.DUMMY_BSP_ID]
      });

      // Send a new storage request, and have the BSP respond to it.
      const source = "res/adolphus.jpg";
      const destination = "test/adolphus.jpg";
      const bucketName = "reorg-bucket-1";
      firstFileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
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
      await userApi.wait.bspStored({ sealBlock: false });
      await userApi.block.seal({ finaliseBlock: false });

      // Saving Forest root after confirming the storage request.
      const rootAfterConfirmResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(rootAfterConfirmResult.isOk);
      rootAfterFirstConfirm = rootAfterConfirmResult.asOk.root.toString();

      // Reorg away from the last block by creating a longer fork.
      await userApi.block.reOrgWithLongerChain();

      // Wait for BSP local Forest root to match on-chain (consistency after reorg).
      // fatxpool may auto-re-include the bspConfirmStoring in the fork blocks
      // (polkadot-sdk#5479) or return it to the pool — either way, BSP state
      // must be consistent with on-chain.
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

          // Check if they match.
          return onChainBspForestRoot === localBspForestRoot;
        }
      });

    });

    it("New non best block built with Forest root change is ignored", async () => {
      // Saving the BSP Forest root before confirming the storage request.
      const onChainBspInfoBeforeResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(onChainBspInfoBeforeResult.isOk);
      const onChainBspForestRootBefore = onChainBspInfoBeforeResult.asOk.root.toString();

      // Build a non-best block. With fatxpool (polkadot-sdk#5479), the bspConfirmStoring
      // may or may not be in the pool — either way, the BSP should ignore non-best blocks.
      const parentHash = (await userApi.rpc.chain.getHeader()).parentHash.toString();
      const { blockReceipt } = await userApi.block.seal({
        parentHash,
        finaliseBlock: false
      });
      ignoredBlockHash = blockReceipt.blockHash.toString();

      // Check that the BSP root has not changed.
      // We check for 3 seconds expecting to have no change, i.e. expecting the check in the
      // lambda to fail all throughout those 3 seconds.
      await rejects(
        waitFor({
          lambda: async () => {
            const localBspForestRoot = (
              await bspApi.rpc.storagehubclient.getForestRoot(null)
            ).toString();
            return onChainBspForestRootBefore !== localBspForestRoot;
          },
          delay: 100,
          iterations: 30 // 3 seconds
        })
      );
    });

    it("Ignored Forest root change is reorged in and BSP now processes it", async () => {
      // Build a new block on top of the ignored block to trigger a reorg.
      const parentHash = ignoredBlockHash;
      const {
        blockReceipt: { blockHash: reorgBlockHash }
      } = await userApi.block.seal({
        parentHash,
        finaliseBlock: false
      });

      // Check that reorg was processed both by the User and BSP nodes.
      const bestBlockHash = (await userApi.rpc.chain.getHeader()).hash.toString();
      assert(bestBlockHash === reorgBlockHash.toString());
      await userApi.wait.nodeCatchUpToChainTip(bspApi);

      // Check that the file is included in the BSP's local Forest, and that the
      // Forest root is back to being the one including the file.
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            firstFileMetadata.fileKey
          );
          return isFileInForest.isTrue;
        }
      });

      strictEqual(
        rootAfterFirstConfirm,
        (await bspApi.rpc.storagehubclient.getForestRoot(null)).toString()
      );
    });

    it("BSP requests stop storing file", async () => {
      // Build transaction for BSP-Three to stop storing the only file it has.
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        firstFileMetadata.fileKey
      ]);
      await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
      const blockResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspRequestStopStoring(
            firstFileMetadata.fileKey,
            firstFileMetadata.bucketId,
            firstFileMetadata.location,
            firstFileMetadata.owner,
            firstFileMetadata.fingerprint,
            firstFileMetadata.fileSize,
            false,
            inclusionForestProof.toString()
          )
        ],
        signer: bspKey
      });
      assert(blockResult.extSuccess, "Extrinsic was part of the block so its result should exist.");
      assert(
        blockResult.extSuccess === true,
        "Extrinsic to request stop storing should have been successful"
      );

      userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspRequestedToStopStoring,
        await userApi.query.system.events()
      );
    });

    it("Wait for BSP to be able to confirm file deletion, and send new storage request before confirming deletion", async () => {
      // Wait the required time for the BSP to be able to confirm the deletion.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            MinWaitForStopStoring: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      // Skip to a block BEFORE the confirm becomes valid.
      // This ensures the BSP won't auto-send the confirm before we do the reorg test.
      // We need some blocks for: bucket creation, storage request, MSP response + BSP volunteer,
      // BSP confirm storing, and an extra seal, so we use minWaitForStopStoring - 5.
      const blockToAdvanceTo = currentBlockNumber + minWaitForStopStoring - 5;
      await userApi.block.skipTo(blockToAdvanceTo, {
        watchForBspProofs: [userApi.shConsts.DUMMY_BSP_ID]
      });

      // Send a new storage request, and have the BSP respond to it.
      const source = "res/cloud.jpg";
      const destination = "test/cloud.jpg";
      const bucketName = "reorg-bucket-2";
      secondFileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        1
      );

      // Wait for both the BSP and MSP to respond and have their respective transactions in the tx pool.
      await userApi.wait.mspResponseInTxPool();
      await userApi.wait.bspVolunteerInTxPool();
      const {
        blockReceipt: { blockHash }
      } = await userApi.block.seal();
      volunteerBlockHash = blockHash.toString();

      // Check that the BSP was able to correctly volunteer for the storage request.
      const { event: acceptedBspVolunteerEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "AcceptedBspVolunteer"
      );
      const acceptedBspVolunteerDataBlob =
        userApi.events.fileSystem.AcceptedBspVolunteer.is(acceptedBspVolunteerEvent) &&
        acceptedBspVolunteerEvent.data;
      assert(acceptedBspVolunteerDataBlob, "AcceptedBspVolunteer event data does not match type");
      strictEqual(acceptedBspVolunteerDataBlob.bspId.toString(), userApi.shConsts.DUMMY_BSP_ID);
      strictEqual(
        acceptedBspVolunteerDataBlob.fingerprint.toString(),
        secondFileMetadata.fingerprint.toString()
      );

      // Wait for the BSP to send the confirm storage extrinsic, and then seal a block,
      // without finalising it, to be able to reorg it out.
      await userApi.wait.bspStored({ sealBlock: false });
      await userApi.block.seal({ finaliseBlock: false });

      // Check that the BSP confirm storing extrinsic is successfully included in the block.
      await userApi.assert.eventPresent("fileSystem", "BspConfirmedStoring");

      // Wait for confirmation line in docker logs.
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "New local Forest root matches the one in the block for BSP"
      });
      // Check that the file is included in the BSP's local Forest.
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            secondFileMetadata.fileKey
          );
          return isFileInForest.isTrue;
        }
      });
    });

    it("File deletion confirmation is included in a forked non-best block", async () => {
      // Create and save a valid inclusion Forest proof for confirming the file deletion, at this point,
      // with this root, with the latest file confirmation included in the forest, so that we can use it
      // in the fork that will be reorged in, also after the file storage confirmation.
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        firstFileMetadata.fileKey
      ]);
      const inclusionForestProofAfterConfirmingStoring = inclusionForestProof.toString();

      // Save the BSP Forest root before doing the reorg.
      const onChainBspInfoBeforeResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(onChainBspInfoBeforeResult.isOk);
      const onChainBspForestRootBefore = onChainBspInfoBeforeResult.asOk.root.toString();

      // We seal another non-final block on top of the block with the file storage confirmation.
      // IMPORTANT!
      // This is because somehow the User node drops the confirm deletion transaction from the
      // tx pool if we try to include it in a non-best block, right after having built the block
      // with the file storage confirmation.
      await userApi.block.seal({ finaliseBlock: false });

      // Seal a finalised block on top of the block with the volunteer transaction.
      // This essentially reorgs out the file storage confirmation as far as the User
      // node is concerned, but not for the BSP.
      // Finality is a node-local concept, so this block is not finalised for the BSP
      // node, which still sees the file storage confirmation as valid and the one in
      // the longest chain.
      await userApi.block.seal({
        parentHash: volunteerBlockHash,
        finaliseBlock: true
      });

      // Wait for the reorged out storage confirmation transaction to be in the tx pool again.
      // Then build a block with it, on top of the above finalised block.
      // Still, this won't trigger a reorg in the BSP. This block will be at the same height
      // of the current best block for the BSP.
      await userApi.wait.bspStored({ sealBlock: false });
      await userApi.block.seal({ finaliseBlock: false });

      // Seal another block with the confirm deletion transaction.
      // This is finally the block that triggers the reorg in the BSP.
      const { events } = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspConfirmStopStoring(
            firstFileMetadata.fileKey,
            inclusionForestProofAfterConfirmingStoring
          )
        ],
        signer: bspKey,
        finaliseBlock: false
      });

      // Check for the confirm stopped storing event.
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring", events);

      // Check that the recently added file is still in the local Forest for the BSP.
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            secondFileMetadata.fileKey
          );
          return isFileInForest.isTrue;
        }
      });

      // Check that the deleted file is not in the local Forest for the BSP.
      // We check for 3 seconds expecting to not find it, i.e. expecting the check in the
      // lambda to fail all throughout those 3 seconds.
      await rejects(
        waitFor({
          lambda: async () => {
            const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
              null,
              firstFileMetadata.fileKey
            );
            return isFileInForest.isTrue;
          },
          delay: 100,
          iterations: 30 // 3 seconds
        })
      );

      // Check that the new local Forest root matches the one on-chain.
      const localBspForestRoot = (await bspApi.rpc.storagehubclient.getForestRoot(null)).toString();
      const onChainBspInfoAfterResult = await userApi.call.storageProvidersApi.getBspInfo(
        ShConsts.DUMMY_BSP_ID
      );
      assert(onChainBspInfoAfterResult.isOk);
      const onChainBspForestRootAfter = await onChainBspInfoAfterResult.asOk.root.toString();
      strictEqual(onChainBspForestRootAfter, localBspForestRoot);

      // Check that the local Forest root is different thant the one before the reorg.
      assert(localBspForestRoot !== onChainBspForestRootBefore);
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

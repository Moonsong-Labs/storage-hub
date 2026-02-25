import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { EventRecord, SignedBlock } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

/**
 * Checks if a BSP confirm extrinsic failed with ForestProofVerificationFailed.
 *
 * This function examines the events from a sealed block to determine if the
 * `bspConfirmStoring` extrinsic failed due to a `ForestProofVerificationFailed`
 * error from the proofsDealer pallet.
 *
 * @param api - The API instance to decode errors
 * @param events - Array of events from a sealed block
 * @param blockData - Block data containing extrinsics array
 * @returns true if BSP confirm extrinsic failed with ForestProofVerificationFailed
 */
const hasBspConfirmProofError = (
  api: ApiPromise,
  events: EventRecord[],
  blockData: SignedBlock
): boolean => {
  for (const { event, phase } of events) {
    if (api.events.system.ExtrinsicFailed.is(event)) {
      if (!phase.isApplyExtrinsic) continue;
      const extIndex = phase.asApplyExtrinsic.toNumber();
      const extrinsic = blockData.block.extrinsics[extIndex];
      if (!extrinsic) continue;

      const { method, section } = extrinsic.method;
      const isBspConfirmExtrinsic = section === "fileSystem" && method === "bspConfirmStoring";

      if (!isBspConfirmExtrinsic) continue;

      const errorEventData = event.data;
      if (errorEventData.dispatchError.isModule) {
        try {
          const decoded = api.registry.findMetaError(errorEventData.dispatchError.asModule);
          if (
            decoded.section === "proofsDealer" &&
            (decoded.method === "ForestProofVerificationFailed" ||
              decoded.method === "FailedToApplyDelta")
          ) {
            return true;
          }
        } catch (_) {
          // Error decoding failed, skip
        }
      }
    }
  }
  return false;
};

/**
 * BSP PROOF ERROR RETRY INTEGRATION TEST
 *
 * This test validates that BSPs correctly retry confirming storage when a
 * `ForestProofVerificationFailed` error occurs due to a concurrent deletion
 * modifying the BSP's forest root.
 *
 * Test Flow:
 * 1. File 1 storage request fulfilled (BSP confirms storing)
 * 2. Generate forest proof for File 1 deletion
 * 3. Issue storage request for File 2
 * 4. Seal MSP response and BSP volunteer
 * 5. Wait for BSP confirm extrinsic in pool (proves file transfer completed)
 * 6. Submit requestDeleteFile to pool (without sealing), pause BSP, drop BSP confirm, seal block
 * 7. Resume BSP, wait for BSP to re-submit confirm
 * 8. Submit deleteFiles extrinsic with HIGH TIP
 * 9. Drop BSP's confirm tx so it gets re-added with lower priority
 * 10. Wait for BSP to re-add confirm extrinsic
 * 11. Seal block → delete executes first (higher tip), BSP confirm fails
 * 12. Wait for BSP forest root to sync, then wait for BSP retry
 * 13. Seal → confirm succeeds
 */
await describeMspNet(
  "BSP retries storage confirmation after proof error",
  { networkConfig: "standard", extrinsicRetryTimeout: 5 },
  ({ before, createMsp1Api, it, createUserApi, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
      bspApi = await createBspApi();
    });

    it("BSP retries confirm storing when proof error occurs due to concurrent deletion", async () => {
      const bspId = userApi.shConsts.DUMMY_BSP_ID;
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);

      // Phase 1: Create first storage request (fulfilled immediately)
      const file1Result = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/whatsup.jpg",
            destination: "test/proof-retry-file1.jpg",
            bucketIdOrName: "bsp-proof-retry-bucket",
            replicationTarget: 1 // Immediately fulfilled (MSP + 1 BSP)
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApis: [bspApi],
        mspApi
      });

      const file1Key = file1Result.fileKeys[0];
      const bucketId = file1Result.bucketIds[0];

      // Phase 2: Generate forest proof for File 1 deletion
      const bspInclusionProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        file1Key
      ]);

      // Phase 3: Issue storage request for File 2
      const { file_metadata: file2Metadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        "res/adolphus.jpg",
        "test/proof-retry-file2.jpg",
        ownerHex,
        bucketId
      );

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file2Metadata.location,
            file2Metadata.fingerprint,
            file2Metadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Basic: null }
          )
        ],
        signer: shUser
      });

      const { event: newStorageRequestEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequestV2"
      );
      assert(
        userApi.events.fileSystem.NewStorageRequestV2.is(newStorageRequestEvent),
        "Event should be NewStorageRequestV2"
      );
      const file2Key = newStorageRequestEvent.data.fileKey.toString();

      // Wait for MSP response in tx pool
      await userApi.wait.mspResponseInTxPool();
      // Wait for BSP to volunteer in tx pool
      await userApi.wait.bspVolunteerInTxPool();

      // Seal block with MSP acceptance and BSP volunteer
      await userApi.block.seal();

      // Phase 4: Wait for BSP to queue confirm storing (proves file transfer completed)
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 15000,
        sealBlock: false
      });

      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true
      });

      // Phase 5: Submit user delete request for File 1 (without sealing)
      const fileOperationIntention = {
        fileKey: file1Key,
        operation: { Delete: null }
      };

      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const intentionPayload = intentionCodec.toU8a();
      const rawSignature = shUser.sign(intentionPayload);
      const userSignature = userApi.createType("MultiSignature", {
        Sr25519: rawSignature
      });

      // Submit requestDeleteFile to pool WITHOUT sealing
      const requestDeleteTx = userApi.tx.fileSystem.requestDeleteFile(
        fileOperationIntention,
        userSignature,
        bucketId,
        file1Result.locations[0],
        file1Result.fileSizes[0],
        file1Result.fingerprints[0]
      );
      await requestDeleteTx.signAndSend(shUser);

      // Verify requestDeleteFile is in pool
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "requestDeleteFile",
        checkTxPool: true,
        timeout: 5000
      });

      // Phase 6: Pause BSP container and drop its confirm tx
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.bsp.containerName);

      // Drop BSP's confirm tx from pool via userApi (BSP's RPC is unavailable while paused)
      await userApi.node.dropTxn({
        module: "fileSystem",
        method: "bspConfirmStoring"
      });

      // Seal block with only requestDeleteFile (BSP confirm is gone)
      await userApi.block.seal();

      // Verify FileDeletionRequested event
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequested");

      // Phase 7: Seal a block and resume BSP container
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
      });

      // Phase 8: Wait for BSP to re-submit confirm storing for file 2
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 15000,
        sealBlock: false
      });

      // Phase 9: Build and submit deleteFiles extrinsic with HIGH TIP
      // Using the forest proof generated earlier
      // Delete from BSP's forest by passing BSP ID as second parameter

      // Build deletion request structure
      const deletionRequest = {
        fileOwner: shUser.address,
        signedIntention: fileOperationIntention,
        signature: userSignature,
        bucketId,
        location: file1Result.locations[0],
        size: file1Result.fileSizes[0],
        fingerprint: file1Result.fingerprints[0]
      };

      // Submit deleteFiles extrinsic to pool with HIGH TIP for priority
      // The tip ensures deleteFiles executes BEFORE BSP's confirm in the same block
      // Pass BSP ID as second parameter to delete from BSP's forest
      const deleteFilesTx = userApi.tx.fileSystem.deleteFiles(
        [deletionRequest],
        bspId, // BSP ID for BSP deletion (null would be bucket deletion)
        bspInclusionProof.toString()
      );

      // Sign and send with high tip to ensure priority over BSP's transaction
      await deleteFilesTx.signAndSend(shUser, {
        tip: 1_000_000_000_000n
      });

      // Verify deleteFiles is in pool
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "deleteFiles",
        checkTxPool: true,
        timeout: 5000
      });

      await userApi.node.dropTxn({
        module: "fileSystem",
        method: "bspConfirmStoring"
      });

      // Phase 11: Wait for BSP to re-add confirm extrinsic
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 15000,
        sealBlock: false
      });

      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspConfirmStoring",
        checkTxPool: true
      });

      // Phase 12: Seal block and verify proof error

      // Seal block with file deletion and BSP confirmation transactions
      const blockResult = await userApi.block.seal();
      const blockData = await userApi.rpc.chain.getBlock(blockResult.blockReceipt.blockHash);

      // Check for ForestProofVerificationFailed error
      const hasProofError = hasBspConfirmProofError(
        userApi,
        blockResult.events || [],
        blockData as SignedBlock
      );
      assert(hasProofError, "Expected ForestProofVerificationFailed error for BSP confirm");

      // Verify BspFileDeletionsCompleted event (delete succeeded)
      await userApi.assert.eventPresent(
        "fileSystem",
        "BspFileDeletionsCompleted",
        blockResult.events
      );

      // Verify NO BspConfirmedStoring event (confirm failed)
      const bspConfirmEvents = (blockResult.events || []).filter(({ event }) =>
        userApi.events.fileSystem.BspConfirmedStoring.is(event)
      );
      assert.equal(bspConfirmEvents.length, 0, "BSP confirm should have failed");

      // Wait for BSP's local forest root to sync with on-chain BSP root
      // This is necessary because the deleteFiles changed the BSP's root on-chain,
      // and the BSP needs to update its local forest storage before it can generate
      // a valid proof for the retry
      await waitFor({
        lambda: async () => {
          const bspLocalRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
          const onChainBsp = (await userApi.query.providers.backupStorageProviders(bspId)).unwrap();
          return bspLocalRoot.unwrap().toString() === onChainBsp.root.toString();
        },
        delay: 100,
        iterations: 50
      });

      // Phase 13: Wait for BSP to retry
      // The BSP client will retry by requeueing the confirm storing request
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 15000,
        sealBlock: false
      });

      // Phase 14: Seal retry and verify success
      const retryResult = await userApi.block.seal();

      // Verify BspConfirmedStoring event
      const confirmedEvent = (retryResult.events || []).find(({ event }) =>
        userApi.events.fileSystem.BspConfirmedStoring.is(event)
      );
      assert(confirmedEvent, "BSP should have successfully confirmed on retry");

      // Verify File 2 is in BSP's forest storage
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file2Key);
          return isFileInForest.isTrue;
        },
        delay: 100,
        iterations: 50
      });

      const isFile2InForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file2Key);
      assert(isFile2InForest.isTrue, "File 2 should be in BSP's forest after retry");

      // Verify File 1 is NOT in the BSP's forest anymore (it was deleted)
      const isFile1InForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file1Key);
      assert(isFile1InForest.isFalse, "File 1 should NOT be in BSP's forest (was deleted)");
    });
  }
);

import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { EventRecord, SignedBlock } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

/**
 * Checks if an MSP accept extrinsic failed with ForestProofVerificationFailed.
 *
 * This function examines the events from a sealed block to determine if the
 * `mspRespondStorageRequestsMultipleBuckets` extrinsic failed due to
 * a `ForestProofVerificationFailed` error from the proofsDealer pallet.
 *
 * @param api - The API instance to decode errors
 * @param events - Array of events from a sealed block
 * @param blockData - Block data containing extrinsics array
 * @returns true if MSP accept extrinsic failed with ForestProofVerificationFailed
 */
const hasMspAcceptProofError = (
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
      const isMspAcceptExtrinsic =
        section === "fileSystem" && method === "mspRespondStorageRequestsMultipleBuckets";

      if (!isMspAcceptExtrinsic) continue;

      const errorEventData = event.data;
      if (errorEventData.dispatchError.isModule) {
        try {
          const decoded = api.registry.findMetaError(errorEventData.dispatchError.asModule);
          if (
            decoded.section === "proofsDealer" &&
            decoded.method === "ForestProofVerificationFailed"
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
 * MSP PROOF ERROR RETRY INTEGRATION TEST
 *
 * This test validates that MSPs correctly retry accepting storage requests when
 * a `ForestProofVerificationFailed` error occurs due to a concurrent deletion
 * modifying the bucket's forest root.
 *
 * Test Flow:
 * 1. File 1 storage request fulfilled (replicationTarget=1)
 * 2. Generate forest proof for File 1 deletion (while MSP is running)
 * 3. Pause MSP container
 * 4. Issue storage request for File 2 (same bucket)
 * 5. User requests deletion of File 1
 * 6. Build and submit deleteFiles extrinsic to pool (using pre-generated proof)
 * 7. Resume MSP → MSP builds accept proof against OLD root
 * 8. Wait for MSP accept extrinsic in pool
 * 9. Seal block → delete changes root, MSP accept fails with ForestProofVerificationFailed
 * 10. Wait for MSP retry
 * 11. Seal → accept succeeds
 */
await describeMspNet(
  "MSP retries storage request acceptance after proof error",
  { networkConfig: "standard" },
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

    it("MSP retries storage request when proof error occurs due to concurrent deletion", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);

      // Phase 1: Create first storage request (fulfilled immediately)
      const file1Result = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/proof-retry-file1.txt",
            bucketIdOrName: "test-proof-retry-bucket",
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

      // Phase 2: Generate forest proof for File 1 deletion BEFORE pausing MSP
      // This proof will be used later to build the deleteFiles extrinsic
      const bucketInclusionProof = await mspApi.rpc.storagehubclient.generateForestProof(bucketId, [
        file1Key
      ]);

      // Phase 3: Pause MSP container before second storage request
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Phase 4: Issue second storage request (while MSP is paused)
      const { file_metadata: file2Metadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        "res/adolphus.jpg",
        "test/proof-retry-file2.txt",
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
            mspId,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Basic: null }
          )
        ],
        signer: shUser
      });

      const { event: newStorageRequestEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );

      assert(
        userApi.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent),
        "Event should be NewStorageRequest"
      );
      const file2Key = newStorageRequestEvent.data.fileKey.toString();

      // Phase 5: Submit user delete request for File 1
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

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketId,
            file1Result.locations[0],
            file1Result.fileSizes[0],
            file1Result.fingerprints[0]
          )
        ],
        signer: shUser
      });

      // Verify FileDeletionRequested event
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequested");

      // Phase 6: Build and submit deleteFiles extrinsic (without sealing)
      // Using the forest proof generated earlier (before MSP was paused)

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
      // The tip ensures deleteFiles executes BEFORE MSP's response in the same block
      const deleteFilesTx = userApi.tx.fileSystem.deleteFiles(
        [deletionRequest],
        null, // null = bucket deletion (not BSP)
        bucketInclusionProof.toString()
      );

      // Sign and send with high tip to ensure priority over MSP's transaction
      // MSP transactions typically have no tip, so even a small tip gives us priority
      await deleteFilesTx.signAndSend(shUser, {
        nonce: -1,
        tip: 1_000_000_000_000n
      });

      // Verify deleteFiles is in pool
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "deleteFiles",
        checkTxPool: true,
        timeout: 5000
      });

      // Phase 7: Resume MSP container
      // MSP will catch up and process file 2 storage request
      // Importantly, it builds its proof against the CURRENT bucket root (before delete executes)
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Phase 8: Wait for MSP accept extrinsic in pool
      await userApi.wait.mspResponseInTxPool(1);

      // Verify both extrinsics are in pool before sealing
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "deleteFiles",
        checkTxPool: true
      });
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets",
        checkTxPool: true
      });

      // Phase 9: Seal block and verify proof error
      const blockResult = await userApi.block.seal();
      const blockData = await userApi.rpc.chain.getBlock(blockResult.blockReceipt.blockHash);

      // Check for ForestProofVerificationFailed error
      const hasProofError = hasMspAcceptProofError(
        userApi,
        blockResult.events || [],
        blockData as SignedBlock
      );
      assert(hasProofError, "Expected ForestProofVerificationFailed error for MSP accept");

      // Verify BucketFileDeletionsCompleted event (delete succeeded)
      await userApi.assert.eventPresent(
        "fileSystem",
        "BucketFileDeletionsCompleted",
        blockResult.events
      );

      // Verify NO MspAcceptedStorageRequest event (accept failed)
      const mspAcceptEvents = (blockResult.events || []).filter(({ event }) =>
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(event)
      );
      assert.equal(mspAcceptEvents.length, 0, "MSP accept should have failed");

      // Wait for MSP's local forest root to sync with on-chain bucket root
      // This is necessary because the deleteFiles changed the bucket root on-chain,
      // and the MSP needs to update its local forest storage before it can generate
      // a valid proof for the retry
      await waitFor({
        lambda: async () => {
          const mspLocalRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);
          const onChainBucket = (await userApi.query.providers.buckets(bucketId)).unwrap();
          return mspLocalRoot.unwrap().toString() === onChainBucket.root.toString();
        },
        delay: 100,
        iterations: 50
      });

      // Seal block to trigger retry
      await userApi.block.seal();

      // Phase 10: Wait for MSP retry
      // The MSP client marks the file as FileKeyStatus::Failed on proof error
      // and will retry by requeueing the storage request
      await userApi.wait.mspResponseInTxPool(1);

      // Phase 11: Seal and verify success
      const retryResult = await userApi.block.seal();

      // Verify MspAcceptedStorageRequest or StorageRequestFulfilled event
      const acceptedEvent = (retryResult.events || []).find(
        ({ event }) =>
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(event) ||
          userApi.events.fileSystem.StorageRequestFulfilled.is(event)
      );
      assert(acceptedEvent, "MSP should have successfully accepted on retry");

      // Verify file is in MSP's forest storage
      await waitFor({
        lambda: async () => {
          const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(
            bucketId,
            file2Key
          );
          return isFileInForest.isTrue;
        }
      });

      const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(bucketId, file2Key);
      assert(isFileInForest.isTrue, "File 2 should be in MSP's forest after retry");
    });
  }
);

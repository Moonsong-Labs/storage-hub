import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  describeMspNet,
  type EnrichedBspApi,
  extractProofFromForestProof,
  shUser,
  waitFor,
  waitForLog
} from "../../../util";

/**
 * MSP Storage Request Accept Reorg Integration Test
 *
 * This test validates that when an MSP accept transaction is reorged out and
 * the bucket root changes due to file deletion, the resubmitted accept fails
 * with a proof verification error, and the MSP eventually retries successfully.
 *
 * Test flow:
 * 1. Issue first storage request, wait for MSP + BSP to fulfill it (FINALIZED)
 * 1b. Generate proof for file 1 deletion (before MSP accepts file 2)
 * 2. Issue second storage request (FINALIZED) - save block hash as forkPoint
 * 3. Wait for MSP to accept second storage request (UNFINALIZED)
 * 4. Finalize a block from forkPoint → reorgs out MSP accept (tx goes back to pool)
 * 5. Submit requestDeleteFile and deleteFiles with HIGH TIP (MSP accept has no tip)
 * 6. Seal block → deletions execute first (tip priority), MSP accept fails with proof error
 * 7. Wait for MSP to detect failure, clear Processing status, and submit fresh accept
 *
 * Key verification:
 * - MSP's stale proof (based on old bucket root) fails with ForestProofVerificationFailed
 * - MSP eventually retries with a fresh proof and succeeds
 */
await describeMspNet(
  "MSP storage request acceptance resubmitted on chain re-org",
  {
    networkConfig: "standard",
    // Short retry timeout (5 seconds) so MSP retries quickly after failures
    extrinsicRetryTimeout: 5
  },
  ({ before, createMsp1Api, createUserApi, createBspApi, it }) => {
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

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("MSP retries accept after reorg with proof error due to file deletion", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);

      // ===== STEP 1: Create first storage request =====
      const file1Result = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/reorg-test-file-1.txt",
            bucketIdOrName: "reorg-test-bucket",
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

      // Generate forest proof for file 1 deletion NOW (before MSP accepts file 2)
      // This ensures the proof only contains file 1, matching on-chain state after reorg
      const bucketInclusionProof = await mspApi.rpc.storagehubclient.generateForestProof(bucketId, [
        file1Key
      ]);

      // ===== STEP 2: Issue second storage request (FINALIZED) =====
      const { file_metadata: file2Metadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        "res/adolphus.jpg",
        "test/reorg-test-file-2.txt",
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
        signer: shUser,
        finaliseBlock: true
      });

      const { event: storageRequestEvent2 } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );
      assert(
        userApi.events.fileSystem.NewStorageRequest.is(storageRequestEvent2),
        "Event should be NewStorageRequest"
      );
      const file2Key = storageRequestEvent2.data.fileKey.toString();

      // Save fork point - the block AFTER second storage request is finalized
      const forkPointHash = await userApi.rpc.chain.getFinalizedHead();

      // ===== STEP 3: Wait for MSP to accept (UNFINALIZED) =====
      // Wait for MSP to receive second file
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(file2Key)).isFileFound
      });

      // Wait for MSP's accept response in tx pool
      await userApi.wait.mspResponseInTxPool();

      // Seal block WITHOUT finalizing - includes MSP accept for second file
      await userApi.block.seal({
        finaliseBlock: false
      });

      // Verify MSP accepted storage request event
      await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

      // Verify that the ProcessMspRespondStoringRequest event handler reached the end of the process.
      // This means that `msp_upload_file` task has finished processing the storage request, and in
      // processing this block, the Blockchain Service would have removed the file key from Processing status.
      await waitForLog({
        containerName: "storage-hub-sh-msp-1",
        searchString: "Processed ProcessMspRespondStoringRequest for MSP",
        timeout: 10000
      });

      // ===== STEP 4: Trigger reorg by finalizing a block from forkPoint =====
      // This reorgs out the MSP accept block
      // The MSP's accept transaction goes back to the tx pool

      // First seal an empty block from forkPoint to create a competing chain
      await userApi.block.seal({
        parentHash: forkPointHash.toString(),
        finaliseBlock: true // Finalizing triggers the reorg
      });

      // Wait for MSP to process the reorg
      // The MSP's accept transaction should be back in the tx pool
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets",
        timeout: 10000
      });

      // ===== STEP 5: Submit both deletion txs with HIGH TIP =====
      // Both requestDeleteFile and deleteFiles will execute before MSP accept (no tip)
      // This ensures: requestDeleteFile → deleteFiles → MSP accept (fails)

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

      // Get current nonce for proper sequencing
      const currentNonce = (await userApi.rpc.system.accountNextIndex(shUser.address)).toNumber();

      // Submit requestDeleteFile with very high tip (executes first)
      const requestDeleteTx = userApi.tx.fileSystem.requestDeleteFile(
        fileOperationIntention,
        userSignature,
        bucketId,
        file1Result.locations[0],
        file1Result.fileSizes[0],
        file1Result.fingerprints[0]
      );
      await requestDeleteTx.signAndSend(shUser, {
        nonce: currentNonce,
        tip: 2_000_000_000_000n // Very high tip
      });

      // Submit deleteFiles with high tip (executes second)
      const deletionRequest = {
        fileOwner: shUser.address,
        signedIntention: fileOperationIntention,
        signature: userSignature,
        bucketId,
        location: file1Result.locations[0],
        size: file1Result.fileSizes[0],
        fingerprint: file1Result.fingerprints[0]
      };

      const decodedBucketInclusionProof = extractProofFromForestProof(
        userApi,
        bucketInclusionProof
      );
      const deleteFilesTx = userApi.tx.fileSystem.deleteFiles(
        [deletionRequest],
        null,
        decodedBucketInclusionProof
      );
      await deleteFilesTx.signAndSend(shUser, {
        nonce: currentNonce + 1,
        tip: 1_000_000_000_000n // High tip (less than requestDelete)
      });

      // Verify both deletion txs are in pool
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "requestDeleteFile",
        checkTxPool: true,
        timeout: 5000
      });
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "deleteFiles",
        checkTxPool: true
      });
      // MSP accept should still be in pool from after reorg
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets",
        checkTxPool: true
      });

      // ===== STEP 6: Seal block - all 3 txs execute with proper ordering =====
      const blockResult = await userApi.block.seal();

      // Verify deletion requested
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequested", blockResult.events);

      // Verify deletion succeeded
      await userApi.assert.eventPresent(
        "fileSystem",
        "BucketFileDeletionsCompleted",
        blockResult.events
      );

      // Verify MSP accept failed with ForestProofVerificationFailed
      // It is expected since the first storage request acceptance extrinsic watcher
      // should have attempted to resubmit the transaction after a reorg.
      const mspAcceptFailedEvent = (blockResult.events || []).find(
        ({ event }) =>
          userApi.events.system.ExtrinsicFailed.is(event) &&
          event.data.dispatchError.isModule &&
          userApi.registry.findMetaError(event.data.dispatchError.asModule).method ===
            "ForestProofVerificationFailed"
      );
      assert(
        mspAcceptFailedEvent,
        "MSP accept should have failed with ForestProofVerificationFailed"
      );

      // Verify file 2's storage request is still pending and verify
      // runtime api is returning the correct pending storage requests
      // which the blockchain service will check to retry accepting the storage request.
      const pendingStorageRequests =
        await userApi.call.fileSystemApi.pendingStorageRequestsByMsp(mspId);
      const pendingArray = Array.from(pendingStorageRequests);
      const file2IsPending = pendingArray.some(([fileKey]) => fileKey.toHex() === file2Key);
      assert(
        file2IsPending,
        "File 2 storage request should still be pending after MSP accept failed"
      );

      // ===== STEP 7: Wait for MSP to retry and succeed =====
      // The MSP's original msp accept transaction (in step 3) removed the file key from Processing status so the MSP should retry the storage request since it is found
      // again in the pending storage requests and is not in the file key statuses.

      // Wait for MSP to submit a fresh accept to the tx pool
      await userApi.wait.mspResponseInTxPool();

      // Seal block to include MSP accept
      await userApi.block.seal();

      // Verify MSP accepted storage request event
      await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

      // Verify file 2 is in MSP's forest
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
      assert(isFileInForest.isTrue, "File 2 should be in MSP's forest after successful retry");
    });
  }
);

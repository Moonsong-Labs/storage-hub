import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { describeMspNet, type EnrichedBspApi, shUser, waitFor } from "../../../util";

/**
 * MSP Storage Request Accept Reorg Integration Test
 *
 * This test validates that MSPs correctly handle reorgs when accepting storage requests.
 * When an MSP's accept transaction is included in a block that gets reorged out,
 * the MSP should detect this and automatically retry accepting the storage request.
 *
 * Test flow:
 * 1. Issue a storage request to the MSP
 * 2. Wait for MSP to accept (transaction included in unfinalized block)
 * 3. Reorg the block out using reOrgWithLongerChain()
 * 4. Verify storage request is still pending on-chain
 * 5. Wait for MSP to automatically retry and accept again
 */
await describeMspNet(
  "MSP storage request acceptance resubmitted on chain re-org",
  { networkConfig: "standard" },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("MSP resubmits accept after longer chain reorg", async () => {
      // Ensure we have a finalized head to start from
      await userApi.block.seal();

      const source = "res/adolphus.jpg";
      const destination = "test/reorg-test-file.jpg";
      const bucketName = "reorg-test-bucket";

      // Step 1: Create bucket and issue storage request
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      assert(newBucketEventData, "NewBucket event data doesn't match expected type");

      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHex,
        newBucketEventData.bucketId
      );

      // Issue storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventData.bucketId,
            destination,
            userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
            userApi.shConsts.TEST_ARTEFACTS[source].size,
            userApi.shConsts.DUMMY_MSP_ID,
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

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent) &&
        newStorageRequestEvent.data;

      assert(
        newStorageRequestDataBlob,
        "NewStorageRequest event data does not match expected type"
      );

      const fileKey = newStorageRequestDataBlob.fileKey.toString();

      // Step 2: Wait for MSP to receive the file and queue accept response
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(newStorageRequestDataBlob.fileKey))
            .isFileFound
      });

      // Wait for MSP's accept response to be in the transaction pool
      await userApi.wait.mspResponseInTxPool();

      // Step 3: Seal block WITHOUT finalizing - this includes the MSP accept
      const { events: eventsFork1 } = await userApi.block.seal({
        finaliseBlock: false
      });

      // Verify the MSP accepted the storage request in this block
      assert(eventsFork1 && eventsFork1.length > 0, "No events emitted in sealed block");
      const acceptEventFork1 = eventsFork1.find(
        (e) =>
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) ||
          userApi.events.fileSystem.StorageRequestFulfilled.is(e.event)
      );
      assert(
        acceptEventFork1,
        "MSP should have accepted storage request or fulfilled it in the first fork"
      );

      // Step 4: Reorg the block out by creating a longer chain
      await userApi.block.reOrgWithLongerChain();

      // Finalize the reorg in the MSP node as well
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await mspApi.wait.blockImported(finalisedBlockHash.toString());
      await mspApi.block.finaliseBlock(finalisedBlockHash.toString());

      // Step 5: Verify storage request is still pending (accept was reorged out)
      // Query pending storage requests - the file key should still be there
      const pendingRequests = await userApi.call.fileSystemApi.pendingStorageRequestsByMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );

      const pendingRequest = [...pendingRequests.entries()].find(
        ([key]) => key.toString() === fileKey
      );
      assert(pendingRequest, "Storage request should still be pending after reorg");

      // Verify the MSP confirmation boolean is false (not confirmed after reorg)
      const [, metadata] = pendingRequest;
      assert(metadata.msp.isSome, "Storage request should have an MSP assigned");
      const [_mspId, isConfirmed] = metadata.msp.unwrap();
      assert(
        isConfirmed.isFalse,
        "MSP confirmation should be false after reorg (accept was reverted)"
      );

      // Step 6: Wait for transaction to be re-included in the tx pool after the reorg
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets",
        timeout: 30000
      });

      // Step 7: Seal the block and verify MSP successfully re-accepts
      const { events: eventsAfterReorg } = await userApi.block.seal();

      // Check for accept event after reorg
      assert(
        eventsAfterReorg && eventsAfterReorg.length > 0,
        "No events emitted in sealed block after reorg"
      );
      const acceptEventAfterReorg = eventsAfterReorg.find(
        (e) =>
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) ||
          userApi.events.fileSystem.StorageRequestFulfilled.is(e.event)
      );

      assert(
        acceptEventAfterReorg,
        "MSP should have re-accepted storage request after reorg was detected"
      );

      // Verify the file key matches
      let acceptedFileKey: string | null = null;
      if (userApi.events.fileSystem.MspAcceptedStorageRequest.is(acceptEventAfterReorg.event)) {
        acceptedFileKey = acceptEventAfterReorg.event.data.fileKey.toString();
      } else if (
        userApi.events.fileSystem.StorageRequestFulfilled.is(acceptEventAfterReorg.event)
      ) {
        acceptedFileKey = acceptEventAfterReorg.event.data.fileKey.toString();
      }

      strictEqual(acceptedFileKey, fileKey, "Re-accepted file key should match the original");
    });
  }
);

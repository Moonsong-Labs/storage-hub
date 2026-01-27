import assert, { strictEqual } from "node:assert";
import {
  bspKey,
  describeBspNet,
  type EnrichedBspApi,
  type FileMetadata,
  waitFor
} from "../../../util";

await describeBspNet(
  "BSPNet: Stop Storing File RPC",
  { initialised: false },
  ({ before, createBspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    const source = "res/whatsup.jpg";
    const destination = "test/stop-storing-rpc.jpg";
    const bucketName = "stop-storing-rpc-test";

    let fileMetadata: FileMetadata;

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

    it("BSP can stop storing a file via RPC and penalty is charged", async () => {
      // ================ Step 1: Upload a file and wait for BSP to store it ================
      fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1 // Request only 1 BSP to store
      );

      // Wait for the BSP volunteer transaction to be in the transaction pool
      await userApi.wait.bspVolunteerInTxPool(1);

      // Seal the block with the BSP volunteer transaction
      await userApi.block.seal();

      // Wait for the BSP to confirm storing the file
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 30000,
        sealBlock: true
      });

      // Wait for BSP to update its local Forest with the file
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileMetadata.fileKey
          );
          return isFileInForest.isTrue;
        }
      });

      // Verify file is in BSP's file storage
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey)).isFileFound
      });

      // ================ Step 2: Record initial state for penalty verification ================
      const bspAddress = bspKey.address.toString();
      // Treasury account is AccountId32 with all zeros in the parachain runtime
      const treasuryAddress = "5C4hrfjw9DjXZTzV3MwzrrAr9P1MJhSrvWGWqi1eSuyUpnhM"; // All zeros AccountId32

      const bspBalanceBefore = (await userApi.query.system.account(bspAddress)).data.free;
      const treasuryBalanceBefore = (await userApi.query.system.account(treasuryAddress)).data.free;

      // Set the BspStopStoringFilePenalty to 100
      const bspStopStoringFilePenalty = {
        RuntimeConfig: {
          BspStopStoringFilePenalty: [null, 100n]
        }
      };
      await userApi.block.seal({
        calls: [userApi.tx.sudo.sudo(userApi.tx.parameters.setParameter(bspStopStoringFilePenalty))]
      });

      // Get the BspStopStoringFilePenalty value
      const penalty = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: { BspStopStoringFilePenalty: null }
        })
      ).unwrap().asRuntimeConfig.asBspStopStoringFilePenalty;
      assert(penalty.toString() === "100", "BspStopStoringFilePenalty should be 100");

      // ================ Step 3: Call the stopStoringFile RPC ================
      const rpcResult = await bspApi.rpc.storagehubclient.bspStopStoringFile(fileMetadata.fileKey);
      strictEqual(rpcResult.isSuccess, true, "RPC should return Success");

      // ================ Step 4: Wait for bspRequestStopStoring in tx pool and seal ================
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspRequestStopStoring",
        timeout: 30000
      });

      // Seal the block containing the request
      await userApi.block.seal();

      // Assert the BspRequestedToStopStoring event is present
      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");

      // ================ Step 5: Verify penalty was charged ================
      const bspBalanceAfterRequest = (await userApi.query.system.account(bspAddress)).data.free;
      const treasuryBalanceAfterRequest = (await userApi.query.system.account(treasuryAddress)).data
        .free;

      // BSP balance should have decreased by at least the penalty (may also pay tx fees)
      const bspBalanceDecrease = bspBalanceBefore.sub(bspBalanceAfterRequest);
      assert(
        bspBalanceDecrease.gte(penalty),
        `BSP balance should decrease by at least the penalty. Decrease: ${bspBalanceDecrease.toString()}, Penalty: ${penalty.toString()}`
      );

      // Treasury balance should have increased by the penalty
      const treasuryBalanceIncrease = treasuryBalanceAfterRequest.sub(treasuryBalanceBefore);
      assert(
        treasuryBalanceIncrease.eq(penalty),
        `Treasury balance should increase by the penalty. Increase: ${treasuryBalanceIncrease.toString()}, Penalty: ${penalty.toString()}`
      );

      // ================ Step 6: Skip MinWaitForStopStoring + 1 blocks ================
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: { MinWaitForStopStoring: null }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      const confirmBlock = currentBlockNumber + minWaitForStopStoring + 1;

      // Skip to the block where BSP can confirm stop storing
      await userApi.block.skipTo(confirmBlock);

      // ================ Step 7: Wait for bspConfirmStopStoring in tx pool and seal ================
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspConfirmStopStoring",
        timeout: 30000
      });

      // Seal the block containing the confirmation
      await userApi.block.seal();

      // Assert the BspConfirmStoppedStoring event is present
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      // ================ Step 8: Verify file is no longer in forest storage ================
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileMetadata.fileKey
          );
          return isFileInForest.isFalse;
        }
      });

      // ================ Step 9: Finalize block and verify file is no longer in file storage ================
      // Seal and finalize a block to trigger file storage cleanup
      const { blockReceipt } = await userApi.block.seal({ finaliseBlock: true });
      const finalisedBlockHash = blockReceipt.blockHash.toString();

      // Wait for BSP to have imported the finalised block
      await bspApi.wait.blockImported(finalisedBlockHash);
      await bspApi.block.finaliseBlock(finalisedBlockHash);

      // Wait for file to be removed from file storage after finalization
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey))
            .isFileNotFound
      });
    });

    it("BSP can stop storing multiple files via RPC in the same block", async () => {
      // This test verifies that the BSP handles multiple stop storing
      // requests on the same block correctly, avoiding invalid proofs.

      const source1 = "res/whatsup.jpg";
      const source2 = "res/cloud.jpg";
      const destination1 = "test/stop-storing-multi-1.jpg";
      const destination2 = "test/stop-storing-multi-2.jpg";
      const bucketName1 = "stop-storing-multi-test-1";
      const bucketName2 = "stop-storing-multi-test-2";

      // ================ Step 1: Upload two files and wait for BSP to store both ================
      // Upload first file
      const fileMetadata1 = await userApi.file.createBucketAndSendNewStorageRequest(
        source1,
        destination1,
        bucketName1,
        null,
        null,
        null,
        1
      );

      await userApi.wait.bspVolunteerInTxPool(1);
      await userApi.block.seal();

      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 30000,
        sealBlock: true
      });

      // Upload second file
      const fileMetadata2 = await userApi.file.createBucketAndSendNewStorageRequest(
        source2,
        destination2,
        bucketName2,
        null,
        null,
        null,
        1
      );

      await userApi.wait.bspVolunteerInTxPool(1);
      await userApi.block.seal();

      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 30000,
        sealBlock: true
      });

      // Wait for BSP to update its local Forest with both files
      await waitFor({
        lambda: async () => {
          const isFile1InForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileMetadata1.fileKey
          );
          const isFile2InForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileMetadata2.fileKey
          );
          return isFile1InForest.isTrue && isFile2InForest.isTrue;
        }
      });

      // ================ Step 2: Call stopStoringFile RPC for both files in quick succession ================
      const rpcResult1 = await bspApi.rpc.storagehubclient.bspStopStoringFile(
        fileMetadata1.fileKey
      );
      const rpcResult2 = await bspApi.rpc.storagehubclient.bspStopStoringFile(
        fileMetadata2.fileKey
      );

      strictEqual(rpcResult1.isSuccess, true, "First RPC should return Success");
      strictEqual(rpcResult2.isSuccess, true, "Second RPC should return Success");

      // ================ Step 3: Wait for both bspRequestStopStoring transactions ================
      // Due to the forest lock, the requests are processed sequentially.
      // Wait for first request
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspRequestStopStoring",
        timeout: 30000
      });
      await userApi.block.seal();
      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");
      const firstRequestBlock = await userApi.rpc.chain.getBlock();
      const firstRequestBlockNumber = firstRequestBlock.block.header.number.toNumber();

      // Wait for second request
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspRequestStopStoring",
        timeout: 30000
      });
      await userApi.block.seal();
      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");
      const secondRequestBlock = await userApi.rpc.chain.getBlock();
      const secondRequestBlockNumber = secondRequestBlock.block.header.number.toNumber();
      assert(
        secondRequestBlockNumber === firstRequestBlockNumber + 1,
        "Second request should be in the next block after the first request"
      );

      // ================ Step 4: Skip to confirm block of the first request ================
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: { MinWaitForStopStoring: null }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();

      // Skip to the block where BSP can confirm the first stop storing request
      const confirmBlock = firstRequestBlockNumber + minWaitForStopStoring + 1;
      await userApi.block.skipTo(confirmBlock);

      // ================ Step 5: Wait for the first bspConfirmStopStoring transaction ================
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspConfirmStopStoring",
        timeout: 30000
      });
      await userApi.block.seal();
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      // ================ Step 6: Wait for the second bspConfirmStopStoring transaction ================
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "bspConfirmStopStoring",
        timeout: 30000
      });
      await userApi.block.seal();
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      // ================ Step 7: Verify both files are no longer in forest storage ================
      await waitFor({
        lambda: async () => {
          const isFile1InForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileMetadata1.fileKey
          );
          const isFile2InForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileMetadata2.fileKey
          );
          return isFile1InForest.isFalse && isFile2InForest.isFalse;
        }
      });

      // ================ Step 7: Finalize and verify both files removed from file storage ================
      const { blockReceipt } = await userApi.block.seal({ finaliseBlock: true });
      const finalisedBlockHash = blockReceipt.blockHash.toString();

      await bspApi.wait.blockImported(finalisedBlockHash);
      await bspApi.block.finaliseBlock(finalisedBlockHash);

      await waitFor({
        lambda: async () => {
          const file1Status = await bspApi.rpc.storagehubclient.isFileInFileStorage(
            fileMetadata1.fileKey
          );
          const file2Status = await bspApi.rpc.storagehubclient.isFileInFileStorage(
            fileMetadata2.fileKey
          );
          return file1Status.isFileNotFound && file2Status.isFileNotFound;
        }
      });
    });
  }
);

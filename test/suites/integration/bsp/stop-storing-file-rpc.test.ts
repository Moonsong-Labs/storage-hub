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
  { initialised: false, only: true, keepAlive: true, networkConfig: "standard" },
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
      const rpcResult = await bspApi.rpc.storagehubclient.stopStoringFile(fileMetadata.fileKey);
      console.log(`stopStoringFile RPC result: ${JSON.stringify(rpcResult.toHuman())}`);
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
        treasuryBalanceIncrease === penalty,
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

      console.log(
        `Current block: ${currentBlockNumber}, MinWaitForStopStoring: ${minWaitForStopStoring}, skipping to block: ${confirmBlock}`
      );

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

      console.log("File is no longer in BSP forest storage");

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

      console.log("File is no longer in BSP file storage after finalization");
      console.log("Test completed successfully!");
    });
  }
);

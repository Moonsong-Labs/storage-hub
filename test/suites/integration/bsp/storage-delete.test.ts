import assert, { strictEqual } from "node:assert";
import { bspKey, describeBspNet, type EnrichedBspApi, waitFor } from "../../../util";

await describeBspNet(
  "BSPNet: Stop storing file and other BSPs taking the relay",
  { initialised: "multi", networkConfig: "standard" },
  ({ before, createUserApi, after, it, createApi, createBspApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;

    before(async () => {
      const launchResponse = await getLaunchResponse();
      assert(
        launchResponse && "bspTwoRpcPort" in launchResponse && "bspThreeRpcPort" in launchResponse,
        "BSPNet failed to initialise with required ports"
      );
      userApi = await createUserApi();
      bspApi = await createBspApi();
      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse.bspThreeRpcPort}`);
    });

    after(async () => {
      await bspTwoApi.disconnect();
      await bspThreeApi.disconnect();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);
      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("bsp one stop storing and bsp three volunteer", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "tastytest";

      // Pause BSP-Three.
      await userApi.docker.pauseContainer("sh-bsp-three");

      const { fileKey, location, fingerprint, fileSize, bucketId } =
        await userApi.file.createBucketAndSendNewStorageRequest(source, destination, bucketName);

      // Wait for the two BSP to volunteer and the MSP to accept the storage request
      await userApi.wait.bspVolunteerInTxPool(2);
      await userApi.wait.mspResponseInTxPool(1);

      // Seal the block with the MSP acceptance and BSP volunteer
      await userApi.block.seal();

      // Wait for the BSPs to confirm storing
      await userApi.wait.bspStored({ expectedExts: 2 });

      // Let the storage request expire
      // This keeps the file stored by BSPs but removes the active storage request
      const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
      const expiresAt = storageRequest.unwrap().expiresAt.toNumber();

      // Seal blocks until the expiration tick
      while ((await userApi.call.proofsDealerApi.getCurrentTick()).toNumber() < expiresAt) {
        await userApi.block.seal();
      }

      await userApi.assert.eventPresent("fileSystem", "StorageRequestExpired");

      // Unpause BSP Three
      await userApi.docker.resumeContainer({
        containerName: "sh-bsp-three"
      });
      await userApi.wait.nodeCatchUpToChainTip(bspThreeApi);

      // TODO: create an RPC to automatically execute everything below
      // TODO: everything below should be removed and replaced with other testing logic

      // Wait for BSP to update its local Forest root before starting to generate the inclusion proofs
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          return isFileInForest.isTrue;
        }
      });

      // Add the file key to the exclude list
      bspApi.rpc.storagehubclient.addToExcludeList(fileKey, "file");

      // Request to stop storing a file with Dummy BSP
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);
      await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
      await userApi.block.seal({
        calls: [
          bspApi.tx.fileSystem.bspRequestStopStoring(
            fileKey,
            bucketId,
            location,
            userApi.shConsts.NODE_INFOS.user.AddressId,
            fingerprint,
            fileSize,
            false,
            inclusionForestProof.toString()
          )
        ],
        signer: bspKey
      });

      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");

      // When requested to stop storing a file we should also receive an event new storage request
      // to replace the bsp leaving
      // TODO: add rpc to add user to blacklisted users to skip any storage request from them
      // TODO: add rpc to add bucket to blacklisted buckets to skip any storage request from them
      await userApi.assert.eventPresent("fileSystem", "NewStorageRequestV2");

      // Wait for the right moment to confirm stop storing
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
      const cooldown = currentBlockNumber + minWaitForStopStoring;

      // Waint until the BSP is allowed to confirm the stop storing
      await userApi.block.skipTo(cooldown);

      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.bspConfirmStopStoring(fileKey, inclusionForestProof)],
        signer: bspKey
      });

      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");
    });
  }
);

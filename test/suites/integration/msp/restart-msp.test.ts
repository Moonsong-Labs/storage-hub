import assert, { strictEqual } from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  getContainerPeerId,
  restartContainer,
  waitFor
} from "../../../util";

await describeMspNet(
  "Single MSP accepts storage request, stops, restarts and can still work with persistent bucket stored",
  {
    initialised: false,
    networkConfig: [{ noisy: false, rocksdb: true }],
    only: true
  },
  ({ before, after, createUserApi, createBspApi, createMsp1Api, createApi, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let newMspApi: EnrichedBspApi;
    let newBspApi: EnrichedBspApi;

    const bucketName = "restart-msp-bucket";
    let file1: FileMetadata;
    let file2: FileMetadata;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMspApi = await createMsp1Api();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;
    });

    after(async () => {
      await newMspApi.disconnect();
      await newBspApi.disconnect();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("MSP and BSP accept first storage request", async () => {
      const source = "res/smile.jpg";
      const destination = "test/smile.jpg";

      file1 = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName
      );

      // MSP completes file storage locally
      await mspApi.wait.fileStorageComplete(file1.fileKey);

      // Ensure acceptance and BSP volunteer -> stored
      await userApi.wait.mspResponseInTxPool();
      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspVolunteer(1); // seals block with volunteer
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      // Assert MSP and BSP forest contain the file
      await waitFor({
        lambda: async () => {
          const inMspForest = await mspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file1.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inBspForest = await bspApi.rpc.storagehubclient.isFileInForest(null, file1.fileKey);
          return inBspForest.isTrue;
        }
      });
    });

    it("MSP shuts down, restarts and bucket is still accessible", async () => {
      // Capture MSP bucket root before restart
      const rootBefore = await mspApi.rpc.storagehubclient.getForestRoot(file1.bucketId);
      const mspBucketRootBefore = rootBefore.toString();

      // Restart MSP container (preserves writable layer including RocksDB path)
      await mspApi.disconnect();
      await restartContainer({ containerName: userApi.shConsts.NODE_INFOS.msp1.containerName });

      // Wait for MSP RPC to respond
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`, true);

      // Wait for MSP to be idle again
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 20000,
        tail: 50
      });

      // Reconnect MSP API
      newMspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Ensure MSP catches up
      await userApi.wait.nodeCatchUpToChainTip(newMspApi);

      // Persisted: bucket root unchanged and file still present in MSP forest
      await waitFor({
        lambda: async () => {
          const rootAfter = await newMspApi.rpc.storagehubclient.getForestRoot(file1.bucketId);
          return rootAfter.toString() === mspBucketRootBefore;
        }
      });

      // Persisted: file still present in MSP forest
      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file1.fileKey
          );
          return inMspForest.isTrue;
        }
      });
    });

    it("BSP shuts down, restarts and its forest storage is still accessible", async () => {
      // Capture BSP forest root before restart
      const rootBefore = await bspApi.rpc.storagehubclient.getForestRoot(file1.bucketId);
      const bspForestRootBefore = rootBefore.toString();

      // Restart BSP container
      await bspApi.disconnect();
      await restartContainer({ containerName: userApi.shConsts.NODE_INFOS.bsp.containerName });

      // Wait for BSP RPC to respond
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`, true);

      // Wait for BSP to be idle again
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName,
        searchString: "💤 Idle",
        timeout: 20000,
        tail: 50
      });

      // Reconnect BSP API
      newBspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`);

      // Ensure BSP catches up
      await userApi.wait.nodeCatchUpToChainTip(newBspApi);

      // Persisted: forest root unchanged and file still present in BSP forest
      await waitFor({
        lambda: async () => {
          const rootAfter = await newBspApi.rpc.storagehubclient.getForestRoot(file1.bucketId);
          return rootAfter.toString() === bspForestRootBefore;
        }
      });

      // Persisted: file still present in BSP forest
      await waitFor({
        lambda: async () => {
          const inBspForest = await newBspApi.rpc.storagehubclient.isFileInForest(
            null,
            file1.fileKey
          );
          return inBspForest.isTrue;
        }
      });
    });

    it("MSP and BSP can accept subsequent storage request for another file in the same bucket", async () => {
      const source = "res/cloud.jpg";
      const destination = "test/cloud-after-restart.jpg";

      const bucketIdH256 = userApi.createType("H256", file1.bucketId);
      file2 = await userApi.file.newStorageRequest(source, destination, bucketIdH256);

      // MSP completes file storage locally
      await newMspApi.wait.fileStorageComplete(file2.fileKey);

      // Ensure acceptance and BSP volunteer -> stored
      await userApi.wait.mspResponseInTxPool();
      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      // Assert presence in both forests
      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file2.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inBspForest = await newBspApi.rpc.storagehubclient.isFileInForest(
            null,
            file2.fileKey
          );
          return inBspForest.isTrue;
        }
      });
    });

    it("Both files are accessible in MSP and BSP forests", async () => {
      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file1.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file1.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inMspForest = await newMspApi.rpc.storagehubclient.isFileInForest(
            file1.bucketId,
            file2.fileKey
          );
          return inMspForest.isTrue;
        }
      });

      await waitFor({
        lambda: async () => {
          const inBspForest = await newBspApi.rpc.storagehubclient.isFileInForest(
            null,
            file2.fileKey
          );
          return inBspForest.isTrue;
        }
      });
    });
  }
);

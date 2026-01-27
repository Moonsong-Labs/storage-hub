import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  getContainerPeerId,
  restartContainer,
  waitFor
} from "../../../util";

await describeMspNet(
  "MSP recovers files missing in its file storage from BSPs that have them",
  { initialised: "multi", networkConfig: [{ noisy: false, rocksdb: true }], only: true },
  ({
    before,
    after,
    createMsp1Api,
    createBspApi,
    it,
    createUserApi,
    createApi,
    getLaunchResponse
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;

    const bucketName = "recover-files-from-bsps-bucket";
    const files: FileMetadata[] = [];

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMspApi = await createMsp1Api();
      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      const launchResponse = await getLaunchResponse();
      assert(launchResponse, "Network launch response not available");
      assert(
        "bspTwoRpcPort" in launchResponse,
        "BSP two RPC port not available in launch response"
      );
      assert(
        "bspThreeRpcPort" in launchResponse,
        "BSP three RPC port not available in launch response"
      );

      bspTwoApi = await createApi(`ws://127.0.0.1:${launchResponse.bspTwoRpcPort}`);
      bspThreeApi = await createApi(`ws://127.0.0.1:${launchResponse.bspThreeRpcPort}`);

      // Ensure extra BSPs are caught up (avoid flakiness when immediately sending requests).
      await userApi.wait.nodeCatchUpToChainTip(bspTwoApi);
      await userApi.wait.nodeCatchUpToChainTip(bspThreeApi);
    });

    after(async () => {
      await bspApi?.disconnect();
      await bspTwoApi?.disconnect();
      await bspThreeApi?.disconnect();
      await mspApi?.disconnect();
    });

    it("Create multiple storage requests accepted by MSP and confirmed by BSPs", async () => {
      const result = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/recover-files-from-bsps-1.jpg",
            bucketIdOrName: bucketName,
            replicationTarget: 3
          },
          {
            source: "res/cloud.jpg",
            destination: "test/recover-files-from-bsps-2.jpg",
            bucketIdOrName: bucketName,
            replicationTarget: 3
          },
          {
            source: "res/whatsup.jpg",
            destination: "test/recover-files-from-bsps-3.jpg",
            bucketIdOrName: bucketName,
            replicationTarget: 3
          }
        ],
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        bspApis: [bspApi, bspTwoApi, bspThreeApi],
        mspApi
      });

      files.push(
        ...result.fileKeys.map((fileKey, idx) => ({
          fileKey,
          bucketId: result.bucketIds[idx],
          location: result.locations[idx],
          owner: userApi.accounts.shUser.address,
          fingerprint: result.fingerprints[idx],
          fileSize: result.fileSizes[idx]
        }))
      );
    });

    it("Restart MSP and check that forest roots are validated and all files are present in the forest", async () => {
      // Restart MSP container (preserves writable layer including RocksDB path).
      await mspApi.disconnect();
      await restartContainer({ containerName: userApi.shConsts.NODE_INFOS.msp1.containerName });

      // Wait for MSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`, true);

      // Wait for MSP to be idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 20_000,
        tail: 200
      });

      // Reconnect MSP API.
      mspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Ensure MSP catches up.
      await userApi.wait.nodeCatchUpToChainTip(mspApi);

      // Ensure the new bucket-check task reports the expected success message.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "OK: all 3 forest files are present and complete in file storage",
        timeout: 60_000,
        tail: 2_000
      });

      // Also confirm all files are present in the MSP forest.
      for (const f of files) {
        await waitFor({
          lambda: async () =>
            (await mspApi.rpc.storagehubclient.isFileInForest(f.bucketId, f.fileKey)).isTrue
        });
      }
    });

    it("Delete files from MSP file storage", async () => {
      const fileKeys = files.map((f) => f.fileKey);

      // Remove all 3 files from MSP file storage via RPC (so recovery must fetch them).
      await mspApi.rpc.storagehubclient.removeFilesFromFileStorage(fileKeys);

      // Wait until all files are actually deleted locally.
      for (const fileKey of fileKeys) {
        await mspApi.wait.fileDeletionFromFileStorage(fileKey);
      }
    });

    it("Restart MSP and verify files are recovered", async () => {
      // Restart MSP to trigger file-storage recovery logic.
      await mspApi.disconnect();
      await restartContainer({ containerName: userApi.shConsts.NODE_INFOS.msp1.containerName });

      // Wait for MSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`, true);

      // Wait for MSP to be idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 20_000,
        tail: 200
      });

      // Reconnect MSP API.
      mspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);
      await userApi.wait.nodeCatchUpToChainTip(mspApi);

      // MSP should recover all three files.
      for (const f of files) {
        await mspApi.wait.fileStorageComplete(f.fileKey);
      }

      // Assert that the task logs the recovery summary and that it reports 3 recovered.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "recovery finished: recovered=3, failed=0, panicked=0, total=3",
        timeout: 120_000,
        tail: 5_000
      });

      // Ensure BSP two and BSP three were contacted and rejected download requests.
      // These BSPs are not started with --trusted-msps, so they should reject MSP download requests.
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-two",
        searchString: "Received unexpected download request",
        timeout: 120_000,
        tail: 5_000
      });
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-three",
        searchString: "Received unexpected download request",
        timeout: 120_000,
        tail: 5_000
      });
    });

    it("Pause BSP one and create storage request with replication target 2", async () => {
      // TODO: Pause BSP one and create a storage request with replication target 2, wait for MSP to accept and BSPs to confirm.
    });

    it("Delete new file from MSP file storage", async () => {
      // TODO: Delete a new file from MSP file storage.
    });

    it("Restart MSP and verify new file cannot be recovered from unfriendly BSPs", async () => {
      // TODO: Restart MSP and check logs for the recovery process.
      // TODO: Check that MSP does not recover the file from BSP two and BSP three.
    });
  }
);

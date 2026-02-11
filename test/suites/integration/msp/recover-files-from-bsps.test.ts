import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  getContainerPeerId,
  waitFor
} from "../../../util";

await describeMspNet(
  "MSP recovers files missing in its file storage from BSPs that have them",
  {
    initialised: "multi",
    indexer: true,
    logLevel: "file-transfer-service=debug", // This test requires debug logging for file-transfer-service to see the unexpected download requests.
    networkConfig: [{ noisy: false, rocksdb: true }],
    only: true
  },
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
    let newFile: FileMetadata | undefined;

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

    it("Restart BSP 1 so that it registers MSP 1 as a trusted MSP", async () => {
      await bspApi.disconnect();
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
      });

      // Wait for BSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`);

      // Ensure BSP registers MSP 1 as a trusted MSP.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName,
        searchString: "Configured 1 trusted MSP(s) with 1 resolved peer ID(s)",
        timeout: 10_000,
        tail: 5_000
      });

      // Ensure BSP is idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName,
        searchString: "💤 Idle",
        timeout: 10_000,
        tail: 5_000
      });

      // Reconnect BSP API.
      bspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`);
      await userApi.wait.nodeCatchUpToChainTip(bspApi);
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
      // Stop MSP container (preserves writable layer including RocksDB path).
      await mspApi.disconnect();
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Advance blocks to ensure MSP triggers initial sync when it restarts
      await userApi.block.skip(20);

      // Restart MSP container
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Wait for MSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Wait for MSP to be idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 10_000,
        tail: 200
      });

      // Reconnect MSP API.
      mspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Ensure MSP catches up.
      await userApi.wait.nodeCatchUpToChainTip(mspApi);

      // Build another block to trigger block import notification, after coming out of sync mode.
      await userApi.block.seal();

      // Ensure the new bucket-check task reports the expected success message.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "OK: all 3 forest files are present and complete in file storage",
        timeout: 10_000,
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
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Advance blocks to ensure MSP triggers initial sync when it restarts
      await userApi.block.skip(20);

      // Restart MSP container
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Wait for MSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Wait for MSP to be idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 10_000,
        tail: 200
      });

      // Reconnect MSP API.
      mspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);
      await userApi.wait.nodeCatchUpToChainTip(mspApi);

      // Build another block to trigger block import notification, after coming out of sync mode.
      await userApi.block.seal();

      // MSP should recover all three files.
      for (const f of files) {
        await mspApi.wait.fileStorageComplete(f.fileKey);
      }

      // Assert that the task logs the recovery summary and that it reports 3 recovered.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "recovery finished: recovered=3, failed=0, panicked=0, total=3",
        timeout: 10_000,
        tail: 5_000
      });

      // Ensure BSP two and BSP three were contacted and rejected download requests.
      // These BSPs are not started with --trusted-msps, so they should reject MSP download requests.
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-two",
        searchString: "Received unexpected download request",
        timeout: 10_000,
        tail: 5_000
      });
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-three",
        searchString: "Received unexpected download request",
        timeout: 10_000,
        tail: 5_000
      });
    });

    it("Pause BSP one and create storage request with replication target 2", async () => {
      // Pause BSP one so it cannot volunteer/store the new file.
      await bspApi.disconnect();
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.bsp.containerName);

      const result = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/adolphus.jpg",
            destination: "test/recover-files-from-bsps-new-rt2.jpg",
            bucketIdOrName: bucketName,
            replicationTarget: 2
          }
        ],
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        // Only BSP two and BSP three are expected to store/confirm.
        bspApis: [bspTwoApi, bspThreeApi],
        mspApi
      });

      newFile = {
        fileKey: result.fileKeys[0],
        bucketId: result.bucketIds[0],
        location: result.locations[0],
        owner: userApi.accounts.shUser.address,
        fingerprint: result.fingerprints[0],
        fileSize: result.fileSizes[0]
      };

      // Extra safety: ensure the two expected BSPs and MSP stored the file locally.
      await bspTwoApi.wait.fileStorageComplete(newFile.fileKey);
      await bspThreeApi.wait.fileStorageComplete(newFile.fileKey);
      await mspApi.wait.fileStorageComplete(newFile.fileKey);
    });

    it("Restart BSP one", async () => {
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName
      });

      // Wait for BSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`);

      // Ensure BSP is idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.bsp.containerName,
        searchString: "💤 Idle",
        timeout: 10_000,
        tail: 5_000
      });

      // Reconnect BSP API.
      bspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.bsp.port}`);
      await userApi.wait.nodeCatchUpToChainTip(bspApi);
    });

    it("Delete new file from MSP file storage", async () => {
      assert(newFile, "New file metadata not set");

      await mspApi.rpc.storagehubclient.removeFilesFromFileStorage([newFile.fileKey]);
      await mspApi.wait.fileDeletionFromFileStorage(newFile.fileKey);
    });

    it("Restart MSP and verify new file cannot be recovered from unfriendly BSPs", async () => {
      assert(newFile, "New file metadata not set");

      // Restart MSP to trigger file-storage recovery logic.
      await mspApi.disconnect();
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Advance blocks to ensure MSP triggers initial sync when it restarts.
      await userApi.block.skip(20);

      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Wait for MSP RPC to respond.
      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);

      // Wait for MSP to be idle again.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 10_000,
        tail: 5_000
      });

      // Reconnect MSP API.
      mspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);
      await userApi.wait.nodeCatchUpToChainTip(mspApi);

      // Build another block to trigger block import notification, after coming out of sync mode.
      await userApi.block.seal();

      // The MSP should attempt recovery but fail, since BSP two/three are not trusted by them.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "recovery finished: recovered=0, failed=1, panicked=0, total=1",
        timeout: 30_000, // Timeout needs to be longer here, accounting for the failed attempts.
        tail: 20_000
      });

      // Ensure MSP still doesn't have the file after recovery completes.
      const afterRecovery = await mspApi.rpc.storagehubclient.isFileInFileStorage(newFile.fileKey);
      assert(!afterRecovery.isFileFound, "MSP unexpectedly recovered the new file");

      // Give it a few more blocks and ensure it is still missing (guards against delayed recovery).
      await userApi.block.skip(5);
      const stillMissing = await mspApi.rpc.storagehubclient.isFileInFileStorage(newFile.fileKey);
      assert(!stillMissing.isFileFound, "MSP unexpectedly recovered the new file later");

      // Check BSP two and BSP three rejected the MSP download requests for this specific file.
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-two",
        searchString: "Received unexpected download request from",
        timeout: 10_000,
        tail: 50_000
      });
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-two",
        searchString: `for file key [${newFile.fileKey}]`,
        timeout: 10_000,
        tail: 50_000
      });
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-three",
        searchString: "Received unexpected download request from",
        timeout: 10_000,
        tail: 50_000
      });
      await userApi.docker.waitForLog({
        containerName: "sh-bsp-three",
        searchString: `for file key [${newFile.fileKey}]`,
        timeout: 10_000,
        tail: 50_000
      });
    });
  }
);

await describeMspNet(
  "MSP skips recovery gracefully when indexer is disabled",
  {
    initialised: "multi",
    logLevel: "file-transfer-service=debug",
    networkConfig: [{ noisy: false, rocksdb: true }],
    only: true
  },
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
    let recoverableFile: FileMetadata | undefined;

    const bucketName = "recover-files-indexer-disabled-bucket";

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
      await userApi.wait.nodeCatchUpToChainTip(bspTwoApi);
      await userApi.wait.nodeCatchUpToChainTip(bspThreeApi);
    });

    after(async () => {
      await bspApi?.disconnect();
      await bspTwoApi?.disconnect();
      await bspThreeApi?.disconnect();
      await mspApi?.disconnect();
    });

    it("Create file, delete it from MSP storage, restart MSP and verify graceful skip", async () => {
      const result = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/recover-files-indexer-disabled.jpg",
            bucketIdOrName: bucketName,
            replicationTarget: 3
          }
        ],
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        bspApis: [bspApi, bspTwoApi, bspThreeApi],
        mspApi
      });

      recoverableFile = {
        fileKey: result.fileKeys[0],
        bucketId: result.bucketIds[0],
        location: result.locations[0],
        owner: userApi.accounts.shUser.address,
        fingerprint: result.fingerprints[0],
        fileSize: result.fileSizes[0]
      };

      await mspApi.wait.fileStorageComplete(recoverableFile.fileKey);

      await mspApi.rpc.storagehubclient.removeFilesFromFileStorage([recoverableFile.fileKey]);
      await mspApi.wait.fileDeletionFromFileStorage(recoverableFile.fileKey);

      await mspApi.disconnect();
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);
      await userApi.block.skip(20);
      await userApi.docker.restartContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      await getContainerPeerId(`http://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "💤 Idle",
        timeout: 20_000,
        tail: 10_000
      });

      mspApi = await createApi(`ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`);
      await userApi.wait.nodeCatchUpToChainTip(mspApi);
      await userApi.block.seal();

      // Recovery should be skipped gracefully because indexer is unavailable.
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "indexer is disabled; cannot schedule downloads",
        timeout: 20_000,
        tail: 30_000
      });

      const afterRecoveryAttempt = await mspApi.rpc.storagehubclient.isFileInFileStorage(
        recoverableFile.fileKey
      );
      assert(
        !afterRecoveryAttempt.isFileFound,
        "MSP unexpectedly recovered file while indexer is disabled"
      );

      await userApi.block.skip(5);
      const stillMissing = await mspApi.rpc.storagehubclient.isFileInFileStorage(
        recoverableFile.fileKey
      );
      assert(!stillMissing.isFileFound, "MSP unexpectedly recovered file later");
    });
  }
);

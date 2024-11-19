import { strictEqual } from "node:assert";
import { bspKey, describeBspNet, shUser, type EnrichedBspApi } from "../../../util";
import invariant from "tiny-invariant";

describeBspNet(
  "BSPNet : stop storing file and other BSPs taking the relay",
  { initialised: "multi", networkConfig: "standard" },
  ({ before, createUserApi, after, it, createApi, createBspApi, getLaunchResponse }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let bspThreeApi: EnrichedBspApi;

    before(async () => {
      const launchResponse = await getLaunchResponse();
      invariant(
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
      await userApi.docker.pauseBspContainer("sh-bsp-three");

      const { fileKey, location, fingerprint, fileSize, bucketId } =
        await userApi.file.createBucketAndSendNewStorageRequest(source, destination, bucketName);

      // Wait for the two BSP to volunteer
      await userApi.wait.bspVolunteer(2);
      await userApi.wait.bspStored(2);

      // Revoke the storage request otherwise the new storage request event is not being triggered
      await userApi.sealBlock(userApi.tx.fileSystem.revokeStorageRequest(fileKey), shUser);

      await userApi.assert.eventPresent("fileSystem", "StorageRequestRevoked");

      // Unpause bsp three
      await userApi.docker.resumeBspContainer({ containerName: "sh-bsp-three" });

      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);
      await userApi.sealBlock(
        bspApi.tx.fileSystem.bspRequestStopStoring(
          fileKey,
          bucketId,
          location,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          fingerprint,
          fileSize,
          false,
          inclusionForestProof.toString()
        ),
        bspKey
      );

      await userApi.assert.eventPresent("fileSystem", "BspRequestedToStopStoring");

      // When requested to stop storing a file we should also receive an event new storage request
      // to replace the bsp leaving
      await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

      // Wait for the right moment to confirm stop storing
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const cooldown =
        currentBlockNumber + userApi.consts.fileSystem.minWaitForStopStoring.toNumber();
      await userApi.advanceToBlock(cooldown);

      // Confirm stop storing
      await userApi.sealBlock(
        userApi.tx.fileSystem.bspConfirmStopStoring(fileKey, inclusionForestProof),
        bspKey
      );

      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");
    });
  }
);

import { strictEqual } from "node:assert";
import { bspKey, describeBspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";
import invariant from "tiny-invariant";

describeBspNet(
  "Multiple BSPs working together ",
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

    it("bsp stop storing and other bsp volunteer", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "tastytest";

      // Pause BSP-Three.
      await userApi.docker.pauseBspContainer("sh-bsp-three");

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const txs = [];
      const { fingerprint, file_size, location } =
        await userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          destination,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          newBucketEventDataBlob.bucketId
        );

      txs.push(
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          fingerprint,
          file_size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId]
        )
      );

      await userApi.sealBlock(txs, shUser);

      // Get the new storage request event
      const storageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "NewStorageRequest"
      );
      strictEqual(storageRequestEvents.length, 1);

      // Get the file keys from the storage request events
      const fileKeys = storageRequestEvents.map((event) => {
        const dataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
        if (!dataBlob) {
          throw new Error("Event doesn't match Type");
        }
        return dataBlob.fileKey;
      });

      // Wait for the two BSP to volunteer
      await userApi.wait.bspVolunteer(2);
      await userApi.wait.bspStored(2);

      // Wait for the BSP to receive and store all files
      const fileKey = fileKeys[0];
      await bspApi.wait.bspFileStorageComplete(fileKey);

      // Revoke the storage request otherwise the new storage request event is not being triggered
      await userApi.sealBlock(userApi.tx.fileSystem.revokeStorageRequest(fileKey), shUser);

      await sleep(10000);
      userApi.assert.fetchEventData(
        userApi.events.fileSystem.StorageRequestRevoked,
        await userApi.query.system.events()
      );

      // unpause bsp three
      await userApi.docker.resumeBspContainer({ containerName: "sh-bsp-three" });

      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKey
      ]);
      await userApi.sealBlock(
        bspApi.tx.fileSystem.bspRequestStopStoring(
          fileKey,
          newBucketEventDataBlob.bucketId,
          location,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          fingerprint,
          file_size,
          false,
          inclusionForestProof.toString()
        ),
        bspKey
      );

      await sleep(500);
      userApi.assert.fetchEventData(
        userApi.events.fileSystem.BspRequestedToStopStoring,
        await userApi.query.system.events()
      );

      // when requested to stop storing a file we should also receive an event new storage request
      // to replace the bsp leaving
      userApi.assert.fetchEventData(
        userApi.events.fileSystem.NewStorageRequest,
        await userApi.query.system.events()
      );

      // wait for the irght moment to confirm stop storing
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const cooldown =
        currentBlockNumber + userApi.consts.fileSystem.minWaitForStopStoring.toNumber();
      await userApi.advanceToBlock(cooldown);

      // confirm stop storing
      await userApi.sealBlock(
        userApi.tx.fileSystem.bspConfirmStopStoring(fileKey, inclusionForestProof),
        bspKey
      );

      await sleep(500);
      userApi.assert.fetchEventData(
        userApi.events.fileSystem.BspConfirmStoppedStoring,
        await userApi.query.system.events()
      );
    });
  }
);

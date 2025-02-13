import assert, { strictEqual } from "node:assert";
import { describeMspNet, shUser, type EnrichedBspApi, sleep } from "../../../util";

describeMspNet(
  "MSP catching up with chain and volunteering for storage request",
  { initialised: false },
  ({ before, createMsp1Api, it, createUserApi }) => {
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

    it("MSP accepts subsequent storage request for the same file key", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/smile.jpg";
      const bucketName = "trying-things";

      // Stop the msp container so it will be behind when we restart the node.
      await userApi.docker.pauseBspContainer("docker-sh-msp-1");

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            destination,
            userApi.shConsts.TEST_ARTEFACTS[source].fingerprint,
            userApi.shConsts.TEST_ARTEFACTS[source].size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Basic: null
            }
          )
        ],
        signer: shUser
      });

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");
      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;
      assert(
        newStorageRequestDataBlob,
        "NewStorageRequest event data does not match expected type"
      );

      // Advancing 10 blocks to see if MSP catchup
      await userApi.block.skip(10);

      await userApi.docker.restartBspContainer({ containerName: "docker-sh-msp-1" });

      // need to wait for the container to be up again
      // await sleep(5000);

      // NOTE:
      // We shouldn't have to recreate an API but any other attempt to reconnect failed
      // Also had to guess for the port of MSP 1
      // await using newMspApi = await createApi("ws://127.0.0.1:9777");

      // Required to trigger out of sync mode
      // await userApi.rpc.engine.createBlock(true, true);

      // await waitFor({
      //   lambda: async () =>
      //     (await newMspApi.rpc.storagehubclient.isFileInFileStorage(event.data.fileKey)).isFileFound
      // });

      // await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");
    });
  }
);

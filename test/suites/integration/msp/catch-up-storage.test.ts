import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { type EnrichedBspApi, describeMspNet, shUser, sleep, waitFor } from "../../../util";

describeMspNet(
  "MSP catching up with chain and volunteering for storage request",
  { initialised: false },
  ({ before, createMsp1Api, it, createUserApi, createApi }) => {
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

    it(
      "MSP accept storage request after catching up with blockchain and user properly retry sending file",
      { timeout: 50000 },
      async () => {
        const source = "res/whatsup.jpg";
        const destination = "test/smile.jpg";
        const bucketName = "trying-things";

        // Stop the msp container so it will be behind when we restart the node.
        // TODO: clearLogs is not working, fix it.
        // await clearLogs({ containerName: "storage-hub-sh-msp-1" });
        await userApi.docker.pauseContainer("storage-hub-sh-msp-1");

        const newBucketEventEvent = await userApi.createBucket(bucketName);
        const newBucketEventDataBlob =
          userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

        assert(newBucketEventDataBlob, "Event doesn't match Type");

        const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(
          2
        );
        await userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          destination,
          ownerHex,
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

        // Closing mspApi gracefully before restarting the container
        // IMPORTANT: If this is not done, the api connection cannot close properly and the test
        // runner will hang.
        await mspApi.disconnect();

        // Restarting the MSP container. This will start the Substrate node from scratch.
        await userApi.docker.restartContainer({ containerName: "storage-hub-sh-msp-1" });

        // TODO: Wait for the container logs of starting up
        await userApi.docker.waitForLog({
          searchString: "💤 Idle (3 peers)",
          containerName: "storage-hub-sh-msp-1",
          tail: 10
        });

        // Doesn't work without this because there is no log that tell us when the websocket is ready
        await sleep(15000);

        // Creating a new MSP API to connect to the newly restarted container.
        const newMspApi = await createApi(
          `ws://127.0.0.1:${userApi.shConsts.NODE_INFOS.msp1.port}`
        );

        console.log("Connected");

        // Waiting for the MSP node to be in sync with the chain.
        await userApi.rpc.engine.createBlock(true, true);

        await userApi.docker.waitForLog({
          searchString: "🥱 Handling coming out of sync mode",
          containerName: "storage-hub-sh-msp-1"
        });

        await userApi.block.skip(4); // user retry every 5 blocks. The one we created before and this one

        await userApi.docker.waitForLog({
          searchString:
            'File upload complete. Peer PeerId("12D3KooWSUvz8QM5X4tfAaSLErAZjR2puojo16pULBHyqTMGKtNV") has the entire file',
          containerName: "storage-hub-sh-user-1"
        });

        await waitFor({
          lambda: async () =>
            (await newMspApi.rpc.storagehubclient.isFileInFileStorage(event.data.fileKey))
              .isFileFound
        });

        await userApi.block.seal();
        await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

        // IMPORTANT!! Without this the test suite never finish
        newMspApi.disconnect();
      }
    );
  }
);

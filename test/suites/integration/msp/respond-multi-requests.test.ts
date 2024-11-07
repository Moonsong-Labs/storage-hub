import { strictEqual } from "node:assert";
import { describeMspNet, shUser, sleep, type EnrichedBspApi } from "../../../util";

describeMspNet(
  "Single MSP accepting multiple storage requests",
  ({ before, createMspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMspApi();
      if (maybeMspApi) {
        mspApi = maybeMspApi;
      } else {
        throw new Error("MSP API not available");
      }
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("MSP receives files from user after issued storage requests", async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
      const bucketName = "nothingmuch-3";

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );

      const valuePropId = valueProps[0].id;

      const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      const txs = [];
      for (let i = 0; i < source.length; i++) {
        const { fingerprint, file_size, location } =
          await userApi.rpc.storagehubclient.loadFileInStorage(
            source[i],
            destination[i],
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
      }

      await userApi.sealBlock(txs, shUser);

      // Allow time for the MSP to receive and store the file from the user
      await sleep(3000);

      const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");

      const matchedEvents = events.filter((e) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );

      if (matchedEvents.length !== source.length) {
        throw new Error(`Expected ${source.length} NewStorageRequest events`);
      }

      const issuedFileKeys = [];
      for (const e of matchedEvents) {
        const newStorageRequestDataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

        if (!newStorageRequestDataBlob) {
          throw new Error("Event doesn't match Type");
        }

        const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
          newStorageRequestDataBlob.fileKey
        );

        if (!result.isFileFound) {
          throw new Error(
            `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
          );
        }

        issuedFileKeys.push(newStorageRequestDataBlob.fileKey);
      }
    });
  }
);

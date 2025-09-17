import { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { describeBspNet, type EnrichedBspApi, shUser } from "../../../util";

await describeBspNet("User: Issue Storage Requests", ({ before, createUserApi, it }) => {
  let userApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
  });

  it("issueStorageRequest fails if file is empty", async () => {
    const location = "test/empty-file";
    const bucketName = "bucket-3";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const issueStorageRequestResult = await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          userApi.shConsts.TEST_ARTEFACTS["res/empty-file"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/empty-file"].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    strictEqual(issueStorageRequestResult.extSuccess, false);
  });

  it("issueStorageRequest for file with 512 bytes or half a chunk", async () => {
    const source = "res/half-chunk-file";
    const destination = "test/half-chunk-file";
    const bucketName = "bucket-6";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const ownerHex1 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex1,
      newBucketEventDataBlob.bucketId
    );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);
  });

  it("issueStorageRequest works even if peerIds are missing", async () => {
    const location = "test/half-chunk-file";
    const bucketName = "bucket-7";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          userApi.shConsts.TEST_ARTEFACTS["res/half-chunk-file"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/half-chunk-file"].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");

    const dataBlob = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

    if (!dataBlob) {
      throw new Error("Event doesn't match Type");
    }

    strictEqual(dataBlob.who.toString(), userApi.shConsts.NODE_INFOS.user.AddressId);
    strictEqual(dataBlob.location.toHuman(), location);
    strictEqual(
      dataBlob.fingerprint.toString(),
      userApi.shConsts.TEST_ARTEFACTS["res/half-chunk-file"].fingerprint
    );
    strictEqual(
      dataBlob.size_.toBigInt(),
      userApi.shConsts.TEST_ARTEFACTS["res/half-chunk-file"].size
    );
    strictEqual(dataBlob.peerIds.length, 0);
  });

  it("issueStorageRequest fails if bucket does not exist", async () => {
    const location = "test/empty-file";

    // random 32 bytes
    const bucketId = "1ce1a1614e9798e9c7f2b7214ca73c87";

    const issueStorageRequestResult = await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          bucketId,
          location,
          userApi.shConsts.TEST_ARTEFACTS["res/empty-file"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/empty-file"].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    strictEqual(issueStorageRequestResult.extSuccess, false);
  });

  it("issueStorageRequest fails if MSP is not valid", async () => {
    const location = "test/adolphus.jpg";
    const bucketName = "bucket-88";
    const INVALID_MSP_ID = "0x0000000000000000000000000000000000000000000000000000000000000222";

    // Creates bucket using `bsp_api` but will submit extrinsic using `userApi`
    const newBucketEventEvent = await userApi.createBucket(bucketName);

    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const issueStorageRequestResult = await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].size,
          INVALID_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    strictEqual(issueStorageRequestResult.extSuccess, false);
  });

  it("issueStorageRequest twice for the same file fails", async () => {
    const destination = "test/smile.jpg";
    const bucketName = "bucket-9";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          destination,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size,
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

    const dataBlob = userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

    if (!dataBlob) {
      throw new Error("Event doesn't match Type");
    }

    strictEqual(dataBlob.who.toString(), userApi.shConsts.NODE_INFOS.user.AddressId);
    strictEqual(dataBlob.location.toHuman(), destination);
    strictEqual(
      dataBlob.fingerprint.toString(),
      userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint
    );
    strictEqual(dataBlob.size_.toBigInt(), userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size);
    strictEqual(dataBlob.peerIds.length, 1);
    strictEqual(dataBlob.peerIds[0].toHuman(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

    const issueStorageRequestResultTwice = await userApi.block.seal({
      calls: [
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          destination,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
          userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Basic: null
          }
        )
      ],
      signer: shUser
    });

    strictEqual(issueStorageRequestResultTwice.extSuccess, false);
  });
});

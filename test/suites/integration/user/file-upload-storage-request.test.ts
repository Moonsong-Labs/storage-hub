import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  TEST_ARTEFACTS,
  shUser,
  sleep,
  describeBspNet,
  type EnrichedBspApi
} from "../../../util";

describeBspNet("User: Issue Storage Requests", ({ before, createUserApi, it }) => {
  let user_api: EnrichedBspApi;

  before(async () => {
    user_api = await createUserApi();
  });

  it("issueStorageRequest fails if file is empty", async () => {
    const location = "test/empty-file";
    const bucketName = "bucket-3";

    const newBucketEventEvent = await user_api.createBucket(bucketName);
    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const issueStorageRequestResult = await user_api.sealBlock(
      user_api.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        location,
        TEST_ARTEFACTS["res/empty-file"].fingerprint,
        TEST_ARTEFACTS["res/empty-file"].size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    strictEqual(issueStorageRequestResult.extSuccess, false);
  });

  it("issueStorageRequest for file with 512 bytes or half a chunk", async () => {
    const source = "res/half-chunk-file";
    const destination = "test/half-chunk-file";
    const bucketName = "bucket-6";

    const newBucketEventEvent = await user_api.createBucket(bucketName);
    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const { location, fingerprint, file_size } =
      await user_api.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), TEST_ARTEFACTS[source].fingerprint);
    strictEqual(file_size.toBigInt(), TEST_ARTEFACTS[source].size);
  });

  it("issueStorageRequest works even if peerIds are missing", async () => {
    const location = "test/half-chunk-file";
    const bucketName = "bucket-7";

    const newBucketEventEvent = await user_api.createBucket(bucketName);
    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const issueStorageRequestResult = await user_api.sealBlock(
      user_api.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        location,
        TEST_ARTEFACTS["res/half-chunk-file"].fingerprint,
        TEST_ARTEFACTS["res/half-chunk-file"].size,
        DUMMY_MSP_ID,
        []
      ),
      shUser
    );

    // wait for the bsp to volunteer
    await sleep(500);

    const { event } = user_api.assertEvent(
      "fileSystem",
      "NewStorageRequest",
      issueStorageRequestResult.events
    );

    const dataBlob = user_api.events.fileSystem.NewStorageRequest.is(event) && event.data;

    if (!dataBlob) {
      throw new Error("Event doesn't match Type");
    }

    strictEqual(dataBlob.who.toString(), NODE_INFOS.user.AddressId);
    strictEqual(dataBlob.location.toHuman(), location);
    strictEqual(dataBlob.fingerprint.toString(), TEST_ARTEFACTS["res/half-chunk-file"].fingerprint);
    strictEqual(dataBlob.size_.toBigInt(), TEST_ARTEFACTS["res/half-chunk-file"].size);
    strictEqual(dataBlob.peerIds.length, 0);
  });

  it("issueStorageRequest fails if bucket does not exist", async () => {
    const location = "test/empty-file";

    // random 32 bytes
    const bucketId = "1ce1a1614e9798e9c7f2b7214ca73c87";

    const issueStorageRequestResult = await user_api.sealBlock(
      user_api.tx.fileSystem.issueStorageRequest(
        bucketId,
        location,
        TEST_ARTEFACTS["res/empty-file"].fingerprint,
        TEST_ARTEFACTS["res/empty-file"].size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    strictEqual(issueStorageRequestResult.extSuccess, false);
  });

  it("issueStorageRequest fails if MSP is not valid", async () => {
    const location = "test/adolphus.jpg";
    const bucketName = "bucket-88";
    const INVALID_MSP_ID = "0x0000000000000000000000000000000000000000000000000000000000000222";

    // Creates bucket using `bsp_api` but will submit extrinsic using `user_api`
    const newBucketEventEvent = await user_api.createBucket(bucketName);

    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const issueStorageRequestResult = await user_api.sealBlock(
      user_api.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        location,
        TEST_ARTEFACTS["res/adolphus.jpg"].fingerprint,
        TEST_ARTEFACTS["res/adolphus.jpg"].size,
        INVALID_MSP_ID,
        [NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    strictEqual(issueStorageRequestResult.extSuccess, false);
  });

  it("issueStorageRequest twice for the same file fails", async () => {
    const destination = "test/smile.jpg";
    const bucketName = "bucket-9";

    const newBucketEventEvent = await user_api.createBucket(bucketName);
    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const issueStorageRequestResult = await user_api.sealBlock(
      user_api.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        destination,
        TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
        TEST_ARTEFACTS["res/smile.jpg"].size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    // wait for the bsp to volunteer
    await sleep(500);

    const { event } = user_api.assertEvent(
      "fileSystem",
      "NewStorageRequest",
      issueStorageRequestResult.events
    );

    const dataBlob = user_api.events.fileSystem.NewStorageRequest.is(event) && event.data;

    if (!dataBlob) {
      throw new Error("Event doesn't match Type");
    }

    strictEqual(dataBlob.who.toString(), NODE_INFOS.user.AddressId);
    strictEqual(dataBlob.location.toHuman(), destination);
    strictEqual(dataBlob.fingerprint.toString(), TEST_ARTEFACTS["res/smile.jpg"].fingerprint);
    strictEqual(dataBlob.size_.toBigInt(), TEST_ARTEFACTS["res/smile.jpg"].size);
    strictEqual(dataBlob.peerIds.length, 1);
    strictEqual(dataBlob.peerIds[0].toHuman(), NODE_INFOS.user.expectedPeerId);

    const issueStorageRequestResultTwice = await user_api.sealBlock(
      user_api.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        destination,
        TEST_ARTEFACTS["res/smile.jpg"].fingerprint,
        TEST_ARTEFACTS["res/smile.jpg"].size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    await sleep(500);

    strictEqual(issueStorageRequestResultTwice.extSuccess, false);
  });
});

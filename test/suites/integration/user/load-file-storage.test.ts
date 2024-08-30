import "@storagehub/api-augment";
import assert, { strictEqual } from "node:assert";
import { NODE_INFOS, TEST_ARTEFACTS, type BspNetApi, describeBspNet } from "../../../util";

describeBspNet("User: Load File Into Storage", ({ before, createUserApi, it }) => {
  let user_api: BspNetApi;

  before(async () => {
    user_api = await createUserApi();
  });

  it("loadFileInStorage works", async () => {
    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const bucketName = "bucket-0";

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

  it("loadFileInStorage fails if file is empty", async () => {
    const source = "res/empty-file";
    const destination = "test/empty-file";
    const bucketName = "bucket-1";

    const newBucketEventEvent = await user_api.createBucket(bucketName);
    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    try {
      await user_api.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );
    } catch (e: any) {
      strictEqual(e.message, "-32603: Internal error: FileIsEmpty");
    }
  });

  it("loadFileInStorage for file with exactly 1024 bytes or 1 chunk", async () => {
    const source = "res/one-chunk-file";
    const destination = "test/one-chunk-file";
    const bucketName = "bucket-5";

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

  it("loadFileInStorage for the same file twice fails", async () => {
    const source = "res/one-chunk-file";
    const destination = "test/one-chunk-file";
    const bucketName = "bucket-10";

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

    try {
      await user_api.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );
    } catch (e: any) {
      strictEqual(e.message, "-32603: Internal error: FileAlreadyExists");
    }
  });

  it("loadFileInStorage for inexistent file fails", async () => {
    const source = "res/inexistent-file";
    const destination = "test/inexistent-file";
    const bucketName = "bucket-11";

    const newBucketEventEvent = await user_api.createBucket(bucketName);
    const newBucketEventDataBlob =
      user_api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    await assert.rejects(
      () =>
        user_api.rpc.storagehubclient.loadFileInStorage(
          source,
          destination,
          NODE_INFOS.user.AddressId,
          newBucketEventDataBlob.bucketId
        ),
      /-32603: Internal error: Os { code: 2, kind: NotFound, message: "No such file or directory" }/,
      "Error message should be 'No such file or directory'"
    );
  });
});

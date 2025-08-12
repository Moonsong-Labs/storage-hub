import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { type EnrichedBspApi, describeBspNet } from "../../../util";

describeBspNet("User: Load File Into Storage", ({ before, createUserApi, it }) => {
  let userApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
  });

  it("loadFileInStorage works", async () => {
    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const bucketName = "bucket-0";

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

  it("loadFileInStorage fails if file is empty", async () => {
    const source = "res/empty-file";
    const destination = "test/empty-file";
    const bucketName = "bucket-1";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    try {
      const ownerHexEmpty = u8aToHex(
        decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)
      ).slice(2);
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHexEmpty,
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

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const ownerHexChunk = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(
      2
    );
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHexChunk,
      newBucketEventDataBlob.bucketId
    );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);
  });

  it("loadFileInStorage for the same file twice fails", async () => {
    const source = "res/one-chunk-file";
    const destination = "test/one-chunk-file";
    const bucketName = "bucket-10";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const ownerHexFirst = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(
      2
    );
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHexFirst,
      newBucketEventDataBlob.bucketId
    );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);

    try {
      const ownerHex2 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(
        2
      );
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHex2,
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

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    await assert.rejects(
      () =>
        userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          destination,
          u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2),
          newBucketEventDataBlob.bucketId
        ),
      /-32603: Internal error: Os { code: 2, kind: NotFound, message: "No such file or directory" }/,
      "Error message should be 'No such file or directory'"
    );
  });
});

import assert, { strictEqual } from "node:assert";
import { describeBspNet, type EnrichedBspApi, addCopypartyContainer } from "../../../util";
import type Docker from "dockerode";

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

    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
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
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
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

    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
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

    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
      newBucketEventDataBlob.bucketId
    );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS[source].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS[source].size);

    try {
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
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
          userApi.shConsts.NODE_INFOS.user.AddressId,
          newBucketEventDataBlob.bucketId
        ),
      /-32603: Internal error: File not found/,
      "Error message should be 'File not found'"
    );
  });
});

describeBspNet("User: Load File Into Storage - Remote URLs", ({ before, after, createUserApi, it }) => {
  let userApi: EnrichedBspApi;
  let copypartyContainer: Docker.Container;
  let serverIp: string;
  let httpPort: number;
  let ftpPort: number;

  before(async () => {
    userApi = await createUserApi();

    // Setup Copyparty server
    const copypartyInfo = await addCopypartyContainer();
    copypartyContainer = copypartyInfo.container;
    serverIp = copypartyInfo.containerIp;
    httpPort = copypartyInfo.httpPort;
    ftpPort = copypartyInfo.ftpPort;
  });

  after(async () => {
    if (copypartyContainer) {
      await copypartyContainer.stop();
      await copypartyContainer.remove();
    }
  });

  it("loadFileInStorage works with HTTP URL", async () => {
    const source = `http://${serverIp}:${httpPort}/res/adolphus.jpg`;
    const destination = "test/adolphus-http.jpg";
    const bucketName = "bucket-http-remote";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
      newBucketEventDataBlob.bucketId
    );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].size);
  });

  it("loadFileInStorage works with FTP URL", async () => {
    const source = `ftp://${serverIp}:${ftpPort}/res/smile.jpg`;
    const destination = "test/smile-ftp.jpg";
    const bucketName = "bucket-ftp-remote";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      userApi.shConsts.NODE_INFOS.user.AddressId,
      newBucketEventDataBlob.bucketId
    );

    strictEqual(location.toHuman(), destination);
    strictEqual(fingerprint.toString(), userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint);
    strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size);
  });
});

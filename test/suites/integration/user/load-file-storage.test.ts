import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { addCopypartyContainer, describeBspNet, type EnrichedBspApi } from "../../../util";

await describeBspNet("User: Load File Into Storage", ({ before, createUserApi, it }) => {
  let userApi: EnrichedBspApi;
  let remoteServerInfo:
    | {
        containerName: string;
        httpPort: number;
        ftpPort: number;
      }
    | undefined;

  before(async () => {
    userApi = await createUserApi();

    // Setup Copyparty server for remote file tests (HTTP and FTP)
    const copypartyInfo = await addCopypartyContainer();
    remoteServerInfo = {
      containerName: copypartyInfo.containerName,
      httpPort: copypartyInfo.httpPort,
      ftpPort: copypartyInfo.ftpPort
    };
  });

  // Helper function to create a bucket and get its ID
  const createBucketAndGetId = async (bucketName: string) => {
    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    return newBucketEventDataBlob.bucketId;
  };

  // Helper function to verify file metadata
  const verifyFileMetadata = (
    location: any,
    fingerprint: any,
    file_size: any,
    expectedDestination: string,
    expectedArtefact: { fingerprint: string; size: bigint }
  ) => {
    strictEqual(location.toHuman(), expectedDestination);
    strictEqual(fingerprint.toString(), expectedArtefact.fingerprint);
    strictEqual(file_size.toBigInt(), expectedArtefact.size);
  };

  // === Local File Tests ===

  it("loadFileInStorage works with local file", async () => {
    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const bucketId = await createBucketAndGetId("bucket-0");

    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex,
      bucketId
    );

    verifyFileMetadata(
      location,
      fingerprint,
      file_size,
      destination,
      userApi.shConsts.TEST_ARTEFACTS[source]
    );
  });

  it("loadFileInStorage works with single chunk file (1024 bytes)", async () => {
    const source = "res/one-chunk-file";
    const destination = "test/one-chunk-file";
    const bucketId = await createBucketAndGetId("bucket-5");

    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex,
      bucketId
    );

    verifyFileMetadata(
      location,
      fingerprint,
      file_size,
      destination,
      userApi.shConsts.TEST_ARTEFACTS[source]
    );
  });

  // === Error Handling Tests ===

  it("loadFileInStorage fails if file is empty", async () => {
    const source = "res/empty-file";
    const destination = "test/empty-file";
    const bucketId = await createBucketAndGetId("bucket-1");

    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    await assert.rejects(
      () => userApi.rpc.storagehubclient.loadFileInStorage(source, destination, ownerHex, bucketId),
      /-32603: Internal error: FileIsEmpty/,
      "Should reject with FileIsEmpty error"
    );
  });

  it("loadFileInStorage fails when loading the same file twice", async () => {
    const source = "res/one-chunk-file";
    const destination = "test/duplicate-file";
    const bucketId = await createBucketAndGetId("bucket-10");

    // First upload should succeed
    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex,
      bucketId
    );

    verifyFileMetadata(
      location,
      fingerprint,
      file_size,
      destination,
      userApi.shConsts.TEST_ARTEFACTS[source]
    );

    // Second upload with same destination should fail
    const ownerHex2 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    await assert.rejects(
      () =>
        userApi.rpc.storagehubclient.loadFileInStorage(source, destination, ownerHex2, bucketId),
      /-32603: Internal error: FileAlreadyExists/,
      "Should reject with FileAlreadyExists error"
    );
  });

  it("loadFileInStorage fails for non-existent file", async () => {
    const source = "res/inexistent-file";
    const destination = "test/inexistent-file";
    const bucketId = await createBucketAndGetId("bucket-11");

    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    await assert.rejects(
      () => userApi.rpc.storagehubclient.loadFileInStorage(source, destination, ownerHex, bucketId),
      /-32603: Internal error: File not found/,
      "Should reject with 'File not found' error"
    );
  });

  // === Remote File Tests (HTTP & FTP) ===

  it("loadFileInStorage works with HTTP URL", async () => {
    assert(remoteServerInfo, "Remote server not initialized");
    const { containerName, httpPort } = remoteServerInfo;

    const source = `http://${containerName}:${httpPort}/res/adolphus.jpg`;
    const destination = "test/adolphus-http.jpg";
    const bucketId = await createBucketAndGetId("bucket-http-remote");

    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex,
      bucketId
    );

    verifyFileMetadata(
      location,
      fingerprint,
      file_size,
      destination,
      userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"]
    );
  });

  it("loadFileInStorage works with FTP URL", async () => {
    assert(remoteServerInfo, "Remote server not initialized");
    const { containerName, ftpPort } = remoteServerInfo;

    const source = `ftp://${containerName}:${ftpPort}/res/smile.jpg`;
    const destination = "test/smile-ftp.jpg";
    const bucketId = await createBucketAndGetId("bucket-ftp-remote");

    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    const {
      file_metadata: { location, fingerprint, file_size }
    } = await userApi.rpc.storagehubclient.loadFileInStorage(
      source,
      destination,
      ownerHex,
      bucketId
    );

    verifyFileMetadata(
      location,
      fingerprint,
      file_size,
      destination,
      userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"]
    );
  });
});

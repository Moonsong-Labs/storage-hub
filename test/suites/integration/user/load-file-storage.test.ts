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

describeBspNet(
  "User: Load File Into Storage - Remote URLs",
  ({ before, after, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let copypartyContainer: Docker.Container;
    let containerName: string;
    let httpPort: number;
    let ftpPort: number;

    before(async () => {
      userApi = await createUserApi();

      // Setup Copyparty server
      const copypartyInfo = await addCopypartyContainer();
      copypartyContainer = copypartyInfo.container;
      containerName = copypartyInfo.containerName;
      httpPort = copypartyInfo.httpPort;
      ftpPort = copypartyInfo.ftpPort;

      // Clean up uploads directory to ensure tests start fresh
      await copypartyContainer
        .exec({
          Cmd: ["sh", "-c", "rm -rf /uploads/* 2>/dev/null || true"],
          AttachStdout: true,
          AttachStderr: true
        })
        .then((exec) => exec.start({}));
    });

    after(async () => {
      if (copypartyContainer) {
        try {
          await copypartyContainer.stop();
          await copypartyContainer.remove();
        } catch (e: any) {
          // Container might already be removed
          console.log("Error cleaning up copyparty container:", e.message);
        }
      }
    });

    it("loadFileInStorage works with HTTP URL", async () => {
      // Use container name for inter-container communication
      // Note: We use container name here because loadFileInStorage runs
      // inside the user container and needs to reach copyparty via Docker's internal network
      const source = `http://${containerName}:${httpPort}/res/adolphus.jpg`;
      const destination = "test/adolphus-http.jpg";
      const bucketName = "bucket-http-remote";

      // First, let's verify copyparty is serving the correct file
      console.log(`\n=== DEBUG: Checking copyparty content ===`);
      console.log(`Source URL: ${source}`);
      
      // Try to download and check the file from within the user container (where the RPC runs)
      try {
        // Get the user container
        const docker = new Docker();
        const userContainer = docker.getContainer("docker-sh-user-1");
        
        // Check from user container perspective
        const execResult = await userContainer.exec({
          Cmd: ["sh", "-c", `curl -s http://${containerName}:${httpPort}/res/adolphus.jpg | sha256sum`],
          AttachStdout: true,
          AttachStderr: true
        });
        const stream = await execResult.start({});
        const output = await new Promise<string>((resolve) => {
          let data = '';
          stream.on('data', (chunk: Buffer) => data += chunk.toString());
          stream.on('end', () => resolve(data));
        });
        console.log(`SHA256 from user container: ${output.trim()}`);
        console.log(`Expected SHA256: 739fb97f7c2b8e7f192b608722a60dc67ee0797c85ff1ea849c41333a40194f2`);
        
        // Also check file size
        const sizeExec = await userContainer.exec({
          Cmd: ["sh", "-c", `curl -sI http://${containerName}:${httpPort}/res/adolphus.jpg | grep -i content-length`],
          AttachStdout: true,
          AttachStderr: true
        });
        const sizeStream = await sizeExec.start({});
        const sizeOutput = await new Promise<string>((resolve) => {
          let data = '';
          sizeStream.on('data', (chunk: Buffer) => data += chunk.toString());
          sizeStream.on('end', () => resolve(data));
        });
        console.log(`Content-Length from user container: ${sizeOutput.trim()}`);
      } catch (e) {
        console.error(`Failed to check from user container: ${e}`);
      }

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("Event doesn't match Type");
      }

      console.log(`\n=== DEBUG: Calling loadFileInStorage ===`);
      console.log(`Expected fingerprint: ${userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].fingerprint}`);
      
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        userApi.shConsts.NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

      console.log(`\n=== DEBUG: loadFileInStorage result ===`);
      console.log(`Actual fingerprint: ${fingerprint.toString()}`);
      console.log(`File size: ${file_size.toString()}`);

      strictEqual(location.toHuman(), destination);
      strictEqual(
        fingerprint.toString(),
        userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].fingerprint
      );
      strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS["res/adolphus.jpg"].size);
    });

    it("loadFileInStorage works with FTP URL", async () => {
      // Use container name for inter-container communication
      const source = `ftp://${containerName}:${ftpPort}/res/smile.jpg`;
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
      strictEqual(
        fingerprint.toString(),
        userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].fingerprint
      );
      strictEqual(file_size.toBigInt(), userApi.shConsts.TEST_ARTEFACTS["res/smile.jpg"].size);
    });
  }
);

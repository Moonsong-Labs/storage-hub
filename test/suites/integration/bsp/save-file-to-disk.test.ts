import assert, { strictEqual } from "node:assert";
import {
  describeBspNet,
  shUser,
  type EnrichedBspApi,
  addCopypartyContainer,
  waitFor,
  sleep
} from "../../../util";

describeBspNet(
  "BSP: Save File To Disk",
  ({ before, createBspApi, createUserApi, it }) => {
    let bspApi: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let fileKey: string;
    let containerName: string | undefined;
    let httpPort: number | undefined;
    let ftpPort: number | undefined;

    const source = "res/whatsup.jpg";
    const destination = "test/whatsup-for-save.jpg";
    const bucketName = "bucket-save-test";

    before(async () => {
      bspApi = await createBspApi();
      userApi = await createUserApi();

      // Setup Copyparty server for remote tests
      const copypartyInfo = await addCopypartyContainer();
      containerName = copypartyInfo.containerName;
      httpPort = copypartyInfo.httpPort;
      ftpPort = copypartyInfo.ftpPort;

      // Setup: Store a file first so we have something to save
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

      // Issue storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Custom: 1
            }
          )
        ],
        signer: shUser
      });

      // Wait for BSP to volunteer
      await userApi.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true
      });

      const { event } = await userApi.assert.eventPresent("fileSystem", "NewStorageRequest");
      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(event) && event.data;

      assert(
        newStorageRequestDataBlob,
        "NewStorageRequest event data does not match expected type"
      );

      await userApi.block.seal();

      // Wait for BSP to accept volunteer
      await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer");

      // Wait for file to be in BSP storage
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(newStorageRequestDataBlob.fileKey))
            .isFileFound
      });

      // Wait for BSP to confirm storage
      await userApi.wait.bspStored({
        expectedExts: 1,
        sealBlock: false
      });

      await userApi.block.seal();

      const {
        data: { confirmedFileKeys: bspConfirmRes_fileKeys }
      } = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      // Store the fileKey for use in tests
      fileKey = bspConfirmRes_fileKeys[0].toString();

      // Give some time for everything to settle
      await sleep(1000);
    });

    it("saveFileToDisk works with local path", async () => {
      const saveResult = await bspApi.rpc.storagehubclient.saveFileToDisk(
        fileKey,
        "/storage/test/whatsup-local.jpg"
      );

      assert(saveResult.isSuccess);
      const sha = await bspApi.docker.checkFileChecksum("test/whatsup-local.jpg");
      strictEqual(sha, userApi.shConsts.TEST_ARTEFACTS["res/whatsup.jpg"].checksum);
    });

    it("saveFileToDisk works with HTTP URL", async () => {
      assert(containerName, "Container name not initialized");
      assert(httpPort, "HTTP port not initialized");

      const httpDestination = `http://${containerName}:${httpPort}/uploads/whatsup-http.jpg`;
      const saveResult = await bspApi.rpc.storagehubclient.saveFileToDisk(fileKey, httpDestination);

      assert(saveResult.isSuccess);
    });

    it("saveFileToDisk works with FTP URL", async () => {
      assert(containerName, "Container name not initialized");
      assert(ftpPort, "FTP port not initialized");

      const ftpDestination = `ftp://${containerName}:${ftpPort}/uploads/whatsup-ftp.jpg`;
      const saveResult = await bspApi.rpc.storagehubclient.saveFileToDisk(fileKey, ftpDestination);

      assert(saveResult.isSuccess);
    });
  }
);

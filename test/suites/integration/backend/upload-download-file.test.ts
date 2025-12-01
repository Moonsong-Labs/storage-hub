import assert, { strictEqual } from "node:assert";
import fs from "node:fs";
import path from "node:path";
import type { H256 } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import * as $ from "scale-codec";
import { bspKey, describeMspNet, type EnrichedBspApi, waitFor } from "../../../util";
import type { FileInfo, HealthResponse } from "../../../util/backend";
import { fetchJwtToken } from "../../../util/backend/jwt";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import {
  BALTATHAR_PRIVATE_KEY,
  ETH_SH_USER_ADDRESS,
  ETH_SH_USER_PRIVATE_KEY,
  ethShUser
} from "../../../util/evmNet/keyring";

await describeMspNet(
  "Backend file upload integration",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let uploadedFileKeyHex: string;
    let originalFileBuffer: Buffer;
    const TEST_FILE_NAME = "whatsup.jpg";

    let bucketId: string;
    let freshBucketRoot: H256;
    const fileLocation = `test/${TEST_FILE_NAME}`;
    const source = `res/${TEST_FILE_NAME}`;
    let fileKey: H256;
    let fileMetadata: any; // util/FileMetadata is not the same type returned by the RPC
    let form: FormData;
    let userJWT: string;

    before(async () => {
      userApi = await createUserApi();
      const maybeMsp1Api = await createMsp1Api();
      if (maybeMsp1Api) {
        msp1Api = maybeMsp1Api;
      } else {
        throw new Error("MSP API for first MSP not available");
      }
    });

    it("Postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.backend.containerName,
        searchString: "Server listening",
        timeout: 15000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Backend health endpoint reports healthy status", async () => {
      const response = await fetch("http://localhost:8080/health");
      strictEqual(response.status, 200, "Health endpoint should return 200 OK");

      const healthData: HealthResponse = (await response.json()) as HealthResponse;

      // Verify overall health structure
      assert(healthData.status, "Health response should have status field");
      assert(healthData.components, "Health response should have components field");

      // Verify storage health
      assert(healthData.components.storage, "Should have storage component");
      strictEqual(healthData.components.storage.status, "healthy", "Storage should be healthy");

      // Verify Postgres health
      assert(healthData.components.postgres, "Should have postgres component");
      strictEqual(healthData.components.postgres.status, "healthy", "Postgres should be healthy");

      // Verify RPC health (this is the key test - ensures RPC is actually working)
      assert(healthData.components.rpc, "Should have RPC component");
      strictEqual(healthData.components.rpc.status, "healthy", "RPC should be healthy");

      // If RPC is healthy, it means getForestRoot call succeeded
      assert(
        !healthData.components.rpc.message || !healthData.components.rpc.message.includes("failed"),
        "RPC should not have error messages"
      );
    });

    it("Should successfully create a bucket and file", async () => {
      const bucketName = "backend-test-bucket";

      // Create a new bucket with the MSP
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      const valuePropId = valueProps[0].id;

      const newBucketEvent = await userApi.createBucket(bucketName, valuePropId);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("NewBucket event data does not match expected type");
      }
      bucketId = newBucketEventDataBlob.bucketId.toString();

      // Get the root of the created bucket
      freshBucketRoot = (await userApi.rpc.storagehubclient.getForestRoot(bucketId)).unwrap();

      // Load a file into storage to get its metadata, then remove it from the user's node storage so it doesn't get sent to the MSP automatically.
      const userAddress = ETH_SH_USER_ADDRESS.slice(2);
      const file = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        fileLocation,
        userAddress,
        bucketId
      );
      fileKey = file.file_key;
      fileMetadata = file.file_metadata;

      await userApi.rpc.storagehubclient.removeFilesFromFileStorage([fileKey]);

      // Issue the storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            fileMetadata.location,
            fileMetadata.fingerprint,
            fileMetadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.msp1.expectedPeerId],
            { Custom: 2 }
          )
        ],
        signer: ethShUser
      });

      // Poll until the file is expected
      await waitFor({
        lambda: async () => (await msp1Api.rpc.storagehubclient.isFileKeyExpected(fileKey)).isTrue
      });
    });

    it("Prepare upload form", async () => {
      // Ensure prerequisite data is present
      assert(fileMetadata, "Should have some file metadata from bucket and file creation");

      const localSource = "docker/resource/whatsup.jpg";

      // Prepare a multipart HTTP request to send to the backend's upload endpoint
      const fileBuffer = fs.readFileSync(path.join("..", localSource));
      originalFileBuffer = fileBuffer;
      form = new FormData();

      // SCALE-encode the file metadata and add it to the multipart form
      const FileMetadataCodec = $.object(
        $.field("owner", $.uint8Array),
        $.field("bucket_id", $.uint8Array),
        $.field("location", $.uint8Array),
        $.field("file_size", $.compact($.u64)),
        $.field("fingerprint", $.sizedArray($.u8, 32))
      );
      const encoded_file_metadata = FileMetadataCodec.encode(fileMetadata);
      const fileMetadataBlob = new Blob([Buffer.from(encoded_file_metadata)], {
        type: "application/octet-stream"
      });
      form.append("file_metadata", fileMetadataBlob, "file_metadata");

      // Add the file data stream to the multipart form
      const fileBlob = new Blob([fileBuffer], { type: "image/jpeg" });
      form.append("file", fileBlob, path.basename(fileLocation));
    });

    it.skip(
      "Should not upload file as other user",
      { todo: "when backend checks user permissions" },
      async () => {
        // Ensure prerequisite data is present
        assert(bucketId, "Bucket should have been created");
        assert(fileKey, "File should have been created");
        assert(form, "Upload form should be ready");

        // Generate a JWT token for Baltathar using the backend's auth endpoints
        // Trying to upload this file with it should fail
        const baltatharToken = await fetchJwtToken(
          BALTATHAR_PRIVATE_KEY,
          SH_EVM_SOLOCHAIN_CHAIN_ID
        );

        // Send the HTTP request to backend upload endpoint
        const baltatharUploadResponse = await fetch(
          `http://localhost:8080/buckets/${bucketId}/upload/${fileKey}`,
          {
            method: "PUT",
            body: form,
            headers: {
              Authorization: `Bearer ${baltatharToken}`
            }
          }
        );

        // Verify that the backend upload failed
        strictEqual(
          baltatharUploadResponse.status,
          401,
          "Upload should return UNAUTHORIZED status"
        );
      }
    );

    it("Should be able to retrieve unfulfilled file info", async () => {
      assert(fileKey, "File should have been created");
      assert(bucketId, "Bucket should have been created");

      // Generate a JWT token using the backend's auth endpoints
      userJWT = await fetchJwtToken(ETH_SH_USER_PRIVATE_KEY, SH_EVM_SOLOCHAIN_CHAIN_ID);

      const response = await fetch(
        `http://localhost:8080/buckets/${bucketId}/info/${fileKey.toHex()}`,
        {
          headers: {
            Authorization: `Bearer ${userJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/bucket/bucket_id/info/fileKey should return OK status");
      const file = (await response.json()) as FileInfo;

      strictEqual(file.status, "inProgress", "Should have not been fulfilled yet");
    });

    it("Should successfully upload file via the backend API", async () => {
      // Ensure prerequisite data is present
      assert(bucketId, "Bucket should have been created");
      assert(freshBucketRoot, "Bucket should have been created");
      assert(fileKey, "File should have been created");
      assert(form, "Upload form should be ready");
      assert(userJWT, "User authenticated with the backend");

      // Send the HTTP request to backend upload endpoint
      const uploadResponse = await fetch(
        `http://localhost:8080/buckets/${bucketId}/upload/${fileKey}`,
        {
          method: "PUT",
          body: form,
          headers: {
            Authorization: `Bearer ${userJWT}`
          }
        }
      );



      // Verify that the backend upload was successful
      // strictEqual(uploadResponse.status, 201, "Upload should return CREATED status");
      const responseBody = await uploadResponse.text();
      console.log(uploadResponse);
      const uploadResult = JSON.parse(responseBody);
      console.log(uploadResult);
      strictEqual(uploadResponse.status, 201, "Upload should return CREATED status");
      uploadedFileKeyHex = u8aToHex(fileKey);
      strictEqual(
        uploadResult.fileKey,
        uploadedFileKeyHex,
        "Response should contain correct file key"
      );
      strictEqual(
        `0x${uploadResult.bucketId}`,
        bucketId,
        "Response should contain correct bucket ID"
      );

      // Wait until the MSP has received and stored the file
      await msp1Api.wait.fileStorageComplete(fileKey);

      // Make sure the accept transaction from the MSP is in the tx pool
      await userApi.wait.mspResponseInTxPool(1);
      await userApi.wait.bspVolunteerInTxPool(1);

      // Seal the block containing the MSP's acceptance and the BSP's volunteer
      await userApi.block.seal();

      await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer");

      // Check that there's a `MspAcceptedStorageRequest` event
      const mspAcceptedStorageRequestEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      // Get its file key
      let mspAcceptedStorageRequestDataBlob: any;
      if (mspAcceptedStorageRequestEvent) {
        mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(
            mspAcceptedStorageRequestEvent.event
          ) && mspAcceptedStorageRequestEvent.event.data;
      }
      const acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();
      assert(acceptedFileKey, "MspAcceptedStorageRequest event were found");

      // The file key accepted by the MSP should be the same as the one uploaded
      assert(
        uploadedFileKeyHex === acceptedFileKey,
        "File key accepted by the MSP should be the same as the one uploaded"
      );

      // Ensure the file is now stored in the MSP's file storage
      await msp1Api.wait.fileStorageComplete(fileKey);

      // Check that the root of the bucket has changed
      const localBucketRoot = (await msp1Api.rpc.storagehubclient.getForestRoot(bucketId)).unwrap();
      assert(
        localBucketRoot.toString() !== freshBucketRoot.toString(),
        "Root of bucket should have changed"
      );
    });

    it("MSP should successfully distribute the file to BSPs who have volunteered to store it", async () => {
      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress
      });
    });

    it("Should successfully download a file via the backend API", async () => {
      // Ensure the upload test completed successfully
      assert(uploadedFileKeyHex, "Upload test must complete successfully before download test");
      assert(originalFileBuffer, "Original file buffer must be available from upload test");
      assert(userJWT, "User authenticated with the backend");

      const response = await fetch(`http://localhost:8080/download/${uploadedFileKeyHex}`, {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });
      strictEqual(response.status, 200, "Download endpoint should return 200 OK");

      const contentDisposition = response.headers.get("content-disposition");
      assert(contentDisposition, "Content disposition should be present");
      // Filename is preserved from the upload request
      strictEqual(
        contentDisposition,
        `attachment; filename="${TEST_FILE_NAME}"`,
        "Content disposition should match"
      );

      const arrayBuffer = await response.arrayBuffer();
      const downloadedBuffer = Buffer.from(arrayBuffer);

      strictEqual(
        downloadedBuffer.length,
        originalFileBuffer.length,
        "Downloaded file length should match uploaded file length"
      );
      assert(
        downloadedBuffer.equals(originalFileBuffer),
        "Downloaded file contents should match the uploaded file"
      );
    });
  }
);

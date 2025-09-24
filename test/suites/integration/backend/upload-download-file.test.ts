import assert, { strictEqual } from "node:assert";
import fs from "node:fs";
import path from "node:path";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import * as $ from "scale-codec";
import {
  type EnrichedBspApi,
  describeMspNet,
  shUser,
  waitFor,
  generateMockJWT
} from "../../../util";
import type { HealthResponse } from "./types";

await describeMspNet(
  "Backend file upload integration",
  {
    initialised: false,
    indexer: true,
    backend: true
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let uploadedFileKeyHex: string;
    let originalFileBuffer: Buffer;

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
        containerName: "storage-hub-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-backend-1",
        searchString: "Server listening on",
        timeout: 10000
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

    it("Should successfully upload a file via the backend API", async () => {
      const source = "res/whatsup.jpg";
      const localSource = "docker/resource/whatsup.jpg";
      const destination = "test/whatsup.jpg";
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
      const bucketId = newBucketEventDataBlob.bucketId.toString();

      // Get the root of the created bucket
      const bucketRoot = (await userApi.rpc.storagehubclient.getForestRoot(bucketId)).unwrap();

      // Load a file into storage to get its metadata, then remove it from the user's node storage so it doesn't get sent to the MSP automatically.
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      const { file_key, file_metadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        ownerHex,
        bucketId
      );
      await userApi.rpc.storagehubclient.removeFilesFromFileStorage([file_key]);

      // Issue the storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file_metadata.location,
            file_metadata.fingerprint,
            file_metadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Custom: 2 }
          )
        ],
        signer: shUser
      });

      // Poll until the file is expected
      await waitFor({
        lambda: async () => (await msp1Api.rpc.storagehubclient.isFileKeyExpected(file_key)).isTrue
      });

      // Prepare a multipart HTTP request to send to the backend's upload endpoint
      const fileBuffer = fs.readFileSync(path.join("..", localSource));
      originalFileBuffer = fileBuffer;
      const form = new FormData();

      // SCALE-encode the file metadata and add it to the multipart form
      const FileMetadataCodec = $.object(
        $.field("owner", $.uint8Array),
        $.field("bucket_id", $.uint8Array),
        $.field("location", $.uint8Array),
        $.field("file_size", $.compact($.u64)),
        $.field("fingerprint", $.sizedArray($.u8, 32))
      );
      const encoded_file_metadata = FileMetadataCodec.encode(file_metadata);
      const fileMetadataBlob = new Blob([Buffer.from(encoded_file_metadata)], {
        type: "application/octet-stream"
      });
      form.append("file_metadata", fileMetadataBlob, "file_metadata");

      // Add the file data stream to the multipart form
      const fileBlob = new Blob([fileBuffer], { type: "image/jpeg" });
      form.append("file", fileBlob, path.basename(source));

      // Generate a mock JWT token that matches the backend's mock
      // TODO: Once the backend has proper auth, we will have to update this (if the mock is removed)
      const mockJWT = generateMockJWT();

      // Send the HTTP request to backend upload endpoint
      const uploadResponse = await fetch(
        `http://localhost:8080/buckets/${bucketId}/upload/${file_key}`,
        {
          method: "PUT",
          body: form,
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      // Verify that the backend upload was successful
      strictEqual(uploadResponse.status, 201, "Upload should return CREATED status");
      const responseBody = await uploadResponse.text();
      const uploadResult = JSON.parse(responseBody);
      const hexFileKey = u8aToHex(file_key);
      strictEqual(uploadResult.fileKey, hexFileKey, "Response should contain correct file key");
      strictEqual(uploadResult.bucketId, bucketId, "Response should contain correct bucket ID");
      uploadedFileKeyHex = hexFileKey;

      // Wait until the MSP has received and stored the file
      await msp1Api.wait.fileStorageComplete(file_key);

      // Make sure the accept transaction from the MSP is in the tx pool
      await userApi.wait.mspResponseInTxPool(1);

      // Seal the block containing the MSP's acceptance
      await userApi.block.seal();

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
        hexFileKey === acceptedFileKey,
        "File key accepted by the MSP should be the same as the one uploaded"
      );

      // Ensure the file is now stored in the MSP's file storage
      await msp1Api.wait.fileStorageComplete(file_key);

      // Check that the root of the bucket has changed
      const localBucketRoot = (await msp1Api.rpc.storagehubclient.getForestRoot(bucketId)).unwrap();
      assert(
        localBucketRoot.toString() !== bucketRoot.toString(),
        "Root of bucket should have changed"
      );
    });

    it("Should successfully download a file via the backend API", async () => {
      // Ensure the upload test completed successfully
      assert(uploadedFileKeyHex, "Upload test must complete successfully before download test");
      assert(originalFileBuffer, "Original file buffer must be available from upload test");

      const response = await fetch(`http://localhost:8080/download/${uploadedFileKeyHex}`);
      strictEqual(response.status, 200, "Download endpoint should return 200 OK");

      const contentDisposition = response.headers.get("content-disposition");
      assert(contentDisposition, "Content disposition should be present");
      // Filename is preserved from the upload request
      strictEqual(
        contentDisposition,
        'attachment; filename="whatsup.jpg"',
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

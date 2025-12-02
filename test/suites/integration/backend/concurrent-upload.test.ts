import assert, { strictEqual } from "node:assert";
import fs from "node:fs";
import path from "node:path";
import type { H256 } from "@polkadot/types/interfaces";
import * as $ from "scale-codec";
import { describeMspNet, type EnrichedBspApi, waitFor } from "../../../util";
import { BACKEND_URI } from "../../../util/backend/consts";
import { fetchJwtToken } from "../../../util/backend/jwt";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import {
  ETH_SH_USER_ADDRESS,
  ETH_SH_USER_PRIVATE_KEY,
  ethMspKey,
  ethShUser
} from "../../../util/evmNet/keyring";

await describeMspNet(
  "Backend concurrent upload test",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true,
    logLevel: "debug"
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    // Use a large file (9.7MB) to ensure multiple batches and true concurrent uploads
    const TEST_FILE_NAME = "big_chart.jpg";

    let bucketId: string;
    const fileLocation = `test/${TEST_FILE_NAME}`;
    const source = `res/${TEST_FILE_NAME}`;
    let fileKey: H256;
    let fileMetadata: any;
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

    it("Should successfully create a bucket and prepare for concurrent upload", async () => {
      const bucketName = "concurrent-upload-test-bucket";

      // The default value proposition has too small of a max data limit for the bigger file,
      // so we need to add a new value proposition with a larger limit.
      const largeBucketLimit = 100n * 1024n * 1024n; // 100 MB

      // Get existing value prop to copy its price and commitment
      const existingValueProps =
        await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
          userApi.shConsts.DUMMY_MSP_ID
        );
      const existingValueProp = existingValueProps[0].valueProp;

      // Add the new value proposition to the MSP and get its ID
      await userApi.block.seal({
        calls: [
          userApi.tx.providers.addValueProp(
            existingValueProp.pricePerGigaUnitOfDataPerBlock,
            existingValueProp.commitment,
            largeBucketLimit
          )
        ],
        signer: ethMspKey
      });

      const valuePropAddedEvent = await userApi.assert.eventPresent("providers", "ValuePropAdded");
      const valuePropAddedEventDataBlob =
        userApi.events.providers.ValuePropAdded.is(valuePropAddedEvent.event) &&
        valuePropAddedEvent.event.data;
      assert(valuePropAddedEventDataBlob, "Event doesn't match Type");
      const valuePropId = valuePropAddedEventDataBlob.valuePropId.toString();

      // Create the bucket with the new value proposition ID
      const newBucketEvent = await userApi.createBucket(bucketName, valuePropId as `0x${string}`);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("NewBucket event data does not match expected type");
      }
      bucketId = newBucketEventDataBlob.bucketId.toString();

      // Load a file into storage to get its metadata
      // DO NOT remove it from the user's storage afterwards, so the user node uploads the file to the MSP via P2P
      const userAddress = ETH_SH_USER_ADDRESS.slice(2);
      const file = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        fileLocation,
        userAddress,
        bucketId
      );
      fileKey = file.file_key;
      fileMetadata = file.file_metadata;

      // Generate a JWT token for backend authentication
      userJWT = await fetchJwtToken(ETH_SH_USER_PRIVATE_KEY, SH_EVM_SOLOCHAIN_CHAIN_ID);

      // Prepare the multipart form for the backend upload
      const localSource = `docker/resource/${TEST_FILE_NAME}`;
      const fileBuffer = fs.readFileSync(path.join("..", localSource));
      form = new FormData();

      // SCALE-encode the file metadata
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

      // Add the file data stream
      const fileBlob = new Blob([fileBuffer], { type: "image/jpeg" });
      form.append("file", fileBlob, path.basename(fileLocation));

      assert(bucketId, "Bucket should have been created");
      assert(fileKey, "File should have been loaded");
      assert(fileMetadata, "File metadata should exist");
      assert(form, "Upload form should be ready");
      assert(userJWT, "User should be authenticated");
    });

    it("Should successfully upload file concurrently via P2P and backend", async () => {
      assert(bucketId, "Bucket should have been created");
      assert(fileKey, "File should have been created");
      assert(fileMetadata, "File metadata should exist");
      assert(form, "Upload form should be ready");
      assert(userJWT, "User authenticated with the backend");

      // Issue the storage request and seal the block
      // This triggers the user node to automatically send the file to MSP via P2P
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            fileMetadata.location,
            fileMetadata.fingerprint,
            fileMetadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Custom: 2 }
          )
        ],
        signer: ethShUser
      });

      // Wait for the MSP to process the storage request event and expect this file key
      await waitFor({
        lambda: async () => {
          const result = await msp1Api.rpc.storagehubclient.isFileKeyExpected(fileKey);
          return result.isTrue;
        },
        iterations: 50,
        delay: 50
      });

      // Now do the backend upload while P2P upload is happening concurrently
      const uploadResponse = await fetch(`${BACKEND_URI}/buckets/${bucketId}/upload/${fileKey}`, {
        method: "PUT",
        body: form,
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      // Verify that the backend upload was successful
      strictEqual(uploadResponse.status, 201, "Backend upload should return CREATED status");

      // Wait until the MSP has received and stored the file
      // This should succeed without FingerprintAndStoredFileMismatch errors
      await msp1Api.wait.fileStorageComplete(fileKey);

      // Verify that the chunk bitmap optimization worked by checking MSP logs
      // Should see "already present" messages indicating duplicate chunks were skipped
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName,
        searchString: "already present for file key",
        timeout: 5000
      });

      // Verify that the P2P upload completed successfully by checking user node logs
      const msp1PeerId = userApi.shConsts.NODE_INFOS.msp1.expectedPeerId;
      const fingerprint = `0x${Buffer.from(fileMetadata.fingerprint).toString("hex")}`;
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        searchString: `File upload complete. Peer PeerId("${msp1PeerId}") has the entire file fingerprint ${fingerprint}`,
        timeout: 15000
      });

      // Make sure the accept transaction from the MSP is in the tx pool
      await userApi.wait.mspResponseInTxPool(1);

      // Seal the block containing the MSP's acceptance
      await userApi.block.seal();

      // Check that there's a `MspAcceptedStorageRequest` event
      const mspAcceptedStorageRequestEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      // Verify the file key in the event matches
      let mspAcceptedStorageRequestDataBlob: any;
      if (mspAcceptedStorageRequestEvent) {
        mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(
            mspAcceptedStorageRequestEvent.event
          ) && mspAcceptedStorageRequestEvent.event.data;
      }
      const acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();
      assert(acceptedFileKey, "MspAcceptedStorageRequest event should be found");
      strictEqual(
        acceptedFileKey,
        fileKey.toString(),
        "File key accepted by the MSP should match the uploaded file key"
      );
    });

    it("Should verify file is correctly stored in MSP storage and forest", async () => {
      assert(fileKey, "File key should exist");
      assert(bucketId, "Bucket ID should exist");

      // Verify file is in MSP file storage
      const fileStorageResult = await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey);
      assert(fileStorageResult.isFileFound, "File should be in MSP file storage");

      // Wait until the file is in MSP forest
      await waitFor({
        lambda: async () => {
          const forestResult = await msp1Api.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
          return forestResult.isTrue;
        }
      });
    });
  }
);

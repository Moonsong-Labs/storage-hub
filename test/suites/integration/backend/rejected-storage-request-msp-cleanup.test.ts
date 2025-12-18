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
  ethShUser
} from "../../../util/evmNet/keyring";

await describeMspNet(
  "MSP storage cleanup after StorageRequest acceptance extrinsic failure",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true
  },
  ({ before, it, createUserApi, createMsp1Api }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;

    // Shared across tests
    let bucketId: string;
    let fileKey: H256;
    let form: FormData;
    let userJWT: string;
    let originalFileBuffer: Buffer;
    let uploadedFileKeyHex: string;

    const TEST_FILE_NAME = "whatsup.jpg";
    const fileLocation = `test/${TEST_FILE_NAME}`;
    const source = `res/${TEST_FILE_NAME}`;

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

    it("Create storage request and upload to MSP (no sealing acceptance)", async () => {
      // Create bucket
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      const valuePropId = valueProps[0].id;
      const newBucketEvent = await userApi.createBucket("backend-rejection-bucket", valuePropId);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;
      if (!newBucketEventDataBlob) throw new Error("NewBucket event data not found");
      bucketId = newBucketEventDataBlob.bucketId.toString();

      // Load file into user's local storage to get metadata and then remove so it doesn't auto-send
      const userAddress = ETH_SH_USER_ADDRESS.slice(2);
      const file = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        fileLocation,
        userAddress,
        bucketId
      );
      fileKey = file.file_key;
      const fileMetadata = file.file_metadata;
      await userApi.rpc.storagehubclient.removeFilesFromFileStorage([fileKey]);

      // Issue storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            fileMetadata.location,
            fileMetadata.fingerprint,
            fileMetadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.msp1.expectedPeerId],
            { Basic: null }
          )
        ],
        signer: ethShUser
      });

      // MSP expects the file
      await waitFor({
        lambda: async () => (await msp1Api.rpc.storagehubclient.isFileKeyExpected(fileKey)).isTrue
      });

      // Prepare upload form
      const localSource = `docker/resource/${TEST_FILE_NAME}`;
      const fileBuffer = fs.readFileSync(path.join("..", localSource));
      originalFileBuffer = fileBuffer;
      form = new FormData();

      // SCALE-encode file metadata
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

      const fileBlob = new Blob([fileBuffer], { type: "image/jpeg" });
      form.append("file", fileBlob, path.basename(fileLocation));

      // Auth token
      userJWT = await fetchJwtToken(ETH_SH_USER_PRIVATE_KEY, SH_EVM_SOLOCHAIN_CHAIN_ID);

      // Upload to backend
      const uploadResponse = await fetch(`${BACKEND_URI}/buckets/${bucketId}/upload/${fileKey}`, {
        method: "PUT",
        body: form,
        headers: { Authorization: `Bearer ${userJWT}` }
      });
      strictEqual(uploadResponse.status, 201, "Upload should return CREATED status");

      // File stored locally in MSP file storage
      await msp1Api.wait.fileStorageComplete(fileKey);

      // MSP acceptance extrinsic in tx pool (do not seal)
      await userApi.wait.mspResponseInTxPool(1);

      // Verify we can download
      uploadedFileKeyHex = fileKey.toHex();
      const preRejectDownload = await fetch(`${BACKEND_URI}/download/${uploadedFileKeyHex}`, {
        headers: { Authorization: `Bearer ${userJWT}` }
      });
      strictEqual(preRejectDownload.status, 200, "Download should succeed before rejection");
      const preArrayBuffer = await preRejectDownload.arrayBuffer();
      const downloadedBuffer = Buffer.from(preArrayBuffer);
      strictEqual(Buffer.from(preArrayBuffer).length, originalFileBuffer.length);
      assert(
        downloadedBuffer.equals(originalFileBuffer),
        "Downloaded file contents should match the uploaded file"
      );
    });

    it("Drop MSP SR acceptance extrinsic, advance to rejection, expect download to fail as MSP should have deleted the file", async () => {
      assert(fileKey, "File key should be available from previous step");

      // Remove MSP response extrinsic from tx pool
      await userApi.node.dropTxn({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets"
      });
      await userApi.block.seal();

      // Find expiration and advance to it
      const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
      assert(storageRequest.isSome, "Storage request should exist");
      const expiresAt = storageRequest.unwrap().expiresAt.toNumber();
      const expiredStorageRequestBlock = await userApi.block.skipTo(expiresAt);

      // Expect StorageRequestRejected event
      const StorageRequestRejectedEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "StorageRequestRejected",
        expiredStorageRequestBlock.events
      );
      assert(StorageRequestRejectedEvent, "StorageRequestRejected event not found");

      const StorageRequestEventData =
        userApi.events.fileSystem.StorageRequestRejected.is(StorageRequestRejectedEvent.event) &&
        StorageRequestRejectedEvent.event.data;
      assert(StorageRequestEventData, "StorageRequestRejectedEvent event data not found");
      strictEqual(
        StorageRequestEventData.fileKey.toString(),
        fileKey.toHex(),
        "File key should match the deleted file key"
      );

      // Storage Request should not exist anymore
      const storageRequestAfter = await userApi.query.fileSystem.storageRequests(fileKey);
      assert(storageRequestAfter.isNone, "Storage request should not exist anymore");

      await msp1Api.rpc.engine.finalizeBlock(expiredStorageRequestBlock.blockReceipt.blockHash);

      // Wait until the MSP detects the on-chain deletion and updates its local bucket forest
      await msp1Api.wait.fileDeletionFromFileStorage(fileKey.toHex());

      // Download should now fail (MSP should no longer have the file)
      const postRejectDownload = await fetch(`${BACKEND_URI}/download/${uploadedFileKeyHex}`, {
        headers: { Authorization: `Bearer ${userJWT}` }
      });
      strictEqual(
        postRejectDownload.status,
        404,
        "Download should fail before storage request rejection"
      );
    });
  }
);

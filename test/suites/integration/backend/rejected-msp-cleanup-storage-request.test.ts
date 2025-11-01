import assert, { strictEqual } from "node:assert";
import fs from "node:fs";
import path from "node:path";
import * as $ from "scale-codec";
import type { H256 } from "@polkadot/types/interfaces";
import { describeMspNet, type EnrichedBspApi, waitFor } from "../../../util";
import { fetchJwtToken } from "../../../util/backend/jwt";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import { ETH_SH_USER_ADDRESS, ETH_SH_USER_PRIVATE_KEY, ethShUser } from "../../../util/evmNet/keyring";

await describeMspNet(
  "Backend cleanup after MSP rejection (drop acceptance, expect 404)",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true,
    only: true
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
        containerName: "storage-hub-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Backend service is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-backend-1",
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
      // Make TTL short so rejection happens quickly
      const tickRangeToMaximumThreshold = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: { TickRangeToMaximumThreshold: null }
        })
      )
        .unwrap()
        .asRuntimeConfig.asTickRangeToMaximumThreshold.toNumber();
      const storageRequestTtlRuntimeParameter = {
        RuntimeConfig: { StorageRequestTtl: [null, tickRangeToMaximumThreshold] }
      } as const;
      await userApi.block.seal({
        calls: [userApi.tx.sudo.sudo(userApi.tx.parameters.setParameter(storageRequestTtlRuntimeParameter))]
      });

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
      const localSource = "docker/resource/" + TEST_FILE_NAME;
      const fileBuffer = fs.readFileSync(path.join("..", localSource));
      originalFileBuffer = fileBuffer;
      form = new FormData();

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
      const uploadResponse = await fetch(
        `http://localhost:8080/buckets/${bucketId}/upload/${fileKey}`,
        {
          method: "PUT",
          body: form,
          headers: { Authorization: `Bearer ${userJWT}` }
        }
      );
      strictEqual(uploadResponse.status, 201, "Upload should return CREATED status");

      // File stored locally in MSP file storage
      await msp1Api.wait.fileStorageComplete(fileKey);

      // MSP acceptance extrinsic in tx pool (do not seal)
      await userApi.wait.mspResponseInTxPool(1);

      // Verify we can download before sealing acceptance
      uploadedFileKeyHex = fileKey.toHex();
      const preRejectDownload = await fetch(`http://localhost:8080/download/${uploadedFileKeyHex}`, {
        headers: { Authorization: `Bearer ${userJWT}` }
      });
      strictEqual(preRejectDownload.status, 200, "Download should succeed before rejection");
      const preArrayBuffer = await preRejectDownload.arrayBuffer();
      strictEqual(Buffer.from(preArrayBuffer).length, originalFileBuffer.length);
    });

    it("Drop MSP acceptance, advance to rejection, assert and expect download to fail", async () => {
      assert(fileKey, "File key should be available from previous step");

      // Remove MSP response extrinsic from tx pool
      await userApi.node.dropTxn({
        module: "fileSystem",
        method: "mspRespondStorageRequestsMultipleBuckets"
      });

      // Find expiration and advance to it
      const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
      assert(storageRequest.isSome, "Storage request should exist");
      const expiresAt = storageRequest.unwrap().expiresAt.toNumber();
      const result = await userApi.block.skipTo(expiresAt);

      // Expect StorageRequestRejected event
      await userApi.assert.eventPresent("fileSystem", "StorageRequestRejected", result.events);

      // Download should now fail (backend should no longer serve the file)
      await waitFor({
        lambda: async () => {
          const resp = await fetch(`http://localhost:8080/download/${uploadedFileKeyHex}`, {
            headers: { Authorization: `Bearer ${userJWT}` }
          });
          return resp.status === 404;
        },
        iterations: 60,
        delay: 250
      });
    });
  }
);

import assert, { strictEqual } from "node:assert";
import type { Hash } from "@polkadot/types/interfaces";
import { describeMspNet, type EnrichedBspApi } from "../../../util";
import { BACKEND_URI } from "../../../util/backend/consts";
import { fetchJwtToken } from "../../../util/backend/jwt";
import type { Bucket, FileInfo, FileListResponse } from "../../../util/backend/types";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import {
  ETH_SH_USER_ADDRESS,
  ETH_SH_USER_PRIVATE_KEY,
  ethShUser
} from "../../../util/evmNet/keyring";

await describeMspNet(
  "Backend bucket endpoints",
  {
    indexer: true,
    backend: true,
    runtimeType: "solochain"
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let userJWT: string;

    const bucketName = "backend-test-bucket";
    let bucketId: string;

    let fileKey: Hash;
    const fileLocationSubPath = "test";
    const fileLocationBasename = "whatsup.jpg";
    const fileLocation = `${fileLocationSubPath}/${fileLocationBasename}`;

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
        timeout: 10000
      });

      userJWT = await fetchJwtToken(ETH_SH_USER_PRIVATE_KEY, SH_EVM_SOLOCHAIN_CHAIN_ID);
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Should successfully list no buckets", async () => {
      assert(userJWT, "User token is initialized");

      const response = await fetch(`${BACKEND_URI}/buckets`, {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      strictEqual(response.status, 200, "/buckets should return OK status");

      const buckets = (await response.json()) as Bucket[];

      strictEqual(buckets.length, 0);
    });

    it("Should create a bucket with a file", async () => {
      assert(userJWT, "User token is initialized");

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
      const newBucketId = newBucketEventDataBlob.bucketId.toString();
      bucketId = newBucketId.slice(2);

      const source = "res/whatsup.jpg";

      const userAddress = ETH_SH_USER_ADDRESS.slice(2);
      const { file_key, file_metadata } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        fileLocation,
        userAddress,
        newBucketId
      );
      fileKey = file_key;

      // Issue the storage request
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketId,
            file_metadata.location,
            file_metadata.fingerprint,
            file_metadata.file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            // match replication target with number of BSPs
            // to ensure request can be fulfilled
            { Custom: 1 }
          )
        ],
        signer: ethShUser
      });

      // Wait until the MSP has received and stored the file
      await msp1Api.wait.fileStorageComplete(fileKey);
    });

    it("Should successfully get specific bucket info", async () => {
      assert(userJWT, "User token is initialized");
      assert(bucketId, "Bucket should have been created");

      const response = await fetch(`${BACKEND_URI}/buckets/${bucketId}`, {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      strictEqual(response.status, 200, "/bucket/bucket_id should return OK status");

      const bucket = (await response.json()) as Bucket;

      strictEqual(bucket.bucketId, bucketId, "Returned bucket should match the one in the query");
      strictEqual(bucket.name, bucketName, "Should have same name as creation");
    });

    it("Should successfully list user buckets", async () => {
      assert(userJWT, "User token is initialized");
      assert(bucketId, "Bucket should have been created");

      const response = await fetch(`${BACKEND_URI}/buckets`, {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      strictEqual(response.status, 200, "/buckets should return OK status");

      const buckets = (await response.json()) as Bucket[];

      assert(buckets.length > 0, "should contain at least the bucket added during init");

      const sample_bucket = buckets.find((bucket) => bucket.bucketId === bucketId);
      assert(sample_bucket, "list should include bucket added in initialization");
    });

    it("Should successfully get bucket files", async () => {
      assert(userJWT, "User token is initialized");
      assert(bucketId, "Bucket should have been created");

      const response = await fetch(`${BACKEND_URI}/buckets/${bucketId}/files`, {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      strictEqual(response.status, 200, "/bucket/bucket_id/files should return OK status");

      const fileList = (await response.json()) as FileListResponse;

      strictEqual(fileList.bucketId, bucketId, "file list's bucket id should match queried");

      const files = fileList.tree;
      strictEqual(files.name, "/", "First entry of bucket should be root");
      assert(files.children.length > 0, "At least one file in the root");

      const test = files.children.find((entry) => entry.name === fileLocationSubPath);
      assert(test, `Should have a folder named '${fileLocationSubPath}'`);
      assert(test.type === "folder", "Child entry should be a folder");
    });

    it("Should successfully get bucket files subpath", async () => {
      assert(userJWT, "User token is initialized");
      assert(bucketId, "Bucket should have been created");

      const response = await fetch(
        `${BACKEND_URI}/buckets/${bucketId}/files?path=${fileLocationSubPath}`,
        {
          headers: {
            Authorization: `Bearer ${userJWT}`
          }
        }
      );

      strictEqual(
        response.status,
        200,
        "/bucket/bucket_id/files?path=path should return OK status"
      );

      const fileList = (await response.json()) as FileListResponse;

      strictEqual(fileList.bucketId, bucketId, "file list's bucket id should match queried");

      const files = fileList.tree;
      strictEqual(files.name, fileLocationSubPath, "First entry should be the folder of the path");

      assert(files.children.length > 0, `At least one file in the ${fileLocationSubPath} folder`);

      const whatsup = files.children.find((entry) => entry.name === fileLocationBasename);
      assert(whatsup, `Should have a file named '${fileLocationBasename}'`);

      assert(whatsup.type === "file", "Child entry should be file");
      strictEqual(whatsup.status, "inProgress", "Child entry should be 'inProgress'"); // No BSPs received file yet
      strictEqual(
        whatsup.fileKey,
        fileKey.toHex().slice(2),
        "Returned file key matches the one at time of creation"
      );
    });

    it("Should be able to fulfill storage request", async () => {
      // Seal block containing the MSP's first response.
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Wait for the BSPs to volunteer and confirm storing the file so the storage request gets fulfilled.
      await userApi.wait.storageRequestNotOnChain(fileKey);
    });

    it("Should successfully get file info by key", async () => {
      assert(userJWT, "User token is initialized");
      assert(bucketId, "Bucket should have been created");

      const response = await fetch(`${BACKEND_URI}/buckets/${bucketId}/info/${fileKey.toHex()}`, {
        headers: {
          Authorization: `Bearer ${userJWT}`
        }
      });

      strictEqual(response.status, 200, "/bucket/bucket_id/info/fileKey should return OK status");

      const file = (await response.json()) as FileInfo;

      strictEqual(file.fileKey, fileKey.toHex().slice(2), "Should have same file key as queried");
      strictEqual(file.bucketId, bucketId, "Should have same bucket id as queried");

      strictEqual(file.location, fileLocation, "Should have same location as creation");
      strictEqual(file.status, "ready", "Should have been fulfilled");
    });
  }
);

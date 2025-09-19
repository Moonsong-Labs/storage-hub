import assert, { strictEqual } from "node:assert";
import { type EnrichedBspApi, type FileMetadata, describeMspNet, shUser } from "../../../util";
import { generateMockJWT } from "./util";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import type { Hash } from "@polkadot/types/interfaces";

interface Bucket {
  bucketId: string;
  name: string;
  root: string;
  is_public: boolean;
  size_bytes: number;
  value_prop_id: string;
  file_count: number;
}

type FileTree = {
  name: string;
} & ({
  type: 'file';
  size_bytes: number;
  file_key: string;
} | {
  type: 'folder';
  children: FileTree[]
});

interface FileListResponse {
  bucket_id: string;
  files: FileTree[]
}

interface FileInfo {
  fileKey: string;
  fingerprint: string;
  bucketId: string;
  location: string;
  size: number;
  isPublic: boolean;
  uploadedAt: string;
}

await describeMspNet(
  "Backend bucket endpoints",
  {
    indexer: true,
    backend: true,
  },
  ({ before, createMsp1Api, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let mockJWT: string;

    before(async () => {
      userApi = await createUserApi();
      mockJWT = generateMockJWT(u8aToHex(decodeAddress(userApi.accounts.shUser.address)).slice(2));

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

    it("Should succesfully list no buckets", async () => {
      const response = await fetch(
        `http://localhost:8080/buckets`,
        {
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/buckets should return OK status");

      const buckets = (await response.json()) as Bucket[];
      console.log("/buckets :", buckets);

      assert(buckets.length == 0);
    })

    const bucketName = "backend-test-bucket";
    let bucketId: string;

    it("Should create a bucket with the MSP", async () => {
      const newBucketEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("NewBucket event data does not match expected type");
      }
      bucketId = newBucketEventDataBlob.bucketId.toString();
      console.log(`created bucket: ${bucketId}`)
    })

    it("Should succesfully get specific bucket info", async () => {
      const response = await fetch(
        `http://localhost:8080/buckets/${bucketId}`,
        {
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/bucket/bucker_id should return OK status");

      const bucket = (await response.json()) as Bucket;
      console.log("/buckets/bid :", bucket);

      assert(bucket.bucketId == bucketId, "Returned bucket should match the one in the query");
    })

    it("Should succesfully list buckets", async () => {
      const response = await fetch(
        `http://localhost:8080/buckets`,
        {
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/buckets should return OK status");

      const buckets = (await response.json()) as Bucket[];
      console.log("/buckets :", buckets);

      assert(buckets.length > 0);

      const sample_bucket = buckets.find((bucket) => bucket.bucketId == bucketId);
      assert(sample_bucket, "list should include bucket added in initialization")
    })

    let file_key: Hash;
    const fileLocation = "test/whatsup.jpg";

    it("Should add a file in bucket", async () => {
      const source = "res/whatsup.jpg";

      const ownerHex = u8aToHex(decodeAddress(userApi.accounts.shUser.address)).slice(2);
      const result = await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        fileLocation,
        ownerHex,
        bucketId
      );
      file_key = result.file_key;

      const file_metadata = result.file_metadata;

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
    })

    it("Should succesfully get bucket files", async () => {
      const response = await fetch(
        `http://localhost:8080/buckets/${bucketId}/files`,
        {
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/bucket/bucket_id/files should return OK status");

      const fileList = (await response.json()) as FileListResponse;
      console.log("/buckets/bid/files :", fileList);

      assert(fileList.bucket_id == bucketId, "file list's bucket id should match queried");

      assert(fileList.files.length == 1, "File list should have exactly 1 entry");

      const files = fileList.files[0];
      assert(files.name === '/', "First entry of bucket should be root");
      assert(files.type === 'folder', "Root entry should be a folder");

      assert(files.children.length > 0, "At least one file in the root");

      const test = files.children.find((entry) => entry.name === "test");
      assert(test, "Should have a folder named 'test'");

      assert(test.type === 'folder');
    })

    it("Should succesfully get bucket files subpath", async () => {
      const response = await fetch(
        `http://localhost:8080/buckets/${bucketId}/files?path=test`,
        {
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/bucket/bucket_id/files?path=path should return OK status");

      const fileList = (await response.json()) as FileListResponse;
      console.log("/buckets/bid/files?path=path :", fileList);

      assert(fileList.bucket_id == bucketId, "file list's bucket id should match queried");

      assert(fileList.files.length == 1, "File list should have exactly 1 entry");

      const files = fileList.files[0];
      assert(files.name === 'test', "First entry should be the folder of the path");
      assert(files.type === 'folder', "First entry should be a folder");

      assert(files.children.length > 0, "At least one file in the test folder");

      const whatsup = files.children.find((entry) => entry.name === "whatsup.jpg");
      assert(whatsup, "Should have a file named 'whatsup.jpg'");

      assert(whatsup.type === 'file');
      assert(whatsup.file_key === file_key.toHex(), "Returned file key matches the one at time of creation");
    })

    it("Should succesfully get file info by key", async () => {
      const response = await fetch(
        `http://localhost:8080/buckets/${bucketId}/info/${file_key.toHex()}`,
        {
          headers: {
            Authorization: `Bearer ${mockJWT}`
          }
        }
      );

      strictEqual(response.status, 200, "/bucket/bucket_id/info/file_key should return OK status");

      const file = (await response.json()) as FileInfo;
      console.log("/buckets/bid/info/fid:", file);

      assert(file.fileKey === file_key.toHex(), "Should have same file key as queried");
      assert(file.bucketId === bucketId, "Should have same bucket id as queried");

      assert(file.location === fileLocation, "Should have same location as creation");
    })
  }
);

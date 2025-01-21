import assert from "node:assert";
import {
  BspNetTestApi,
  createBucket,
  sealBlock,
  ShConsts,
  shUser,
  type BspNetConfig
} from "../util";
import { NetworkLauncher } from "../util/netLaunch";
import type { H256 } from "@polkadot/types/interfaces";

const bspNetConfig: BspNetConfig = {
  noisy: process.env.NOISY === "1",
  rocksdb: process.env.ROCKSDB === "1",
  indexer: process.env.INDEXER === "1"
};

let bucketId: H256;

async function bootStrapNetwork() {
  await NetworkLauncher.create("fullnet", {
    ...bspNetConfig,
    initialised: "multi",
    extrinsicRetryTimeout: 60 * 30 // 30 minutes
  });

  console.log("✅ FullNet Bootstrap success");
}

async function createNewBucket() {
  await using api = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

  const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
    ShConsts.DUMMY_MSP_ID
  );
  const localValuePropId = valueProps[0].id;
  const newBucketEventEvent = await createBucket(
    api,
    "cool-new-bucket", // Bucket name
    localValuePropId, // Value proposition ID from MSP
    ShConsts.DUMMY_MSP_ID, // MSP ID
    shUser // Owner (the user)
  );
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  assert(newBucketEventDataBlob, "Event doesn't match Type");

  bucketId = newBucketEventDataBlob.bucketId;

  console.log("✅ Bucket created");
}

async function createStorageRequest() {
  await using api = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

  await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      bucketId, // Bucket ID
      "cool/path/to/messi.jpeg", // Location
      "0x297e3a4112c76eecd91bf3907d99aa1bd6abb8955b0896b31f97916287b10491", // Fingerprint
      55_385, // File size
      ShConsts.DUMMY_MSP_ID, // MSP ID, must match the one of the bucket
      [ShConsts.NODE_INFOS.user.expectedPeerId], // User peer IDs
      {
        LowSecurity: null
      } // Low security replication target
    ),
    shUser
  );

  console.log("✅ Storage request issued");
}

await bootStrapNetwork().catch((e) => {
  console.error("Error running bootstrap script:", e);
  console.log("❌ FullNet Bootstrap Demo failure");
  process.exitCode = 1;
});

await createNewBucket().catch((e) => {
  console.error("Error running create new bucket script:", e);
  console.log("❌ FullNet Bootstrap Demo failure");
  process.exitCode = 1;
});

await createStorageRequest().catch((e) => {
  console.error("Error running create storage request script:", e);
  console.log("❌ FullNet Bootstrap Demo failure");
  process.exitCode = 1;
});

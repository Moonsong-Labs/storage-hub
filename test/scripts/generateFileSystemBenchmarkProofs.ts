import { BspNetTestApi, sleep } from "../util";
import * as ShConsts from "../util/bspNet/consts";
import { NetworkLauncher, type NetLaunchConfig } from "../util/netLaunch";
import * as fs from "node:fs";
import { exec } from "node:child_process";
import type { Option } from "@polkadot/types";
import type { H256 } from "@polkadot/types/interfaces";

//! Configuration options for debugging
const skipProofGeneration = false;
const skipWritingProofs = false;
const keepNetworkAlive = false;

//! Configuration options for logging
const verbose = false;
const extraVerbose = false;

const GREEN_TEXT = "\x1b[32m";
const GRAY_TEXT = "\x1b[90m";
const RESET_TEXT = "\x1b[0m";

const fullNetConfig: NetLaunchConfig = {
  initialised: false,
  noisy: false,
  rocksdb: false,
  // Set up BSP with the maximum u32 weight so they volunteer immediately.
  bspStartingWeight: 4294967295n
};

async function generateBenchmarkProofs() {
  console.log(
    `${GREEN_TEXT}🏗️ Build proofs for benchmarking File System pallet test cases${RESET_TEXT}`
  );
  console.log("");
  console.log(`${GREEN_TEXT}▶ 🥾 Fullnet Bootstrap${RESET_TEXT}`);
  await NetworkLauncher.create("fullnet", fullNetConfig);

  await using userApi = await BspNetTestApi.create(
    `ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`
  );
  await using bspApi = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);
  await using mspApi = await BspNetTestApi.create(
    `ws://127.0.0.1:${ShConsts.NODE_INFOS.msp1.port}`
  );

  const storedFileKeys: string[] = [];
  const nonStoredFileKeys: string[] = [];
  const fileKeysAcceptedCases: string[][] = [];
  const fileKeyProofsAcceptedCases: string[] = [];
  const nonInclusionProofsCases: string[] = [];

  console.log(`${GREEN_TEXT}◀ ✅ BSPNet Bootstrap successful${RESET_TEXT}`);
  console.log("");

  //* =============================================================================
  console.log(`${GREEN_TEXT}▶ 🌳 Add files to the MSP's and BSP's Forests${RESET_TEXT}`);

  const sources = [
    "res/benchmarking/1.jpg",
    "res/benchmarking/2.jpg",
    "res/benchmarking/3.jpg",
    "res/benchmarking/4.jpg",
    "res/benchmarking/5.jpg",
    "res/benchmarking/6.jpg",
    "res/benchmarking/7.jpg",
    "res/benchmarking/8.jpg",
    "res/benchmarking/9.jpg",
    "res/benchmarking/10.jpg",
    "res/benchmarking/11.jpg",
    "res/benchmarking/12.jpg",
    "res/benchmarking/13.jpg",
    "res/benchmarking/14.jpg",
    "res/benchmarking/15.jpg",
    "res/benchmarking/16.jpg",
    "res/benchmarking/17.jpg",
    "res/benchmarking/18.jpg",
    "res/benchmarking/19.jpg",
    "res/benchmarking/20.jpg",
    "res/benchmarking/21.jpg",
    "res/benchmarking/22.jpg",
    "res/benchmarking/23.jpg",
    "res/benchmarking/24.jpg",
    "res/benchmarking/25.jpg",
    "res/benchmarking/26.jpg",
    "res/benchmarking/27.jpg",
    "res/benchmarking/28.jpg",
    "res/benchmarking/29.jpg",
    "res/benchmarking/30.jpg",
    "res/benchmarking/31.jpg",
    "res/benchmarking/32.jpg",
    "res/benchmarking/33.jpg",
    "res/benchmarking/34.jpg",
    "res/benchmarking/35.jpg",
    "res/benchmarking/36.jpg",
    "res/benchmarking/37.jpg",
    "res/benchmarking/38.jpg",
    "res/benchmarking/39.jpg",
    "res/benchmarking/40.jpg"
  ];
  const locations = [
    "test/1.jpg",
    "test/2.jpg",
    "test/3.jpg",
    "test/4.jpg",
    "test/5.jpg",
    "test/6.jpg",
    "test/7.jpg",
    "test/8.jpg",
    "test/9.jpg",
    "test/10.jpg",
    "test/11.jpg",
    "test/12.jpg",
    "test/13.jpg",
    "test/14.jpg",
    "test/15.jpg",
    "test/16.jpg",
    "test/17.jpg",
    "test/18.jpg",
    "test/19.jpg",
    "test/20.jpg",
    "test/21.jpg",
    "test/22.jpg",
    "test/23.jpg",
    "test/24.jpg",
    "test/25.jpg",
    "test/26.jpg",
    "test/27.jpg",
    "test/28.jpg",
    "test/29.jpg",
    "test/30.jpg",
    "test/31.jpg",
    "test/32.jpg",
    "test/33.jpg",
    "test/34.jpg",
    "test/35.jpg",
    "test/36.jpg",
    "test/37.jpg",
    "test/38.jpg",
    "test/39.jpg",
    "test/40.jpg"
  ];
  const bucketName = "benchmarking-bucket";

  // Create the bucket first and get its ID.
  const newBucketEvent = await userApi.file.newBucket(bucketName);
  const newBucketEventDataBlob =
    userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;
  if (!newBucketEventDataBlob) {
    throw new Error("Failed to create new bucket");
  }
  const bucketId = newBucketEventDataBlob.bucketId;

  // Upload files to the MSP but not all since we need non-inclusion forest proofs.
  for (let i = 0; i < sources.length / 2; i++) {
    console.log(`Uploading file ${i + 1} of ${sources.length / 2}`);
    const source = sources[i];
    const destination = locations[i];

    const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketId);
    storedFileKeys.push(fileMetadata.fileKey);

    await userApi.wait.bspVolunteerInTxPool(1);
    await userApi.wait.mspResponseInTxPool(1);
    await userApi.sealBlock();
    await mspApi.wait.mspFileStorageComplete(fileMetadata.fileKey);
    await bspApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await userApi.wait.bspStored(1);
  }

  // Upload the remaining files to the BSP (with another MSP), to have all file keys and to be able to get all key proofs easily.
  // Create a new bucket first and get its ID.
  const otherBucketName = "benchmarking-bucket-2";
  const otherBucketEvent = await userApi.file.newBucket(
    otherBucketName,
    undefined,
    undefined,
    ShConsts.DUMMY_MSP_ID_2
  );
  const otherBucketEventDataBlob =
    userApi.events.fileSystem.NewBucket.is(otherBucketEvent) && otherBucketEvent.data;
  if (!otherBucketEventDataBlob) {
    throw new Error("Failed to create new bucket");
  }
  const otherBucketId = otherBucketEventDataBlob.bucketId;
  for (let i = sources.length / 2; i < sources.length; i++) {
    console.log(`Uploading file ${i + 1} of ${sources.length}`);
    const source = sources[i];
    const destination = locations[i];

    const fileMetadata = await userApi.file.newStorageRequest(
      source,
      destination,
      otherBucketId,
      undefined,
      ShConsts.DUMMY_MSP_ID_2
    );
    nonStoredFileKeys.push(fileMetadata.fileKey);

    await userApi.wait.bspVolunteer(1);
    await bspApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await userApi.wait.bspStored(1);
  }

  // Sort the stored and non-stored file keys.
  storedFileKeys.sort();
  verbose && console.log("Sorted stored file keys: ", storedFileKeys);
  nonStoredFileKeys.sort();
  verbose && console.log("Sorted non-stored file keys: ", nonStoredFileKeys);

  // Wait for the BSP to add the last confirmed file to its Forest.
  await sleep(500);

  // Get the root of the Bucket's Forest after adding the 20 included files.
  const bucketIdOption: Option<H256> = userApi.createType("Option<H256>", bucketId);
  const bucketForestRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketIdOption);
  const bucketRoot = bucketForestRoot.toString().slice(2);
  verbose && console.log("Bucket forest root: ", bucketForestRoot.toString());

  // Get the root of the BSP's forest after adding all files.
  const bspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
  const bspRoot = bspForestRoot.toString().slice(2);
  verbose && console.log("BSP forest root: ", bspForestRoot.toString());

  console.log(`${GREEN_TEXT}◀ ✅ Added files to the MSP's and BSP's Forests${RESET_TEXT}`);
  console.log("");

  if (skipProofGeneration) {
    console.log(`${GRAY_TEXT}Skipping proof generation${RESET_TEXT}`);
    console.log(`${GRAY_TEXT}Exiting...${RESET_TEXT}`);

    await tearDownNetwork();

    return;
  }

  //* =============================================================================
  console.log(
    `${GREEN_TEXT}▶ 📦 Generate non-inclusion forest proofs and file key proofs.${RESET_TEXT}`
  );
  console.log(
    `${GREEN_TEXT} These are going to be used in msp_respond_storage_requests_multiple_buckets and bsp_confirm_storing${RESET_TEXT}`
  );

  // * For a MSP accepting storage requests, we need to generate a non-inclusion proof to be used
  // * when accepting from one to 10 (MaxBatchMspRespondStorageRequests) file keys for a bucket.
  // To do this, we simply generate a non-inclusion forest proof for N file keys (which must be the ones not
  // added to the Forest) and add the proof to the array. Then, generate for each file key its proof.
  // * For a BSP confirming storing, we need the same thing (a non-inclusion forest proof for 1 to 10 file keys that
  // * the BSP wants to confirm) and the file key proofs for each one of the file keys. We can reutilize the
  // * non-inclusion proofs for the MSP since it's the same, setting the BSP root to that of the bucket.

  for (let i = 1; i <= 10; i++) {
    // We generate a non-inclusion proof for i file keys of the nonStoredFileKeys array.
    const fileKeysToAccept = nonStoredFileKeys.slice(0, i);
    const nonInclusionProof = await mspApi.rpc.storagehubclient.generateForestProof(
      bucketIdOption,
      fileKeysToAccept
    );

    // Then, generate the file key proofs for each one of the file keys.
    const fileKeyProofs = [];
    for (const fileKey of fileKeysToAccept) {
      const fileKeyProof = await bspApi.rpc.storagehubclient.generateFileKeyProof(
        fileKey,
        fileKey,
        ShConsts.DUMMY_MSP_ID
      );
      fileKeyProofs.push(fileKeyProof);
    }

    verbose && console.log("\n\n Non-inclusion proof:");
    verbose && console.log(nonInclusionProof);
    verbose && console.log(`\n\n ${i} file keys to accept:`);
    verbose && console.log(fileKeysToAccept);
    verbose && extraVerbose && console.log("File key proofs for those file keys:");
    verbose && extraVerbose && console.log(fileKeyProofs);

    // Remove the 0x prefix from the proofs and the file keys. Save only the last file key proof since it's the one added from the previous iteration
    const lastFileKeyProofHexStr = fileKeyProofs[fileKeyProofs.length - 1].toString().slice(2);
    const nonInclusionProofHexStr = nonInclusionProof.toString().slice(2);
    for (const i in fileKeysToAccept) {
      fileKeysToAccept[i] = fileKeysToAccept[i].slice(2);
    }

    // Add the file keys and proofs to the arrays.
    fileKeysAcceptedCases.push(fileKeysToAccept);
    fileKeyProofsAcceptedCases.push(lastFileKeyProofHexStr);
    nonInclusionProofsCases.push(nonInclusionProofHexStr);
  }

  console.log(
    `${GREEN_TEXT}◀ ✅ Generated a non-inclusion proof for 1 to 10 file keys, each with its file key proof.${RESET_TEXT}`
  );
  console.log("");

  console.log(`${GREEN_TEXT}▶ 📦 Generate inclusion forest proof and file key proof.${RESET_TEXT}`);
  console.log(
    `${GREEN_TEXT} This is going to be used in extrinsics that allow a Provider to stop storing a file.${RESET_TEXT}`
  );

  // * For a Provider that wants to stop storing a file (or a user that calls delete_file), we need to generate
  // * an inclusion forest proof for the file key.

  // Get the file key for which to generate the inclusion proof. Since the BSP has all file keys, we can get the first one.
  const fileKeyForInclusionProof = storedFileKeys[0];

  // Generate the inclusion proof for that file key.
  const inclusionProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
    fileKeyForInclusionProof
  ]);

  verbose && console.log("\n\n Inclusion proof:");
  verbose && console.log(inclusionProof);

  // Remove the 0x prefix from the proof and the file key.
  const inclusionProofHexStr = inclusionProof.toString().slice(2);
  const fileKeyForInclusionProofHexStr = fileKeyForInclusionProof.slice(2);

  console.log(
    `${GREEN_TEXT}◀ ✅ Generated inclusion forest proof and file key proof.${RESET_TEXT}`
  );
  console.log("");

  if (skipWritingProofs) {
    console.log(`${GRAY_TEXT}Skipping writing proofs${RESET_TEXT}`);
    console.log(`${GRAY_TEXT}Exiting...${RESET_TEXT}`);

    await tearDownNetwork();

    return;
  }

  //* =============================================================================
  console.log(
    `${GREEN_TEXT}▶ 📦 Write rust file with MSP and BSP file keys and proofs${RESET_TEXT}`
  );

  const mspIdStr = `hex::decode("${ShConsts.DUMMY_MSP_ID.slice(2)}").expect("MSP ID should be a decodable hex string")`;

  const bucketRootStr = `hex::decode("${bucketRoot}").expect("Bucket root should be a decodable hex string")`;

  const userAccountStr = `<AccountId32 as Ss58Codec>::from_ss58check("${ShConsts.NODE_INFOS.user.AddressId}").expect("User account should be a decodable string")`;

  let proofsStr = "";
  for (const [index, proof] of nonInclusionProofsCases.entries()) {
    const proofVec = `hex::decode("${proof}").expect("Proof should be a decodable hex string")`;
    proofsStr += `${index + 1} => ${proofVec},\n        `;
  }

  let fileKeysStr = "";
  for (const [index, fileKeysToAccept] of fileKeysAcceptedCases.entries()) {
    let fileKeysToAcceptArrayStr = "";
    for (const fileKey of fileKeysToAccept) {
      fileKeysToAcceptArrayStr += `hex::decode("${fileKey}").expect("Proof should be a decodable hex string"),\n            `;
    }
    fileKeysStr += `${index + 1} => vec![\n            ${fileKeysToAcceptArrayStr}\n        ],\n        `;
  }

  let fileKeyProofsStr = "";
  for (const [index, fileKeyProof] of fileKeyProofsAcceptedCases.entries()) {
    const fileKeyProofVec = `hex::decode("${fileKeyProof}").expect("File key proof should be a decodable hex string")`;
    fileKeyProofsStr += `${index} => ${fileKeyProofVec},\n        `;
  }

  const bspIdStr = `hex::decode("${ShConsts.DUMMY_BSP_ID.slice(2)}").expect("BSP ID should be a decodable hex string")`;

  const bspRootStr = `hex::decode("${bspRoot}").expect("BSP root should be a decodable hex string")`;

  const inclusionProofStr = `hex::decode("${inclusionProofHexStr}").expect("Inclusion proof should be a decodable hex string")`;

  const fileKeyForInclusionProofStr = `hex::decode("${fileKeyForInclusionProofHexStr}").expect("File key for inclusion proof should be a decodable hex string")`;

  const template = fs.readFileSync(
    "../pallets/file-system/src/benchmark_proofs_template.rs",
    "utf8"
  );
  const rustCode = template
    .replace("{{date}}", new Date().toISOString())
    .replace("{{msp_id}}", mspIdStr)
    .replace("{{bucket_root}}", bucketRootStr)
    .replace("{{user_account}}", userAccountStr)
    .replace("{{non_inclusion_proofs}}", proofsStr)
    .replace("{{file_keys_to_accept}}", fileKeysStr)
    .replace("{{file_key_proofs}}", fileKeyProofsStr)
    .replace("{{bsp_id}}", bspIdStr)
    .replace("{{bsp_root}}", bspRootStr)
    .replace("{{inclusion_proof}}", inclusionProofStr)
    .replace("{{file_key_inclusion_proof}}", fileKeyForInclusionProofStr);

  fs.writeFileSync("../pallets/file-system/src/benchmark_proofs.rs", rustCode);

  console.log(
    `${GREEN_TEXT}◀ ✅ Wrote rust file with provider ID, bucket root, file keys and proofs${RESET_TEXT}`
  );
  console.log("");

  await tearDownNetwork();
}

async function tearDownNetwork() {
  if (keepNetworkAlive) {
    console.log(
      `${GRAY_TEXT}Keeping network alive. Make sure to manually stop the network when you're done.${RESET_TEXT}`
    );
    console.log(`${GRAY_TEXT}Exiting...${RESET_TEXT}`);
    return;
  }

  console.log(`${GREEN_TEXT}▶ 💣 Tearing down network${RESET_TEXT}`);
  exec("pnpm docker:stop:generateFileSystemBenchmarkProofs");

  console.log(`${GREEN_TEXT}◀ ✅ Network torn down${RESET_TEXT}`);
  console.log("");
}

generateBenchmarkProofs().catch((e) => {
  console.error("Error running generate File System benchmark proofs script:", e);
  console.error(
    "You might need to run `pnpm docker:stop:generateFileSystemBenchmarkProofs` to stop the network"
  );

  process.exitCode = 1;
});
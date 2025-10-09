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

  const storedFileKeysPerBucket: string[][] = [];
  const nonStoredFileKeysPerBucket: string[][] = [];
  const fileKeysAcceptedCases: string[][] = [];
  const fileKeyProofsAcceptedCases: string[][] = [];
  const nonInclusionProofsCases: string[][] = [];
  const fileKeysForBspConfirmCases: string[][] = [];
  const fileKeyProofsForBspConfirmCases: string[] = [];
  const bucketIds: H256[] = [];
  const bucketRoots: string[] = [];

  console.log(`${GREEN_TEXT}◀ ✅ Fullnet Bootstrap successful${RESET_TEXT}`);
  console.log("");

  //* =============================================================================
  console.log(
    `${GREEN_TEXT}▶ 🌳 Creating buckets and adding files to the MSP's and BSP's Forests${RESET_TEXT}`
  );

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

  // Create 10 buckets and upload files to them.
  const bucketAmount = 10;
  for (let i = 0; i < bucketAmount; i++) {
    const bucketName = `benchmarking-bucket-${i}`;
    const storedFileKeysForBucket: string[] = [];
    const nonStoredFileKeysForBucket: string[] = [];

    // Create the bucket and get its ID.
    const newBucketEvent = await userApi.file.newBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;
    if (!newBucketEventDataBlob) {
      throw new Error("Failed to create new bucket");
    }
    const bucketId = newBucketEventDataBlob.bucketId;

    // Upload files to the MSP to that bucket, but not all since we need non-inclusion forest proofs.
    for (let j = 0; j < sources.length / 2; j++) {
      console.log(
        `File upload: Uploading file ${j + 1} of ${sources.length / 2} to MSP bucket ${i + 1}...`
      );
      const source = sources[j];
      const destination = locations[j];

      const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketId);
      storedFileKeysForBucket.push(fileMetadata.fileKey);

      await userApi.wait.bspVolunteerInTxPool(1);
      await userApi.wait.mspResponseInTxPool(1);
      await userApi.block.seal();
      await mspApi.wait.fileStorageComplete(fileMetadata.fileKey);
      await bspApi.wait.fileStorageComplete(fileMetadata.fileKey);
      await userApi.wait.bspStored({ expectedExts: 1 });
    }

    // Save the bucket ID for later use.
    bucketIds.push(bucketId);

    // Save the stored file keys for this bucket.
    storedFileKeysPerBucket.push(storedFileKeysForBucket);

    // Upload the remaining files to the BSP (with another MSP and bucket), to have all file keys and to be able to get all key proofs easily.
    // Create the other bucket first and get its ID.
    const otherBucketName = `other-benchmarking-bucket-${i}`;
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

    // Upload the remaining files to the other bucket.
    for (let j = sources.length / 2; j < sources.length; j++) {
      console.log(`File upload: Uploading file ${j + 1} of ${sources.length} to BSP...`);
      const source = sources[j];
      const destination = locations[j];

      const fileMetadata = await userApi.file.newStorageRequest(
        source,
        destination,
        otherBucketId,
        undefined,
        ShConsts.DUMMY_MSP_ID_2
      );
      nonStoredFileKeysForBucket.push(fileMetadata.fileKey);

      await userApi.wait.bspVolunteer(1);
      await bspApi.wait.fileStorageComplete(fileMetadata.fileKey);
      await userApi.wait.bspStored({ expectedExts: 1 });
    }

    // Save the non-stored file keys for the MSP's bucket.
    nonStoredFileKeysPerBucket.push(nonStoredFileKeysForBucket);
  }

  // Sort the stored and non-stored file keys.
  for (const storedFileKeys of storedFileKeysPerBucket) {
    storedFileKeys.sort();
  }
  verbose && console.log("Sorted stored file keys per bucket: ", storedFileKeysPerBucket);
  for (const nonStoredFileKeys of nonStoredFileKeysPerBucket) {
    nonStoredFileKeys.sort();
  }
  verbose && console.log("Sorted non-stored file keys per bucket: ", nonStoredFileKeysPerBucket);

  // Wait for the BSP to add the last confirmed file to its Forest.
  await sleep(500);

  // Get the root of each Bucket's forest after adding the 20 included files.
  for (const bucketId of bucketIds) {
    const bucketIdOption: Option<H256> = userApi.createType("Option<H256>", bucketId);
    const bucketForestRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketIdOption);
    const bucketRoot = bucketForestRoot.toString().slice(2);
    verbose && console.log("Bucket forest root: ", bucketForestRoot.toString());
    bucketRoots.push(bucketRoot);
  }

  // Get the root of the BSP's forest after adding all files.
  const bspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
  const bspRoot = bspForestRoot.toString().slice(2);
  verbose && console.log("BSP forest root: ", bspForestRoot.toString());

  console.log(
    `${GREEN_TEXT}◀ ✅ Successfully created required buckets and added files to the MSP's and BSP's Forests${RESET_TEXT}`
  );
  console.log("");

  if (skipProofGeneration) {
    console.log(`${GRAY_TEXT}Skipping proof generation${RESET_TEXT}`);
    console.log(`${GRAY_TEXT}Exiting...${RESET_TEXT}`);

    await tearDownNetwork();

    return;
  }

  //* =============================================================================
  console.log(
    `${GREEN_TEXT}▶ 📦 Generating non-inclusion forest proofs and file key proofs.${RESET_TEXT}`
  );
  console.log(
    `${GREEN_TEXT} These are going to be used in msp_respond_storage_requests_multiple_buckets and bsp_confirm_storing${RESET_TEXT}`
  );

  // * For a MSP accepting storage requests, we need to generate a non-inclusion proof to be used
  // * when accepting from 1 to 10 file keys and rejecting 1 to 10 file keys. This has to be done for 1 to 10 buckets.
  // * The reason we use 1 to 10 as the range is because it's big enough for Substrate to extrapolate the information
  // * given by the benchmarks to other cases with good precision while being small enough to not take ages to run.
  // * We also need to generate the file key proofs for each one of the file keys to be accepted.
  // To do this, we generate a non-inclusion forest proof for N file keys (which must be the ones not
  // added to the Forest in the previous step of this script) and add the proof to the array. Then, generate for each file key its proof.
  // Finally, repeat this for each bucket.
  // * For a BSP confirming storing, we need the same thing (a non-inclusion forest proof for 1 to 10 file keys that
  // * the BSP wants to confirm) and the file key proofs for each one of the file keys.
  // Since the BSP only requires one non-inclusion proof and 1 to 10 (MaxBatchBspConfirmStoring) file key proofs, we reuse the
  // non-inclusion proof from the first bucket (setting the root of the BSP to the root of that bucket) but generate the corresponding
  // file key proofs since the challenges for the BSP are different.

  const amountOfNonInclusionProofsToGenerate = 10;
  for (let i = 1; i <= amountOfNonInclusionProofsToGenerate; i++) {
    // We generate a non-inclusion proof for i file keys of the nonStoredFileKeys array for each bucket.
    const nonInclusionProofsForCase: string[] = [];
    const allFileKeysToAccept: string[] = [];
    const allFileKeyProofs = [];
    const fileKeysForBsp: string[] = [];
    const fileKeyProofsForBsp = [];

    // For each bucket, generate the non-inclusion proof for the first i file keys.
    for (let j = 0; j < bucketIds.length; j++) {
      const fileKeysToAcceptForBucket = nonStoredFileKeysPerBucket[j].slice(0, i);
      const bucketIdOption: Option<H256> = userApi.createType("Option<H256>", bucketIds[j]);
      const nonInclusionProof = await mspApi.rpc.storagehubclient.generateForestProof(
        bucketIdOption,
        fileKeysToAcceptForBucket
      );
      verbose && console.log(`\n\n Non-inclusion proof for bucket ${j}:`);
      verbose && console.log(nonInclusionProof);

      // Remove the 0x prefix from the proof and push it and the file keys to accept to the arrays.
      const nonInclusionProofHexStr = nonInclusionProof.toString().slice(2);
      nonInclusionProofsForCase.push(nonInclusionProofHexStr);
      allFileKeysToAccept.push(...fileKeysToAcceptForBucket);

      // If we are in the first bucket, generate the file key proofs for the BSP confirm.
      if (j === 0) {
        for (const fileKeyToAccept of fileKeysToAcceptForBucket) {
          fileKeysForBsp.push(fileKeyToAccept);
        }
        for (const fileKey of fileKeysForBsp) {
          const fileKeyProof = await bspApi.rpc.storagehubclient.generateFileKeyProofBspConfirm(
            ShConsts.DUMMY_BSP_ID,
            fileKey
          );
          fileKeyProofsForBsp.push(fileKeyProof);
        }
      }
    }

    // Then, generate the file key proofs for each one of the file keys to accept.
    for (const fileKey of allFileKeysToAccept) {
      const fileKeyProof = await bspApi.rpc.storagehubclient.generateFileKeyProofMspAccept(
        ShConsts.DUMMY_MSP_ID,
        fileKey
      );
      allFileKeyProofs.push(fileKeyProof);
    }
    verbose && console.log(`\n\n Case ${i} file keys to accept:`);
    verbose && console.log(allFileKeysToAccept);
    verbose && extraVerbose && console.log("File key proofs for those file keys:");
    verbose && extraVerbose && console.log(allFileKeyProofs);

    // Remove the 0x prefix from the proofs and the file keys.
    const allFileKeyProofHexStr = allFileKeyProofs.map((proof) => proof.toString().slice(2));
    for (const i in allFileKeysToAccept) {
      allFileKeysToAccept[i] = allFileKeysToAccept[i].slice(2);
    }
    const lastFileKeyProofForBspHexStr = fileKeyProofsForBsp[fileKeyProofsForBsp.length - 1]
      .toString()
      .slice(2);
    for (const i in fileKeysForBsp) {
      fileKeysForBsp[i] = fileKeysForBsp[i].slice(2);
    }

    // Add the file keys and proofs to the arrays.
    nonInclusionProofsCases.push(nonInclusionProofsForCase);
    fileKeysAcceptedCases.push(allFileKeysToAccept);
    fileKeyProofsAcceptedCases.push(allFileKeyProofHexStr);
    fileKeysForBspConfirmCases.push(fileKeysForBsp);
    fileKeyProofsForBspConfirmCases.push(lastFileKeyProofForBspHexStr);
  }

  console.log(
    `${GREEN_TEXT}◀ ✅ Generated non-inclusion proofs for 1 to 10 file keys, each with its file key proofs.${RESET_TEXT}`
  );
  console.log("");

  console.log(
    `${GREEN_TEXT}▶ 📦 Generating inclusion forest proof and the included file key proof.${RESET_TEXT}`
  );
  console.log(
    `${GREEN_TEXT} This is going to be used in extrinsics that allow a Provider to stop storing a file.${RESET_TEXT}`
  );

  // * For a Provider that wants to stop storing a file (or a user that calls delete_files), we need to generate
  // * an inclusion forest proof for a file key.

  // Get the file key for which to generate the inclusion proof. Since the BSP has all file keys, we can get the first one of the non-stored ones.
  const fileKeyForInclusionProof = nonStoredFileKeysPerBucket[0][0];

  // Generate the inclusion proof for that file key.
  const inclusionProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
    fileKeyForInclusionProof
  ]);

  // Get the metadata of that file key
  const fileMetadata = (
    await bspApi.rpc.storagehubclient.getFileMetadata(null, fileKeyForInclusionProof)
  ).unwrap();

  verbose && console.log("\n\n Inclusion proof:");
  verbose && console.log(inclusionProof);

  // Remove the 0x prefix from the proof, the file key and the file metadata.
  const inclusionProofHexStr = inclusionProof.toString().slice(2);
  const fileKeyForInclusionProofHexStr = fileKeyForInclusionProof.slice(2);
  const fileMetadataForInclusionProofOwnerHexStr = fileMetadata.owner.toString().slice(2);
  const fileMetadataForInclusionProofBucketIdHexStr = fileMetadata.bucket_id.toString().slice(2);
  const fileMetadataForInclusionProofLocationHexStr = fileMetadata.location.toString().slice(2);
  const fileMetadataForInclusionProofFingerprintHexStr = fileMetadata.fingerprint
    .toString()
    .slice(2);

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
    `${GREEN_TEXT}▶ 📦 Writing rust file with MSP and BSP info, buckets info, required proofs and file keys${RESET_TEXT}`
  );

  const mspIdStr = `hex::decode("${ShConsts.DUMMY_MSP_ID.slice(2)}").expect("MSP ID should be a decodable hex string")`;

  const userAccountStr = `<AccountId32 as Ss58Codec>::from_ss58check("${ShConsts.NODE_INFOS.user.AddressId}").expect("User account should be a decodable string")`;

  let bucketIdStr = "";
  for (const [index, bucketId] of bucketIds.entries()) {
    const bucketIdVec = `hex::decode("${bucketId.toString().slice(2)}").expect("Bucket ID should be a decodable hex string")`;
    bucketIdStr += `${index + 1} => ${bucketIdVec},\n		`;
  }

  let bucketRootStr = "";
  for (const [index, bucketRoot] of bucketRoots.entries()) {
    const bucketRootVec = `hex::decode("${bucketRoot}").expect("Bucket root should be a decodable hex string")`;
    bucketRootStr += `${index + 1} => ${bucketRootVec},\n		`;
  }

  let proofsStr = "";
  for (const [index, proofVector] of nonInclusionProofsCases.entries()) {
    let nonInclusionProofForBucketStr = "";
    for (const proof of proofVector) {
      nonInclusionProofForBucketStr += `hex::decode("${proof}").expect("Proof should be a decodable hex string"),\n            `;
    }
    proofsStr += `${index + 1} => vec![\n            ${nonInclusionProofForBucketStr}\n        ],\n        `;
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
  for (const [index, fileKeyProofs] of fileKeyProofsAcceptedCases.entries()) {
    let fileKeysToAcceptProofsArrayStr = "";
    for (const fileKeyProof of fileKeyProofs) {
      fileKeysToAcceptProofsArrayStr += `hex::decode("${fileKeyProof}").expect("Proof should be a decodable hex string"),\n            `;
    }
    fileKeyProofsStr += `${index + 1} => vec![\n            ${fileKeysToAcceptProofsArrayStr}\n        ],\n        `;
  }

  const bspIdStr = `hex::decode("${ShConsts.DUMMY_BSP_ID.slice(2)}").expect("BSP ID should be a decodable hex string")`;

  const bspRootStr = `hex::decode("${bspRoot}").expect("BSP root should be a decodable hex string")`;

  const inclusionProofStr = `hex::decode("${inclusionProofHexStr}").expect("Inclusion proof should be a decodable hex string")`;

  const fileKeyForInclusionProofStr = `hex::decode("${fileKeyForInclusionProofHexStr}").expect("File key for inclusion proof should be a decodable hex string")`;

  const fileMetadataOwnerStr = `hex::decode("${fileMetadataForInclusionProofOwnerHexStr}").expect("Owner in file metadata for inclusion proof should be a decodable hex string")`;
  const fileMetadataBucketIdStr = `hex::decode("${fileMetadataForInclusionProofBucketIdHexStr}").expect("Bucket ID in file metadata for inclusion proof should be a decodable hex string")`;
  const fileMetadataLocationStr = `hex::decode("${fileMetadataForInclusionProofLocationHexStr}").expect("Location in file metadata for inclusion proof should be a decodable hex string")`;
  const fileMetadataFingerprintStr = `hex::decode("${fileMetadataForInclusionProofFingerprintHexStr}").expect("Fingerprint in file metadata for inclusion proof should be a decodable hex string").as_slice().into()`;

  let fileKeysForBspStr = "";
  for (const [index, fileKeysToConfirm] of fileKeysForBspConfirmCases.entries()) {
    let fileKeysToConfirmArrayStr = "";
    for (const fileKey of fileKeysToConfirm) {
      fileKeysToConfirmArrayStr += `hex::decode("${fileKey}").expect("File key should be a decodable hex string"),\n            `;
    }
    fileKeysForBspStr += `${index + 1} => vec![\n            ${fileKeysToConfirmArrayStr}\n        ],\n        `;
  }

  let fileKeyProofsForBspConfirmStr = "";
  for (const [index, fileKeyProof] of fileKeyProofsForBspConfirmCases.entries()) {
    const fileKeyProofVec = `hex::decode("${fileKeyProof}").expect("File key proof should be a decodable hex string")`;
    fileKeyProofsForBspConfirmStr += `${index} => ${fileKeyProofVec},\n        `;
  }

  const template = fs.readFileSync(
    "../pallets/file-system/src/benchmark_proofs_template.rs",
    "utf8"
  );
  const rustCode = template
    .replace("{{date}}", new Date().toISOString())
    .replace("{{msp_id}}", mspIdStr)
    .replace("{{bucket_id}}", bucketIdStr)
    .replace("{{bucket_root}}", bucketRootStr)
    .replace("{{user_account}}", userAccountStr)
    .replace("{{non_inclusion_proofs}}", proofsStr)
    .replace("{{file_keys_to_accept}}", fileKeysStr)
    .replace("{{file_key_proofs}}", fileKeyProofsStr)
    .replace("{{bsp_id}}", bspIdStr)
    .replace("{{bsp_root}}", bspRootStr)
    .replace("{{inclusion_proof}}", inclusionProofStr)
    .replace("{{file_key_inclusion_proof}}", fileKeyForInclusionProofStr)
    .replace("{{file_key_metadata_inclusion_proof_owner}}", fileMetadataOwnerStr)
    .replace("{{file_key_metadata_inclusion_proof_bucket_id}}", fileMetadataBucketIdStr)
    .replace("{{file_key_metadata_inclusion_proof_location}}", fileMetadataLocationStr)
    .replace("{{file_key_metadata_inclusion_proof_file_size}}", fileMetadata.file_size)
    .replace("{{file_key_metadata_inclusion_proof_fingerprint}}", fileMetadataFingerprintStr)
    .replace("{{file_keys_for_bsp_confirm}}", fileKeysForBspStr)
    .replace("{{file_key_proofs_for_bsp_confirm}}", fileKeyProofsForBspConfirmStr);

  fs.writeFileSync("../pallets/file-system/src/benchmark_proofs.rs", rustCode);

  console.log(
    `${GREEN_TEXT}◀ ✅ Wrote rust file with MSP and BSP info, buckets info, required proofs and file keys${RESET_TEXT}`
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

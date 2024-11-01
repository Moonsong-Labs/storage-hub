import { describeBspNet, sleep, type EnrichedBspApi } from "../../../util";
import { DUMMY_BSP_ID } from "../../../util/bspNet/consts";
import * as fs from "node:fs";

describeBspNet(
  "Build proofs for benchmarking test cases",
  { networkConfig: "standard", skip: true },
  ({ before, createBspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    const seed = "0x0000000000000000000000000000000000000000000000000000000000000001";
    const fileKeys: string[] = [];
    const challengesCases: string[][] = [];
    const proofsCases: string[] = [];
    let root: string;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Add files to the BSP's Forest", async () => {
      const sources = ["res/adolphus.jpg", "res/cloud.jpg", "res/smile.jpg", "res/whatsup.jpg"];
      const locations = [
        "test/adolphus.jpg",
        "test/cloud.jpg",
        "test/smile.jpg",
        "test/whatsup.jpg"
      ];
      const bucketNames = ["bucket-1", "bucket-2", "bucket-3", "bucket-4"];

      // Upload files to the BSP.
      for (let i = 0; i < sources.length; i++) {
        const source = sources[i];
        const destination = locations[i];
        const bucketName = bucketNames[i];

        const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketName);
        fileKeys.push(fileMetadata.fileKey);

        await userApi.wait.bspVolunteer(1);
        await bspApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
        await userApi.wait.bspStored(1);
      }

      // Sort the file keys.
      fileKeys.sort();

      // Wait for the BSP to add the last confirmed file to its Forest.
      await sleep(500);

      // Get the root of the Forest.
      console.log("Forest root: ", root);
      const forestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
      root = forestRoot.toString().slice(2);
    });

    it("Generate a proof for 1 file", async () => {
      const challenges = [decrementHash(fileKeys[0])];
      const proof = await bspApi.rpc.storagehubclient.generateProof(DUMMY_BSP_ID, seed, challenges);

      console.log("\n\n Challenges for 1 file:");
      console.log(challenges);
      console.log("Proof for 1 file:");
      console.log(proof.toString());

      // Remove the 0x prefix from the challenges and proof.
      for (const i in challenges) {
        challenges[i] = challenges[i].slice(2);
      }
      const proofHexStr = proof.toString().slice(2);

      // Add the challenges and proof to the arrays.
      challengesCases.push(challenges);
      proofsCases.push(proofHexStr);
    });

    it("Generate a proof for 2 files", async () => {
      const challenges = [decrementHash(fileKeys[1])];
      const proof = await bspApi.rpc.storagehubclient.generateProof(DUMMY_BSP_ID, seed, challenges);

      console.log("\n\n Challenges for 2 files:");
      console.log(challenges);
      console.log("Proof for 2 files:");
      console.log(proof.toString());

      // Remove the 0x prefix from the challenges and proof.
      for (const i in challenges) {
        challenges[i] = challenges[i].slice(2);
      }
      const proofHexStr = proof.toString().slice(2);

      // Add the challenges and proof to the arrays.
      challengesCases.push(challenges);
      proofsCases.push(proofHexStr);
    });

    it("Generate a proof for 3 files", async () => {
      const challenges = [decrementHash(fileKeys[1]), decrementHash(fileKeys[2])];
      const proof = await bspApi.rpc.storagehubclient.generateProof(DUMMY_BSP_ID, seed, challenges);

      console.log("\n\n Challenges for 3 files:");
      console.log(challenges);
      console.log("Proof for 3 files:");
      console.log(proof.toString());

      // Remove the 0x prefix from the challenges and proof.
      for (const i in challenges) {
        challenges[i] = challenges[i].slice(2);
      }
      const proofHexStr = proof.toString().slice(2);

      // Add the challenges and proof to the arrays.
      challengesCases.push(challenges);
      proofsCases.push(proofHexStr);
    });

    it("Generate a proof for 4 files", async () => {
      const challenges = [decrementHash(fileKeys[1]), decrementHash(fileKeys[3])];
      const proof = await bspApi.rpc.storagehubclient.generateProof(DUMMY_BSP_ID, seed, challenges);

      console.log("\n\n Challenges for 4 files:");
      console.log(challenges);
      console.log("Proof for 4 files:");
      console.log(proof.toString());

      // Remove the 0x prefix from the challenges and proof.
      for (const i in challenges) {
        challenges[i] = challenges[i].slice(2);
      }
      const proofHexStr = proof.toString().slice(2);

      // Add the challenges and proof to the arrays.
      challengesCases.push(challenges);
      proofsCases.push(proofHexStr);
    });

    it("Write rust file with seed, root, challenges and proofs", async () => {
      const seedStr = `hex::decode("${seed.slice(2)}").expect("Seed should be a decodable hex string")`;

      const rootStr = `hex::decode("${root}").expect("Root should be a decodable hex string")`;

      let proofsStr = "";
      for (const [index, proof] of proofsCases.entries()) {
        const proofVec = `hex::decode("${proof}").expect("Proof should be a decodable hex string")`;
        proofsStr += `${index + 1} => ${proofVec},\n        `;
      }

      let challengesStr = "";
      for (const [index, challenges] of challengesCases.entries()) {
        let challengesArrayStr = "";
        for (const challenge of challenges) {
          challengesArrayStr += `hex::decode("${challenge}").expect("Challenge key should be a decodable hex string"),\n            `;
        }
        challengesStr += `${index + 1} => vec![\n            ${challengesArrayStr}\n        ],\n        `;
      }

      const template = fs.readFileSync(
        "../pallets/proofs-dealer/src/benchmark_proofs_template.rs",
        "utf8"
      );
      const rustCode = template
        .replace("{{date}}", new Date().toISOString())
        .replace("{{seed}}", seedStr)
        .replace("{{root}}", rootStr)
        .replace("{{proofs}}", proofsStr)
        .replace("{{challenges}}", challengesStr);

      fs.writeFileSync("../pallets/proofs-dealer/src/benchmark_proofs.rs", rustCode);
    });
  }
);

const decrementHash = (hash: string): string => {
  // Convert the hexadecimal hash to a number
  let num = BigInt(hash);

  // Decrement the number by 1
  if (num > 0n) {
    num -= 1n;
  } else {
    throw new Error("Cannot decrement hash below zero");
  }

  // Convert the number back to hexadecimal and remove the "0x" prefix
  const decrementedHash = num.toString(16);

  // Make sure the hash maintains the same length as the original, padding with zeros if necessary
  return `0x${decrementedHash.padStart(hash.length - 2, "0")}`;
};

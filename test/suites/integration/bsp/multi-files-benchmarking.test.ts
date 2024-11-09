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
      const bucketNames = [
        "bucket-1",
        "bucket-2",
        "bucket-3",
        "bucket-4",
        "bucket-5",
        "bucket-6",
        "bucket-7",
        "bucket-8",
        "bucket-9",
        "bucket-10",
        "bucket-11",
        "bucket-12",
        "bucket-13",
        "bucket-14",
        "bucket-15",
        "bucket-16",
        "bucket-17",
        "bucket-18",
        "bucket-19",
        "bucket-20",
        "bucket-21",
        "bucket-22",
        "bucket-23",
        "bucket-24",
        "bucket-25",
        "bucket-26",
        "bucket-27",
        "bucket-28",
        "bucket-29",
        "bucket-30",
        "bucket-31",
        "bucket-32",
        "bucket-33",
        "bucket-34",
        "bucket-35",
        "bucket-36",
        "bucket-37",
        "bucket-38",
        "bucket-39",
        "bucket-40"
      ];

      // Upload files to the BSP.
      for (let i = 0; i < sources.length; i++) {
        console.log(`Uploading file ${i + 1} of ${sources.length}`);
        const source = sources[i];
        const destination = locations[i];
        const bucketName = bucketNames[i];

        const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketName);
        fileKeys.push(fileMetadata.fileKey);

        await userApi.wait.bspVolunteer(1);
        await bspApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
        await userApi.wait.bspStored(1);

        // Advance two blocks to give room for proof submission.
        const currentBlock = await userApi.rpc.chain.getBlock();
        const currentBlockNumber = currentBlock.block.header.number.toNumber();
        await userApi.advanceToBlock(currentBlockNumber + 2, {
          waitForBspProofs: [DUMMY_BSP_ID]
        });
      }

      // Sort the file keys.
      fileKeys.sort();
      console.log("Sorted file keys: ", fileKeys);

      // Wait for the BSP to add the last confirmed file to its Forest.
      await sleep(500);

      // Get the root of the Forest.
      const forestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
      root = forestRoot.toString().slice(2);
      console.log("Forest root: ", forestRoot.toString());
    });

    it("Generate a proof with 1 to 20 file key proofs, plus 10 custom challenges with TrieRemoveMutation", async () => {
      // Case: There are 10 random challenges, which can be responded with 1 to 20 file key proofs,
      // depending on the Forest of the BSP and where the challenges fall within it. Additionally,
      // in the worst case scenario for this amount of file key proofs, there can be 10 more file keys
      // proven in the forest proof, that correspond to an exact match of a challenge with TrieRemoveMutation.
      // File keys that would be removed from the Forest, are not meant to also send a file key proof, and
      // that is the case for an exact match of a custom challenge with TrieRemoveMutation.

      for (let i = 1; i <= 20; i++) {
        // Create an array of odd indexes from 1 up to (i - 1), appending (i - 1) if `i` is odd.
        const filteredIndexes = Array.from({ length: i - 1 }, (_, index) => index + 1)
          .filter((num) => num % 2 !== 0)
          .concat(i % 2 !== 0 ? [i - 1] : []);

        // With those indexes, create an array of challenges that correspond to the indexed file
        // key hashes, minus one. That way the challenge falls in between the file key proofs.
        // - For 1 challenge: [first file key hash - 1] (only the first file key in the proof)
        // - For 2 challenges: [second file key hash - 1] (first and second file key in the proof)
        // - For 3 challenges: [second file key hash - 1, third file key hash - 1] (first, second and third file key in the proof)
        // - For 4 challenges: [second file key hash - 1, fourth file key hash - 1] (first, second, third and fourth file key in the proof)
        // - For 5 challenges: [second file key hash - 1, fourth file key hash - 1, fifth file key hash - 1] (first, second, third, fourth and fifth file key in the proof)
        const randomChallenges = filteredIndexes.map((index) => decrementHash(fileKeys[index]));

        // There should be always at least 10 challenges, representing the random challenges.
        // So we extend the challenges array with the last element repeatedly until it has 10 elements.
        while (randomChallenges.length < 10) {
          randomChallenges.push(randomChallenges[randomChallenges.length - 1]);
        }

        // Add the last 10 file keys as challenges, with a TrieRemoveMutation.
        // This will account for the worst case scenario possible between 1 to 20 file key proofs.
        // That is when, on the one hand, the 1 to 20 file key proofs respond to random challenges,
        // and there are 10 more checkpoint challenges with file deletions, that do not require a
        // file key proof, but execute a TrieRemoveMutation.
        const last10FileKeys = fileKeys.slice(fileKeys.length - 10);
        const challenges = randomChallenges.concat(last10FileKeys);

        // Add TrieRemoveMutation to all the challenges.
        const challengesWithMutation: [string, boolean][] = challenges.map((key) => [key, true]);

        // Generate the proof for the file keys.
        const proof = await bspApi.rpc.storagehubclient.generateProof(
          DUMMY_BSP_ID,
          seed,
          challengesWithMutation
        );

        console.log(`\n\n Challenges for ${i} files:`);
        console.log(challenges);
        // console.log("Proof for 1 file:");
        // console.log(proof.toString());

        // Remove the 0x prefix from the challenges and proof.
        for (const i in challenges) {
          challenges[i] = challenges[i].slice(2);
        }
        const proofHexStr = proof.toString().slice(2);

        // Add the challenges and proof to the arrays.
        challengesCases.push(challenges);
        proofsCases.push(proofHexStr);
      }
    });

    it("Write rust file with seed, provider ID, root, challenges and proofs", async () => {
      const seedStr = `hex::decode("${seed.slice(2)}").expect("Seed should be a decodable hex string")`;

      const providerIdStr = `hex::decode("${DUMMY_BSP_ID.slice(2)}").expect("Provider ID should be a decodable hex string")`;

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
        .replace("{{provider_id}}", providerIdStr)
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

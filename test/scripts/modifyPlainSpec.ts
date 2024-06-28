import fs from "node:fs/promises";
import { convertExponentials } from "@zombienet/utils";
import jsonBg from "json-bigint";

const JSONbig = jsonBg({ useNativeBigInt: true });

const args = process.argv.slice(2); // remove node and script name from args

if (args.length !== 2) {
  console.error("Usage: node script.js <inputPath> <outputPath>");
  process.exit(1);
}

const [inputPath, outputPath] = args;

if (!inputPath) {
  throw new Error("Input path is required");
}

if (!outputPath) {
  throw new Error("Output path is required");
}

process.stdout.write(`Reading from: ${inputPath} ...`);
const plainSpec = JSONbig.parse((await fs.readFile(inputPath)).toString());
process.stdout.write("Done \n");

const balances = plainSpec.genesis.runtimeGenesis.patch.balances.balances;

plainSpec.genesis.runtimeGenesis.patch.balances.balances = [
  ...balances,
  ["5FHSHEFWHVGDnyiw66DoRUpLyh5RouWkXo9GT1Sjk8qw7MAg", balances[0][1]]
];

const patch = plainSpec.genesis.runtimeGenesis.patch;
plainSpec.genesis.runtimeGenesis.patch = {
  ...patch,
  providers: {
    backupStorageProviders: [
      {
        capacity: 333,
        dataUsed: 0,
        multiaddresses: [3_333],
        root: "0x0000000000000000000000000000000000000000000000000000000000000000",
        lastCapacityChange: 93
      }
    ]
  }
};

process.stdout.write(`Writing to: ${outputPath} ...`);
await fs.writeFile(outputPath, convertExponentials(JSONbig.stringify(plainSpec, null, 3)));
process.stdout.write("Done \n");

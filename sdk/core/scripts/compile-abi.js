// Minimal solc-js ABI compiler for Core
// Purpose: Compile Solidity sources into ABI JSONs that Core can ship and use with viem/abitype.
// Config:  Add entries to CONTRACTS below to support more contracts.
// Output:  ./src/abi/<ContractName>.abi.json

import fs from "node:fs";
import path from "node:path";
import solc from "solc";

// -------- Configuration (add new contracts here) --------
const CONTRACTS = [
  {
    name: "FileSystem",
    sourcePath: "precompiles/pallet-file-system/FileSystem.sol"
  }
  // { name: 'AnotherContract', sourcePath: 'path/to/AnotherContract.sol' },
];
const OUT_DIR = "src/abi";
// --------------------------------------------------------

// Resolve paths relative to the Core package
const repoRoot = path.resolve(process.cwd(), "../..");
const outDir = path.resolve(process.cwd(), OUT_DIR);
fs.mkdirSync(outDir, { recursive: true });

function compileAbi({ name, sourcePath }) {
  const inputSol = path.join(repoRoot, sourcePath);

  // Guard: Ensure the Solidity file exists to provide a clear error in CI/local
  if (!fs.existsSync(inputSol)) {
    console.error(`[compile-abi] Missing Solidity file for ${name}: ${inputSol}`);
    process.exit(1);
  }

  // Prepare a single-file compiler input.
  const sourceKey = path.basename(inputSol); // e.g. FileSystem.sol
  const sources = {
    [sourceKey]: {
      content: fs.readFileSync(inputSol, "utf8")
    }
  };

  // Standard JSON input for solc
  const input = {
    language: "Solidity",
    sources,
    settings: {
      outputSelection: {
        "*": {
          "*": ["abi"]
        }
      }
    }
  };

  // Compile and parse the Standard JSON output
  const output = JSON.parse(solc.compile(JSON.stringify(input)));
  if (output.errors?.some((e) => e.severity === "error")) {
    console.error(`[compile-abi] solc errors for ${name}:\n`, output.errors);
    process.exit(1);
  }

  // Extract the compiled contracts for our single source
  const compiledSourceKey = Object.keys(output.contracts || {})[0];
  const contracts = output.contracts?.[compiledSourceKey];
  if (!contracts) {
    console.error(`[compile-abi] No contracts found in output for ${name}`);
    process.exit(1);
  }

  // Prefer the configured name; fallback to first key if necessary
  const selectedName = contracts[name] ? name : Object.keys(contracts)[0];
  const abi = contracts[selectedName]?.abi;
  if (!abi) {
    console.error(`[compile-abi] ABI not found for ${name}`);
    process.exit(1);
  }

  const outFile = path.join(outDir, `${name}.abi.json`);
  fs.writeFileSync(outFile, JSON.stringify(abi, null, 2));
  console.log(`[compile-abi] ${name} → ${outFile}`);
}

// Compile all configured contracts
for (const entry of CONTRACTS) {
  compileAbi(entry);
}

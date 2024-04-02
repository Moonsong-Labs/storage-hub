import fs from "node:fs";
import path from "node:path";

async function main() {
  const binaries = ["polkadot", "polkadot-prepare-worker", "polkadot-execute-worker"];
  const version = getVersionArg();
  const ghRoot = "https://github.com/paritytech/polkadot-sdk/releases/download/";
  const writeDir = path.join(process.cwd(), "./tmp/");

  try {
    for (const binary of binaries) {
      const downloadUri = `${ghRoot}polkadot-v${version}/${binary}`;
      console.log(`💾 Downloading ${binary} from ${downloadUri}`);

      const writePath = path.join(writeDir, binary);
      const blob = await fetch(downloadUri);
      await Bun.write(writePath, blob);
      fs.chmod(writePath, 0o755, (err) => {
        if (err) {
          throw err;
        }
      });
    }
  } catch (error) {
    console.error(`❌ Failed to download Polkadot binaries: ${error}`);
    process.exit(1);
  }

  console.log(`✅ Polkadot binaries successfully downloaded to: ${process.cwd()}`);
}

function getVersionArg() {
  const args = process.argv.slice(2);
  if (args.length === 0) {
    throw new Error("No version provided. Usage: bun scripts/downloadPolkadot.ts <version>");
  }
  return args[0];
}

main();

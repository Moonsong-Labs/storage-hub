import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";

async function main() {
  const binaries = ["polkadot", "polkadot-prepare-worker", "polkadot-execute-worker"];
  const version = getVersionArg();
  const ghRoot = "https://github.com/paritytech/polkadot-sdk/releases/download/";
  const writeDir = path.join(process.cwd(), "./tmp/");

  if (!fs.existsSync(writeDir)) {
    fs.mkdirSync(writeDir, { recursive: true });
  }

  try {
    for (const binary of binaries) {
      const writePath = path.join(writeDir, binary);

      if (fs.existsSync(writePath)) {
        console.log(`📂 ${binary} already exists at ${writePath}`);
        continue;
      }

      const downloadUri = `${ghRoot}polkadot-${version}/${binary}`;
      console.log(`💾 Downloading ${binary} from ${downloadUri}`);
      const blob = await fetch(downloadUri);
      const arrayBuffer = await blob.arrayBuffer();
      const buffer = Buffer.from(arrayBuffer);

      fs.writeFileSync(writePath, buffer);

      fs.chmod(writePath, 0o755, (err) => {
        if (err) {
          throw err;
        }
        console.log(`File permissions set for ${writePath}`);
      });
    }
  } catch (error) {
    console.error(`❌ Failed to download Polkadot binaries: ${error}`);
    process.exit(1);
  }

  console.log(`✅ Polkadot downloader script completed at: ${process.cwd()}`);
}

function getVersionArg() {
  const args = process.argv.slice(2);
  assert(
    args.length > 0,
    "No version provided. Usage: pnpm tsx scripts/downloadPolkadot.ts <version>"
  );
  return args[0];
}

await main();

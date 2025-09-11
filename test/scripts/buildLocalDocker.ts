import { execSync } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";

async function main() {
  process.chdir("..");
  const ARCH = execSync("uname -m").toString().trim();
  const OS = execSync("uname -s").toString().trim();

  const BINARY_PATH =
    ARCH === "arm64" && OS === "Darwin"
      ? "target/x86_64-unknown-linux-gnu/release/storage-hub-node"
      : "target/release/storage-hub-node";

  if (!fs.existsSync(BINARY_PATH)) {
    console.error(`No node found at ${BINARY_PATH}, you probably need to build.`);

    if (OS === "Darwin") {
      console.error("You are on a Mac, you need to build for Linux. Run `pnpm crossbuild:mac`");
    }
    process.exitCode = 1;
    return;
  }

  const fileOutput = execSync(`file ${BINARY_PATH}`).toString();
  if (!fileOutput.includes("x86-64")) {
    console.error("The binary is not for x86 architecture, something must have gone wrong.");
    process.exitCode = 1;
    return;
  }

  const buildDir = path.resolve(process.cwd(), "build");
  fs.mkdirSync(buildDir, { recursive: true });

  fs.copyFileSync(BINARY_PATH, path.join(buildDir, path.basename(BINARY_PATH)));

  try {
    // TODO: Replace with dockerode
    execSync("docker build -t storage-hub:local -f docker/storage-hub-node.Dockerfile .", {
      stdio: "inherit"
    });
    console.log("Docker image built successfully.");
  } catch (_error) {
    console.error("Docker build failed.");
    process.exitCode = 1;
    return;
  }
}

await main();

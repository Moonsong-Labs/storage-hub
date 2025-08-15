import { execSync } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";

async function main() {
  process.chdir("..");
  const ARCH = execSync("uname -m").toString().trim();
  const OS = execSync("uname -s").toString().trim();

  console.log(`Building Docker image on ${OS} ${ARCH}`);

  const BINARY_PATH =
    ARCH === "arm64" && OS === "Darwin"
      ? "target/x86_64-unknown-linux-gnu/release/storage-hub-node"
      : "target/release/storage-hub-node";

  console.log(`Looking for binary at: ${BINARY_PATH}`);

  if (!fs.existsSync(BINARY_PATH)) {
    console.error(`No node found at ${BINARY_PATH}, you probably need to build.`);

    if (OS === "Darwin") {
      console.error("You are on a Mac, you need to build for Linux. Run `pnpm crossbuild:mac`");
    }
    process.exitCode = 1;
    return;
  }

  console.log(`Binary found at ${BINARY_PATH}`);
  const stats = fs.statSync(BINARY_PATH);
  console.log(`Binary size: ${stats.size} bytes, executable: ${!!(stats.mode & 0o111)}`);
  console.log(`Binary permissions: ${stats.mode.toString(8)}`);

  const fileOutput = execSync(`file ${BINARY_PATH}`).toString();
  console.log(`Binary file output: ${fileOutput.trim()}`);
  if (!fileOutput.includes("x86-64")) {
    console.error("The binary is not for x86 architecture, something must have gone wrong.");
    console.error("Expected x86-64 architecture for Docker build");
    process.exitCode = 1;
    return;
  }

  const buildDir = path.resolve(process.cwd(), "build");
  fs.mkdirSync(buildDir, { recursive: true });
  console.log(`Created build directory: ${buildDir}`);

  const targetBinary = path.join(buildDir, path.basename(BINARY_PATH));
  fs.copyFileSync(BINARY_PATH, targetBinary);
  console.log(`Copied binary to: ${targetBinary}`);

  // Verify the copied binary
  const copiedStats = fs.statSync(targetBinary);
  console.log(
    `Copied binary size: ${copiedStats.size} bytes, executable: ${!!(copiedStats.mode & 0o111)}`
  );

  try {
    console.log("Starting Docker build...");
    // TODO: Replace with dockerode
    execSync("docker build -t storage-hub:local -f docker/storage-hub-node.Dockerfile .", {
      stdio: "inherit"
    });
    console.log("Docker image built successfully.");

    // Verify the image exists
    const imageCheck = execSync(
      "docker images storage-hub:local --format '{{.Repository}}:{{.Tag}} {{.Size}}'"
    )
      .toString()
      .trim();
    console.log(`Built image: ${imageCheck}`);
  } catch (error) {
    console.error("Docker build failed:", error);
    process.exitCode = 1;
    return;
  }
}

main();

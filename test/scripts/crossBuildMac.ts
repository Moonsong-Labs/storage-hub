import { execSync } from "node:child_process";
import inquirer from "inquirer";
import path from "path";
import fs from "fs";
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function main() {
  const { confirm } = await inquirer.prompt({
    type: "confirm",
    name: "confirm",
    message: "This script will build the project for Linux. Continue?",
    default: true
  });

  if (!confirm) {
    return;
  }

  const ARCH = execSync("uname -m").toString().trim();
  const OS = execSync("uname -s").toString().trim();

  if (ARCH !== "arm64" || OS !== "Darwin") {
    const { wrongArch } = await inquirer.prompt({
      type: "confirm",
      name: "wrongArch",
      message:
        "⚠️ This script is intended for Apple Silicon devices ⚠️\nℹ️ You can probably just run 'cargo build --release' to build the node.\n Are you sure you want to crossbuild?",
      default: true
    });
    if (!wrongArch) {
      return;
    }
  }

  if (!isCommandAvailable("zig")) {
    console.error("Zig is not installed. Please install Zig to proceed.");
    console.error(
      "Instructions to install can be found here: https://ziglang.org/learn/getting-started/"
    );
    process.exit(1);
  }

  installCargoZigbuild();

  const target = "x86_64-unknown-linux-gnu";
  addRustupTarget(target);

  // Build and copy libpq.so before cargo zigbuild
  await buildAndCopyLibpq(target);

  execSync(`cargo zigbuild --target ${target} --release`, { stdio: "inherit" });
}

main();

const execCommand = (command: string): string => {
  try {
    return execSync(command, { stdio: "pipe" }).toString().trim();
  } catch (error) {
    return "";
  }
};

const isCommandAvailable = (command: string): boolean => {
  try {
    execSync(`command -v ${command}`, { stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
};

const installCargoZigbuild = (): void => {
  if (!execCommand("cargo install --list").includes("cargo-zigbuild")) {
    execSync("cargo install cargo-zigbuild --locked", { stdio: "inherit" });
  }
};

const addRustupTarget = (target: string): void => {
  if (!execCommand("rustup target list --installed").includes(target)) {
    execSync(`rustup target add ${target}`, { stdio: "inherit" });
  }
};

// Updated function to build and copy libpq.so
const buildAndCopyLibpq = async (target: string): Promise<void> => {
  console.log("Building and copying libpq.so...");

  // Set Docker platform
  process.env.DOCKER_DEFAULT_PLATFORM = "linux/amd64";

  // Build Docker image
  const dockerfilePath = path.join(__dirname, "crossbuild-mac-libpq.dockerfile");
  execSync(`docker build -f ${dockerfilePath} -t crossbuild-libpq ${path.join(__dirname, "..", "..")}`, { stdio: "inherit" });

  // Create container and copy libpq.so
  execSync("docker create --name linux-libpq-container crossbuild-libpq", { stdio: "inherit" });
  
  const destPath = path.join(__dirname, "..", "..", "target", target, "release", "deps");
  
  // Ensure the destination directory exists
  fs.mkdirSync(destPath, { recursive: true });

  execSync(`docker cp linux-libpq-container:/artifacts/libpq.so ${path.join(destPath, "libpq.so")}`, { stdio: "inherit" });

  // Remove container
  execSync("docker rm linux-libpq-container", { stdio: "inherit" });

  console.log(`libpq.so has been copied to ${destPath}`);
};

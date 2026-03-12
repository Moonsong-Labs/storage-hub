import { execSync } from "node:child_process";
import inquirer from "inquirer";
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const execSyncWithEnv = (command: string, options: { stdio: "inherit" | "pipe" }) =>
  execSync(command, { ...options, env: process.env });

async function main() {
  const ARCH = execSync("uname -m").toString().trim();
  const OS = execSync("uname -s").toString().trim();

  if (ARCH !== "arm64" || OS !== "Darwin") {
    const { wrongArch } = await inquirer.prompt({
      type: "confirm",
      name: "wrongArch",
      message:
        "⚠️ This script is intended for Apple Silicon devices ⚠️\nℹ️ You can probably just run 'cargo build --release -p sh-msp-backend' to build the backend.\n Are you sure you want to crossbuild?",
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

  // Get additional arguments from command line
  const additionalArgs = process.argv.slice(2).join(" ");

  console.log(
    `Running build command: cargo zigbuild --target ${target} --release -p sh-msp-backend ${additionalArgs}`
  );
  execSyncWithEnv(`cargo zigbuild --target ${target} --release -p sh-msp-backend ${additionalArgs}`, {
    stdio: "inherit"
  });
}

const execCommand = (command: string): string => {
  try {
    return execSyncWithEnv(command, { stdio: "pipe" }).toString().trim();
  } catch {
    return "";
  }
};

const isCommandAvailable = (command: string): boolean => {
  try {
    execSyncWithEnv(`command -v ${command}`, { stdio: "pipe" });
    return true;
  } catch {
    return false;
  }
};

const installCargoZigbuild = (): void => {
  if (!execCommand("cargo install --list").includes("cargo-zigbuild")) {
    execSyncWithEnv("cargo install cargo-zigbuild --locked", { stdio: "inherit" });
  }
};

const addRustupTarget = (target: string): void => {
  if (!execCommand("rustup target list --installed").includes(target)) {
    execSyncWithEnv(`rustup target add ${target}`, { stdio: "inherit" });
  }
};

// Updated function to build and copy libpq.so
const buildAndCopyLibpq = async (target: string): Promise<void> => {
  console.log("Building and copying libpq.so...");

  // Set Docker platform
  process.env.DOCKER_DEFAULT_PLATFORM = "linux/amd64";

  // Build Docker image
  const dockerfilePath = path.join(__dirname, "crossbuild-mac-libpq.dockerfile");
  execSyncWithEnv(
    `docker build -f ${dockerfilePath} -t crossbuild-libpq ${path.join(__dirname, "..", "..")}`,
    { stdio: "inherit" }
  );

  // Create container and copy libpq.so
  execSyncWithEnv("docker create --name linux-libpq-container crossbuild-libpq", {
    stdio: "inherit"
  });

  const destPath = path.join(__dirname, "..", "..", "target", target, "release", "deps");

  // Ensure the destination directory exists
  fs.mkdirSync(destPath, { recursive: true });

  execSyncWithEnv(
    `docker cp linux-libpq-container:/artifacts/libpq.so ${path.join(destPath, "libpq.so")}`,
    { stdio: "inherit" }
  );

  // Remove container
  execSyncWithEnv("docker rm linux-libpq-container", { stdio: "inherit" });

  console.log(`libpq.so has been copied to ${destPath}`);

  const pqLibDirTargetVar = `PQ_LIB_DIR_${target.toUpperCase().replace(/-/g, "_")}`;

  // Make libpq visible both to rustc native linking and to the final binary at runtime.
  const linkerFlags = [
    `-L native=${destPath}`,
    "-C link-arg=-Wl,-rpath,$ORIGIN/../release/deps"
  ];

  process.env.RUSTFLAGS = [process.env.RUSTFLAGS, ...linkerFlags]
    .filter(Boolean)
    .join(" ");
  process.env.CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = [
    process.env.CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS,
    ...linkerFlags
  ]
    .filter(Boolean)
    .join(" ");
  process.env.LIBRARY_PATH = [destPath, process.env.LIBRARY_PATH].filter(Boolean).join(":");
  process.env.PQ_LIB_DIR = destPath;
  process.env[pqLibDirTargetVar] = destPath;

  console.log(`RUSTFLAGS set to: ${process.env.RUSTFLAGS}`);
  console.log(
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS set to: " +
      process.env.CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS
  );
  console.log(`LIBRARY_PATH set to: ${process.env.LIBRARY_PATH}`);
  console.log(`PQ_LIB_DIR set to: ${process.env.PQ_LIB_DIR}`);
  console.log(`${pqLibDirTargetVar} set to: ${process.env[pqLibDirTargetVar]}`);
};

await main();

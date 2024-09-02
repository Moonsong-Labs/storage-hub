import { execSync } from "node:child_process";
import inquirer from "inquirer";

async function main() {
  const { confirm } = await inquirer.prompt({
    type: "confirm",
    name: "confirm",
    // @ts-expect-error bug with inquirer
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
      // @ts-expect-error bug with inquirer
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

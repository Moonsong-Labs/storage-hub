import inquirer from "inquirer";
import { checkNetworkRunning } from "../util";

async function main() {
  const running = await checkNetworkRunning();

  if (!running) {
    console.log("ℹ️ StorageHub nodes are not running on this machine, continuing...");
    return;
  }

  const { proceed } = await inquirer.prompt({
    type: "confirm",
    name: "proceed",
    default: false,
    message:
      "⚠️ Local StorageHub nodes are already running on this machine.\n Are you sure you would like to proceed (may give inconsistent behaviour)?"
  });

  if (!proceed) {
    process.exit(2);
  }
}

await main();

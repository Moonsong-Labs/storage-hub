import { spawn, execSync, spawnSync } from "node:child_process";
import { GenericContainer, type StartedTestContainer, Wait } from "testcontainers";

let container: StartedTestContainer;

async function main() {
  // run the node
  process.stdout.write("Starting container... ");
  container = await new GenericContainer("storage-hub:local")
    .withExposedPorts(9944)
    .withCommand([
      "--dev",
      "--rpc-cors=all",
      "--no-hardware-benchmarks",
      "--no-telemetry",
      "--no-prometheus",
      "--unsafe-rpc-external",
      "--sealing=instant",
    ])
    .withWaitStrategy(Wait.forLogMessage("Development Service Ready"))
    .start();
  process.stdout.write("✅\n");

  const connectString = `ws://${container.getHost()}:${container.getMappedPort(9944)}`;

  console.log(`Connecting APIs at ${connectString}... `);
  // run scalegen
  execSync(`pnpm papi add --config papi.json --wsUrl ${connectString} dev_node`, {
    stdio: "inherit",
  });

  // run typegen
  execSync("pnpm papi generate --config papi.json", { stdio: "inherit" });
}

main().finally(() => {
  try {
    console.log("Stopping container...");
    container.stop();
  } catch (e) {
    console.error(e);
  } finally {
    console.log("✅");
  }
});

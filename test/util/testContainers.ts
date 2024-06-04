import "@storagehub/api-augment";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { GenericContainer, Wait, type StartedTestContainer } from "testcontainers";
import { createBlock } from "./blocks";
export { ApiPromise } from "@polkadot/api";
export type { StartedTestContainer } from "testcontainers";

type TestApis = {
  extendedApi: ExtendedApiPromise;
  runningContainer: StartedTestContainer;
};

export type ExtendedApiPromise = ApiPromise & {
  createBlock: () => ReturnType<typeof createBlock>;
};

export type TestOptions = {
  keepOpen?: boolean;
};

export const devnodeSetup = async (options: TestOptions): Promise<TestApis> => {
  process.stdout.write("Starting container... ");
  options.keepOpen && console.log("Keep Node Open = true");
  const runningContainer = await new GenericContainer("storage-hub:local")
    .withExposedPorts(9944)
    .withCommand([
      "--dev",
      "--rpc-cors=all",
      "--no-hardware-benchmarks",
      "--no-telemetry",
      "--no-prometheus",
      "--unsafe-rpc-external",
      "--sealing=manual",
    ])
    // replace with a health check
    .withWaitStrategy(Wait.forLogMessage("Development Service Ready"))
    // .withLogConsumer((stream) => {
    //   stream.on("data", (line) => console.log(line));
    //   stream.on("err", (line) => console.error(line));
    //   stream.on("end", () => console.log("Stream closed"));
    // })
    .start();
  process.stdout.write("✅\n");

  const connectString = `ws://${runningContainer.getHost()}:${runningContainer.getMappedPort(
    9944
  )}`;
  process.stdout.write(`Connecting APIs at ${connectString}... `);
  const connectedApi = await ApiPromise.create({
    provider: new WsProvider(connectString),
  });
  process.stdout.write("✅\n");

  const extendedApi = Object.assign(connectedApi, {
    createBlock: () => createBlock(connectedApi),
  });

  return { extendedApi, runningContainer };
};

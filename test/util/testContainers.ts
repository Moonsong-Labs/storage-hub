import "@storagehub/api-augment";
(Symbol as any).dispose ??= Symbol("Symbol.dispose");
(Symbol as any).asyncDispose ??= Symbol("Symbol.asyncDispose");
import { ApiPromise, WsProvider } from "@polkadot/api";
import { GenericContainer, Wait, type StartedTestContainer } from "testcontainers";
import { createBlock } from "./blocks";
export { ApiPromise } from "@polkadot/api";
export type { StartedTestContainer } from "testcontainers";
import dotenv from "dotenv";
import assert from "node:assert";
dotenv.config();

export class DevTestContext implements AsyncDisposable {
  private _api?: ExtendedApiPromise;
  private _container?: StartedTestContainer;
  #disposed = false;
  #initialized = false;

  constructor(public readonly options?: TCTestOptions) {}

  public async initialize() {
    const { extendedApi, runningContainer } = await devnodeSetup(this.options);
    this._api = extendedApi;
    this._container = runningContainer;
    this.#initialized = true;
    return this.api;
  }

  public async dispose() {
    if (this.options?.keepOpen) {
      console.log("⏯️ 'keepOpen' is set to true, not disposing");
      console.log(
        `🏃 Container still running at: ws://${this.container.getHost()}:${this.container.getMappedPort(
          9944
        )}`
      );
      return;
    }
    this.api.disconnect();
    this.container.stop();
    this.#disposed = true;
  }

  public get api(): ExtendedApiPromise {
    assert(this.#initialized, "API is not initialized");
    assert(this._api, "API is not initialized");
    return this._api;
  }

  public get container(): StartedTestContainer {
    assert(this.#initialized, "Container is not initialized");
    assert(this._container, "Container is not initialized");
    return this._container;
  }

  // This doesnt work nicely with describe() blocks interact with test runners
  async [Symbol.asyncDispose]() {
    if (this.#disposed || !this.#initialized) {
      return;
    }
    await this._api?.disconnect();
    if (!this.options?.keepOpen) {
      await this._container?.stop();
      this.#disposed = true;
    }
  }
}

type TestApis = {
  extendedApi: ExtendedApiPromise;
  runningContainer: StartedTestContainer;
};

export type ExtendedApiPromise = ApiPromise & {
  createBlock: () => ReturnType<typeof createBlock>;
};

export type TCTestOptions = {
  keepOpen?: boolean;
  printLogs?: boolean;
};

export const devnodeSetup = async (options?: TCTestOptions): Promise<TestApis> => {
  process.stdout.write("Starting container... ");
  const container = new GenericContainer("storage-hub:local")
    .withExposedPorts(9944)
    .withCommand([
      "--dev",
      "--network-backend",
      "libp2p",
      "--rpc-cors=all",
      "--no-hardware-benchmarks",
      "--no-telemetry",
      "--no-prometheus",
      "--unsafe-rpc-external",
      "--sealing=manual"
    ])
    // replace with a health check
    .withWaitStrategy(Wait.forLogMessage("Development Service Ready"));

  if (options?.printLogs) {
    container.withLogConsumer((stream) => {
      stream.on("data", (line) => console.log(line));
      stream.on("err", (line) => console.error(line));
      stream.on("end", () => console.log("Stream closed"));
    });
  }

  const runningContainer = await container.start();
  process.stdout.write("✅\n");
  options?.keepOpen && console.log("Keep Node Open = true");

  const connectString = `ws://${runningContainer.getHost()}:${runningContainer.getMappedPort(
    9944
  )}`;
  process.stdout.write(`Connecting APIs at ${connectString}... `);
  const connectedApi = await ApiPromise.create({
    isPedantic: false,
    throwOnConnect: true,
    // noInitWarn: true,
    rpc: {},
    provider: new WsProvider(connectString)
  });
  process.stdout.write("✅\n");

  const extendedApi = Object.assign(connectedApi, {
    createBlock: () => createBlock(connectedApi)
  });

  return { extendedApi, runningContainer };
};

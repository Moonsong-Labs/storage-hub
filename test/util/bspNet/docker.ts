import Docker from "dockerode";
import { execSync } from "node:child_process";
import path from "node:path";
import { DOCKER_IMAGE } from "../constants";
import { sendCustomRpc } from "../rpc";
import { checkNodeAlive } from "./helpers";
import { BspNetTestApi, type EnrichedBspApi } from "./test-api";

export const checkBspForFile = async (filePath: string) => {
  const containerId = "docker-sh-bsp-1";
  const loc = path.join("/storage", filePath);

  for (let i = 0; i < 10; i++) {
    try {
      // TODO: Replace with dockerode
      execSync(`docker exec ${containerId} ls ${loc}`, { stdio: "ignore" });
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }
  throw new Error(`File not found: ${loc} in ${containerId}`);
};

export const checkFileChecksum = async (filePath: string) => {
  const containerId = "docker-sh-bsp-1";
  const loc = path.join("/storage", filePath);
  const output = execSync(`docker exec ${containerId} sha256sum ${loc}`);
  return output.toString().split(" ")[0];
};

export const showContainers = () => {
  try {
    // TODO: Replace with dockerode
    execSync("docker ps -a", { stdio: "inherit" });
  } catch (e) {
    console.log(e);
    console.log("Error displaying docker containers");
  }
};

export const addBspContainer = async (options?: {
  name?: string;
  connectToPeer?: boolean; // unused
  additionalArgs?: string[];
}) => {
  const docker = new Docker();
  const existingBsps = (
    await docker.listContainers({
      filters: { ancestor: [DOCKER_IMAGE] }
    })
  )
    .flatMap(({ Command }) => Command)
    .filter((cmd) => cmd.includes("--provider-type=bsp"));

  const bspNum = existingBsps.length;

  if (bspNum < 1) {
    throw new Error("No existing BSP containers");
  }
  const p2pPort = 30350 + bspNum;
  const rpcPort = 9977 + bspNum * 7;
  const containerName = options?.name || `docker-sh-bsp-${bspNum + 1}`;
  // get bootnode from docker args

  const { Args } = await docker.getContainer("docker-sh-user-1").inspect();

  const bootNodeArg = Args.find((arg) => arg.includes("--bootnodes="));

  if (!bootNodeArg) {
    throw new Error("No bootnode found in docker args");
  }

  let keystorePath: string;
  const keystoreArg = Args.find((arg) => arg.includes("--keystore-path="));
  if (keystoreArg) {
    keystorePath = keystoreArg.split("=")[1];
  } else {
    keystorePath = "/keystore";
  }

  const container = await docker.createContainer({
    Image: DOCKER_IMAGE,
    name: containerName,
    platform: "linux/amd64",
    NetworkingConfig: {
      EndpointsConfig: {
        docker_default: {}
      }
    },
    HostConfig: {
      PortBindings: {
        "9944/tcp": [{ HostPort: rpcPort.toString() }],
        [`${p2pPort}/tcp`]: [{ HostPort: p2pPort.toString() }]
      },
      Binds: [`${process.cwd()}/../docker/dev-keystores:${keystorePath}:rw`]
    },
    Cmd: [
      "--dev",
      "--sealing=manual",
      "--provider",
      "--provider-type=bsp",
      `--name=${containerName}`,
      "--no-hardware-benchmarks",
      "--unsafe-rpc-external",
      "--rpc-methods=unsafe",
      "--rpc-cors=all",
      `--port=${p2pPort}`,
      "--base-path=/data",
      bootNodeArg,
      ...(options?.additionalArgs || [])
    ]
  });
  await container.start();

  let peerId: string | undefined;
  for (let i = 0; i < 20; i++) {
    try {
      peerId = await sendCustomRpc(`http://127.0.0.1:${rpcPort}`, "system_localPeerId");
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }

  if (!peerId) {
    console.error("Failed to connect after 10s. Exiting...");
    throw new Error("Failed to connect to the new BSP container");
  }

  const api = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);

  const chainName = api.consts.system.version.specName.toString();
  if (chainName !== "storage-hub-runtime") {
    console.log(chainName);
    throw new Error(`Error connecting to BSP via api ${containerName}`);
  }

  await api.disconnect();

  console.log(
    `▶️ BSP container started with name: ${containerName}, rpc port: ${rpcPort}, p2p port: ${p2pPort}, peerId: ${peerId}`
  );

  return { containerName, rpcPort, p2pPort, peerId };
};

// Make this a rusty style OO function with api contexts
export const pauseBspContainer = async (containerName: string) => {
  const docker = new Docker();
  const container = docker.getContainer(containerName);
  await container.pause();
};

export const stopBspContainer = async (options: { containerName: string; api: EnrichedBspApi }) => {
  await options.api.disconnect();
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.stop();
};

export const startBspContainer = async (options: {
  containerName: string;
  endpoint?: `ws://${string}`;
}) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.start();

  if (options.endpoint) {
    await checkNodeAlive(options.endpoint);
    return await BspNetTestApi.create(options.endpoint);
  }

  return undefined;
};

export const restartBspContainer = async (options: {
  containerName: string;
  api: EnrichedBspApi;
  endpoint?: `ws://${string}`;
}) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.restart();

  return options.endpoint ? await BspNetTestApi.create(options.endpoint) : undefined;
};

export const resumeBspContainer = async (options: {
  containerName: string;
  endpoint?: `ws://${string}`;
}) => {
  const docker = new Docker();
  const container = docker.getContainer(options.containerName);
  await container.unpause();

  return options.endpoint ? await BspNetTestApi.create(options.endpoint) : undefined;
};

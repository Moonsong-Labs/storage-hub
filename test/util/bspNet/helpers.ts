import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import Docker from "dockerode";
import fs from "node:fs/promises";
import * as child_process from "node:child_process";
import { execSync } from "node:child_process";
import crypto from "node:crypto";
import * as util from "node:util";
import invariant from "tiny-invariant";
import stripAnsi from "strip-ansi";
import tmp from "tmp";
import { DOCKER_IMAGE } from "../constants.ts";
import { sealBlock } from "./block.ts";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "./consts";
import * as ShConsts from "./consts.ts";
import { addBspContainer, showContainers } from "./docker";
import type { EnrichedBspApi } from "./test-api.ts";
import { sleep } from "../timer.ts";
import postgres from "postgres";

const exec = util.promisify(child_process.exec);

export const getContainerIp = async (containerName: string, verbose = false): Promise<string> => {
  const maxRetries = 60;
  const sleepTime = 500;

  for (let i = 0; i < maxRetries; i++) {
    verbose && console.log(`Waiting for ${containerName} to launch...`);

    // TODO: Replace with dockerode command
    try {
      const { stdout } = await exec(
        `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${containerName}`
      );
      return stdout.trim();
    } catch {
      await sleep(sleepTime);
    }
  }
  // TODO: Replace with dockerode
  execSync("docker ps -a", { stdio: "inherit" });
  try {
    execSync("docker logs docker-sh-bsp-1", { stdio: "inherit" });
    execSync("docker logs docker-sh-user-1", { stdio: "inherit" });
  } catch (e) {
    console.log(e);
  }
  console.log(
    `Error fetching container IP for ${containerName} after ${
      (maxRetries * sleepTime) / 1000
    } seconds`
  );
  showContainers();
  throw "Error fetching container IP";
};

export const checkNodeAlive = async (url: string, verbose = false) => getContainerIp(url, verbose);

export const getContainerPeerId = async (url: string, verbose = false) => {
  const maxRetries = 60;
  const sleepTime = 500;

  const payload = {
    id: "1",
    jsonrpc: "2.0",
    method: "system_localPeerId",
    params: []
  };
  verbose && console.log(`Waiting for node at ${url} to launch...`);

  for (let i = 0; i < maxRetries; i++) {
    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(payload)
      });

      invariant(response.ok, `HTTP error! status: ${response.status}`);

      const resp = (await response.json()) as any;
      return resp.result as string;
    } catch {
      await sleep(sleepTime);
    }
  }

  console.log(`Error fetching peerId from ${url} after ${(maxRetries * sleepTime) / 1000} seconds`);
  showContainers();
  throw `Error fetching peerId from ${url}`;
};

export const forceSignupBsp = async (options: {
  api: EnrichedBspApi;
  multiaddress: string;
  who: string | Uint8Array;
  bspId?: string;
  capacity?: bigint;
  payeeAddress?: string;
  weight?: bigint;
}) => {
  const bspId = options.bspId || `0x${crypto.randomBytes(32).toString("hex")}`;
  const blockResults = await options.api.sealBlock(
    options.api.tx.sudo.sudo(
      options.api.tx.providers.forceBspSignUp(
        options.who,
        bspId,
        options.capacity || ShConsts.CAPACITY_512,
        [options.multiaddress],
        options.payeeAddress || options.who,
        options.weight ?? null
      )
    )
  );
  return Object.assign(bspId, blockResults);
};

export const checkSHRunningContainers = async (docker: Docker) => {
  const allContainers = await docker.listContainers({ all: true });
  return allContainers.filter((container) => container.Image === DOCKER_IMAGE);
};

export const cleanupEnvironment = async (verbose = false) => {
  await printDockerStatus();

  const docker = new Docker();

  let allContainers = await docker.listContainers({ all: true });

  const existingNodes = allContainers.filter((container) => container.Image === DOCKER_IMAGE);

  const toxiproxyContainer = allContainers.find((container) =>
    container.Names.some((name) => name.includes("toxiproxy"))
  );

  const postgresContainer = allContainers.find((container) =>
    container.Names.some((name) => name.includes("docker-sh-postgres-1"))
  );

  const tmpDir = tmp.dirSync({ prefix: "bsp-logs-", unsafeCleanup: true });

  const logPromises = existingNodes.map(async (node) => {
    const container = docker.getContainer(node.Id);
    try {
      const logs = await container.logs({
        stdout: true,
        stderr: true,
        timestamps: true
      });
      verbose && console.log(`Extracting logs for container ${node.Names[0]}`);
      const containerName = node.Names[0].replace("/", "");

      await fs.writeFile(`${tmpDir.name}/${containerName}.log`, stripAnsi(logs.toString()), {
        encoding: "utf8"
      });
    } catch (e) {
      console.warn(`Failed to extract logs for container ${node.Names[0]}:`, e);
    }
  });

  await Promise.all(logPromises);
  console.log(`Container logs saved to ${tmpDir.name}`);

  const promises = existingNodes.map(async (node) => {
    const container = docker.getContainer(node.Id);
    await container.remove({ force: true });
  });

  if (toxiproxyContainer && toxiproxyContainer.State === "running") {
    console.log("Stopping toxiproxy container");
    promises.push(docker.getContainer(toxiproxyContainer.Id).stop());
  } else {
    verbose && console.log("No running toxiproxy container found, skipping");
  }

  if (postgresContainer) {
    console.log("Stopping postgres container");
    promises.push(docker.getContainer(postgresContainer.Id).remove({ force: true }));
  } else {
    verbose && console.log("No postgres container found, skipping");
  }

  await Promise.all(promises);

  await docker.pruneContainers();
  await docker.pruneVolumes();

  for (let i = 0; i < 10; i++) {
    allContainers = await docker.listContainers({ all: true });
    const remainingNodes = allContainers.filter((container) => container.Image === DOCKER_IMAGE);
    if (remainingNodes.length === 0) {
      await printDockerStatus();
      verbose && console.log("All nodes verified to be removed, continuing");
      return;
    }
  }
  invariant(false, `Failed to stop all nodes: ${JSON.stringify(allContainers)}`);
};

export const printDockerStatus = async (verbose = false) => {
  const docker = new Docker();

  verbose && console.log("\n=== Docker Container Status ===");

  const containers = await docker.listContainers({ all: true });

  if (containers.length === 0) {
    verbose && console.log("No containers found");
    return;
  }

  if (verbose) {
    for (const container of containers) {
      console.log(`\nContainer: ${container.Names.join(", ")}`);
      console.log(`ID: ${container.Id}`);
      console.log(`Image: ${container.Image}`);
      console.log(`Status: ${container.State}/${container.Status}`);
      console.log(`Created: ${new Date(container.Created * 1000).toISOString()}`);

      if (container.State === "running") {
        try {
          const stats = await docker.getContainer(container.Id).stats({ stream: false });
          console.log("Memory Usage:", {
            usage: `${Math.round(stats.memory_stats.usage / 1024 / 1024)}MB`,
            limit: `${Math.round(stats.memory_stats.limit / 1024 / 1024)}MB`
          });
        } catch (e) {
          console.log("Could not fetch container stats");
        }
      }
    }
  }
  verbose && console.log("\n===============================\n");
};

export const verifyContainerFreshness = async () => {
  const docker = new Docker();
  const containers = await docker.listContainers({ all: true });

  const existingContainers = containers.filter(
    (container) =>
      container.Image === DOCKER_IMAGE || container.Names.some((name) => name.includes("toxiproxy"))
  );

  if (existingContainers.length > 0) {
    console.log("\n=== WARNING: Found existing containers ===");
    for (const container of existingContainers) {
      console.log(`Container: ${container.Names.join(", ")}`);
      console.log(`Created: ${new Date(container.Created * 1000).toISOString()}`);
      console.log(`Status: ${container.State}/${container.Status}`);

      const containerInfo = await docker.getContainer(container.Id).inspect();
      console.log(
        "Mounts:",
        containerInfo.Mounts.map((m) => m.Source)
      );
      console.log("---");
    }
    throw new Error("Test environment is not clean - found existing containers");
  }
};

export const cleardownTest = async (cleardownOptions: {
  api: EnrichedBspApi | EnrichedBspApi[];
  keepNetworkAlive?: boolean;
}) => {
  try {
    if (Array.isArray(cleardownOptions.api)) {
      for (const api of cleardownOptions.api) {
        await api.disconnect();
      }
    } else {
      await cleardownOptions.api.disconnect();
    }
  } catch (e) {
    console.error("Error disconnecting APIs:", e);
  }

  if (!cleardownOptions.keepNetworkAlive) {
    await cleanupEnvironment();

    const docker = new Docker();
    const remainingContainers = await docker.listContainers({ all: true });
    const relevantContainers = remainingContainers.filter(
      (container) =>
        container.Image === DOCKER_IMAGE ||
        container.Names.some((name) => name.includes("toxiproxy"))
    );

    if (relevantContainers.length > 0) {
      console.error("WARNING: Containers still present after cleanup!");
      await printDockerStatus();
      throw new Error("Failed to clean up test environment");
    }
  }
};

export const createSqlClient = () => {
  return postgres({
    host: "localhost",
    port: 5432,
    database: "storage_hub",
    username: "postgres",
    password: "postgres"
  });
};

export const createCheckBucket = async (api: EnrichedBspApi, bucketName: string) => {
  const newBucketEventEvent = await api.createBucket(bucketName);
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  invariant(newBucketEventDataBlob, "Event doesn't match Type");

  return newBucketEventDataBlob;
};

export const addBsp = async (
  api: ApiPromise,
  bspKey: KeyringPair,
  options?: {
    name?: string;
    rocksdb?: boolean;
    bspKeySeed?: string;
    bspId?: string;
    bspStartingWeight?: bigint;
    maxStorageCapacity?: number;
    extrinsicRetryTimeout?: number;
    additionalArgs?: string[];
  }
) => {
  // Launch a BSP node.
  const additionalArgs = options?.additionalArgs ?? [];
  if (options?.extrinsicRetryTimeout) {
    additionalArgs.push(`--extrinsic-retry-timeout=${options.extrinsicRetryTimeout}`);
  }
  if (options?.rocksdb) {
    additionalArgs.push("--storage-layer=rocks-db");
  }
  additionalArgs.push(`--storage-path=/tmp/bsp/${bspKey.address}`);
  additionalArgs.push(
    `--max-storage-capacity=${options?.maxStorageCapacity ?? MAX_STORAGE_CAPACITY}`
  );
  additionalArgs.push(`--jump-capacity=${options?.maxStorageCapacity ?? CAPACITY[1024]}`);
  const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer({
    ...options,
    additionalArgs
  });

  //Give it some balance.
  const amount = 10000n * 10n ** 12n;
  await sealBlock(api, api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bspKey.address, amount)));

  const bspIp = await getContainerIp(containerName);
  const multiAddressBsp = `/ip4/${bspIp}/tcp/${p2pPort}/p2p/${peerId}`;

  // Make BSP
  await sealBlock(
    api,
    api.tx.sudo.sudo(
      api.tx.providers.forceBspSignUp(
        bspKey.address,
        options?.bspId ?? bspKey.publicKey,
        ShConsts.CAPACITY_512,
        [multiAddressBsp],
        bspKey.address,
        options?.bspStartingWeight ?? null
      )
    )
  );

  return { containerName, rpcPort, p2pPort, peerId };
};

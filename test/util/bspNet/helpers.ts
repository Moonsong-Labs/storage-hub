import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import Docker from "dockerode";
import * as child_process from "node:child_process";
import { execSync } from "node:child_process";
import crypto from "node:crypto";
import * as util from "node:util";
import invariant from "tiny-invariant";
import { DOCKER_IMAGE } from "../constants.ts";
import { sealBlock } from "./block.ts";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "./consts";
import * as ShConsts from "./consts.ts";
import { addBspContainer, showContainers } from "./docker";
import type { EnrichedBspApi } from "./test-api.ts";

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
      await new Promise((resolve) => setTimeout(resolve, sleepTime));
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

  for (let i = 0; i < maxRetries; i++) {
    verbose && console.log(`Waiting for node at ${url} to launch...`);

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
      await new Promise((resolve) => setTimeout(resolve, sleepTime));
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

export const closeSimpleBspNet = async () => {
  const docker = new Docker();

  const allContainers = await docker.listContainers({ all: true });

  const existingNodes = allContainers.filter((container) => container.Image === DOCKER_IMAGE);

  const toxiproxyContainer = allContainers.find((container) =>
    container.Names.some((name) => name.includes("toxiproxy"))
  );

  const promises = existingNodes.map(async (node) => {
    const container = docker.getContainer(node.Id);

    if (node.State === "running") {
      await container.stop();
    }

    await container.remove();
  });

  if (toxiproxyContainer && toxiproxyContainer.State === "running") {
    console.log("Stopping toxiproxy container");
    promises.push(docker.getContainer(toxiproxyContainer.Id).stop());
  } else {
    console.log("No running toxiproxy container found, skipping");
  }

  await Promise.allSettled(promises);

  await docker.pruneContainers();
  await docker.pruneVolumes();
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
    console.error(e);
    console.log("cleardown failed, but we will continue.");
  }
  cleardownOptions.keepNetworkAlive === true ? null : await closeSimpleBspNet();
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

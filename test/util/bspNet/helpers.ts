import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import * as child_process from "node:child_process";
import { execSync } from "node:child_process";
import crypto from "node:crypto";
import Docker from "dockerode";
import * as util from "node:util";
import assert from "node:assert";
import { sleep } from "../timer.ts";
import { sealBlock } from "./block.ts";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "./consts";
import * as ShConsts from "./consts.ts";
import { addBspContainer, showContainers } from "./docker";
import type { EnrichedBspApi } from "./test-api.ts";
import { cleanupEnvironment, printDockerStatus } from "../helpers.ts";
import { DOCKER_IMAGE } from "../constants.ts";
import { assertDockerLog } from "../asserts.ts";

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
    execSync("docker logs storage-hub-sh-bsp-1", { stdio: "inherit" });
    execSync("docker logs storage-hub-sh-user-1", { stdio: "inherit" });
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

      assert(response.ok, `HTTP error! status: ${response.status}`);

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
        container.Names.some((name) => name.includes("toxiproxy")) ||
        container.Names.some((name) => name.includes("storage-hub-sh-copyparty")) ||
        container.Names.some((name) => name.includes("storage-hub-sh-backend"))
    );

    if (relevantContainers.length > 0) {
      console.error("WARNING: Containers still present after cleanup!");
      await printDockerStatus();
      throw new Error("Failed to clean up test environment");
    }
  }
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
  const blockResults = await options.api.block.seal({
    calls: [
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
    ]
  });
  return Object.assign(bspId, blockResults);
};

export const createCheckBucket = async (api: EnrichedBspApi, bucketName: string) => {
  const newBucketEventEvent = await api.createBucket(bucketName);
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  assert(newBucketEventDataBlob, "Event doesn't match Type");

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
    waitForIdle?: boolean;
    initialCapacity?: bigint;
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

  if (options?.waitForIdle) {
    await assertDockerLog(containerName, "💤 Idle", 15000);
  }

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
        options?.initialCapacity ?? ShConsts.CAPACITY_512,
        [multiAddressBsp],
        bspKey.address,
        options?.bspStartingWeight ?? null
      )
    )
  );

  return { containerName, rpcPort, p2pPort, peerId };
};

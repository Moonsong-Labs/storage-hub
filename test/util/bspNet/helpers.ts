import type { KeyringPair } from "@polkadot/keyring/types";
import "@storagehub/api-augment";
import { v2 as compose } from "docker-compose";
import Docker from "dockerode";
import * as child_process from "node:child_process";
import { execSync } from "node:child_process";
import crypto from "node:crypto";
import path from "node:path";
import * as util from "node:util";
import { DOCKER_IMAGE, MILLIUNIT, UNIT } from "../constants.ts";
import {
  alice,
  bspDownKey,
  bspDownSeed,
  bspKey,
  bspThreeKey,
  bspThreeSeed,
  bspTwoKey,
  bspTwoSeed,
  shUser
} from "../pjsKeyring";
import { addBspContainer, showContainers } from "./docker";
import type { BspNetConfig, InitialisedMultiBspNetwork } from "./types";
import { CAPACITY, MAX_STORAGE_CAPACITY } from "./consts";
import * as ShConsts from "./consts.ts";
import { BspNetTestApi, type EnrichedBspApi } from "./test-api.ts";
import invariant from "tiny-invariant";
import * as fs from "node:fs";
import { parse, stringify } from "yaml";
import { sealBlock } from "./block.ts";
import type { ApiPromise } from "@polkadot/api";
import { sleep } from "@zombienet/utils";

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

export const runSimpleBspNet = async (bspNetConfig: BspNetConfig, verbose = false) => {
  let userApi: EnrichedBspApi | undefined;
  try {
    console.log(`SH user id: ${shUser.address}`);
    console.log(`SH BSP id: ${bspKey.address}`);
    let file = "local-dev-bsp-compose.yml";
    if (bspNetConfig.rocksdb) {
      file = "local-dev-bsp-rocksdb-compose.yml";
    }
    if (bspNetConfig.noisy) {
      file = "noisy-bsp-compose.yml";
    }
    const composeFilePath = path.resolve(process.cwd(), "..", "docker", file);
    const cwd = path.resolve(process.cwd(), "..", "docker");
    const composeFile = fs.readFileSync(composeFilePath, "utf8");
    const composeYaml = parse(composeFile);
    if (bspNetConfig.extrinsicRetryTimeout) {
      composeYaml.services["sh-bsp"].command.push(
        `--extrinsic-retry-timeout=${bspNetConfig.extrinsicRetryTimeout}`
      );
      composeYaml.services["sh-user"].command.push(
        `--extrinsic-retry-timeout=${bspNetConfig.extrinsicRetryTimeout}`
      );
    }

    const updatedCompose = stringify(composeYaml);

    if (bspNetConfig.noisy) {
      await compose.upOne("toxiproxy", { cwd: cwd, configAsString: updatedCompose, log: true });
    }

    await compose.upOne("sh-bsp", { cwd: cwd, configAsString: updatedCompose, log: true });

    const bspIp = await getContainerIp(
      bspNetConfig.noisy ? "toxiproxy" : ShConsts.NODE_INFOS.bsp.containerName
    );

    if (bspNetConfig.noisy) {
      verbose && console.log(`toxiproxy IP: ${bspIp}`);
    } else {
      verbose && console.log(`sh-bsp IP: ${bspIp}`);
    }

    const bspPeerId = await getContainerPeerId(`http://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);

    process.env.BSP_IP = bspIp;
    process.env.BSP_PEER_ID = bspPeerId;

    await compose.upOne("sh-user", {
      cwd: cwd,
      configAsString: updatedCompose,
      log: true,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId
      }
    });

    const peerIDUser = await getContainerPeerId(
      `http://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`
    );
    verbose && console.log(`sh-user Peer ID: ${peerIDUser}`);

    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    // Create Connection API Object to User Node
    userApi = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

    // Give Balances
    const amount = 10000n * 10n ** 12n;
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(bspKey.address, amount))
    );
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(shUser.address, amount))
    );

    // Setting:
    // replication_target = 1 -> One BSP is enough to fulfil a storage request.
    // block_range_to_maximum_threshold = 1 -> The threshold goes from the minimum to the maximum in 1 tick.
    await userApi.sealBlock(userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(1, 1)));

    // Adjusting runtime parameters...
    // The `set_parameter` extrinsic receives an object like this:
    // {
    //   RuntimeConfig: Enum {
    //     SlashAmountPerMaxFileSize: [null, {VALUE_YOU_WANT}],
    //     StakeToChallengePeriod: [null, {VALUE_YOU_WANT}],
    //     CheckpointChallengePeriod: [null, {VALUE_YOU_WANT}],
    //     MinChallengePeriod: [null, {VALUE_YOU_WANT}],
    //   }
    // }
    const slashAmountPerMaxFileSizeRuntimeParameter = {
      RuntimeConfig: {
        SlashAmountPerMaxFileSize: [null, 20n * MILLIUNIT]
      }
    };
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.parameters.setParameter(slashAmountPerMaxFileSizeRuntimeParameter)
      )
    );
    const stakeToChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        StakeToChallengePeriod: [null, 1000n * UNIT]
      }
    };
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.parameters.setParameter(stakeToChallengePeriodRuntimeParameter)
      )
    );
    const checkpointChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        CheckpointChallengePeriod: [null, 10]
      }
    };
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.parameters.setParameter(checkpointChallengePeriodRuntimeParameter)
      )
    );
    const minChallengePeriodRuntimeParameter = {
      RuntimeConfig: {
        MinChallengePeriod: [null, 5]
      }
    };
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.parameters.setParameter(minChallengePeriodRuntimeParameter))
    );

    // Make BSP
    await forceSignupBsp({
      api: userApi,
      who: bspKey.address,
      multiaddress: multiAddressBsp,
      bspId: ShConsts.DUMMY_BSP_ID,
      capacity: bspNetConfig.capacity || ShConsts.CAPACITY_512,
      weight: bspNetConfig.bspStartingWeight
    });

    // Make MSP
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.providers.forceMspSignUp(
          alice.address,
          ShConsts.DUMMY_MSP_ID,
          bspNetConfig.capacity || ShConsts.CAPACITY_512,
          // The peer ID has to be different from the BSP's since the user now attempts to send files to MSPs when new storage requests arrive.
          ["/ip4/127.0.0.1/tcp/30350/p2p/12D3KooWNEZ8PGNydcdXTYy1SPHvkP9mbxdtTqGGFVrhorDzeTfA"],
          {
            identifier: ShConsts.VALUE_PROP,
            dataLimit: 500,
            protocols: ["https", "ssh", "telnet"]
          },
          alice.address
        )
      )
    );
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
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

  const promises = existingNodes.map(async (node) => docker.getContainer(node.Id).stop());

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

export const runInitialisedBspsNet = async (bspNetConfig: BspNetConfig) => {
  await runSimpleBspNet(bspNetConfig);

  let userApi: EnrichedBspApi | undefined;
  let bspApi: EnrichedBspApi | undefined;
  try {
    userApi = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
    bspApi = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);

    /**** CREATE BUCKET AND ISSUE STORAGE REQUEST ****/
    const source = "res/whatsup.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const fileMetadata = await userApi.file.newStorageRequest(source, destination, bucketName);

    await userApi.wait.bspVolunteer(1);
    await bspApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await userApi.wait.bspStored(1);
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
    bspApi?.disconnect();
  }
};

export const runMultipleInitialisedBspsNet = async (
  bspNetConfig: BspNetConfig
): Promise<undefined | InitialisedMultiBspNetwork> => {
  await runSimpleBspNet(bspNetConfig);

  let userApi: EnrichedBspApi | undefined;
  let bspApi: EnrichedBspApi | undefined;
  try {
    userApi = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);
    bspApi = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.bsp.port}`);

    await userApi.sealBlock(userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(5, 1)));

    // Add more BSPs to the network.
    // One BSP will be down, two more will be up.
    const { containerName: bspDownContainerName, rpcPort: bspDownRpcPort } = await addBsp(
      userApi,
      bspDownKey,
      {
        name: "sh-bsp-down",
        rocksdb: bspNetConfig.rocksdb,
        bspKeySeed: bspDownSeed,
        bspId: ShConsts.BSP_DOWN_ID,
        bspStartingWeight: bspNetConfig.bspStartingWeight,
        additionalArgs: ["--keystore-path=/keystore/bsp-down"]
      }
    );
    const bspDownApi = await BspNetTestApi.create(`ws://127.0.0.1:${bspDownRpcPort}`);

    const { rpcPort: bspTwoRpcPort } = await addBsp(userApi, bspTwoKey, {
      name: "sh-bsp-two",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspTwoSeed,
      bspId: ShConsts.BSP_TWO_ID,
      bspStartingWeight: bspNetConfig.bspStartingWeight,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });
    const bspTwoApi = await BspNetTestApi.create(`ws://127.0.0.1:${bspTwoRpcPort}`);

    const { rpcPort: bspThreeRpcPort } = await addBsp(userApi, bspThreeKey, {
      name: "sh-bsp-three",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspThreeSeed,
      bspId: ShConsts.BSP_THREE_ID,
      bspStartingWeight: bspNetConfig.bspStartingWeight,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });
    const bspThreeApi = await BspNetTestApi.create(`ws://127.0.0.1:${bspThreeRpcPort}`);

    // Wait a few seconds for all BSPs to be synced.
    await sleep(5000);

    // Everything executed below is tested in `volunteer.test.ts` and `onboard.test.ts` files.
    // For the context of this test, this is a preamble, so that a BSP has a challenge cycle initiated.

    /**** CREATE BUCKET AND ISSUE STORAGE REQUEST ****/
    const source = "res/whatsup.jpg";
    const location = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const fileMetadata = await userApi.file.newStorageRequest(source, location, bucketName);

    await userApi.wait.bspVolunteer(4);
    await bspApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await bspTwoApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await bspThreeApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await bspDownApi.wait.bspFileStorageComplete(fileMetadata.fileKey);
    await userApi.wait.bspStored(4);

    // Disconnecting temporary api connections
    await bspTwoApi.disconnect();
    await bspThreeApi.disconnect();
    await bspDownApi.disconnect();

    // Stopping BSP that is supposed to be down.
    await userApi.docker.stopBspContainer(bspDownContainerName);

    return {
      bspTwoRpcPort,
      bspThreeRpcPort,
      fileData: {
        fileKey: fileMetadata.fileKey,
        bucketId: fileMetadata.bucketId,
        location: location,
        owner: fileMetadata.owner,
        fingerprint: fileMetadata.fingerprint,
        fileSize: fileMetadata.fileSize
      }
    };
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
    bspApi?.disconnect();
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

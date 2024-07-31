import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import type { CreatedBlock, EventRecord, Hash, SignedBlock } from "@polkadot/types/interfaces";
import type { ISubmittableResult } from "@polkadot/types/types";
import { v2 as compose } from "docker-compose";
import Docker from "dockerode";
import * as child_process from "node:child_process";
import { execSync } from "node:child_process";
import path from "node:path";
import * as util from "node:util";
import { assertEventPresent } from "../asserts.ts";
import { DOCKER_IMAGE } from "../constants.ts";
import { isExtSuccess } from "../extrinsics";
import {
  alice,
  bspKey,
  shUser,
  bspDownKey,
  bspTwoKey,
  bspThreeKey,
  bspTwoSeed,
  bspThreeSeed
} from "../pjsKeyring";
import { createApiObject } from "./api";
import { CAPACITY_512, DUMMY_BSP_ID, DUMMY_MSP_ID, NODE_INFOS, VALUE_PROP } from "./consts";
import { addBspContainer, showContainers } from "./docker";
import type { BspNetApi } from "./types";
import { sleep } from "../timer.ts";
import { strictEqual } from "node:assert";

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
  throw new Error("Error fetching container IP");
};

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

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const resp = (await response.json()) as any;
      return resp.result as string;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, sleepTime));
    }
  }

  console.log(`Error fetching peerId from ${url} after ${(maxRetries * sleepTime) / 1000} seconds`);
  showContainers();
  throw new Error(`Error fetching peerId from ${url}`);
};

export type BspNetConfig = {
  noisy: boolean;
  rocksdb: boolean;
};

export const runSimpleBspNet = async (bspNetConfig: BspNetConfig) => {
  let api: BspNetApi | undefined;
  try {
    console.log(`sh user id: ${shUser.address}`);
    console.log(`sh bsp id: ${bspKey.address}`);
    let file = "local-dev-bsp-compose.yml";
    if (bspNetConfig.rocksdb) {
      file = "local-dev-bsp-rocksdb-compose.yml";
    }
    if (bspNetConfig.noisy) {
      file = "noisy-bsp-compose.yml";
    }
    const composeFilePath = path.resolve(process.cwd(), "..", "docker", file);

    if (bspNetConfig.noisy) {
      await compose.upOne("toxiproxy", { config: composeFilePath, log: true });
    }

    await compose.upOne("sh-bsp", { config: composeFilePath, log: true });

    const bspIp = await getContainerIp(
      bspNetConfig.noisy ? "toxiproxy" : NODE_INFOS.bsp.containerName
    );

    if (bspNetConfig.noisy) {
      console.log(`toxiproxy IP: ${bspIp}`);
    } else {
      console.log(`sh-bsp IP: ${bspIp}`);
    }

    const bspPeerId = await getContainerPeerId(`http://127.0.0.1:${NODE_INFOS.bsp.port}`, true);
    console.log(`sh-bsp Peer ID: ${bspPeerId}`);

    process.env.BSP_IP = bspIp;
    process.env.BSP_PEER_ID = bspPeerId;

    await compose.upOne("sh-user", {
      config: composeFilePath,
      log: true,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId
      }
    });

    const peerIDUser = await getContainerPeerId(`http://127.0.0.1:${NODE_INFOS.user.port}`);
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    // Create Connection API Object to User Node
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);

    // Give Balances
    const amount = 10000n * 10n ** 12n;
    await api.sealBlock(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bspKey.address, amount)));
    await api.sealBlock(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(shUser.address, amount)));

    // Make BSP
    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.providers.forceBspSignUp(
          bspKey.address,
          DUMMY_BSP_ID,
          CAPACITY_512,
          [multiAddressBsp],
          bspKey.address
        )
      )
    );

    // Make MSP
    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.providers.forceMspSignUp(
          alice.address,
          DUMMY_MSP_ID,
          CAPACITY_512,
          [multiAddressBsp],
          {
            identifier: VALUE_PROP,
            dataLimit: 500,
            protocols: ["https", "ssh", "telnet"]
          },
          alice.address
        )
      )
    );

    // u128 max value
    const u128Max = (BigInt(1) << BigInt(128)) - BigInt(1);

    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.fileSystem.forceUpdateBspsAssignmentThreshold(u128Max))
    );
  } catch (e) {
    console.error("Error ", e);
  } finally {
    api?.disconnect();
  }
};

export const closeSimpleBspNet = async () => {
  const docker = new Docker();

  const existingNodes = await docker.listContainers({
    filters: { ancestor: [DOCKER_IMAGE] }
  });

  const promises = existingNodes.map(async (node) => docker.getContainer(node.Id).stop());
  await Promise.all(promises);

  await docker.pruneContainers();
  await docker.pruneVolumes();
};

export const runMultipleInitialisedBspsNet = async (bspNetConfig: BspNetConfig) => {
  let userApi: BspNetApi | undefined;
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

    if (bspNetConfig.noisy) {
      await compose.upOne("toxiproxy", { config: composeFilePath, log: true });
    }

    await compose.upOne("sh-bsp", { config: composeFilePath, log: true });

    const bspIp = await getContainerIp(
      bspNetConfig.noisy ? "toxiproxy" : NODE_INFOS.bsp.containerName
    );

    if (bspNetConfig.noisy) {
      console.log(`toxiproxy IP: ${bspIp}`);
    } else {
      console.log(`sh-bsp IP: ${bspIp}`);
    }

    const bspPeerId = await getContainerPeerId(`http://127.0.0.1:${NODE_INFOS.bsp.port}`, true);
    console.log(`sh-bsp Peer ID: ${bspPeerId}`);

    process.env.BSP_IP = bspIp;
    process.env.BSP_PEER_ID = bspPeerId;

    await compose.upOne("sh-user", {
      config: composeFilePath,
      log: true,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId
      }
    });

    const peerIDUser = await getContainerPeerId(`http://127.0.0.1:${NODE_INFOS.user.port}`);
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    // Create Connection API Object to User Node
    userApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);

    // Give Balances
    const amount = 10000n * 10n ** 12n;
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(bspKey.address, amount))
    );
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(shUser.address, amount))
    );

    // Make BSP
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.providers.forceBspSignUp(
          bspKey.address,
          DUMMY_BSP_ID,
          CAPACITY_512,
          [multiAddressBsp],
          bspKey.address
        )
      )
    );

    // Make MSP
    await userApi.sealBlock(
      userApi.tx.sudo.sudo(
        userApi.tx.providers.forceMspSignUp(
          alice.address,
          DUMMY_MSP_ID,
          CAPACITY_512,
          [multiAddressBsp],
          {
            identifier: VALUE_PROP,
            dataLimit: 500,
            protocols: ["https", "ssh", "telnet"]
          },
          alice.address
        )
      )
    );

    // u128 max value
    const u128Max = (BigInt(1) << BigInt(128)) - BigInt(1);

    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.fileSystem.forceUpdateBspsAssignmentThreshold(u128Max))
    );

    // Add more BSPs to the network.
    // One BSP will be down, two more will be up.
    const { containerName: bspDownContainerName } = await addBsp(userApi, bspDownKey, {
      name: "sh-bsp-down",
      rocksdb: bspNetConfig.rocksdb
    });
    await stopBsp(bspDownContainerName);
    const { rpcPort: bspTwoRpcPort } = await addBsp(userApi, bspTwoKey, {
      name: "sh-bsp-two",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspTwoSeed,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });
    const { rpcPort: bspThreeRpcPort } = await addBsp(userApi, bspThreeKey, {
      name: "sh-bsp-three",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspThreeSeed,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });

    // Everything executed below is tested in `volunteer.test.ts` and `onboard.test.ts` files.
    // For the context of this test, this is a preamble, so that a BSP has a challenge cycle initiated.

    /**** CREATE BUCKET AND ISSUE STORAGE REQUEST ****/
    const source = "res/whatsup.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const { fingerprint, file_size, location } =
      await userApi.rpc.storagehubclient.loadFileInStorage(
        source,
        destination,
        NODE_INFOS.user.AddressId,
        newBucketEventDataBlob.bucketId
      );

    await userApi.sealBlock(
      userApi.tx.fileSystem.issueStorageRequest(
        newBucketEventDataBlob.bucketId,
        location,
        fingerprint,
        file_size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId]
      ),
      shUser
    );

    /**** BSP VOLUNTEERS ****/
    await sleep(500); // wait for the BSPs to volunteer

    const volunteerPending = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
      volunteerPending.length,
      3,
      "There should be three pending extrinsics from BSPs (volunteer)"
    );

    await userApi.sealBlock();

    await sleep(5000); // wait for the BSPs to download the file
    const confirmPending = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
      confirmPending.length,
      3,
      "There should be three pending extrinsics from BSPs (confirm store)"
    );

    await userApi.sealBlock();

    await sleep(1000); // wait for the BSPs to process the BspConfirmedStoring event

    return { bspTwoRpcPort, bspThreeRpcPort };
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
  }
};

// TODO: Add a successful flag to track whether ext was successful or not
//        Determine whether extrinsic was successful or not based on the
//        ExtrinsicSuccess event
export interface SealedBlock {
  blockReceipt: CreatedBlock;
  txHash?: string;
  blockData?: SignedBlock;
  events?: EventRecord[];
  extSuccess?: boolean;
}

// TODO: extend to take multiple exts in one block
// TODO: Accept ext hash strings as well
export const sealBlock = async (
  api: ApiPromise,
  call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
  signer?: KeyringPair
): Promise<SealedBlock> => {
  const initialHeight = (await api.rpc.chain.getHeader()).number.toNumber();

  const results: {
    hash?: Hash;
    events?: EventRecord[];
    blockData?: SignedBlock;
    success?: boolean;
  } = {};

  if (call?.isSigned) {
    results.hash = await call.send();
  } else if (call) {
    results.hash = await call.signAndSend(signer || alice);
  }

  const sealedResults = {
    blockReceipt: await api.rpc.engine.createBlock(true, true),
    txHash: results.hash?.toString()
  };

  const blockHash = sealedResults.blockReceipt.blockHash;
  const allEvents = await (await api.at(blockHash)).query.system.events();

  if (results.hash) {
    const blockData = await api.rpc.chain.getBlock(blockHash);
    const getExtIndex = (txHash: Hash) => {
      return blockData.block.extrinsics.findIndex((ext) => ext.hash.toHex() === txHash.toString());
    };
    const extIndex = getExtIndex(results.hash);
    const extEvents = allEvents.filter(
      ({ phase }) =>
        phase.isApplyExtrinsic && Number(phase.asApplyExtrinsic.toString()) === extIndex
    );
    results.blockData = blockData;
    results.events = extEvents;
    results.success = isExtSuccess(extEvents);
  } else {
    results.events = allEvents;
  }

  // Allow time for chain to settle
  for (let i = 0; i < 20; i++) {
    const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();
    if (currentHeight > initialHeight) {
      break;
    }
    console.log("Waiting for block to be finalized...");
    console.log("You shouldn't see this message often, if you do, something is wrong");
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  return Object.assign(sealedResults, {
    events: results.events,
    extSuccess: results.success
  }) satisfies SealedBlock;
};

export const createBucket = async (api: ApiPromise, bucketName: string) => {
  const createBucketResult = await sealBlock(
    api,
    api.tx.fileSystem.createBucket(DUMMY_MSP_ID, bucketName, false),
    shUser
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};

export const cleardownTest = async (api: BspNetApi) => {
  await api.disconnect();
  await closeSimpleBspNet();
};

export const createCheckBucket = async (api: BspNetApi, bucketName: string) => {
  const newBucketEventEvent = await api.createBucket(bucketName);
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  if (!newBucketEventDataBlob) {
    throw new Error("Event doesn't match Type");
  }
  return newBucketEventDataBlob;
};

const addBsp = async (
  api: BspNetApi,
  bspKey: KeyringPair,
  options?: { name?: string; rocksdb?: boolean; bspKeySeed?: string; additionalArgs?: string[] }
) => {
  // Launch a BSP node.
  const additionalArgs = options?.additionalArgs ?? [];
  if (options?.rocksdb) {
    additionalArgs.push("--storage-layer=rocks-db");
    additionalArgs.push(`--storage-path=/tmp/bsp/${bspKey.address}`);
  }
  const { containerName, rpcPort, p2pPort, peerId } = await addBspContainer(options);

  //Give it some balance.
  const amount = 10000n * 10n ** 12n;
  await api.sealBlock(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bspKey.address, amount)));

  const bspIp = await getContainerIp(containerName);
  const multiAddressBsp = `/ip4/${bspIp}/tcp/${p2pPort}/p2p/${peerId}`;

  // Make BSP
  await api.sealBlock(
    api.tx.sudo.sudo(
      api.tx.providers.forceBspSignUp(
        bspKey.address,
        bspKey.publicKey,
        CAPACITY_512,
        [multiAddressBsp],
        bspKey.address
      )
    )
  );

  return { containerName, rpcPort, p2pPort, peerId };
};

const stopBsp = async (name: string) => {
  const docker = new Docker();

  const containersToStop = await docker.listContainers({
    filters: { name: [name] }
  });

  await docker.getContainer(containersToStop[0].Id).stop();
  await docker.getContainer(containersToStop[0].Id).remove();
};

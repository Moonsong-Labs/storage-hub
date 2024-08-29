import "@storagehub/api-augment";
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
  bspThreeSeed,
  bspDownSeed
} from "../pjsKeyring";
import { createApiObject } from "./api";
import {
  BSP_DOWN_ID,
  BSP_THREE_ID,
  BSP_TWO_ID,
  CAPACITY_512,
  DUMMY_BSP_ID,
  DUMMY_MSP_ID,
  NODE_INFOS,
  VALUE_PROP
} from "./consts";
import { addBspContainer, showContainers } from "./docker";
import type { BspNetApi, FileMetadata } from "./types";
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

    // u32 max value
    const u32Max = (BigInt(1) << BigInt(32)) - BigInt(1);

    await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(1, u32Max, 1)));
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

export const runInitialisedBspsNet = async (bspNetConfig: BspNetConfig) => {
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

    // u32 max value
    const u32Max = (BigInt(1) << BigInt(32)) - BigInt(1);

    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(1, u32Max, 1))
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
    // Wait for the BSPs to volunteer.
    await sleep(500);

    const volunteerPending = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
      volunteerPending.length,
      1,
      "There should be one pending extrinsic from the BSP (volunteer)"
    );

    await userApi.sealBlock();

    // Wait for the BSPs to download the file.
    await sleep(5000);
    const confirmPending = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
      confirmPending.length,
      1,
      "There should be one pending extrinsic from the BSP (confirm store)"
    );

    await userApi.sealBlock();

    // Wait for the BSPs to process the BspConfirmedStoring event.
    await sleep(1000);
  } catch (e) {
    console.error("Error ", e);
  } finally {
    userApi?.disconnect();
  }
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

    // u32 max value
    const u32Max = (BigInt(1) << BigInt(32)) - BigInt(1);

    await userApi.sealBlock(
      userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(5, u32Max, 1))
    );

    // Add more BSPs to the network.
    // One BSP will be down, two more will be up.
    const { containerName: bspDownContainerName } = await addBsp(userApi, bspDownKey, {
      name: "sh-bsp-down",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspDownSeed,
      bspId: BSP_DOWN_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-down"]
    });
    const { rpcPort: bspTwoRpcPort } = await addBsp(userApi, bspTwoKey, {
      name: "sh-bsp-two",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspTwoSeed,
      bspId: BSP_TWO_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });
    const { rpcPort: bspThreeRpcPort } = await addBsp(userApi, bspThreeKey, {
      name: "sh-bsp-three",
      rocksdb: bspNetConfig.rocksdb,
      bspKeySeed: bspThreeSeed,
      bspId: BSP_THREE_ID,
      additionalArgs: ["--keystore-path=/keystore/bsp-three"]
    });

    // Everything executed below is tested in `volunteer.test.ts` and `onboard.test.ts` files.
    // For the context of this test, this is a preamble, so that a BSP has a challenge cycle initiated.

    /**** CREATE BUCKET AND ISSUE STORAGE REQUEST ****/
    const source = "res/whatsup.jpg";
    const location = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const fileMetadata = await sendNewStorageRequest(userApi, source, location, bucketName);

    /**** BSP VOLUNTEERS ****/
    // Wait for the BSPs to volunteer.
    await sleep(500);

    const volunteerPending = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
      volunteerPending.length,
      4,
      "There should be four pending extrinsics from BSPs (volunteer)"
    );

    await userApi.sealBlock();

    // Wait for the BSPs to download the file.
    await sleep(5000);
    const confirmPending = await userApi.rpc.author.pendingExtrinsics();
    strictEqual(
      confirmPending.length,
      4,
      "There should be four pending extrinsics from BSPs (confirm store)"
    );

    await userApi.sealBlock();

    // Wait for the BSPs to process the confirmation of the file.
    await sleep(1000);

    // Stopping BSP that is supposed to be down.
    await stopBsp(bspDownContainerName);

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

export const sealBlock = async (
  api: ApiPromise,
  calls?:
    | SubmittableExtrinsic<"promise", ISubmittableResult>
    | SubmittableExtrinsic<"promise", ISubmittableResult>[],
  signer?: KeyringPair
): Promise<SealedBlock> => {
  const initialHeight = (await api.rpc.chain.getHeader()).number.toNumber();

  const results: {
    hashes: Hash[];
    events: EventRecord[];
    blockData?: SignedBlock;
    success: boolean[];
  } = {
    hashes: [],
    events: [],
    success: []
  };

  // Normalize to array
  const callArray = Array.isArray(calls) ? calls : calls ? [calls] : [];

  if (callArray.length > 0) {
    const nonce = await api.rpc.system.accountNextIndex((signer || alice).address);

    // Send all transactions in sequence
    for (let i = 0; i < callArray.length; i++) {
      const call = callArray[i];
      let hash: Hash;

      if (call.isSigned) {
        hash = await call.send();
      } else {
        hash = await call.signAndSend(signer || alice, { nonce: nonce.addn(i) });
      }

      results.hashes.push(hash);
    }
  }

  const sealedResults = {
    blockReceipt: await api.rpc.engine.createBlock(true, true),
    txHashes: results.hashes.map((hash) => hash.toString())
  };

  const blockHash = sealedResults.blockReceipt.blockHash;
  const allEvents = await (await api.at(blockHash)).query.system.events();

  if (results.hashes.length > 0) {
    const blockData = await api.rpc.chain.getBlock(blockHash);
    results.blockData = blockData;

    const getExtIndex = (txHash: Hash) => {
      return blockData.block.extrinsics.findIndex((ext) => ext.hash.toHex() === txHash.toString());
    };

    for (const hash of results.hashes) {
      const extIndex = getExtIndex(hash);
      const extEvents = allEvents.filter(
        ({ phase }) =>
          phase.isApplyExtrinsic && Number(phase.asApplyExtrinsic.toString()) === extIndex
      );
      results.events.push(...extEvents);
      results.success.push(isExtSuccess(extEvents) ?? false);
    }
  } else {
    results.events.push(...allEvents);
  }

  const extSuccess = results.success.every((success) => success);

  // Allow time for chain to settle
  for (let i = 0; i < 20; i++) {
    const currentHeight = (await api.rpc.chain.getHeader()).number.toNumber();
    if (currentHeight > initialHeight) {
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }

  return Object.assign(sealedResults, {
    events: results.events,
    extSuccess: extSuccess
  }) satisfies SealedBlock;
};

export const advanceToBlock = async (
  api: ApiPromise,
  blockNumber: number,
  waitBetweenBlocks?: number | boolean
): Promise<SealedBlock> => {
  const currentBlock = await api.rpc.chain.getBlock();
  const currentBlockNumber = currentBlock.block.header.number.toNumber();

  let blockResult = null;
  if (blockNumber > currentBlockNumber) {
    const blocksToAdvance = blockNumber - currentBlockNumber;
    for (let i = 0; i < blocksToAdvance; i++) {
      blockResult = await sealBlock(api);

      if (waitBetweenBlocks) {
        if (typeof waitBetweenBlocks === "number") {
          await sleep(waitBetweenBlocks);
        } else {
          await sleep(500);
        }
      }
    }
  } else {
    throw new Error(
      `Block number ${blockNumber} is lower than current block number ${currentBlockNumber}`
    );
  }

  if (blockResult) {
    return blockResult;
  }

  throw new Error("Block wasn't sealed");
};

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string
): Promise<FileMetadata> => {
  const newBucketEventEvent = await createBucket(api, bucketName);
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  if (!newBucketEventDataBlob) {
    throw new Error("Event doesn't match Type");
  }

  const fileMetadata = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    NODE_INFOS.user.AddressId,
    newBucketEventDataBlob.bucketId
  );

  const issueStorageRequestResult = await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      newBucketEventDataBlob.bucketId,
      location,
      fileMetadata.fingerprint,
      fileMetadata.file_size,
      DUMMY_MSP_ID,
      [NODE_INFOS.user.expectedPeerId]
    ),
    shUser
  );

  const newStorageRequestEvent = assertEventPresent(
    api,
    "fileSystem",
    "NewStorageRequest",
    issueStorageRequestResult.events
  );
  const newStorageRequestEventDataBlob =
    api.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent.event) &&
    newStorageRequestEvent.event.data;

  if (!newStorageRequestEventDataBlob) {
    throw new Error("Event doesn't match Type");
  }

  return {
    fileKey: newStorageRequestEventDataBlob.fileKey.toString(),
    bucketId: newBucketEventDataBlob.bucketId.toString(),
    location: newStorageRequestEventDataBlob.location.toString(),
    owner: newBucketEventDataBlob.who.toString(),
    fingerprint: fileMetadata.fingerprint,
    fileSize: fileMetadata.file_size
  };
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

export const cleardownTest = async (cleardownOptions: {
  api: BspNetApi;
  keepNetworkAlive?: boolean;
}) => {
  await cleardownOptions.api.disconnect();
  cleardownOptions.keepNetworkAlive === true ? null : await closeSimpleBspNet();
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
  options?: {
    name?: string;
    rocksdb?: boolean;
    bspKeySeed?: string;
    bspId?: string;
    additionalArgs?: string[];
  }
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
        options?.bspId ?? bspKey.publicKey,
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

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
import { alice, bsp, shUser } from "../pjsKeyring";
import { createApiObject } from "./api";
import { CAPACITY_512, DUMMY_BSP_ID, DUMMY_MSP_ID, NODE_INFOS, VALUE_PROP } from "./consts";
import { showContainers } from "./docker";
import type { BspNetApi } from "./types";

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

export const runBspNet = async (bspNetConfig: BspNetConfig) => {
  let api: BspNetApi | undefined;
  try {
    console.log(`sh user id: ${shUser.address}`);
    console.log(`sh bsp id: ${bsp.address}`);
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
    await api.sealBlock(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bsp.address, amount)));
    await api.sealBlock(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(shUser.address, amount)));

    // Make BSP
    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.providers.forceBspSignUp(
          bsp.address,
          DUMMY_BSP_ID,
          CAPACITY_512,
          [multiAddressBsp],
          bsp.address
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

export const closeBspNet = async () => {
  const docker = new Docker();

  const existingNodes = await docker.listContainers({
    filters: { ancestor: [DOCKER_IMAGE] }
  });

  const promises = existingNodes.map(async (node) => docker.getContainer(node.Id).stop());
  await Promise.all(promises);

  await docker.pruneContainers();
  await docker.pruneVolumes();
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
  calls?: SubmittableExtrinsic<"promise", ISubmittableResult> | SubmittableExtrinsic<"promise", ISubmittableResult>[],
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
    txHashes: results.hashes.map(hash => hash.toString())
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
  await closeBspNet();
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

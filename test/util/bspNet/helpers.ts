import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import compose from "docker-compose";
import { alice, bsp, shUser } from "../pjsKeyring";
import { DUMMY_MSP_ID, VALUE_PROP, NODE_INFOS, DUMMY_BSP_ID, CAPACITY_512 } from "./consts";
import { createApiObject } from "./api";
import path from "node:path";
import { u8aToHex } from "@polkadot/util";
import * as util from "node:util";
import * as child_process from "node:child_process";
import type { BspNetApi } from "./types";
import type { CreatedBlock, EventRecord, Hash, SignedBlock } from "@polkadot/types/interfaces";
const exec = util.promisify(child_process.exec);

export const sendFileSendRpc = async (
  api: ApiPromise,
  filePath: string,
  remotePath: string,
  userNodeAccountId: string
): Promise<FileSendResponse> => {
  try {
    // @ts-expect-error - rpc provider not officially exposed
    const resp = await api._rpcCore.provider.send("filestorage_loadFileInStorage", [
      filePath,
      remotePath,
      userNodeAccountId,
    ]);
    const { owner, location, size, fingerprint } = resp;
    return {
      owner: u8aToHex(owner),
      location: u8aToHex(location),
      size: BigInt(size),
      fingerprint: u8aToHex(fingerprint),
    };
  } catch (e) {
    console.error("Error sending file to user node:", e);
    throw new Error("filestorage_loadFileInStorage RPC call failed");
  }
};

export const getContainerIp = async (containerName: string, verbose = false): Promise<string> => {
  for (let i = 0; i < 20; i++) {
    verbose && console.log(`Waiting for ${containerName} to launch...`);

    try {
      const { stdout } = await exec(
        `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${containerName}`
      );
      return stdout.trim();
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Error fetching container IP for ${containerName}`);
};

export interface FileSendResponse {
  owner: string;
  location: string;
  size: bigint;
  fingerprint: string;
}

export const getContainerPeerId = async (url: string, verbose = false) => {
  const payload = {
    id: "1",
    jsonrpc: "2.0",
    method: "system_localPeerId",
    params: [],
  };

  for (let i = 0; i < 10; i++) {
    verbose && console.log(`Waiting for node at ${url} to launch...`);

    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const resp = (await response.json()) as any;
      return resp.result as string;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Error fetching peerId from ${url}`);
};

export const runBspNet = async () => {
  let api: BspNetApi | undefined;

  try {
    console.log(`sh user id: ${shUser.address}`);
    console.log(`sh bsp id: ${bsp.address}`);
    const composeFilePath = path.resolve(
      process.cwd(),
      "..",
      "docker",
      "local-dev-bsp-compose.yml"
    );

    await compose.upOne("sh-bsp", { config: composeFilePath, log: true });

    const bspIp = await getContainerIp(NODE_INFOS.bsp.containerName);
    console.log(`sh-bsp IP: ${bspIp}`);

    const bspPeerId = await getContainerPeerId(`http://127.0.0.1:${NODE_INFOS.bsp.port}`);
    console.log(`sh-bsp Peer ID: ${bspPeerId}`);

    process.env.BSP_IP = bspIp;
    process.env.BSP_PEER_ID = bspPeerId;

    await compose.upOne("sh-user", {
      config: composeFilePath,
      log: true,
      env: {
        ...process.env,
        BSP_IP: bspIp,
        BSP_PEER_ID: bspPeerId,
      },
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
            protocols: ["https", "ssh", "telnet"],
          },
          alice.address
        )
      )
    );
  } catch (e) {
    console.error("Error sending file to user node:", e);
  } finally {
    api?.disconnect();
  }
};

export const closeBspNet = async () => {
  const composeFilePath = path.resolve(process.cwd(), "..", "docker", "local-dev-bsp-compose.yml");

  return compose.down({
    config: composeFilePath,
    log: true,
  });
};

// TODO: Add a succesful flag to track whether ext was successful or not
export interface SealedBlock {
  blockReceipt: CreatedBlock;
  txHash?: string;
  blockData?: SignedBlock;
  events?: EventRecord[];
}

export const sealBlock = async (
  api: ApiPromise,
  call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
  signer?: KeyringPair
): Promise<SealedBlock> => {
  const results: {
    hash?: Hash;
    events?: EventRecord[];
    blockData?: SignedBlock;
  } = {};

  // TODO: extend to take multiple exts in one block
  if (call) {
    results.hash = await call.signAndSend(signer || alice);
  }

  // TODO: Accept ext hash strings as well
  // if (call && !call?.isEmpty && !call?.isSigned) {
  //   const tx =  api.tx(call)
  //   results.hash = await tx.signAndSend(signer || alice)
  // }

  const sealedResults = {
    blockReceipt: await api.rpc.engine.createBlock(true, true),
    txHash: results.hash?.toString(),
  };

  if (results.hash) {
    const blockHash = sealedResults.blockReceipt.blockHash;
    const allEvents = await (await api.at(blockHash)).query.system.events();
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
  }

  return Object.assign(sealedResults, {
    events: results.events,
  }) satisfies SealedBlock;
};

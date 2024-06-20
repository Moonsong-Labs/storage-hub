import type { ApiPromise } from "@polkadot/api";
import type { SubmittableExtrinsic } from "@polkadot/api/types";
import type { ISubmittableResult } from "@polkadot/types/types";
import type { KeyringPair } from "@polkadot/keyring/types";
import compose from "docker-compose";
import { alice, bsp, shUser } from "../pjsKeyring";
import { nodeInfo } from "./consts";
import { createApiObject } from "./api";
import path from "node:path";
import { u8aToHex } from "@polkadot/util";
import * as util from "node:util";
import * as child_process from "node:child_process";
import type { BspNetApi } from "./types";
const exec = util.promisify(child_process.exec);

export const sendFileSendRpc = async (
  api: ApiPromise,
  filePath: string,
  remotePath: string,
  userNodeAccountId: string,
): Promise<FileSendResponse> => {
  try {
    // @ts-expect-error - rpc provider not officially exposed
    const resp = await api._rpcCore.provider.send(
      "filestorage_loadFileInStorage",
      [filePath, remotePath, userNodeAccountId],
    );
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

export const getContainerIp = async (
  containerName: string,
  verbose = false,
): Promise<string> => {
  for (let i = 0; i < 20; i++) {
    verbose && console.log(`Waiting for ${containerName} to launch...`);

    try {
      const { stdout } = await exec(
        `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${containerName}`,
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
      "local-dev-bsp-compose.yml",
    );

    await compose.upOne("sh-bsp", { config: composeFilePath, log: true });

    const bspIp = await getContainerIp(nodeInfo.bsp.containerName);
    console.log(`sh-bsp IP: ${bspIp}`);

    const bspPeerId = await getContainerPeerId(
      `http://127.0.0.1:${nodeInfo.bsp.port}`,
    );
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

    const peerIDUser = await getContainerPeerId(
      `http://127.0.0.1:${nodeInfo.user.port}`,
    );
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${bspPeerId}`;

    // Create Connection API Object to User Node
    api = await createApiObject(`ws://127.0.0.1:${nodeInfo.user.port}`);

    // Give Balances
    const amount = 10000n * 10n ** 12n;
    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bsp.address, amount)),
    );
    await api.sealBlock(
      api.tx.sudo.sudo(api.tx.balances.forceSetBalance(shUser.address, amount)),
    );

    // Make BSP
    // This is hardcoded to be same as fingerprint of whatsup.jpg
    // This is to game the XOR so that this BSP is always chosen by network
    const bspId =
      "0x002aaf768af5b738eea96084f10dac7ad4f6efa257782bdb9823994ffb233300";
    const capacity = 1024n * 1024n * 512n; // 512 MB

    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.providers.forceBspSignUp(
          bsp.address,
          bspId,
          capacity,
          [multiAddressBsp],
          bsp.address,
        ),
      ),
    );

    // Make MSP
    const mspId =
      "0x0000000000000000000000000000000000000000000000000000000000000300";
    const valueProp =
      "0x0000000000000000000000000000000000000000000000000000000000000770";
    await api.sealBlock(
      api.tx.sudo.sudo(
        api.tx.providers.forceMspSignUp(
          alice.address,
          mspId,
          capacity,
          [multiAddressBsp],
          {
            identifier: valueProp,
            dataLimit: 500,
            protocols: ["https", "ssh", "telnet"],
          },
          alice.address,
        ),
      ),
    );

    // Issue file Storage request
    const rpcResponse = await sendFileSendRpc(
      api,
      "/res/whatsup.jpg",
      "cat/whatsup.jpg",
      nodeInfo.user.AddressId,
    );

    console.log(rpcResponse);

    await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(
        "cat/whatsup.jpg",
        rpcResponse.fingerprint,
        rpcResponse.size,
        mspId,
        [peerIDUser],
      ),
      shUser,
    );

    // Seal the block from BSP volunteer
    await api.sealBlock();
  } catch (e) {
    console.error("Error sending file to user node:", e);
  } finally {
    api?.disconnect();
  }
};

export const closeBspNet = async () => {
  const composeFilePath = path.resolve(
    process.cwd(),
    "..",
    "docker",
    "local-dev-bsp-compose.yml",
  );

  return compose.down({
    config: composeFilePath,
    log: true,
  });
};

export const sealBlock = async (
  api: ApiPromise,
  call?: SubmittableExtrinsic<"promise", ISubmittableResult>,
  signer?: KeyringPair,
) => {
  if (call) {
    await call.signAndSend(signer || alice);
  }
  const resp = await api.rpc.engine.createBlock(true, true);
  return resp;
};

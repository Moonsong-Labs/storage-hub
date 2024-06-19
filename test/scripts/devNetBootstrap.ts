import "@storagehub/api-augment";
import * as compose from "docker-compose";
import * as util from "node:util";
import * as child_process from "node:child_process";
import * as path from "node:path";
import { ApiPromise, WsProvider } from "@polkadot/api";
import { alice, bsp, sealBlock, shUser } from "../util";
import { u8aToHex } from "@polkadot/util";

const exec = util.promisify(child_process.exec);

interface FileSendResponse {
  owner: string;
  location: string;
  size: bigint;
  fingerprint: string;
}

const getContainerPeerId = async (url: string, verbose = false) => {
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

const nodeInfo = {
  user: {
    containerName: "docker-sh-user-1",
    port: 9977,
    p2pPort: 30444,
    AddressId: "5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o",
    expectedPeerId: "12D3KooWMvbhtYjbhgjoDzbnf71SFznJAKBBkSGYEUtnpES1y9tM"
  },
  bsp: {
    containerName: "docker-sh-bsp-1",
    port: 9966,
    p2pPort: 30350,
    AddressId: "5FHSHEFWHVGDnyiw66DoRUpLyh5RouWkXo9GT1Sjk8qw7MAg",
    expectedPeerId: "12D3KooWNEZ8PGNydcdXTYy1SPHvkP9mbxdtTqGGFVrhorDzeTfU"
  },
  collator: {
    containerName: "docker-sh-collator-1",
    port: 9955,
    p2pPort: 30333,
    AddressId: "5C8NC6YuAivp3knYC58Taycx2scQoDcDd3MCEEgyw36Gh1R4",
  },
} as const;

const getContainerIp = async (
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

const sendFileSendRpc = async (
  url: string,
  filePath: string,
  remotePath: string,
  userNodeAccountId: string,
): Promise<FileSendResponse> => {
  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "filestorage_loadFileInStorage",
        params: [filePath, remotePath, userNodeAccountId],
      }),
    });

    const resp = (await response.json()) as any;
    const { owner, location, size, fingerprint } = resp.result;
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

let api: ApiPromise;

async function main() {
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

  api = await ApiPromise.create({
    provider: new WsProvider(`ws://127.0.0.1:${nodeInfo.user.port}`),
    noInitWarn: true,
  });

  // Give Balances
  const amount = 10000n * 10n ** 12n;
  await sealBlock(
    api,
    api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bsp.address, amount)),
  );
  await sealBlock(
    api,
    api.tx.sudo.sudo(api.tx.balances.forceSetBalance(shUser.address, amount)),
  );

  // Make BSP
  // This is hardcoded to be same as fingerprint of whatsup.jpg
  // This is to game the XOR so that this BSP is always chosen by network

  const bspId =
    "0x002aaf768af5b738eea96084f10dac7ad4f6efa257782bdb9823994ffb233300";
  const capacity = 1024n * 1024n * 512n; // 512 MB

  await sealBlock(
    api,
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
  await sealBlock(
    api,
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
    `http://127.0.0.1:${nodeInfo.user.port}`,
    "/res/whatsup.jpg",
    "tim/whatsup.jpg",
    nodeInfo.user.AddressId,
  );

  console.log(rpcResponse);
 

  await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      "tim/whatsup.jpg",
      rpcResponse.fingerprint,
      rpcResponse.size,
      mspId,
      [peerIDUser],
    ),
    shUser,
  );
  
  // Seal the block from BSP volunteer
  await sealBlock(api)
}

main()
  .catch((err) => {
    console.error("Error running bootstrap script:", err);
  })
  .finally(() => {
    api.disconnect();
  });

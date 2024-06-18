import "@storagehub/api-augment";
import * as compose from "docker-compose";
import * as util from "node:util";
import * as child_process from "node:child_process";
import * as path from "node:path";
import { ApiPromise, WsProvider } from "@polkadot/api";
import {  bsp, collator, sendTransaction, shUser } from "../util";

const exec = util.promisify(child_process.exec);

async function getContainerIp(containerName: string): Promise<string> {
  const { stdout } = await exec(
    `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${containerName}`
  );
  return stdout.trim();
}

async function getContainerPeerId(url: string): Promise<string> {
  const response = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "system_localPeerId",
      params: [],
    }),
  });
  const data = (await response.json()) as any;
  return data.result;
}

async function sendFileSendRpc(
  url: string,
  filePath: string,
  remotePath: string,
  userNodeAccountId: string
) {
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

    const data = (await response.json()) as any;
    return data.result;
  } catch (e) {
    console.error("Error sending file to user node:", e);
  }
}


let api: ApiPromise;

async function main() {
  console.log(`sh user id: ${shUser.address}`);
  console.log(`sh bsp id: ${bsp.address}`);
  console.log(`collator id: ${collator.address}`);
  const composeFilePath = path.resolve(process.cwd(), "..", "docker", "local-dev-bsp-compose.yml");

  await compose.upOne("sh-collator", { config: composeFilePath, log: true });

  //todo replace with poll
  console.log("Waiting for sh-collator to start...");
  await new Promise((resolve) => setTimeout(resolve, 10000));

  const collatorIp = await getContainerIp("docker-sh-collator-1");
  console.log(`sh-collator IP: ${collatorIp}`);

  const collatorPeerId = await getContainerPeerId("http://127.0.0.1:9955");
  console.log(`sh-collator Peer ID: ${collatorPeerId}`);

  process.env.COLLATOR_IP = collatorIp;
  process.env.COLLATOR_PEER_ID = collatorPeerId;

  await compose.upMany(["sh-bsp", "sh-user"], {
    config: composeFilePath,
    log: true,
    env: { ...process.env, COLLATOR_IP: collatorIp, COLLATOR_PEER_ID: collatorPeerId },
  });

  //todo replace with poll
  console.log("Waiting for other processes to start...");
  await new Promise((resolve) => setTimeout(resolve, 10000));

  //TODO: Get BSP peerID and pass to multiaddressesVec
  const bspIp = await getContainerIp("docker-sh-collator-1");
  console.log(`sh-bsp IP: ${bspIp}`);

  const peerIDBSP = await getContainerPeerId("http://127.0.0.1:9966");
  console.log(`sh-bsp Peer ID: ${peerIDBSP}`);

  const multiAddressBsp = `/ip4/${bspIp}/tcp/30350/p2p/${peerIDBSP}`;

  api = await ApiPromise.create({
    provider: new WsProvider("ws://127.0.0.1:9955"),
    noInitWarn: true,
    rpc: {
      filestorage: {
        loadFileInStorage: {
          description: "Load file in storage",
          params: [
            {
              name: "localPath",
              type: "string",
            },
            {
              name: "remotePath",
              type: "string",
            },
            {
              name: "userNodeAccountId",
              type: "AccountId",
            },
          ],
          type: "Result<(Owner, Location, Fingerprint), DispatchError>",
        },
      },
    },
  });

  // Give Balances
  const amount = 10000n * 10n ** 12n;
  await sendTransaction(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(bsp.address, amount)));
  await sendTransaction(
    api.tx.sudo.sudo(api.tx.balances.forceSetBalance(collator.address, amount))
  );
  await sendTransaction(api.tx.sudo.sudo(api.tx.balances.forceSetBalance(shUser.address, amount)));

  // Make BSP

  // This is hardcoded to be same as fingerprint of whatsup.jpg
  const bspId = "0x002aaf768af5b738eea96084f10dac7ad4f6efa257782bdb9823994ffb233300";
  const capacity = 1024n * 1024n * 512n; // 512 MB

  await sendTransaction(
    api.tx.sudo.sudo(
      api.tx.providers.forceBspSignUp(bsp.address, bspId, capacity, [multiAddressBsp], bsp.address)
    )
  );

  // Make MSP
  const mspId = "0x0000000000000000000000000000000000000000000000000000000000000300";
  const valueProp = "0x0000000000000000000000000000000000000000000000000000000000000770";
  await sendTransaction(
    api.tx.sudo.sudo(
      api.tx.providers.forceMspSignUp(
        collator.address,
        mspId,
        capacity,
        [multiAddressBsp],
        { identifier: valueProp, dataLimit: 500, protocols: ["https", "ssh", "telnet"] },
        collator.address
      )
    )
  );

  // Issue file Storage request
  // we need to upload and merkle file to user node
  // we user rpc method: filestorage_loadFileInStorage

  //   params:
  // local path
  //   remote path,
  //   user node acocunt identity

  //   result returned is:

  //   owner,
  //    location
  //   fingerprint

  // api.rpc.filestorage.loadFileInStorage(localPath, remotePath, userNodeAccountId)

  ////
  // Fileissue storage request
  // location has to be remote path
  // fingerprint has to  be what was from response
  // size has to match
  // mspId has to match
  // peerId has to be of the user node
}

main()
  .catch((err) => {
    console.error("Error running bootstrap script:", err);
  })
  .finally(() => {
    api.disconnect();
  });

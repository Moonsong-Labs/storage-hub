import {
  createApiObject,
  DUMMY_MSP_ID,
  getContainerPeerId,
  NODE_INFOS,
  runBspNet,
  shUser,
  type ToxicInfo
} from "../util";
import { setTimeout } from "node:timers/promises";

const registerToxic = async (toxicDef: ToxicInfo) => {
  const url = `http://localhost:${NODE_INFOS.toxiproxy.port}/proxies/sh-bsp-proxy/toxics`;

  const options: RequestInit = {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(toxicDef)
  };

  const resp = await fetch(url, options);

  return resp.json();
};

const getToxics = async () => {
  const url = `http://localhost:${NODE_INFOS.toxiproxy.port}/proxies/sh-bsp-proxy/toxics`;
  const resp = await fetch(url);
  return resp.json();
};

async function bootStrapNetwork() {
  try {
    await runBspNet(true);

    // For more info on the kind of toxics you can register,
    // see: https://github.com/Shopify/toxiproxy?tab=readme-ov-file#toxics
    const reqToxics = [
      {
        type: "latency",
        name: "lag-down",
        stream: "downstream",
        toxicity: 1,
        attributes: {
          latency: 50,
          jitter: 10
        }
      },
      {
        type: "latency",
        name: "lag-up",
        stream: "upstream",
        toxicity: 1,
        attributes: {
          latency: 50,
          jitter: 10
        }
      }
    ] satisfies ToxicInfo[];

    // Register toxics with proxy server
    const promises = reqToxics.map(registerToxic);
    await Promise.all(promises);

    // Verify each toxic is registered
    const receivedToxics: any = await getToxics();

    if (receivedToxics.length !== reqToxics.length) {
      console.log("❌ Toxic registration failed");
      console.log(receivedToxics);
      throw new Error("Toxic registration failed");
    }

    // Send file
    var api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);

    const bucketName = "nothingmuch-0";
    const newBucketEventEvent = await api.createBucket(bucketName);
    const newBucketEventDataBlob =
      api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    const localPath = "res/whatsup.jpg";
    const remotePath = "cat/whatsup.jpg";

    // Issue file Storage request
    const rpcResponse = await api.loadFile(
      localPath,
      remotePath,
      NODE_INFOS.user.AddressId,
      newBucketEventDataBlob.bucketId
    );
    console.log(rpcResponse);

    const peerIDUser = await getContainerPeerId(`http://127.0.0.1:${NODE_INFOS.user.port}`);
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(
        rpcResponse.bucket_id,
        remotePath,
        rpcResponse.fingerprint,
        rpcResponse.size,
        DUMMY_MSP_ID,
        [peerIDUser]
      ),
      shUser
    );

    // Seal the block from BSP volunteer
    await setTimeout(1000);
    await api.sealBlock();

    console.log("✅ NoisyNet Bootstrap success");
  } catch (e) {
    console.error("Error running bootstrap script:", e);
    console.log("❌ BSPNet Bootstrap failure");
    process.exitCode = 1;
  }

  // @ts-expect-error - bug in tsc
  await api.disconnect();
}

bootStrapNetwork();

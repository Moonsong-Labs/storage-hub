import { setTimeout } from "node:timers/promises";
import {
  createApiObject,
  createCheckBucket,
  DUMMY_MSP_ID,
  getContainerPeerId,
  NODE_INFOS,
  registerToxics,
  runBspNet,
  shUser,
  type BspNetApi,
  type BspNetConfig,
  type ToxicInfo
} from "../util";

let api: BspNetApi | undefined;
const bspNetConfig: BspNetConfig = {
  noisy: process.env.NOISY === "1" ?? false,
  rocksdb: process.env.ROCKSDB === "1" ?? false
};

const CONFIG = {
  bucketName: "nothingmuch-0",
  localPath: "res/whatsup.jpg",
  remotePath: "cat/whatsup.jpg"
};

async function bootStrapNetwork() {
  await runBspNet(bspNetConfig);

  if (bspNetConfig.noisy) {
    // For more info on the kind of toxics you can register,
    // see: https://github.com/Shopify/toxiproxy?tab=readme-ov-file#toxics
    const reqToxics = [
      {
        type: "latency",
        name: "lag-down",
        stream: "upstream",
        toxicity: 0.8,
        attributes: {
          latency: 25,
          jitter: 7
        }
      },
      {
        type: "bandwidth",
        name: "low-band",
        // Setting as upstream simulates slow user connection
        stream: "upstream",
        // 50% of the time, the toxic will be applied
        toxicity: 0.5,
        attributes: {
          // 10kbps
          rate: 10
        }
      }
    ] satisfies ToxicInfo[];

    await registerToxics(reqToxics);
  }

  api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);

  const newBucketEventDataBlob = await createCheckBucket(api, CONFIG.bucketName);

  const localPath = CONFIG.localPath;
  const remotePath = CONFIG.remotePath;

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

  if (bspNetConfig.noisy) {
    console.log("✅ NoisyNet Bootstrap success");
  } else {
    console.log("✅ BSPNet Bootstrap success");
  }
}

bootStrapNetwork()
  .catch((e) => {
    console.error("Error running bootstrap script:", e);
    if (bspNetconfig.noisy) {
      console.log("❌ NoisyNet Bootstrap failure");
    } else {
      console.log("❌ BSPNet Bootstrap failure");
    }
    process.exitCode = 1;
  })
  .finally(async () => await api?.disconnect());

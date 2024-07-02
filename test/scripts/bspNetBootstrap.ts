import { setTimeout } from "node:timers/promises";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  createApiObject,
  createCheckBucket,
  getContainerPeerId,
  runBspNet,
  shUser,
  type BspNetApi
} from "../util";

let api: BspNetApi | undefined;

const CONFIG = {
  bucketName: "nothingmuch-0",
  localPath: "res/whatsup.jpg",
  remotePath: "cat/whatsup.jpg"
};

async function bootStrapNetwork() {
  await runBspNet();
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
  console.log("✅ BSPNet Bootstrap success");
}

bootStrapNetwork()
  .catch((e) => {
    console.error("Error running bootstrap script:", e);
    console.log("❌ BSPNet Bootstrap failure");
    process.exitCode = 1;
  })
  .finally(async () => await api?.disconnect());

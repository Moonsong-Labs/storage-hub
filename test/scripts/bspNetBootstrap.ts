import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  createApiObject,
  getContainerPeerId,
  runBspNet,
  shUser
} from "../util";
import { setTimeout } from "node:timers/promises";

async function bootStrapNetwork() {
  try {
    await runBspNet();
    // biome-ignore lint/style/noVar: this is neater
    // biome-ignore lint/correctness/noInnerDeclarations: this is neater
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
    console.log("✅ BSPNet Bootstrap success");
  } catch (e) {
    console.error("Error running bootstrap script:", e);
    console.log("❌ BSPNet Bootstrap failure");
    process.exitCode = 1;
  }

  // @ts-expect-error - bug in tsc
  await api.disconnect();
}

bootStrapNetwork();

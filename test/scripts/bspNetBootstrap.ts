import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  createApiObject,
  getContainerPeerId,
  runBspNet,
  sendFileSendRpc,
  shUser,
  type BspNetApi,
} from "../util";
import { setTimeout } from "timers/promises"

let api: BspNetApi;

runBspNet()
  .then(async () => {
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);

    // Issue file Storage request
    const rpcResponse = await sendFileSendRpc(
      api,
      "/res/whatsup.jpg",
      "cat/whatsup.jpg",
      NODE_INFOS.user.AddressId,
    );
    console.log(rpcResponse);

    const peerIDUser = await getContainerPeerId(
      `http://127.0.0.1:${NODE_INFOS.user.port}`,
    );
    console.log(`sh-user Peer ID: ${peerIDUser}`);

    await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(
        "cat/whatsup.jpg",
        rpcResponse.fingerprint,
        rpcResponse.size,
        DUMMY_MSP_ID,
        [peerIDUser],
      ),
      shUser,
    );

    // Seal the block from BSP volunteer
    await setTimeout(1000)
    await api.sealBlock();

    console.log("✅ BSPNet Bootstrap success");
  })
  .catch((err) => {
    console.error("Error running bootstrap script:", err);
    console.log("❌ BSPNet Bootstrap failure");
  })
  .finally(() => {
    api?.disconnect()   
    console.log("🏁 BSPNet Bootstrap script completed");
  });

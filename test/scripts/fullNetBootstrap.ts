import { BspNetTestApi, type BspNetConfig } from "../util";
import * as ShConsts from "../util/bspNet/consts";
import { NetworkLauncher } from "../util/netLaunch";

const bspNetConfig: BspNetConfig = {
  noisy: process.env.NOISY === "1",
  rocksdb: process.env.ROCKSDB === "1"
};

const CONFIG = {
  bucketName: "nothingmuch-0",
  localPath: "res/whatsup.jpg",
  remotePath: "cat/whatsup.jpg"
};

async function bootStrapNetwork() {
  await NetworkLauncher.create("fullnet", bspNetConfig);

  await using api = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

  await api.file.newStorageRequest(
    CONFIG.localPath,
    CONFIG.remotePath,
    CONFIG.bucketName,
    ShConsts.DUMMY_MSP_ID
  );

  await api.wait.bspVolunteer();
  await api.wait.bspStored();

  console.log("✅ FullNet Bootstrap success");
}

bootStrapNetwork().catch((e) => {
  console.error("Error running bootstrap script:", e);
  console.log("❌ BSPNet Bootstrap failure");
  process.exitCode = 1;
});
// .finally(async () => await api?.disconnect());

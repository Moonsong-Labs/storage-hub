import { BspNetTestApi, registerToxics, type ToxicInfo } from "../util";
import * as ShConsts from "../util/bspNet/consts";
import { NetworkLauncher, type NetLaunchConfig } from "../util/netLaunch";

const bspNetConfig: NetLaunchConfig = {
  noisy: process.env.NOISY === "1",
  rocksdb: process.env.ROCKSDB === "1"
};

const CONFIG = {
  bucketName: "nothingmuch-0",
  localPath: "res/whatsup.jpg",
  remotePath: "cat/whatsup.jpg"
};

async function bootStrapNetwork() {
  await NetworkLauncher.create("bspnet", bspNetConfig);

  if (bspNetConfig.noisy) {
    // For more info on the kind of toxics you can register,
    // see: https://github.com/Shopify/toxiproxy?tab=readme-ov-file#toxics
    const reqToxics =
      bspNetConfig.toxics ??
      ([
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
      ] satisfies ToxicInfo[]);

    await registerToxics(reqToxics);
  }

  await using api = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

  await api.file.createBucketAndSendNewStorageRequest(
    CONFIG.localPath,
    CONFIG.remotePath,
    CONFIG.bucketName
  );

  await api.wait.bspVolunteer();
  await api.wait.bspStored();

  if (bspNetConfig.noisy) {
    console.log("✅ NoisyNet Bootstrap success");
  } else {
    console.log("✅ BSPNet Bootstrap success");
  }
}

bootStrapNetwork().catch((e) => {
  console.error("Error running bootstrap script:", e);
  if (bspNetConfig.noisy) {
    console.log("❌ NoisyNet Bootstrap failure");
  } else {
    console.log("❌ BSPNet Bootstrap failure");
  }
  process.exitCode = 1;
});

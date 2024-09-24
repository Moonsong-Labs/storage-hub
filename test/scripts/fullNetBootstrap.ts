import {
  BspNetTestApi,
  registerToxics,
  type BspNetConfig,
  type EnrichedBspApi,
  type ToxicInfo
} from "../util";
import * as ShConsts from "../util/bspNet/consts";
import { runFullNet } from "../util/fullNet/helpers";

let api: EnrichedBspApi | undefined;
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
  await runFullNet(bspNetConfig);

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

  api = await BspNetTestApi.create(`ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`);

  await api.file.newStorageRequest(CONFIG.localPath, CONFIG.remotePath, CONFIG.bucketName, ShConsts.DUMMY_MSP_ID);

  await api.wait.bspVolunteer();
  await api.wait.bspStored();

  if (bspNetConfig.noisy) {
    console.log("✅ NoisyNet Bootstrap success");
  } else {
    console.log("✅ BSPNet Bootstrap success");
  }
}

bootStrapNetwork()
  .catch((e) => {
    console.error("Error running bootstrap script:", e);
    if (bspNetConfig.noisy) {
      console.log("❌ NoisyNet Bootstrap failure");
    } else {
      console.log("❌ BSPNet Bootstrap failure");
    }
    process.exitCode = 1;
  })
  .finally(async () => await api?.disconnect());

import type { BspNetConfig } from "../util";
import { NetworkLauncher } from "../util/netLaunch";

const bspNetConfig: BspNetConfig = {
  noisy: process.env.NOISY === "1",
  rocksdb: process.env.ROCKSDB === "1",
  indexer: process.env.INDEXER === "1"
};

async function bootStrapNetwork() {
  await NetworkLauncher.create("fullnet", {
    ...bspNetConfig,
    initialised: true,
    extrinsicRetryTimeout: 60 * 30, // 30 minutes
    runtimeType: "solochain"
  });

  console.log("✅ Solochain EVM Bootstrap success");
}

await bootStrapNetwork().catch((e) => {
  console.error("Error running bootstrap script:", e);
  console.log("❌ Solochain EVM Bootstrap Demo failure");
  process.exitCode = 1;
});

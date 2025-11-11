import type { ApiPromise } from "@polkadot/api";
import type { BspNetConfig } from "../util";
import { BspNetTestApi, ShConsts, sleep } from "../util";
import { cleanupEnvironment } from "../util/helpers";
import { NetworkLauncher } from "../util/netLaunch";

let currentApi: ApiPromise | undefined;

let isTearingDown = false;
const tearDownNetwork = async () => {
  if (isTearingDown) return;
  isTearingDown = true;
  try {
    console.log("\n🛑 Interrupt received. Tearing down network containers...");
    try {
      await currentApi?.disconnect();
    } catch (e) {
      console.error("Error disconnecting API:", e);
    }
    await cleanupEnvironment();
  } finally {
    process.exit(0);
  }
};

process.on("SIGINT", () => void tearDownNetwork());
process.on("SIGTERM", () => void tearDownNetwork());

const bspNetConfig: BspNetConfig = {
  noisy: process.env.NOISY === "1",
  rocksdb: process.env.ROCKSDB === "1",
  indexer: process.env.INDEXER === "1",
  backend: process.env.BACKEND === "1"
};

async function bootStrapNetwork() {
  await NetworkLauncher.create("fullnet", {
    ...bspNetConfig,
    initialised: true,
    extrinsicRetryTimeout: 60 * 30, // 30 minutes
    runtimeType: "solochain",
    fisherman: true,
    backend: true
  });

  console.log("✅ Solochain EVM Bootstrap success");

  await using api = await BspNetTestApi.create(
    `ws://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`,
    "solochain"
  );
  currentApi = api as unknown as ApiPromise;

  console.log("⛏️  Auto-sealing blocks every 6s. Press Ctrl+C to stop (will tear down network).");
  // Keep sealing blocks every 6 seconds until the process is interrupted
  // eslint-disable-next-line no-constant-condition
  while (true) {
    try {
      await api.block.seal();
    } catch (e) {
      console.error("Auto-seal error:", e);
    }
    await sleep(6000);
  }
}

await bootStrapNetwork().catch((e) => {
  console.error("Error running bootstrap script:", e);
  console.log("❌ Solochain EVM Bootstrap Demo failure");
  process.exitCode = 1;
});

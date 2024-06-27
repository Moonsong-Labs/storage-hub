import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  TEST_ARTEFACTS,
  createApiObject,
  fetchEventData,
  runBspNet,
  shUser,
  checkBspForFile,
  checkFileChecksum,
  type BspNetApi,
  cleardownTest,
  sleep,
  addBspContainer,
} from "../../util";
import { hexToString } from "@polkadot/util";

describe("BSPNet: BSP Onboarding", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet();
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
  });

  after(async () => {
    // await new Promise((resolve) => setTimeout(resolve, 5_000_000));
    await cleardownTest(api);
  });

  it("New BSP can be created", async () => {
    await addBspContainer();
  });
});

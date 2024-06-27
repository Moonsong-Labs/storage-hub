import "@storagehub/api-augment";
import {after, before, describe, it} from "node:test";
import {addBspContainer, type BspNetApi, cleardownTest, createApiObject, NODE_INFOS, runBspNet,} from "../../util";

describe("BSPNet: Adding new BSPs", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet();
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
  });

  after(async () => {
    // await cleardownTest(api);
  });

  it("New BSP can be created", async () => {
    await addBspContainer();

    // new bsp can be connected to API

    // new bsp is peer of other nodes
  });

  // multiple new bsps
});
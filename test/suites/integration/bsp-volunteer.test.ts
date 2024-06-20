import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  closeBspNet,
  createApiObject,
  nodeInfo,
  runBspNet,
  type BspNetApi,
} from "../../util";

describe("BSPNet: BSP Volunteer", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet();
    api = await createApiObject(`ws://127.0.0.1:${nodeInfo.user.port}`);
  });

  after(async () => {
    await closeBspNet();
  });

  it("Network launches and can be queried", async () => {
    const userNodePeerId = await api.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), nodeInfo.user.expectedPeerId);


    const bspApi = await createApiObject(`ws://127.0.0.1:${nodeInfo.bsp.port}`);
    const bspNodePeerId = await bspApi.rpc.system.localPeerId();
    strictEqual(bspNodePeerId.toString(), nodeInfo.bsp.expectedPeerId);
  });
});

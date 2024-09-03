import "@storagehub/api-augment";
import { after, before, describe, it } from "node:test";
import {
  type BspNetApi,
  type BspNetConfig,
  cleardownTest,
  createApiObject,
  NODE_INFOS,
  runSimpleBspNet,
  waitForBspStored,
  waitForBspVolunteer
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

describe("Test Reproduction", async () => {
  let api: BspNetApi;

  before(async () => {
    await runSimpleBspNet(bspNetConfigCases[1]);
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
  });

  after(async () => {
    await cleardownTest({ api });
  });

  it("Can send file of 1MB size", async () => {
    const source = "res/1MB_file";
    const location = "test/1MB_file";
    const bucketName = "nothingmuch-2";
    await api.sendNewStorageRequest(source, location, bucketName);
    await waitForBspVolunteer(api);
    await waitForBspStored(api);
    // Check for error event
  });
});

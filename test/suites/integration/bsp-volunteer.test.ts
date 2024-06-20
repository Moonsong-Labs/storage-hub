import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  TEST_ARTEFACTS,
  closeBspNet,
  createApiObject,
  runBspNet,
  shUser,
  type BspNetApi,
} from "../../util";
import { hexToString } from "@polkadot/util";

describe("BSPNet: BSP Volunteer", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet();
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
  });

  after(async () => {
    await api.disconnect();
    await closeBspNet();
  });

  it("Network launches and can be queried", async () => {
    const userNodePeerId = await api.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

    const bspApi = await createApiObject(
      `ws://127.0.0.1:${NODE_INFOS.bsp.port}`,
    );
    const bspNodePeerId = await bspApi.rpc.system.localPeerId();
    bspApi.disconnect();
    strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
  });

  it("file is finger printed correctly", async () => {
    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const { fingerprint, size, location } = await api.sendFile(
      source,
      destination,
      NODE_INFOS.user.AddressId,
    );

    strictEqual(hexToString(location), destination);
    strictEqual(fingerprint, TEST_ARTEFACTS[source].fingerprint);
    strictEqual(size, TEST_ARTEFACTS[source].size);
  });

  it("issueStorageRequest sent correctly", async () => {
    const source = "res/smile.jpg";
    const destination = "test/smile.jpg";
    const { fingerprint, size, location } = await api.sendFile(
      source,
      destination,
      NODE_INFOS.user.AddressId,
    );

    const result = await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(
        location,
        fingerprint,
        size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId],
      ),
      shUser,
    );

    const event = api.assertEvent(
      "fileSystem",
      "NewStorageRequest",
      result.events,
    );

    const dataBlob = event.data.toHuman() as any;
    strictEqual(dataBlob["who"], NODE_INFOS.user.AddressId);
    strictEqual(dataBlob["location"], destination);
    strictEqual(dataBlob["fingerprint"], fingerprint);
    strictEqual(BigInt(dataBlob["size_"].replaceAll(",", "")), size);
    strictEqual(dataBlob["peerIds"].length, 1);
    strictEqual(dataBlob["peerIds"][0], NODE_INFOS.user.expectedPeerId);
  });

  // it("bsp volunteers when issueStorageRequest sent", async ()=>{

  // })

  // File can be copied from bsp and matches checksum
});

import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  DUMMY_MSP_ID,
  NODE_INFOS,
  TEST_ARTEFACTS,
  closeBspNet,
  createApiObject,
  fetchEventData,
  runBspNet,
  shUser,
  type BspNetApi,
} from "../../util";
import { type PalletFileSystemEvent } from "@polkadot/types/lookup";
import { hexToString, hexToU8a } from "@polkadot/util";
import { blake2AsU8a, encodeAddress } from "@polkadot/util-crypto";

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

    const { event } = api.assertEvent(
      "fileSystem",
      "NewStorageRequest",
      result.events,
    );

    const dataBlob =
      api.events.fileSystem.NewStorageRequest.is(event) && event.data;

    if (!dataBlob) {
      throw new Error("Event doesn't match Type");
    }

    strictEqual(dataBlob.who.toString(), NODE_INFOS.user.AddressId);
    strictEqual(dataBlob.location.toHuman(), destination);
    strictEqual(dataBlob.fingerprint.toString(), fingerprint);
    strictEqual(dataBlob.size_.toBigInt(), size);
    strictEqual(dataBlob.peerIds.length, 1);
    strictEqual(dataBlob.peerIds[0].toHuman(), NODE_INFOS.user.expectedPeerId);
  });

  it("bsp volunteers when issueStorageRequest sent", async () => {
    const source = "res/whatsup.jpg";
    const destination = "test/whatsup.jpg";
    const { fingerprint, size, location } = await api.sendFile(
      source,
      destination,
      NODE_INFOS.user.AddressId,
    );

    await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(
        location,
        fingerprint,
        size,
        DUMMY_MSP_ID,
        [NODE_INFOS.user.expectedPeerId],
      ),
      shUser,
    );

    const pending = await api.rpc.author.pendingExtrinsics();
    strictEqual(
      pending.length,
      1,
      "There should be one pending extrinsic from BSP",
    );

    await api.sealBlock();
    const events = await api.query.system.events();

    const [resBspId, resLoc, resFinger, resMulti, resOwner, resSize] = fetchEventData(
      api.events.fileSystem.AcceptedBspVolunteer,
      events,
    );
    
    console.log(resBspId.toHuman())
    
    // TODO: Fix Address Encoding
    // strictEqual(encodeAddress(resBspId), NODE_INFOS.bsp.AddressId);
    strictEqual(resLoc.toHuman(), destination);
    strictEqual(resFinger.toString(), fingerprint);
    strictEqual(resMulti.length, 1);
    strictEqual((resMulti[0].toHuman() as string).includes(NODE_INFOS.bsp.expectedPeerId),true );
    strictEqual(resOwner.toString(), NODE_INFOS.user.AddressId);
    strictEqual(resSize.toBigInt(), size);

  });

  // File can be copied from bsp and matches checksum
});

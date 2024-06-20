import "@storagehub/api-augment";
import { strictEqual } from "node:assert";
import { execSync } from "node:child_process";
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
import { hexToString } from "@polkadot/util";

describe("BSPNet: BSP Volunteer", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet();
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
  });

  after(async () => {
    // await api.disconnect();
    // await closeBspNet();
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

  it("Successful file transfer", async () => {
    await it("bsp volunteers when issueStorageRequest sent", async () => {
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
      const [resBspId, resLoc, resFinger, resMulti, resOwner, resSize] =
        fetchEventData(
          api.events.fileSystem.AcceptedBspVolunteer,
          await api.query.system.events(),
        );

      strictEqual(resBspId.toHuman(), TEST_ARTEFACTS[source].fingerprint);
      strictEqual(resLoc.toHuman(), destination);
      strictEqual(resFinger.toString(), fingerprint);
      strictEqual(resMulti.length, 1);
      strictEqual(
        (resMulti[0].toHuman() as string).includes(
          NODE_INFOS.bsp.expectedPeerId,
        ),
        true,
      );
      strictEqual(resSize.toBigInt(), size);
    });
    
    //TODO: Fix below
    await it("file is downloaded successfully", async () => {
      await new Promise((resolve) => setTimeout(resolve, 1000))
      execSync("docker cp docker-sh-bsp-1:/storage/test/whatsup.jpg /tmp/", {
        stdio: "inherit",
      });
      const checksum = execSync(`sha256sum ./whatsup.jpg`, {
        cwd: "/tmp",
        stdio: "inherit",
      });

      console.log("remove me");
      console.log(checksum.toString());
      // compare checksum
    });
    
  });
  
});

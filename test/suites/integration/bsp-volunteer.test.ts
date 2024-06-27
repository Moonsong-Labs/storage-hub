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
} from "../../util";
import { hexToString } from "@polkadot/util";
import {randomBytes} from "node:crypto";

describe("BSPNet: BSP Volunteer", () => {
  let api: BspNetApi;

  before(async () => {
    await runBspNet();
    api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
  });

  after(async () => {
    // await cleardownTest(api);
  });

  it("Network launches and can be queried", async () => {
    const userNodePeerId = await api.rpc.system.localPeerId();
    strictEqual(userNodePeerId.toString(), NODE_INFOS.user.expectedPeerId);

    const bspApi = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.bsp.port}`);
    const bspNodePeerId = await bspApi.rpc.system.localPeerId();
    await bspApi.disconnect();
    strictEqual(bspNodePeerId.toString(), NODE_INFOS.bsp.expectedPeerId);
  });

  it("file is finger printed correctly", async () => {
    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const bucketId = randomBytes(16).toString('hex');
    const { fingerprint, size, location } = await api.sendFile(
      source,
      destination,
      NODE_INFOS.user.AddressId,
        bucketId
    );

    strictEqual(hexToString(location), destination);
    strictEqual(fingerprint, TEST_ARTEFACTS[source].fingerprint);
    strictEqual(size, TEST_ARTEFACTS[source].size);
  });

  async function createBucket(bucketName: string) {
    const createBucketResult = await api.sealBlock(
        api.tx.fileSystem.createBucket(DUMMY_MSP_ID, bucketName, false),
        shUser
    );
    const {event: newBucketEvent} = api.assertEvent("fileSystem", "NewBucket", createBucketResult.events);

    const newBucketEventDataBlob = api.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

    if (!newBucketEventDataBlob) {
      throw new Error("Event doesn't match Type");
    }

    return newBucketEventDataBlob;
  }

  it("issueStorageRequest sent correctly", async () => {
    const source = "res/smile.jpg";
    const destination = "test/smile.jpg";
    const bucketName = "nothingmuch-1";

    const newBucketEventDataBlob = await createBucket(bucketName);
    const bucketId = newBucketEventDataBlob.bucketId.toString();

    const { fingerprint, size, location } = await api.sendFile(
      source,
      destination,
      NODE_INFOS.user.AddressId,
      bucketId
    );

    const issueStorageRequestResult = await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(bucketId, location, fingerprint, size, DUMMY_MSP_ID, [
        NODE_INFOS.user.expectedPeerId,
      ]),
      shUser
    );
    await sleep(500); // wait for the bsp to volunteer

    const { event } = api.assertEvent("fileSystem", "NewStorageRequest", issueStorageRequestResult.events);

    const dataBlob = api.events.fileSystem.NewStorageRequest.is(event) && event.data;

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
    const bucketName = "nothingmuch-2";

    const newBucketEventDataBlob = await createBucket(bucketName);
    const bucketId = newBucketEventDataBlob.bucketId.toString();

    const { fingerprint, size, location } = await api.sendFile(
      source,
      destination,
      NODE_INFOS.user.AddressId,
      bucketId
    );

    await api.sealBlock(
      api.tx.fileSystem.issueStorageRequest(bucketId, location, fingerprint, size, DUMMY_MSP_ID, [
        NODE_INFOS.user.expectedPeerId,
      ]),
      shUser
    );

    await sleep(500); // wait for the bsp to volunteer
    const pending = await api.rpc.author.pendingExtrinsics();
    strictEqual(pending.length, 1, "There should be one pending extrinsic from BSP");

    await api.sealBlock();
    const [resBspId, resBucketId, resLoc, resFinger, resMulti, _, resSize] = fetchEventData(
      api.events.fileSystem.AcceptedBspVolunteer,
      await api.query.system.events()
    );

    strictEqual(resBspId.toHuman(), TEST_ARTEFACTS[source].fingerprint);
    strictEqual(resBucketId.toString(), bucketId);
    strictEqual(resLoc.toHuman(), destination);
    strictEqual(resFinger.toString(), fingerprint);
    strictEqual(resMulti.length, 1);
    strictEqual((resMulti[0].toHuman() as string).includes(NODE_INFOS.bsp.expectedPeerId), true);
    strictEqual(resSize.toBigInt(), size);

    await it("downloaded file passed integrity checks", async () => {
      await checkBspForFile("test/whatsup.jpg");
      const sha = await checkFileChecksum("test/whatsup.jpg");
      strictEqual(sha, TEST_ARTEFACTS["res/whatsup.jpg"].checksum);
    });
  });
});

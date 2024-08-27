import "@storagehub/api-augment";
import assert from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  assertExtrinsicPresent,
  bspKey,
  type BspNetApi,
  type BspNetConfig,
  CAPACITY,
  CAPACITY_512,
  cleardownTest,
  createApiObject,
  DUMMY_BSP_ID,
  ferdie,
  fetchEventData,
  NODE_INFOS,
  runSimpleBspNet,
  skipBlocks,
  skipBlocksToMinChangeTime,
  sleep
} from "../../../util";

const bspNetConfigCases: BspNetConfig[] = [
  { noisy: false, rocksdb: false },
  { noisy: false, rocksdb: true }
];

for (const bspNetConfig of bspNetConfigCases) {
  describe("BSPNet: Validating max storage", () => {
    let api: BspNetApi;

    before(async () => {
      await runSimpleBspNet(bspNetConfig);
      api = await createApiObject(`ws://127.0.0.1:${NODE_INFOS.user.port}`);
    });

    after(async () => {
      await cleardownTest({ api });
    });

    it("Unregistered accounts fail when changing capacities", async () => {
      const totalCapacityBefore = await api.query.providers.totalBspsCapacity();
      const bspCapacityBefore = await api.query.providers.backupStorageProviders(DUMMY_BSP_ID);
      assert.ok(bspCapacityBefore.unwrap().capacity.eq(totalCapacityBefore));

      const { events, extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(CAPACITY[1024]),
        ferdie
      );
      assert.strictEqual(extSuccess, false);

      await skipBlocks(api, 20);
      const [eventInfo, _eventError] = fetchEventData(api.events.system.ExtrinsicFailed, events);
      assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
      assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0f000000"); // NotRegistered

      const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
      const bspCapacityAfter = await api.query.providers.backupStorageProviders(DUMMY_BSP_ID);
      assert.ok(bspCapacityAfter.unwrap().capacity.eq(totalCapacityBefore));
      assert.ok(totalCapacityAfter.eq(totalCapacityBefore));
    });

    it("Maxed out storages not volunteered", { skip: "Not Implemented yet" }, async () => {
      const capacityUsed = (await api.query.providers.backupStorageProviders(DUMMY_BSP_ID))
        .unwrap()
        .capacityUsed.toNumber();
      await skipBlocksToMinChangeTime(api);
      const minCapacity = api.consts.providers.spMinCapacity.toNumber();
      const newCapacity = Math.max(minCapacity, capacityUsed + 1);

      const { extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(newCapacity),
        bspKey
      );
      assert.strictEqual(extSuccess, true);

      const source = "res/cloud.jpg";
      const location = "test/cloud.jpg";
      const bucketName = "toobig-1";
      await api.sendNewStorageRequest(source, location, bucketName);
      await sleep(500);

      await assert.rejects(
        async () => {
          assertExtrinsicPresent(api, {
            module: "fileSystem",
            method: "bspVolunteer",
            checkTxPool: true,
            skipSuccessCheck: true
          });
        },
        /No events matching system\.ExtrinsicSuccess/,
        "BSP should not have volunteered to a file that's too big"
      );
    });

    it("Total capacity updated when single BSP capacity updated", async () => {
      const newCapacity = BigInt(Math.floor(Math.random() * 1000 * 1024 * 1024)) + CAPACITY_512;

      await api.sealBlock(api.tx.providers.changeCapacity(newCapacity), bspKey);

      const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
      const bspCapacityAfter = await api.query.providers.backupStorageProviders(DUMMY_BSP_ID);
      assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), newCapacity);
      assert.strictEqual(totalCapacityAfter.toBigInt(), newCapacity);
    });

    it(
      "File sent for storage bigger than max filesize",
      { skip: "blocked by multi-proof defect" },
      async () => {
        const source = "res/big_chart.jpg";
        const location = "test/big_chart.jpg";
        const bucketName = "toobig-1";
        await api.sendNewStorageRequest(source, location, bucketName);

        // Check for error event
      }
    );

    it("Test BSP storage size can not be decreased below used", async () => {
      const source = "res/adolphus.jpg";
      const location = "test/adolphus.jpg";
      const bucketName = "nothingmuch-2";
      await api.sendNewStorageRequest(source, location, bucketName);

      // Wait for BSP to volunteer
      await sleep(500);
      await api.sealBlock();

      // Wait for file to be transferred and confirmed
      await sleep(5000);
      await api.sealBlock();

      // Skip block height past threshold
      await skipBlocksToMinChangeTime(api);

      const { events, extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(2n),
        bspKey
      );
      assert.strictEqual(extSuccess, false);
      const [eventInfo, _eventError] = fetchEventData(api.events.system.ExtrinsicFailed, events);
      assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
      assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0b000000"); // NewCapacityLessThanUsedStorage
    });
  });
}
